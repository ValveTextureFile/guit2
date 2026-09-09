use prost::Message;

use crate::{compiledprotos::{
    FenderMessageLt, ModalContext, ModalState, ModalStatusMessage, ResponseType, fender_message_lt,
}, report::FenderReport};

mod report;
mod device;
mod compiledprotos;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // goal 2: turn message into hid
    let msg = FenderMessageLt {
        response_type: ResponseType::Unsolicited.into(),
        r#type: Some(fender_message_lt::Type::ModalStatusMessage(
            ModalStatusMessage {
                context: ModalContext::SyncBegin.into(),
                state: ModalState::Ok.into(),
            },
        )),
    };

    let payload = msg.encode_to_vec();

    let hid_report = FenderReport {
        tag: report::HID_END_MP,
        len: payload.len() as u8, // ik ik im very tired and i want to go to bed
        payload: payload.clone(),
    };

    println!("{}", hid_report);
    println!("full: {:?}", hid_report.to_data()?);

    assert_eq!(hid_report.to_data()?.len(), 64, "check for hid_datalen = 64");
    assert_eq!(payload.len(), 9, "check for payloadlen = 9");
    Ok(())
}
