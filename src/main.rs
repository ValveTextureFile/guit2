use std::{
    error::Error,
    sync::{Arc, Mutex, mpsc::{self, Sender}},
    thread,
};

use eframe::egui;
use serde_json::Value;

use crate::amp::{AmpState, UiCmd};

pub type DynamicError = Box<dyn Error>;

mod amp;
mod backend;

struct Gui {
    state: Arc<Mutex<AmpState>>,
    cmds: Sender<UiCmd>,
    /// slot the user clicked while having unsaved edits, waiting for confirmation
    pending_load: Option<i32>,
}

impl Gui {
    fn send(&self, cmd: UiCmd) {
        let _ = self.cmds.send(cmd);
    }
}

/// draws an editor for one parameter, returns true if the user changed it.
fn param_widget(ui: &mut egui::Ui, value: &mut Value) -> bool {
    match value {
        Value::Bool(b) => ui.checkbox(b, "").changed(),

        Value::Number(n) if n.is_i64() => {
            let mut i = n.as_i64().unwrap();
            let changed = ui.add(egui::DragValue::new(&mut i)).changed();
            *value = Value::from(i);
            changed
        }

        Value::Number(n) => {
            // ranges differ per model (0..1 knobs, dB volumes, Hz rates), so no slider bounds
            let mut f = n.as_f64().unwrap_or_default();
            let changed = ui
                .add(egui::DragValue::new(&mut f).speed(0.01).max_decimals(3))
                .changed();
            *value = Value::from(f);
            changed
        }

        // shortcut: string params are enums whose valid values we don't know, so read-only.
        // upgrade with a per-model option table to make them combo boxes.
        other => {
            ui.label(other.as_str().map(str::to_owned).unwrap_or_else(|| other.to_string()));
            false
        }
    }
}

impl eframe::App for Gui {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let state = self.state.clone();
        let mut s = state.lock().unwrap();
        let mut cmds = Vec::new();

        egui::Panel::top("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("guit2");
                ui.separator();
                if s.connected {
                    ui.label(format!("Mustang LT25 · firmware {}", s.firmware));
                } else if ui.button("Connect").clicked() {
                    cmds.push(UiCmd::Connect);
                }
                ui.separator();
                ui.label(&s.status);
            });
        });

        if !s.connected {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.centered_and_justified(|ui| ui.label(&s.status));
            });
        } else {
            egui::Panel::left("presets").resizable(true).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (i, preset) in s.presets.iter().enumerate() {
                        let slot = i as i32 + 1;
                        let label = format!("{slot:02}  {}", preset.info.display_name);
                        if ui.selectable_label(slot == s.current_slot, label).clicked()
                            && slot != s.current_slot
                        {
                            if s.dirty {
                                self.pending_load = Some(slot);
                            } else {
                                cmds.push(UiCmd::Load(slot));
                            }
                        }
                    }
                });
            });

            egui::CentralPanel::default().show(ui, |ui| {
                let dirty = s.dirty;
                let slot = s.current_slot;
                let Some(current) = s.current.as_mut() else {
                    ui.label("no preset loaded");
                    return;
                };

                ui.horizontal(|ui| {
                    ui.heading(format!("{slot:02}  {}", current.info.display_name));
                    if dirty {
                        ui.label("(edited)");
                    }
                    if ui.add_enabled(dirty, egui::Button::new("Save to slot")).clicked() {
                        cmds.push(UiCmd::Save);
                    }
                });
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for node in current.audio_graph.nodes.iter_mut() {
                        egui::CollapsingHeader::new(format!("{} · {}", node.node_id, node.fender_id))
                            .default_open(true)
                            .show(ui, |ui| {
                                let mut keys: Vec<_> = node.dsp_unit_parameters.keys().cloned().collect();
                                keys.sort();

                                egui::Grid::new(&node.node_id).num_columns(2).striped(true).show(ui, |ui| {
                                    for key in keys {
                                        ui.label(&key);
                                        let value = node.dsp_unit_parameters.get_mut(&key).unwrap();
                                        if param_widget(ui, value) {
                                            cmds.push(UiCmd::SetParam {
                                                node: node.node_id.clone(),
                                                param: key,
                                                value: value.clone(),
                                            });
                                        }
                                        ui.end_row();
                                    }
                                });
                            });
                    }
                });
            });
        }

        if let Some(slot) = self.pending_load {
            egui::Modal::new(egui::Id::unique("discard")).show(ui.ctx(), |ui| {
                ui.label("The current preset has unsaved changes. Discard them?");
                ui.horizontal(|ui| {
                    if ui.button("Discard and load").clicked() {
                        cmds.push(UiCmd::Load(slot));
                        self.pending_load = None;
                    }
                    if ui.button("Cancel").clicked() {
                        self.pending_load = None;
                    }
                });
            });
        }

        drop(s);
        for cmd in cmds {
            self.send(cmd);
        }
    }
}

fn main() -> eframe::Result {
    let state = Arc::new(Mutex::new(AmpState::default()));
    let (tx, rx) = mpsc::channel();

    eframe::run_native(
        "guit2",
        eframe::NativeOptions::default(),
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();
            let amp_state = state.clone();
            thread::spawn(move || amp::run(amp_state, rx, move || ctx.request_repaint()));

            Ok(Box::new(Gui {
                state,
                cmds: tx,
                pending_load: None,
            }))
        }),
    )
}
