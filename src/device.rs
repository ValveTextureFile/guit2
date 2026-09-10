use std::{error::Error, fmt::Display, process::Output};

use hidapi::{DeviceInfo, HidApi, HidDevice, HidError};

use crate::report::FenderData;

pub const FENDER_VID: u16 = 0x1ed8;
pub const LT25_PID: u16 = 0x0037;
pub const LT25_INTR: i32 = 0;

pub fn new_api_instance() -> Result<HidApi, Box<dyn Error>> {
    let api = HidApi::new()?;
    Ok(api)
}
pub struct FenderDevice {
    device: HidDevice,
    pub vendor: u16,
    pub product: u16,
}

pub enum HIDSendType {
    Output,
    Feature,
}

struct OpenStatus {
    pub vid_true: bool,
    pub pid_true: bool,
    pub interface_true: bool,
}

impl Display for OpenStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Opening Status :: VID: {}  PID: {}  Interface: {}",
            self.vid_true, self.pid_true, self.interface_true
        )
    }
}

impl OpenStatus {
    pub fn new() -> Self {
        Self {
            vid_true: true,
            pid_true: true,
            interface_true: true,
        }
    }
}

impl FenderDevice {
    /// UNTESTED

    pub fn open(api: &HidApi) -> Result<Self, Box<dyn Error>> {
        for d in api.device_list() {
            let matches = d.vendor_id() == FENDER_VID
                && d.product_id() == LT25_PID
                && d.interface_number() == LT25_INTR;

            if !matches {
                continue;
            }

            println!(
                "found match: {:04x}:{:04x} @ interface {}",
                d.vendor_id(),
                d.product_id(),
                d.interface_number()
            );

            return Ok(Self {
                device: d.open_device(api)?,
                vendor: d.vendor_id(),
                product: d.product_id(),
            });
        }

        Err("Fender Mustang LT25 was not found".into())
    }

    pub fn send(&self, message: &FenderData, typ: HIDSendType) -> Result<(), Box<dyn Error>> {
        let mut _result;
        match typ {
            HIDSendType::Output => {
                _result = self.device.send_output_report(message)?;
            }
            HIDSendType::Feature => {
                _result = self.device.send_feature_report(message)?;
            }
        }

        Ok(_result)
    }

    /// returns data and bytes read
    pub fn receive(&self) -> Result<(FenderData, usize), Box<dyn Error>> {
        let mut recieved: FenderData = [0u8; 64];
        let recieved_bytes = self.device.read(&mut recieved)?;

        Ok((recieved.to_owned(), recieved_bytes))
    }
}
