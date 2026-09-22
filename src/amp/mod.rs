use std::ptr::read;

use crate::{DynamicError, amp::values::Preset, backend::client::FenderClient};

pub mod values;
pub mod dsp;

pub struct FenderAmp {
    client: FenderClient,
    pub firmware: Option<String>,
    pub current_preset: Option<Preset>,
    presets: Vec<Preset>
}

impl FenderAmp {
    pub fn connect(client: FenderClient) -> Result<Self, DynamicError> {
        let result = client.sync()?;

        let firmware = Some(result.firmware);
        let presets = result.presets;

        let current_preset = Some(client.get_preset(client.get_current_preset()?.current_slot_index)?);

        Ok(Self {
            firmware,
            presets, current_preset,
            client
        })        
    }

    pub fn load_preset(&self, preset: Preset) {
        todo!()
    }
}
