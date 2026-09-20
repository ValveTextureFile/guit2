use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DSPSlot {
    Amp,
    Stomp,
    Mod,
    Delay,
    Reverb,
}
pub type DSPParameters = HashMap<String, serde_json::Value>;

#[derive(Debug, Clone)]
pub struct DSPUnit {
    pub slot: DSPSlot,
    pub model: String,
    // pub enabled: bool,
    pub param: DSPParameters
}