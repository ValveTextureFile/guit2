use std::{
    collections::VecDeque,
    fs::{File, TryLockError},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, MutexGuard,
        mpsc::{Receiver, RecvTimeoutError, TryRecvError},
    },
    time::{Duration, Instant},
};

use serde_json::Value;

use crate::{
    DynamicError,
    amp::values::Preset,
    backend::{
        client::{self, FenderClient},
        device::{self, FenderDevice},
        fenderprotos::{FenderMessageLt, fender_message_lt::Type as fmltType},
    },
};

pub mod dsp;
pub mod values;

/// everything the gui draws. written by the amp thread, read (and optimistically edited) by the gui.
#[derive(Default)]
pub struct AmpState {
    pub connected: bool,
    pub status: String,
    pub firmware: String,
    /// index 0 is slot 1
    pub presets: Vec<Preset>,
    pub current: Option<Preset>,
    pub current_slot: i32,
    pub dirty: bool,
    pub usb_gain: Option<f32>,
    pub line_out_gain: Option<f32>,
    pub dsp_load: VecDeque<f32>,
}

const DSP_HISTORY: usize = 120;
const DSP_POLL: Duration = Duration::from_millis(500);

pub enum UiCmd {
    Connect,
    Load(i32),
    /// the gui already wrote `value` into `AmpState::current`, this pushes it to the amp.
    SetParam {
        node: String,
        param: String,
        value: Value,
    },
    Save,
    ReplaceNode { node: String, model: String },
    SaveAs { name: String, slot: i32 },
    SetUsbGain(f32),
    SetLineOutGain(f32),
    Import { data: String, slot: i32 },
}

pub struct FenderAmp {
    client: FenderClient,
    events: Receiver<FenderMessageLt>,
    state: Arc<Mutex<AmpState>>,
    _lock: File,
}

fn single_instance_lock(dir: &Path) -> Result<File, DynamicError> {
    let file = File::create(dir.join("guit2.lock"))?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => {
            Err("guit2 is already running in another window. close it, then press Connect".into())
        }
        Err(TryLockError::Error(e)) => Err(e.into()),
    }
}

impl FenderAmp {
    pub fn connect(state: Arc<Mutex<AmpState>>) -> Result<Self, DynamicError> {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
        let lock = single_instance_lock(&runtime)?;
        let device = FenderDevice::open(&device::new_api_instance()?).map_err(|e| {
            format!("{e} (plugged in? without root you need the udev rule from the readme)")
        })?;
        let (client, events) = FenderClient::new(device);
        let result = client.sync()?;

        let amp = Self {
            client,
            events,
            state,
            _lock: lock,
        };

        {
            let mut s = amp.state();
            s.firmware = result.firmware;
            s.presets = result.presets;
            s.connected = true;
            s.status = "connected".into();
        }

        amp.refresh_current()?;

        {
            let usb = amp.client.get_usb_gain().ok();
            let line_out = amp.client.get_line_out_gain().ok();
            let mut s = amp.state();
            s.usb_gain = usb;
            s.line_out_gain = line_out;
            s.dsp_load.clear();
        }

        Ok(amp)
    }

    fn state(&self) -> MutexGuard<'_, AmpState> {
        self.state.lock().unwrap()
    }

    fn refresh_current(&self) -> Result<(), DynamicError> {
        let current = self.client.get_current_preset()?;
        let preset = Preset::from_data(&current.current_preset_data)?;

        let mut s = self.state();
        s.current_slot = current.current_slot_index;
        s.dirty = current.current_preset_dirty_status;
        s.current = Some(preset);

        Ok(())
    }

    fn refresh_slot(&self, slot: i32) -> Result<(), DynamicError> {
        let preset = self.client.get_preset(slot)?;

        if let Some(p) = self.state().presets.get_mut(slot as usize - 1) {
            *p = preset;
        }

        Ok(())
    }

    pub fn handle(&self, cmd: UiCmd) -> Result<(), DynamicError> {
        match cmd {
            UiCmd::Connect => Ok(()),

            UiCmd::Load(slot) => {
                self.client.set_preset_by_index(slot)?;
                self.refresh_current()
            }

            UiCmd::SetParam { node, param, value } => {
                self.client.set_dsp_param(&node, &param, &value)?;
                self.state().dirty = true;
                Ok(())
            }

            UiCmd::ReplaceNode { node, model } => {
                self.client.replace_node(&node, &model)?;
                self.refresh_current()
            }

            UiCmd::SaveAs { name, slot } => {
                self.client.save_current_preset_to(&name, slot)?;
                self.refresh_slot(slot)?;
                self.refresh_current()
            }

            UiCmd::SetUsbGain(db) => {
                self.client.set_usb_gain(db)?;
                let v = self.client.get_usb_gain()?;
                self.state().usb_gain = Some(v);
                Ok(())
            }

            UiCmd::SetLineOutGain(db) => {
                self.client.set_line_out_gain(db)?;
                let v = self.client.get_line_out_gain()?;
                self.state().line_out_gain = Some(v);
                Ok(())
            }

            UiCmd::Import { data, slot } => {
                self.client.save_preset_as(&data, slot)?;
                self.refresh_slot(slot)?;
                self.refresh_current()
            }

            UiCmd::Save => {
                let slot = self.state().current_slot;
                self.client.save_current_preset()?;
                self.state().dirty = false;
                self.refresh_slot(slot)
            }
        }
    }

    /// keeps state in sync with what's happening on the physical amp.
    pub fn on_event(&self, msg: FenderMessageLt) -> Result<(), DynamicError> {
        match msg.r#type {
            Some(fmltType::CurrentLoadedPresetIndexStatus(_)) => self.refresh_current(),

            Some(fmltType::DspUnitParameterStatus(p)) => {
                let mut s = self.state();
                if let (Some(current), Some(value)) = (s.current.as_mut(), p.r#type) {
                    if let Some(node) = current
                        .audio_graph
                        .nodes
                        .iter_mut()
                        .find(|n| n.node_id == p.node_id)
                    {
                        node.dsp_unit_parameters
                            .insert(p.parameter_id, client::param_from_proto(value));
                    }
                }
                s.dirty = true;
                Ok(())
            }

            Some(fmltType::PresetEditedStatus(p)) => {
                self.state().dirty = p.preset_edited;
                Ok(())
            }

            Some(fmltType::PresetSavedStatus(p)) => self.refresh_slot(p.slot),

            Some(fmltType::ReplaceNodeStatus(_)) => self.refresh_current(),

            _ => Ok(()),
        }
    }
}

/// amp thread main loop. `repaint` is called whenever state changed.
pub fn run(state: Arc<Mutex<AmpState>>, cmds: Receiver<UiCmd>, repaint: impl Fn()) {
    let mut amp: Option<FenderAmp> = None;
    let mut want_connect = true;
    let mut last_poll = Instant::now();
    let mut poll_dsp = true;

    let set_status = |status: String| state.lock().unwrap().status = status;

    loop {
        if want_connect {
            want_connect = false;
            set_status("connecting…".into());
            repaint();

            match FenderAmp::connect(state.clone()) {
                Ok(a) => {
                    amp = Some(a);
                    poll_dsp = true;
                }
                Err(e) => set_status(format!("connect failed: {e}")),
            }
            repaint();
        }

        let mut changed = false;

        match cmds.recv_timeout(Duration::from_millis(20)) {
            Ok(UiCmd::Connect) => want_connect = amp.is_none(),

            Ok(cmd) => {
                if let Some(a) = &amp {
                    if let Err(e) = a.handle(cmd) {
                        set_status(format!("error: {e}"));
                    }
                }
                changed = true;
            }

            Err(RecvTimeoutError::Timeout) => {}

            Err(RecvTimeoutError::Disconnected) => break,
        }

        let mut lost = false;

        if let Some(a) = &amp {
            loop {
                match a.events.try_recv() {
                    Ok(msg) => {
                        if let Err(e) = a.on_event(msg) {
                            set_status(format!("error: {e}"));
                        }
                        changed = true;
                    }

                    Err(TryRecvError::Empty) => break,

                    Err(TryRecvError::Disconnected) => {
                        lost = true;
                        break;
                    }
                }
            }
        }

        if let Some(a) = amp.as_ref().filter(|_| poll_dsp && last_poll.elapsed() >= DSP_POLL) {
            last_poll = Instant::now();
            match a.client.get_processor_utilization() {
                Ok(p) => {
                    let mut s = state.lock().unwrap();
                    s.dsp_load.push_back(p.percent);
                    if s.dsp_load.len() > DSP_HISTORY {
                        s.dsp_load.pop_front();
                    }
                    changed = true;
                }
                Err(_) => poll_dsp = false,
            }
        }

        if lost {
            amp = None;
            let mut s = state.lock().unwrap();
            s.connected = false;
            s.status = "amp disconnected".into();
            changed = true;
        }

        if changed {
            repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_lock_is_refused_until_first_drops() {
        let dir = std::env::temp_dir().join("guit2-lock-test");
        std::fs::create_dir_all(&dir).unwrap();
        let first = single_instance_lock(&dir).unwrap();
        let second = single_instance_lock(&dir).unwrap_err().to_string();
        assert!(second.contains("already running"), "{second}");
        drop(first);
        assert!(single_instance_lock(&dir).is_ok());
    }
}
