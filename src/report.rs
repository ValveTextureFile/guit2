use std::fmt::Display;

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
        write!(f, "[{:02X}], [{:02X}], {}", self.tag, self.len, hex::encode(self.payload.clone()))
    }
}

impl FenderReport {

}