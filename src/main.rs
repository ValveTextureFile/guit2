use core::sync;
use std::error::Error;

use hidapi::HidApi;
use prost::Message;

use crate::{
    device::FenderDevice, fenderprotos::{
        FenderMessageLt, FirmwareVersionRequest, ModalContext, ModalState, ModalStatusMessage, ResponseType, fender_message_lt,
    }, protocol::LTMessage, report::{FenderReport, HID_END_MP},
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = device::new_api_instance()?;

    sync_begin_req(&api)?;

    let device = FenderDevice::open(&api)?;

    // send firmware request

    let fw_msg = LTMessage::firmware_version_request(FirmwareVersionRequest {
        request: true
    });

    let fw_payload = fw_msg.encode();

    let fw_report = FenderReport::new(HID_END_MP, fw_payload)?;

    let fw_data = fw_report.to_data()?;

    println!("sending outpute message to {:04x}:{:04x}", device.vendor, device.product);
    device.send(&fw_data, device::HIDSendType::Output)?;

    let (d, b) = device.receive()?;
    println!("from {:04x}:{:04x}", device.vendor, device.product);
    println!("data: {:?}", d);
    println!("bytes read: {}", b);
    let report = FenderReport::from_data(d)?;

    let response = protocol::decode(&report.payload)?;

    println!("{response:#?}");




    Ok(())
}
