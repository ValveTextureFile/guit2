use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, SystemTime},
};

use crate::{
    backend::device::{self, FenderDevice},
    backend::fenderprotos::{
        FenderMessageLt, FirmwareVersionRequest, ModalContext, ModalState, ResponseType,
        fender_message_lt::Type as fmltType,
    },
    backend::protocol::{self, LTMessage},
    backend::report::{FenderReport, HID_END_MP},
};

use crate::DynamicError;

pub struct FenderClient {
    tx: Sender<DeviceCommand>,
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

pub fn preform_send(device: &FenderDevice, msg: LTMessage) -> Result<(), DynamicError> {
    let report = FenderReport::new(HID_END_MP, msg.encode())?;

    device.higher_send(&report, device::HIDSendType::Output)?;

    Ok(())
}

pub fn preform_transact(
    device: &FenderDevice,
    msg: LTMessage,
) -> Result<FenderMessageLt, DynamicError> {
    let report = FenderReport::new(HID_END_MP, msg.encode())?;

    device.higher_send(&report, device::HIDSendType::Output)?;

    let response = protocol::decode(&device.higher_receive()?.payload)?;

    Ok(response)
}

fn device_worker(device: FenderDevice, rx: Receiver<DeviceCommand>) {
    loop {
        let hb_time = SystemTime::now();
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
                if SystemTime::now()
                    .duration_since(hb_time)
                    .expect("Unable to get duration since HB_Timer.start")
                    >= Duration::from_secs(1)
                {
                    let _ = preform_send(
                        &device,
                        LTMessage {
                            msg: LTMessage::heartbeat().into_inner(),
                        },
                    );
                }
            }

            Err(mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
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

    pub fn hearbeat(&self) -> Result<(), DynamicError> {
        self.send(LTMessage::heartbeat())
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
