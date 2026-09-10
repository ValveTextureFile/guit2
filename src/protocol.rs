use std::error::Error;

use prost::Message;

use crate::{fenderprotos::FenderMessageLt, report::FenderData};

pub mod messages {
    use crate::fenderprotos::{
        FenderMessageLt, Heartbeat, ModalContext, ModalState, ModalStatusMessage, ResponseType,
        fender_message_lt,
    };
    pub fn sync_begin() -> FenderMessageLt {
        FenderMessageLt {
            response_type: ResponseType::Unsolicited.into(),
            r#type: Some(fender_message_lt::Type::ModalStatusMessage(
                ModalStatusMessage {
                    context: ModalContext::SyncBegin.into(),
                    state: ModalState::Ok.into(),
                },
            )),
        }
    }

    pub fn sync_end() -> FenderMessageLt {
        FenderMessageLt {
            response_type: ResponseType::Unsolicited.into(),
            r#type: Some(fender_message_lt::Type::ModalStatusMessage(
                ModalStatusMessage {
                    context: ModalContext::SyncEnd.into(),
                    state: ModalState::Ok.into(),
                },
            )),
        }
    }

    pub fn heartbeat() -> FenderMessageLt {
        FenderMessageLt {
            response_type: ResponseType::Unsolicited.into(),
            r#type: Some(fender_message_lt::Type::Heartbeat(Heartbeat {
                ..Default::default()
            })),
        }
    }
}

pub fn encode(fmlt: &FenderMessageLt) -> Result<FenderData, Box<dyn Error>>{
    let enc: FenderData = fmlt.encode_to_vec().try_into().unwrap();
    Ok(enc)
}

pub fn decode(payload: &[u8]) -> Result<FenderMessageLt, prost::DecodeError> {
    FenderMessageLt::decode(payload)
}
