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