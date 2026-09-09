use std::{fmt::Display, io::Write};

pub const HID_START_MP: u8 = 0x33;
pub const HID_CONT_MP: u8 = 0x34;
pub const HID_END_MP: u8 = 0x35;

#[derive(Debug, PartialEq, Eq)]
pub struct FenderReport {
    pub tag: u8,
    pub len: u8,
    pub payload: Vec<u8>,
}

impl Display for FenderReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[0x{:02X}], [0x{:02X}], {}",
            self.tag,
            self.len,
            hex::encode(self.payload.clone())
        )
    }
}

pub type FenderData = [u8; 64];

impl FenderReport {
    pub fn to_data(&self) -> Result<FenderData, Box<dyn std::error::Error>> {
        // check payload b4 anything else
        if self.payload.len() > 61 {
            return Err("payload made exceeds max payload size of 61 bytes.".into())
        }

        // 0 data + make writer
        let mut data: FenderData = [0u8;64];
        let mut writer = &mut data[..];

        writer.write_all(&[self.tag])?; // 63 bytes left...
        writer.write_all(&[self.len])?; // 62 bytes left...
        writer.write_all(&self.payload)?; // 61 allowed for payload

        Ok(data)
    }
}
