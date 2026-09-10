use std::{error::Error, fmt::Display};

use hidapi::{HidApi, HidDevice};

pub const FENDER_VID: u16 = 0x1ed8;
pub const LT25_PID: u16 = 0x0037;
pub const LT25_INTR: i32 = 0;

fn new_api_instance() -> Result<HidApi, Box<dyn Error>> {
    let api = HidApi::new()?;
    Ok(api)
}

pub struct FenderDevice {
    device: HidDevice,
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
            "Opening Status ::\nVID: {}\nPID: {}\nInterface: {}",
            self.vid_true, self.pid_true, self.interface_true
        )
    }
}

impl OpenStatus {
    pub fn new() -> Self {
        Self { vid_true: true, pid_true: true, interface_true: true }
    }
}

impl FenderDevice {
    pub fn open(api: &HidApi) -> Result<Self, Box<dyn Error>> {
        let mut status: OpenStatus = OpenStatus::new();
        for d in api.device_list() {
            // check vendor
            if !d.vendor_id().eq(&FENDER_VID) {
                status.vid_true = false;
            }

            // check product
            if !d.product_id().eq(&LT25_PID) {
                status.pid_true = false;
            }

            // check interface todo fix
            if !d.product_id().eq(&LT25_INTR) {
                status.interface_true = false;
            }


        }

        todo!()
    }
}
