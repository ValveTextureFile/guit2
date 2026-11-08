use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

use serde_json::Value;

use crate::{
    amp::values::Preset,
    backend::{
        device::{self, FenderDevice},
        fenderprotos::{
            CurrentPresetRequest, CurrentPresetStatus, FenderMessageLt, FirmwareVersionRequest,
            LineOutGainRequest, LineOutGainSet, ProcessorUtilization, ProcessorUtilizationRequest,
            UsbGainRequest, UsbGainSet,
            LoadPreset, ModalContext, ModalState, ReplaceNode, ResponseType, RetrievePreset, SaveCurrentPreset,
            SaveCurrentPresetTo, SavePresetAs,
            SetDspUnitParameter, dsp_unit_parameter_status, set_dsp_unit_parameter,
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

/// reads one (possibly multi-packet) message. returns None if nothing started arriving within `timeout_ms`.
fn receive_message(
    device: &FenderDevice,
    timeout_ms: i32,
) -> Result<Option<FenderMessageLt>, DynamicError> {
    let mut payload = Vec::new();
    let mut timeout = timeout_ms;

    loop {
        let Some(report) = device.higher_receive(timeout)? else {
            if payload.is_empty() {
                return Ok(None);
            }
            return Err("timed out in the middle of a multi-packet message".into());
        };

        payload.extend_from_slice(&report.payload);

        match report.tag {
            HID_END_MP => break,

            // rest of the message should follow right away
            HID_START_MP | HID_CONT_MP => timeout = 1000,

            tag => {
                return Err(format!("unknown HID packet tag: {tag:#04x}").into());
            }
        }
    }

    Ok(Some(protocol::decode(&payload)?))
}

pub fn preform_send(device: &FenderDevice, msg: LTMessage) -> Result<(), DynamicError> {
    let report = FenderReport::new(HID_END_MP, msg.encode())?;

    device.higher_send(&report, device::HIDSendType::Output)?;

    Ok(())
}

/// sends `msg` and returns the IS_LAST_ACK reply. anything else that arrives meanwhile
/// (NOT_LAST_ACK parts of the reply, unsolicited knob/preset changes) goes to `events`.
pub fn preform_transact(
    device: &FenderDevice,
    msg: LTMessage,
    events: &Sender<FenderMessageLt>,
) -> Result<FenderMessageLt, DynamicError> {
    preform_send(device, msg)?;

    let deadline = Instant::now() + Duration::from_secs(2);

    while Instant::now() < deadline {
        let Some(reply) = receive_message(device, 100)? else {
            continue;
        };

        if reply.response_type == ResponseType::IsLastAck as i32 {
            return Ok(reply);
        }

        let _ = events.send(reply);
    }

    Err("timed out waiting for the amp to reply".into())
}

fn device_worker(device: FenderDevice, rx: Receiver<DeviceCommand>, events: Sender<FenderMessageLt>) {
    let mut hb_time = Instant::now();
    loop {
        match rx.recv_timeout(Duration::from_millis(10)) {
            Ok(DeviceCommand::Transact { message, response }) => {
                let result = preform_transact(&device, message, &events)
                    .map_err(|error| error.to_string());

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
                // nothing to send, listen for the amp telling us about physical changes
                match receive_message(&device, 10) {
                    Ok(Some(msg)) => {
                        let _ = events.send(msg);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        // most likely unplugged. dropping `events` tells the owner we're gone.
                        eprintln!("amp read failed, stopping worker: {e}");
                        break;
                    }
                }
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
    /// also returns the receiver for messages the amp sends on its own (physical knob/preset changes).
    pub fn new(device: FenderDevice) -> (Self, Receiver<FenderMessageLt>) {
        let (tx, rx) = mpsc::channel();
        let (events_tx, events_rx) = mpsc::channel();

        thread::spawn(move || {
            device_worker(device, rx, events_tx);
        });

        (Self { tx }, events_rx)
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

        for index in 1..=60 {
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
                Preset::from_data(&m.data)
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


    pub fn set_dsp_param(
        &self,
        node_id: &str,
        parameter_id: &str,
        value: &Value,
    ) -> Result<(), DynamicError> {
        self.transact(LTMessage::set_dsp_unit_parameter(SetDspUnitParameter {
            node_id: node_id.into(),
            parameter_id: parameter_id.into(),
            r#type: Some(param_to_proto(value)?),
        }))?;

        Ok(())
    }

    pub fn replace_node(&self, node_id: &str, fender_id: &str) -> Result<(), DynamicError> {
        self.transact(LTMessage::replace_node(ReplaceNode {
            node_id_to_replace: node_id.into(),
            fender_id_to_replace_with: fender_id.into(),
        }))?;

        Ok(())
    }

    pub fn get_usb_gain(&self) -> Result<f32, DynamicError> {
        match self.transact(LTMessage::usb_gain_request(UsbGainRequest { request: true }))?.r#type {
            Some(fmltType::UsbGainStatus(s)) => Ok(s.value_db),
            other => Err(format!("unexpected response to UsbGainRequest: {other:?}").into()),
        }
    }

    pub fn set_usb_gain(&self, value_db: f32) -> Result<(), DynamicError> {
        self.transact(LTMessage::usb_gain_set(UsbGainSet { value_db }))?;
        Ok(())
    }

    pub fn get_line_out_gain(&self) -> Result<f32, DynamicError> {
        match self.transact(LTMessage::line_out_gain_request(LineOutGainRequest { request: true }))?.r#type {
            Some(fmltType::LineOutGainStatus(s)) => Ok(s.value_db),
            other => Err(format!("unexpected response to LineOutGainRequest: {other:?}").into()),
        }
    }

    pub fn set_line_out_gain(&self, value_db: f32) -> Result<(), DynamicError> {
        self.transact(LTMessage::line_out_gain_set(LineOutGainSet { value_db }))?;
        Ok(())
    }

    pub fn get_processor_utilization(&self) -> Result<ProcessorUtilization, DynamicError> {
        let msg = LTMessage::processor_utilization_request(ProcessorUtilizationRequest { request: true });
        match self.transact(msg)?.r#type {
            Some(fmltType::ProcessorUtilization(p)) => Ok(p),
            other => Err(format!("unexpected response to ProcessorUtilizationRequest: {other:?}").into()),
        }
    }

    pub fn save_preset_as(&self, data: &str, slot: i32) -> Result<(), DynamicError> {
        if !(1..=60).contains(&slot) {
            return Err("slot must be between 1 and 60".into());
        }

        self.transact(LTMessage::save_preset_as(SavePresetAs {
            preset_data: data.into(),
            is_load_preset: true,
            preset_slot: slot,
        }))?;

        Ok(())
    }

    pub fn save_current_preset_to(&self, name: &str, slot: i32) -> Result<(), DynamicError> {
        if !(1..=60).contains(&slot) {
            return Err("slot must be between 1 and 60".into());
        }

        self.transact(LTMessage::save_current_preset_to(SaveCurrentPresetTo {
            preset_name: name.into(),
            preset_slot: slot,
        }))?;

        Ok(())
    }

    /// writes the current (edited) preset into its slot on the amp.
    pub fn save_current_preset(&self) -> Result<(), DynamicError> {
        self.transact(LTMessage::save_current_preset(SaveCurrentPreset { save: true }))?;

        Ok(())
    }
}

/// shortcut: whole JSON numbers are sent as sint32, fractional ones as float. if the amp ever
/// stores a float param as e.g. `1` this sends the wrong type; upgrade to a per-model param table.
pub fn param_to_proto(value: &Value) -> Result<set_dsp_unit_parameter::Type, DynamicError> {
    use set_dsp_unit_parameter::Type;

    Ok(match value {
        Value::Bool(b) => Type::BoolParameter(*b),
        Value::String(s) => Type::StringParameter(s.clone()),
        Value::Number(n) if n.is_i64() => Type::Sint32Parameter(n.as_i64().unwrap() as i32),
        Value::Number(n) => Type::FloatParameter(n.as_f64().unwrap_or_default() as f32),
        other => return Err(format!("can't send parameter value {other}").into()),
    })
}

pub fn param_from_proto(value: dsp_unit_parameter_status::Type) -> Value {
    use dsp_unit_parameter_status::Type;

    match value {
        Type::BoolParameter(b) => Value::Bool(b),
        Type::StringParameter(s) => Value::String(s),
        Type::Sint32Parameter(i) => Value::from(i),
        Type::FloatParameter(f) => Value::from(f as f64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_round_trip() {
        use dsp_unit_parameter_status::Type as S;
        use set_dsp_unit_parameter::Type as T;

        for (json, sent, status) in [
            (Value::Bool(true), T::BoolParameter(true), S::BoolParameter(true)),
            (Value::from(3), T::Sint32Parameter(3), S::Sint32Parameter(3)),
            (Value::from(0.5), T::FloatParameter(0.5), S::FloatParameter(0.5)),
            (Value::from("jack"), T::StringParameter("jack".into()), S::StringParameter("jack".into())),
        ] {
            assert_eq!(param_to_proto(&json).unwrap(), sent);
            assert_eq!(param_from_proto(status), json);
        }
        assert!(param_to_proto(&Value::Null).is_err());
    }
}
