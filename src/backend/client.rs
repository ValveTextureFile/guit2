use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant, SystemTime},
};

use serde::de::value::BorrowedStrDeserializer;

use crate::{
    amp::values::Preset,
    backend::{
        device::{self, FenderDevice},
        fenderprotos::{
            ActiveDisplay, CurrentDisplayedPresetIndexStatus, CurrentPresetRequest,
            CurrentPresetStatus, FenderMessageLt, FirmwareVersionRequest, LoadPreset, ModalContext,
            ModalState, ResponseType, RetrievePreset, ShiftPreset,
            fender_message_lt::Type as fmltType,
        },
        protocol::{self, LTMessage},
        report::{FenderReport, HID_CONT_MP, HID_END_MP, HID_START_MP},
    },
};

use crate::DynamicError;

pub struct FenderClient {
    tx: Sender<DeviceCommand>,
}

pub struct SyncResult {
    pub firmware: String,
    pub presets: Vec<Preset>,
}

pub enum DeviceCommand {
    Transact {
        message: LTMessage,
        response: Sender<Result<FenderMessageLt, String>>,
    },

    Send {
        message: LTMessage,
        channel_reply: Sender<Result<(), String>>,
    },

    Shutdown,
}

fn receive_message(device: &FenderDevice) -> Result<Vec<u8>, DynamicError> {
    let mut payload = Vec::new();

    loop {
        let report = device.higher_receive()?;

        payload.extend_from_slice(&report.payload);

        match report.tag {
            HID_END_MP => break,

            HID_START_MP | HID_CONT_MP => continue,

            tag => {
                return Err(format!("unknown HID packet tag: {tag:#04x}").into());
            }
        }
    }

    Ok(payload)
}

pub fn preform_send(device: &FenderDevice, msg: LTMessage) -> Result<(), DynamicError> {
    let report = FenderReport::new(HID_END_MP, msg.encode())?;

    device.higher_send(&report, device::HIDSendType::Output)?;

    Ok(())
}

pub fn preform_transact(
    device: &FenderDevice,
    msg: LTMessage,
) -> Result<FenderMessageLt, DynamicError> {
    preform_send(device, msg)?;

    let pl = receive_message(device)?;

    Ok(protocol::decode(&pl)?)
}

fn device_worker(device: FenderDevice, rx: Receiver<DeviceCommand>) {
    let mut hb_time = Instant::now();
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(DeviceCommand::Transact { message, response }) => {
                let result = preform_transact(&device, message).map_err(|error| error.to_string());

                let _ = response.send(result);
            }

            Ok(DeviceCommand::Send {
                message,
                channel_reply,
            }) => {
                let result = preform_send(&device, message).map_err(|error| error.to_string());

                let _ = channel_reply.send(result);
            }

            Ok(DeviceCommand::Shutdown) => {
                break;
            }

            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Nothing arrived. Check if it's time to send a heartbeat.
            }

            Err(mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
        if hb_time.elapsed() >= Duration::from_secs(1) {
            if let Err(e) = preform_send(&device, LTMessage::heartbeat()) {
                eprintln!("heartbeat failed: {e}");
            }
            hb_time = Instant::now();
        }
    }
}

impl FenderClient {
    pub fn new(device: FenderDevice) -> Self {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            device_worker(device, rx);
        });

        Self { tx }
    }

    pub fn fuckoff(&self) {
        let _ = self.tx.send(DeviceCommand::Shutdown);
    }

    pub fn send(&self, message: LTMessage) -> Result<(), DynamicError> {
        let (reply_tx, reply_rx) = mpsc::channel();

        self.tx.send(DeviceCommand::Send {
            message,
            channel_reply: reply_tx,
        })?;

        let result = reply_rx.recv()?;

        result.map_err(|error| std::io::Error::other(error).into())
    }

    pub fn transact(&self, msg: LTMessage) -> Result<FenderMessageLt, DynamicError> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();
        self.tx.send(DeviceCommand::Transact {
            message: msg,
            response: response_tx,
        })?;
        match response_rx.recv() {
            Ok(result) => result.map_err(|e| e.into()),
            Err(e) => Err(format!("Failed to receive response: {}", e).into()),
        }
    }

    pub fn sync_begin(&self) -> Result<(), DynamicError> {
        let response = self.transact(LTMessage::sync_begin())?;

        match response.r#type.as_ref() {
            Some(fmltType::ModalStatusMessage(status))
                if response.response_type == ResponseType::IsLastAck as i32
                    && status.context == ModalContext::SyncBegin as i32
                    && status.state == ModalState::Ok as i32 =>
            {
                Ok(())
            }

            _ => Err(format!("unexpected response to SYNC_BEGIN:\n{response:#?}").into()),
        }
    }

    pub fn sync_end(&self) -> Result<(), DynamicError> {
        let response = self.transact(LTMessage::sync_end())?;

        match response.r#type.as_ref() {
            Some(fmltType::ModalStatusMessage(status))
                if response.response_type == ResponseType::IsLastAck as i32
                    && status.context == ModalContext::SyncEnd as i32
                    && status.state == ModalState::Ok as i32 =>
            {
                Ok(())
            }

            _ => Err(format!("unexpected response to SYNC_BEGIN:\n{response:#?}").into()),
        }
    }

    pub fn get_firmware(&self) -> Result<String, DynamicError> {
        let response = self.transact(LTMessage::firmware_version_request(
            FirmwareVersionRequest { request: true },
        ))?;

        match response.r#type.as_ref() {
            Some(fmltType::FirmwareVersionStatus(s))
                if response.response_type == ResponseType::IsLastAck as i32
                    && !&s.version.is_empty() =>
            {
                Ok(s.version.clone())
            }
            _ => {
                Err(format!("unexpected response to FirmwareVersionRequest:\n{response:#?}").into())
            }
        }
    }

    pub fn sync(&self) -> Result<SyncResult, DynamicError> {
        self.sync_begin()?;

        let firmware = self.get_firmware()?;

        let mut presets = Vec::new();

        for index in 1..60 {
            presets.push(self.get_preset(index)?);
        }

        self.sync_end()?;

        Ok(SyncResult { firmware, presets })
    }

    pub fn hearbeat(&self) -> Result<(), DynamicError> {
        self.send(LTMessage::heartbeat())
    }

    pub fn get_preset(&self, id: i32) -> Result<Preset, DynamicError> {
        // just transacts with the device and returns the preset data
        let response = self.transact(LTMessage::retrieve_preset(RetrievePreset { slot: id }))?;

        match response.r#type.as_ref() {
            Some(fmltType::PresetJsonMessage(m))
                if response.response_type == ResponseType::IsLastAck as i32
                    && !m.data.is_empty()
                    && m.slot_index == id =>
            {
                let ds_json: Preset = serde_json::from_str(&m.data)?;
                println!("got data! ~> {}", serde_json::to_string_pretty(&ds_json)?);
                Ok(ds_json)
            }

            _ => Err(format!("unexpected response to RetrivePreset:\n{response:?}").into()),
        }
    }

    pub fn get_current_preset(&self) -> Result<CurrentPresetStatus, DynamicError> {
        let response = self.transact(LTMessage::current_preset_request(CurrentPresetRequest {
            request: true,
        }))?;

        match response.r#type {
            Some(fmltType::CurrentPresetStatus(s))
                if response.response_type == ResponseType::IsLastAck as i32 =>
            {
                Ok(s)
            }
            _ => Err(format!("unexpected response to CurrentPresetRequest:\n{response:#?}").into()),
        }
    }

    pub fn set_preset_by_index(&self, id: i32) -> Result<(), DynamicError> {
        if id > 60 || id < 1 {
           return Err("the id to switch to must be between the values of 1 and 60".into()) 
        }

        let response = self.transact(LTMessage::load_preset(LoadPreset { preset_index: id }))?;

        match response.r#type {
            Some(fmltType::CurrentLoadedPresetIndexStatus(s))
                if response.response_type == ResponseType::IsLastAck as i32
                && s.current_loaded_preset_index == id => {
                    Ok(())
                }

            _ => Err(format!("unexpected response to LoadPreset:\n{response:#?}").into())
        }
    }
}

// impl FenderClient {
//     pub fn new(device: FenderDevice) -> Self {
//         Self {
//             device,
//             last_message: None,
//         }
//     }

//     pub fn send(&mut self, msg: LTMessage) -> Result<(), DynamicError> {
//         let message = format!("{msg:?}");
//         let report = FenderReport::new(HID_END_MP, msg.encode())?;

//         self.device
//             .higher_send(&report, crate::device::HIDSendType::Output)?;
//         self.last_message = Some(message);

//         Ok(())
//     }
//     pub fn transact(&mut self, msg: LTMessage) -> Result<FenderMessageLt, DynamicError> {
//         let message = format!("{msg:?}");
//         let outreport = FenderReport::new(HID_END_MP, msg.encode())?;

//         println!("TX {message}");
//         self.device
//             .higher_send(&outreport, crate::device::HIDSendType::Output)?;
//         self.last_message = Some(message);
//         println!("waiting on RX...");
//         let inreport = self.device.higher_receive()?;

//         let response = protocol::decode(&inreport.payload)?;
//         println!("RX: {response:?}");

//         Ok(response)
//     }

//     // technical

//     pub fn sync_begin(&mut self) -> Result<(), DynamicError> {
//         let response = self.transact(LTMessage::sync_begin())?;

//         match response.r#type.as_ref() {
//             Some(fmltType::ModalStatusMessage(status))
//                 if response.response_type == ResponseType::IsLastAck as i32
//                     && status.context == ModalContext::SyncBegin as i32
//                     && status.state == ModalState::Ok as i32 =>
//             {
//                 Ok(())
//             }

//             _ => Err(format!("unexpected response to SYNC_BEGIN:\n{response:#?}").into()),
//         }
//     }

//     pub fn sync_end(&mut self) -> Result<(), DynamicError> {
//         let response = self.transact(LTMessage::sync_end())?;

//         match response.r#type.as_ref() {
//             Some(fmltType::ModalStatusMessage(status))
//                 if response.response_type == ResponseType::IsLastAck as i32
//                     && status.context == ModalContext::SyncEnd as i32
//                     && status.state == ModalState::Ok as i32 =>
//             {
//                 Ok(())
//             }

//             _ => Err(format!("unexpected response to SYNC_BEGIN:\n{response:#?}").into()),
//         }
//     }

//     pub fn get_firmware(&mut self) -> Result<String, DynamicError> {
//         let response = self.transact(LTMessage::firmware_version_request(
//             FirmwareVersionRequest { request: true },
//         ))?;

//         match response.r#type.as_ref() {
//             Some(fmltType::FirmwareVersionStatus(s))
//                 if response.response_type == ResponseType::IsLastAck as i32
//                     && !&s.version.is_empty() =>
//             {
//                 Ok(s.version.clone())
//             }
//             _ => {
//                 Err(format!("unexpected response to FirmwareVersionRequest:\n{response:#?}").into())
//             }
//         }
//     }

//     pub fn hearbeat(&mut self) -> Result<(), DynamicError> {
//         self.send(LTMessage::heartbeat())
//     }
// }

#[cfg(test)]
mod tests {

    #[test]
    fn ensure_sync_begin() {}
}
