use std::error::Error;

use hidapi::HidApi;
use prost::Message;

use crate::{
    device::FenderDevice,
    protocol::LTMessage,
    report::{FenderReport, HID_END_MP},
};

mod device;
mod fenderprotos;
mod protocol;
mod report;

fn sync_begin_req(api: &HidApi) -> Result<(), Box<dyn Error>> {
    let device = FenderDevice::open(&api)?;

    let message = LTMessage::sync_begin();

    let payload = message.encode();

    let report = FenderReport::new(HID_END_MP, payload)?;

    let data = report.to_data()?;

    println!(
        "sending outpute message to {}:{}",
        device.vendor, device.product
    );
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let device = FenderDevice::open(&device::new_api_instance()?)?;

    device.higher_send(
        &FenderReport::new(HID_END_MP, protocol::LTMessage::sync_begin().encode())?,
        device::HIDSendType::Output,
    )?;

    let return_report = device.higher_receive()?;
    let repsonse = protocol::decode(&return_report.payload)?;
    println!("{repsonse:#?}");


    Ok(())
}
