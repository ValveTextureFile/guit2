pub mod values;

use values::{FenderSettings, FenderModSettings};

use crate::{DynamicError, backend::client::FenderClient};

pub struct FenderPreset {
    is_current: bool,
    pub name: String,
    pub settings: FenderSettings,
}

pub struct FenderAmp {
    profiles: Vec<FenderPreset>,
    pub current_profile: FenderPreset,
    pub current_settings: FenderSettings,
}

impl FenderAmp {
    pub fn get_current(client: &FenderClient) -> Result<Self, DynamicError> {
        
        todo!()
    }
}