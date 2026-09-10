use core::sync;

use hidapi::HidApi;
use prost::Message;

use crate::{
    device::FenderDevice, fenderprotos::{
        FenderMessageLt, ModalContext, ModalState, ModalStatusMessage, ResponseType,
        fender_message_lt,
    }, protocol::LTMessage, report::{FenderReport, HID_END_MP},
};

mod device;
mod fenderprotos;
mod protocol;
mod report;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = device::new_api_instance()?;

    let device = FenderDevice::open(&api)?;

    let message = LTMessage::sync_begin();

    let payload = message.encode();

    let report = FenderReport::new(HID_END_MP, payload)?;

    let data = report.to_data()?;

    println!("sending outpute message to {}:{}", device.vendor, device.product);
    device.send(&data, device::HIDSendType::Output)?;

    let (respone_data, respone_bytes) = device.receive()?;
    println!("from {}:{}", device.vendor, device.product);
    println!("data: {:?}", respone_data);
    println!("bytes read: {}", respone_bytes);
    let response_report = FenderReport::from_data(respone_data)?;

    let response = protocol::decode(&response_report.payload)?;

    println!("{response:#?}");

    Ok(())
}
