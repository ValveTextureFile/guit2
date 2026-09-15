use std::{error::Error, sync::mpsc::Sender};

use crate::{
    device::FenderDevice,
    fenderprotos::{
        FenderMessageLt, FirmwareVersionRequest, ModalContext, ModalState, ResponseType,
        fender_message_lt::Type as fmltType,
    },
    protocol::{self, LTMessage},
    report::{FenderReport, HID_END_MP},
};

use super::DynamicError;

pub struct FenderClient {
    tx: Sender<DeviceCommand>
}

pub enum DeviceCommand {
    Transact {
        message: LTMessage,
        response: Sender<Result<FenderMessageLt, String>>
    },

    Send {
        message: LTMessage
    },

    Shutdown
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
    fn ensure_sync_begin() {
        
    }
}
