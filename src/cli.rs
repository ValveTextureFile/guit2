
/// every time guit2 runs, it has its own command line, no need for sync commands
pub enum Commands {
    GetFirmware,
    
    LoadPreset(u32),
    SavePreset(u32, String),
    ClearPreset(u32),

}