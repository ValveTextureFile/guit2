use core::sync;

use hidapi::HidApi;
use prost::Message;

use crate::{fenderprotos::{
    FenderMessageLt, ModalContext, ModalState, ModalStatusMessage, ResponseType, fender_message_lt,
}, report::FenderReport};

mod report;
mod device;
mod protocol;
mod fenderprotos;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let api = device::new_api_instance()?;

    let sync_begin = protocol::messages::sync_begin();
    let sync_begin_report = report::FenderReport::from_data(protocol::encode(&sync_begin)?)?;

    let lt25 = device::FenderDevice::open(&api)?;

    let status = fende





    Ok(())
}
