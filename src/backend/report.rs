//! Encoding and decoding of Fender Mustang LT HID reports.
//!
//! A logical report consists of a packet [`tag`](FenderReport::tag), a payload
//! length, and up to 61 bytes of [`payload`](FenderReport::payload). The tag
//! indicates whether the packet starts, continues, or ends a multi-packet
//! message. [`HID_END_MP`] is also used when the complete message fits in one
//! packet.
//!
//! Input buffers passed to [`FenderReport::from_data`] include the zero HID
//! report-ID prefix (`[0, tag, length, payload, ...]`). Buffers produced by
//! [`FenderReport::to_data`] begin directly with the protocol fields
//! (`[tag, length, payload, ...]`).

use super::DynamicError;
use std::{fmt::Display, io::Write};

/// Fender HID protocol for starting a multi-packet report.
/// 
/// Mostly unused until i find something.
#[allow(unused)]
pub const HID_START_MP: u8 = 0x33;
/// Fender HID protocol for continuing a multi-packet report.
/// 
/// Mostly unused until i find something.
#[allow(unused)]
pub const HID_CONT_MP: u8 = 0x34;
/// Fender HID protocol for ending a multi-packet report or making a single packet report.
/// 
/// __Mostly used.__
pub const HID_END_MP: u8 = 0x35;

#[derive(Debug, PartialEq, Eq)]
/// One packet in the Fender Mustang LT HID protocol.
///
/// Use [`FenderReport::new`] when constructing a report from untrusted or
/// variable-length data so the protocol's payload limit is checked.
pub struct FenderReport {
    /// Packet framing tag, normally one of [`HID_START_MP`], [`HID_CONT_MP`],
    /// or [`HID_END_MP`].
    pub tag: u8,
    /// Protocol payload, limited to 61 bytes in a valid report.
    pub payload: Vec<u8>,
}

impl Display for FenderReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[0x{:02X}], [0x{:02X}], {}",
            self.tag,
            self.len(),
            hex::encode(&self.payload)
        )
    }
}

/// The fixed-size buffer used to exchange reports with the HID device.
pub type FenderData = [u8; 64];

impl FenderReport {
    /// Creates a report with the supplied framing `tag` and `payload`.
    ///
    /// # Errors
    ///
    /// Returns an error if `payload` contains more than 61 bytes.
    pub fn new(tag: u8, payload: Vec<u8>) -> Result<Self, DynamicError> {
        if payload.len() > 61 {
            return Err("payload made exceeds max payload size of 61 bytes.".into());
        }

        Ok(Self { tag, payload })
    }

    /// Parses a report from a 64-byte input buffer.
    ///
    /// The expected layout is `[0, tag, length, payload, ...]`, where the
    /// initial zero is the HID report ID and unused bytes are padding.
    ///
    /// # Errors
    ///
    /// Returns an error if the report-ID prefix is not zero or if the encoded
    /// payload length is greater than 61 bytes.
    pub fn from_data(data: FenderData) -> Result<Self, DynamicError> {
        if data[0] != 0 {
            return Err("invalid Fender input report prefix".into());
        }

        let tag = data[1];
        let len = data[2] as usize;

        if len > 61 {
            return Err("Fender payload len exceeds 61 bytes".into());
        }
        let payload = data[3..3 + len].to_vec();

        Ok(Self { tag, payload })
    }

    /// Returns the payload length encoded in the report header.
    ///
    /// A report created with [`Self::new`] or [`Self::from_data`] always has a
    /// length of at most 61. Directly constructing [`FenderReport`] can bypass
    /// that validation, in which case this conversion truncates lengths above
    /// 255.
    pub fn len(&self) -> u8 {
        self.payload.len() as u8
    }

    /// Encodes this report into a zero-padded, fixed-size output buffer.
    ///
    /// The output layout is `[tag, length, payload, ...]`. Unlike the input
    /// accepted by [`Self::from_data`], it does not contain a leading HID
    /// report-ID byte.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the payload does not fit in the 64-byte buffer.
    /// Construct reports with [`Self::new`] to enforce the 61-byte protocol
    /// payload limit before encoding.
    pub fn to_data(&self) -> Result<FenderData, DynamicError> {
        // 0 data + make writer
        let mut data: FenderData = [0u8; 64];
        let mut writer = &mut data[..];

        writer.write_all(&[self.tag])?; // 63 bytes left...
        writer.write_all(&[self.len()])?; // 62 bytes left...
        writer.write_all(&self.payload)?; // 61 allowed for payload

        Ok(data)
    }
}
