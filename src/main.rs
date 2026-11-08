#![cfg_attr(windows, windows_subsystem = "windows")]

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    error::Error,
    ops::RangeInclusive,
    sync::{Arc, Mutex, mpsc::{self, Sender}},
    thread,
};

use eframe::egui;
use serde_json::Value;

use crate::amp::{AmpState, UiCmd, values::{DspNode, Preset}};

pub type DynamicError = Box<dyn Error>;

mod amp;
mod backend;
mod dock;
mod icons;
mod knob;
mod prefs;
mod splash;

struct Gui {
    state: Arc<Mutex<AmpState>>,
    cmds: Sender<UiCmd>,
    /// slot the user clicked while having unsaved edits, waiting for confirmation
    pending_load: Option<i32>,
    save_as: Option<(String, i32)>,
    started: Option<f64>,
    prefs_tab: bool,
    prefs: prefs::Prefs,
    export: Option<(String, Option<String>)>,
    taps: Vec<f64>,
    import: Option<ImportDialog>,
}

fn export_path(dir: &str, slot: i32, name: &str) -> String {
    let safe: String = name.trim().chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
    format!("{}/{slot:02}_{safe}.json", dir.trim_end_matches('/'))
}

fn json_files(dir: &str) -> Vec<String> {
    let mut files: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    files.sort();
    files
}

struct ImportDialog {
    path: String,
    slot: i32,
    error: Option<String>,
}

fn write_json(path: &str, preset: &Preset) -> Result<(), DynamicError> {
    if let Some(dir) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(preset)?)?;
    Ok(())
}

impl Gui {
    fn send(&self, cmd: UiCmd) {
        let _ = self.cmds.send(cmd);
    }
}

#[derive(Default)]
struct Catalog {
    models: BTreeMap<String, BTreeSet<String>>,
    choices: HashMap<(String, String), BTreeSet<String>>,
    ranges: HashMap<(String, String), (f64, f64)>,
    fractional: HashSet<(String, String)>,
}

impl Catalog {
    fn build(presets: &[Preset]) -> Self {
        let mut c = Self::default();
        for node in presets.iter().flat_map(|p| &p.audio_graph.nodes) {
            c.models.entry(node.node_id.clone()).or_default().insert(node.fender_id.clone());
            for (key, value) in &node.dsp_unit_parameters {
                let k = (node.node_id.clone(), key.clone());
                match value {
                    Value::String(v) => {
                        c.choices.entry(k).or_default().insert(v.clone());
                    }
                    Value::Number(n) => {
                        let k = (node.fender_id.clone(), key.clone());
                        let v = n.as_f64().unwrap_or_default();
                        if v.fract() != 0.0 {
                            c.fractional.insert(k.clone());
                        }
                        let r = c.ranges.entry(k).or_insert((v, v));
                        *r = (r.0.min(v), r.1.max(v));
                    }
                    _ => {}
                }
            }
        }
        c
    }

    fn knob_range(&self, model: &str, key: &str) -> RangeInclusive<f64> {
        if key == "volume" {
            return -60.0..=0.0;
        }
        let (lo, hi) = self.ranges.get(&(model.into(), key.into())).copied().unwrap_or((0.0, 1.0));
        lo.min(0.0)..=hi.max(1.0)
    }
}

#[derive(Clone, Copy)]
struct DspNodeRef<'a> {
    id: &'a str,
    model: &'a str,
}

fn pretty_model(id: &str) -> String {
    match id.trim_start_matches("DUBS_") {
        "Passthru" => "— empty —".into(),
        name => name.into(),
    }
}

fn param_widget(ui: &mut egui::Ui, catalog: &Catalog, node: &DspNodeRef, key: &str, value: &mut Value, knob_size: f32) -> bool {
    let DspNodeRef { id: node, model } = *node;
    match value {
        Value::Bool(b) => {
            let label = if *b { "on" } else { "off" };
            ui.toggle_value(b, label).changed()
        }

        Value::Number(n) => {
            let k = (model.to_owned(), key.to_owned());
            let int = n.is_i64() && !catalog.fractional.contains(&k);
            let mut f = n.as_f64().unwrap_or_default();
            let mut changed = knob::knob(ui, &mut f, catalog.knob_range(model, key), knob_size).changed();
            changed |= ui
                .add(egui::DragValue::new(&mut f).speed(0.01).max_decimals(if int { 0 } else { 2 }))
                .changed();
            *value = if int { Value::from(f.round() as i64) } else { Value::from(f) };
            changed
        }

        Value::String(current) => {
            let options = catalog.choices.get(&(node.into(), key.into()));
            let mut changed = false;
            egui::ComboBox::from_id_salt((node, key))
                .selected_text(current.as_str())
                .show_ui(ui, |ui| {
                    for option in options.into_iter().flatten() {
                        changed |= ui.selectable_value(current, option.clone(), option).changed();
                    }
                });
            changed
        }

        other => {
            ui.label(other.to_string());
            false
        }
    }
}

fn control(ui: &mut egui::Ui, label: &str, knob_size: f32, add: impl FnOnce(&mut egui::Ui) -> bool) -> bool {
    let w = knob_size + 34.0;
    ui.allocate_ui_with_layout(egui::vec2(w, 0.0), egui::Layout::top_down(egui::Align::Center), |ui| {
        ui.set_min_width(w);
        ui.add(egui::Label::new(egui::RichText::new(label).small().strong()).truncate());
        add(ui)
    })
    .inner
}

fn node_card(ui: &mut egui::Ui, node: &mut DspNode, catalog: &Catalog, cmds: &mut Vec<UiCmd>, knob_size: f32) {
    let accent = knob::accent(ui);
    egui::Frame::group(ui.style())
        .fill(ui.visuals().faint_bg_color)
        .corner_radius(8.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                let bypassed = node.dsp_unit_parameters.get("bypass") == Some(&Value::Bool(true));
                let active = !bypassed && node.fender_id != "DUBS_Passthru";
                icons::block(ui, &node.node_id, accent, active);
                ui.label(egui::RichText::new(node.node_id.to_uppercase()).strong().color(accent));

                if let Some(Value::Bool(bypass)) = node.dsp_unit_parameters.get_mut("bypass") {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut on = !*bypass;
                        let label = if on { "ON" } else { "BYPASSED" };
                        if ui.toggle_value(&mut on, label).changed() {
                            *bypass = !on;
                            cmds.push(UiCmd::SetParam {
                                node: node.node_id.clone(),
                                param: "bypass".into(),
                                value: Value::Bool(!on),
                            });
                        }
                    });
                }
            });

            let mut model = node.fender_id.clone();
            egui::ComboBox::from_id_salt(("model", &node.node_id))
                .selected_text(pretty_model(&model))
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for m in catalog.models.get(&node.node_id).into_iter().flatten() {
                        ui.selectable_value(&mut model, m.clone(), pretty_model(m));
                    }
                });
            if model != node.fender_id {
                cmds.push(UiCmd::ReplaceNode { node: node.node_id.clone(), model });
            }

            let (mut knobs, mut others): (Vec<_>, Vec<_>) = node
                .dsp_unit_parameters
                .keys()
                .filter(|k| *k != "bypass")
                .cloned()
                .partition(|k| node.dsp_unit_parameters[k].is_number());
            knobs.sort();
            others.sort();

            let node_ref = DspNodeRef { id: &node.node_id, model: &node.fender_id };
            let mut changed = Vec::new();

            ui.add_space(4.0);
            let row = egui::Layout::left_to_right(egui::Align::TOP).with_main_wrap(true);
            ui.with_layout(row, |ui| {
                for key in knobs {
                    let value = node.dsp_unit_parameters.get_mut(&key).unwrap();
                    if control(ui, &key, knob_size, |ui| param_widget(ui, catalog, &node_ref, &key, value, knob_size)) {
                        changed.push(key);
                    }
                }
            });

            if !others.is_empty() {
                ui.separator();
                egui::Grid::new(("others", &node.node_id)).num_columns(2).show(ui, |ui| {
                    for key in others {
                        ui.label(egui::RichText::new(&key).small());
                        let value = node.dsp_unit_parameters.get_mut(&key).unwrap();
                        if param_widget(ui, catalog, &node_ref, &key, value, knob_size) {
                            changed.push(key);
                        }
                        ui.end_row();
                    }
                });
            }

            for key in changed {
                cmds.push(UiCmd::SetParam {
                    node: node.node_id.clone(),
                    value: node.dsp_unit_parameters[&key].clone(),
                    param: key,
                });
            }
        });
    ui.add_space(6.0);
}

impl eframe::App for Gui {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let state = self.state.clone();
        let mut guard = state.lock().unwrap();
        let s = &mut *guard;
        let mut cmds = Vec::new();

        let now = ui.input(|i| i.time);
        let t = now - *self.started.get_or_insert(now);
        if !s.connected || (self.prefs.splash && t < splash::MIN_SECS) {
            let failed = !s.connected && !(s.status.is_empty() || s.status == "connecting…");
            egui::CentralPanel::default().show(ui, |ui| {
                if splash::show(ui, t, &s.status, failed) {
                    cmds.push(UiCmd::Connect);
                }
            });
            ui.ctx().request_repaint();
            drop(guard);
            for cmd in cmds {
                self.send(cmd);
            }
            return;
        }

        egui::Panel::top("status").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("guit2").color(knob::accent(ui)).strong());
                ui.separator();
                ui.selectable_value(&mut self.prefs_tab, false, "Editor");
                ui.selectable_value(&mut self.prefs_tab, true, "Preferences");
                ui.separator();
                ui.label(format!("Mustang LT25 · firmware {}", s.firmware));
                ui.separator();
                ui.weak(&s.status);
            });
            ui.add_space(4.0);
        });

        if self.prefs_tab {
            let before = self.prefs.clone();
            egui::CentralPanel::default().show(ui, |ui| prefs::ui(ui, &mut self.prefs));
            if self.prefs != before {
                self.prefs.apply(ui.ctx());
            }
        } else {
            egui::Panel::left("presets").resizable(true).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (i, preset) in s.presets.iter().enumerate() {
                        let slot = i as i32 + 1;
                        let mut label = egui::text::LayoutJob::default();
                        let font = egui::TextStyle::Body.resolve(ui.style());
                        let number = egui::TextFormat::simple(font.clone(), knob::accent(ui).gamma_multiply(0.8));
                        label.append(&format!("{slot:02}"), 0.0, number);
                        label.append(&preset.info.display_name, 10.0, egui::TextFormat::simple(font, ui.visuals().text_color()));
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

            let catalog = Catalog::build(&s.presets);

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
                        ui.colored_label(knob::accent(ui), "● edited");
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(format!("{}  Save as / rename…", icons::SAVE_PEN)).clicked() {
                            self.save_as = Some((current.info.display_name.clone(), slot));
                        }
                        if ui.add_enabled(dirty, egui::Button::new(format!("{}  Save to slot", icons::SAVE))).clicked() {
                            cmds.push(UiCmd::Save);
                        }
                        if ui.button(format!("{}  Import JSON…", icons::FILE_INPUT)).clicked() {
                            self.import = Some(ImportDialog { path: String::new(), slot, error: None });
                        }
                        if ui.button(format!("{}  Export JSON…", icons::FILE_OUTPUT)).clicked() {
                            self.export = Some((export_path(&self.prefs.preset_dir, slot, &current.info.display_name), None));
                        }
                    });
                });
                ui.add_space(6.0);
                dock::signal_flow(ui, current, &mut cmds);
                ui.add_space(6.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let nodes = &mut current.audio_graph.nodes;
                    let cols = ((ui.available_width() / 250.0) as usize).clamp(1, nodes.len().max(1));
                    let col_w = ui.available_width() / cols as f32;
                    let knob_size = ((col_w - 24.0) / 2.0 - 34.0).clamp(48.0, 64.0);
                    ui.columns(cols, |columns| {
                        let mut nodes: Vec<_> = nodes.iter_mut().collect();
                        nodes.sort_by_key(|n| n.node_id != "amp");
                        for (i, node) in nodes.into_iter().enumerate() {
                            node_card(&mut columns[i % cols], node, &catalog, &mut cmds, knob_size);
                        }
                    });

                    ui.add_space(4.0);
                    if ui.available_width() > 720.0 {
                        ui.columns(3, |c| {
                            dock::tap_tempo(&mut c[0], current, &mut self.taps, &mut cmds);
                            dock::amp_settings(&mut c[1], s.usb_gain, s.line_out_gain, &mut cmds);
                            dock::dsp_meter(&mut c[2], &s.dsp_load);
                        });
                    } else {
                        dock::tap_tempo(ui, current, &mut self.taps, &mut cmds);
                        dock::amp_settings(ui, s.usb_gain, s.line_out_gain, &mut cmds);
                        dock::dsp_meter(ui, &s.dsp_load);
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

        if let Some((name, slot)) = &mut self.save_as {
            let mut close = false;
            egui::Modal::new(egui::Id::unique("save_as")).show(ui.ctx(), |ui| {
                ui.heading("Save preset");
                egui::Grid::new("save_as_grid").num_columns(2).show(ui, |ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(name);
                    ui.end_row();
                    ui.label("Slot");
                    ui.add(egui::DragValue::new(slot).range(1..=60));
                    ui.end_row();
                });
                if *slot != s.current_slot {
                    let target = s.presets.get(*slot as usize - 1).map(|p| p.info.display_name.as_str());
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        format!("This overwrites slot {slot:02}: {}", target.unwrap_or("?")),
                    );
                }
                ui.horizontal(|ui| {
                    if ui.add_enabled(!name.trim().is_empty(), egui::Button::new(format!("{}  Save", icons::SAVE_PEN))).clicked() {
                        cmds.push(UiCmd::SaveAs { name: name.trim().into(), slot: *slot });
                        close = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
            if close {
                self.save_as = None;
            }
        }

        if let (Some((path, error)), Some(preset)) = (&mut self.export, &s.current) {
            let mut close = false;
            egui::Modal::new(egui::Id::unique("export")).show(ui.ctx(), |ui| {
                ui.heading("Export preset as JSON");
                ui.label("Writes the preset as it is now, including unsaved edits.");
                ui.add(egui::TextEdit::singleline(path).desired_width(420.0));
                if let Some(e) = error {
                    ui.colored_label(ui.visuals().error_fg_color, e.as_str());
                }
                ui.horizontal(|ui| {
                    if ui.button(format!("{}  Export", icons::FILE_OUTPUT)).clicked() {
                        match write_json(path, preset) {
                            Ok(()) => {
                                s.status = format!("exported to {path}");
                                close = true;
                            }
                            Err(e) => *error = Some(format!("export failed: {e}")),
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
            if close {
                self.export = None;
            }
        }

        if let Some(dialog) = &mut self.import {
            let mut close = false;
            egui::Modal::new(egui::Id::unique("import")).show(ui.ctx(), |ui| {
                ui.heading("Import preset from JSON");
                let files = json_files(&self.prefs.preset_dir);
                if files.is_empty() {
                    ui.weak(format!("no .json files in {}", self.prefs.preset_dir));
                } else {
                    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                        for f in files {
                            let name = f.rsplit('/').next().unwrap_or(&f).to_owned();
                            if ui.selectable_label(dialog.path == f, name).clicked() {
                                dialog.path = f;
                            }
                        }
                    });
                }
                ui.add(egui::TextEdit::singleline(&mut dialog.path).hint_text("or type a path").desired_width(420.0));
                ui.horizontal(|ui| {
                    ui.label("Into slot");
                    ui.add(egui::DragValue::new(&mut dialog.slot).range(1..=60));
                });
                let target = s.presets.get(dialog.slot as usize - 1).map(|p| p.info.display_name.as_str());
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    format!("This overwrites slot {:02}: {}", dialog.slot, target.unwrap_or("?")),
                );
                if let Some(e) = &dialog.error {
                    ui.colored_label(ui.visuals().error_fg_color, e.as_str());
                }
                ui.horizontal(|ui| {
                    if ui.add_enabled(!dialog.path.trim().is_empty(), egui::Button::new(format!("{}  Import", icons::FILE_INPUT))).clicked() {
                        let data = std::fs::read_to_string(dialog.path.trim()).map_err(DynamicError::from);
                        match data.and_then(|d| Preset::from_data(&d).map(|_| d)) {
                            Ok(data) => {
                                cmds.push(UiCmd::Import { data, slot: dialog.slot });
                                if s.dirty {
                                    s.status = "imported; unsaved edits to the previous preset were dropped".into();
                                }
                                close = true;
                            }
                            Err(e) => dialog.error = Some(format!("not a valid preset: {e}")),
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
            if close {
                self.import = None;
            }
        }

        drop(guard);
        for cmd in cmds {
            self.send(cmd);
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "prefs", &self.prefs);
    }
}

fn main() -> eframe::Result {
    let state = Arc::new(Mutex::new(AmpState::default()));
    let (tx, rx) = mpsc::channel();

    eframe::run_native(
        "guit2",
        eframe::NativeOptions::default(),
        Box::new(move |cc| {
            let prefs: prefs::Prefs = cc.storage.and_then(|st| eframe::get_value(st, "prefs")).unwrap_or_default();
            prefs.apply(&cc.egui_ctx);
            icons::install_lucide(&cc.egui_ctx);
            let ctx = cc.egui_ctx.clone();
            let amp_state = state.clone();
            thread::spawn(move || amp::run(amp_state, rx, move || ctx.request_repaint()));

            Ok(Box::new(Gui {
                state,
                cmds: tx,
                pending_load: None,
                save_as: None,
                started: None,
                prefs_tab: false,
                prefs,
                export: None,
                taps: Vec::new(),
                import: None,
            }))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_writes_preset_json() {
        assert_eq!(export_path("/x/presets/", 3, " ROCK A/BILLY "), "/x/presets/03_ROCK_A_BILLY.json");

        let preset = Preset::from_data(
            r#"{"audioGraph":{"connections":[],"nodes":[]},"info":{"displayName":"x"},
               "nodeId":"preset","nodeType":"preset","numInputs":2,"numOutputs":2,"version":"1"}"#,
        )
        .unwrap();
        let path = std::env::temp_dir().join("guit2-test/sub/p.json");
        write_json(path.to_str().unwrap(), &preset).unwrap();
        assert_eq!(Preset::from_data(std::fs::read_to_string(&path).unwrap()).unwrap(), preset);
    }
}
