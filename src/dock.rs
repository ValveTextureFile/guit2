use std::collections::VecDeque;

use eframe::egui::{self, Align2, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Vec2};
use serde_json::Value;

use crate::{
    amp::{
        UiCmd,
        values::{AudioGraph, Preset},
    },
    icons, knob,
};

pub fn chain_order(graph: &AudioGraph) -> Vec<String> {
    let mut order = Vec::new();
    let mut at = "preset";
    while let Some(c) = graph
        .connections
        .iter()
        .find(|c| c.input.node_id == at && c.input.index == 0)
    {
        at = &c.output.node_id;
        if at == "preset" || order.iter().any(|n| n == at) {
            break;
        }
        order.push(at.to_owned());
    }
    if order.len() != graph.nodes.len() {
        return graph.nodes.iter().map(|n| n.node_id.clone()).collect();
    }
    order
}

pub fn signal_flow(ui: &mut egui::Ui, preset: &mut Preset, cmds: &mut Vec<UiCmd>) {
    let accent = knob::accent(ui);
    let weak = ui.visuals().weak_text_color();
    let order = chain_order(&preset.audio_graph);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 74.0), Sense::hover());

    let slots = order.len() + 2;
    let step = rect.width() / slots as f32;
    let center = |i: usize| Pos2::new(rect.min.x + step * (i as f32 + 0.5), rect.center().y);
    let box_size = Vec2::new((step - 16.0).min(150.0), 62.0);

    ui.painter().line_segment([center(0), center(slots - 1)], Stroke::new(3.0, accent.gamma_multiply(0.5)));

    for (i, label) in [(0, "IN"), (slots - 1, "OUT")] {
        ui.painter().circle(center(i), 16.0, ui.visuals().extreme_bg_color, Stroke::new(2.0, accent));
        ui.painter().text(center(i), Align2::CENTER_CENTER, label, FontId::proportional(11.0), accent);
    }

    for (i, id) in order.iter().enumerate() {
        let Some(node) = preset.audio_graph.nodes.iter_mut().find(|n| &n.node_id == id) else {
            continue;
        };
        let bypass = node.dsp_unit_parameters.get("bypass").and_then(Value::as_bool);
        let active = bypass != Some(true) && node.fender_id != "DUBS_Passthru";
        let r = Rect::from_center_size(center(i + 1), box_size);
        let response = ui.interact(r, ui.scope_id().with(("flow", id)), Sense::click());

        let color = if active { accent } else { weak };
        let fill = if response.hovered() { ui.visuals().widgets.hovered.bg_fill } else { ui.visuals().faint_bg_color };
        let p = ui.painter();
        p.rect(r, CornerRadius::same(8), fill, Stroke::new(1.5, color), StrokeKind::Inside);
        let model = crate::pretty_model(&node.fender_id);
        let status = if bypass == Some(true) { "bypassed".to_owned() } else { model };
        if box_size.x < 130.0 {
            icons::paint(p, Rect::from_center_size(r.center(), Vec2::splat(40.0)), id, color, active);
        } else {
            icons::paint(p, Rect::from_center_size(Pos2::new(r.min.x + 26.0, r.center().y), Vec2::splat(40.0)), id, color, active);
            let text_x = r.min.x + 50.0;
            p.text(Pos2::new(text_x, r.center().y - 9.0), Align2::LEFT_CENTER, id.to_uppercase(), FontId::proportional(12.0), color);
            p.with_clip_rect(r.shrink(2.0)).text(
                Pos2::new(text_x, r.center().y + 9.0),
                Align2::LEFT_CENTER,
                &status,
                FontId::proportional(11.0),
                ui.visuals().text_color(),
            );
        }
        let response = response.on_hover_text(format!("{}: {status}", id.to_uppercase()));

        if let Some(bypass) = bypass {
            let response = response.on_hover_text(if bypass { "click to turn on" } else { "click to bypass" });
            if response.clicked() {
                node.dsp_unit_parameters.insert("bypass".into(), Value::Bool(!bypass));
                cmds.push(UiCmd::SetParam {
                    node: id.clone(),
                    param: "bypass".into(),
                    value: Value::Bool(!bypass),
                });
            }
        }
    }
}

pub fn tap(taps: &mut Vec<f64>, now: f64) -> Option<f64> {
    if taps.last().is_some_and(|last| now - last > 2.0) {
        taps.clear();
    }
    taps.push(now);
    if taps.len() > 5 {
        taps.remove(0);
    }
    let (first, last) = (taps.first()?, taps.last()?);
    (taps.len() >= 2).then(|| 60.0 * (taps.len() - 1) as f64 / (last - first))
}

pub fn tap_tempo(ui: &mut egui::Ui, preset: &mut Preset, taps: &mut Vec<f64>, cmds: &mut Vec<UiCmd>) {
    card(ui, "Tap tempo", |ui| {
        let delay = preset
            .audio_graph
            .nodes
            .iter_mut()
            .find(|n| n.node_id == "delay" && n.dsp_unit_parameters.contains_key("tapTimeBPM"));
        let Some(delay) = delay else {
            ui.weak("Pick a delay model to tap its tempo.");
            return;
        };
        let bpm = delay.dsp_unit_parameters["tapTimeBPM"].as_f64().unwrap_or_default();

        ui.horizontal(|ui| {
            let button = egui::Button::new(RichText::new("TAP").size(22.0).strong()).min_size(Vec2::new(96.0, 64.0));
            if ui.add(button).clicked() {
                let now = ui.input(|i| i.time);
                if let Some(bpm) = tap(taps, now).map(|b| b.clamp(30.0, 300.0)) {
                    let time_param = ["time", "dlyTime"].into_iter().find(|k| delay.dsp_unit_parameters.contains_key(*k));
                    for (param, value) in [("tapTimeBPM", bpm)].into_iter().chain(time_param.map(|k| (k, 60.0 / bpm))) {
                        delay.dsp_unit_parameters.insert(param.into(), Value::from(value));
                        cmds.push(UiCmd::SetParam {
                            node: "delay".into(),
                            param: param.into(),
                            value: Value::from(value),
                        });
                    }
                }
            }
            ui.vertical(|ui| {
                ui.label(RichText::new(format!("{bpm:.0} BPM")).size(26.0).color(knob::accent(ui)));
                ui.weak(format!("{:.0} ms per beat", 60_000.0 / bpm.max(1.0)));
                if taps.len() == 1 {
                    ui.weak("keep tapping…");
                }
            });
        });
    });
}

pub fn amp_settings(ui: &mut egui::Ui, usb_gain: Option<f32>, line_out_gain: Option<f32>, cmds: &mut Vec<UiCmd>) {
    card(ui, "Amp settings", |ui| {
        if usb_gain.is_none() && line_out_gain.is_none() {
            ui.weak("This amp doesn't report any global settings.");
            return;
        }
        ui.horizontal(|ui| {
            for (label, value, make) in [
                ("USB gain", usb_gain, UiCmd::SetUsbGain as fn(f32) -> UiCmd),
                ("line out", line_out_gain, UiCmd::SetLineOutGain),
            ] {
                let Some(mut v) = value.map(f64::from) else { continue };
                ui.vertical(|ui| {
                    ui.label(RichText::new(label).small().strong());
                    let mut changed = knob::knob(ui, &mut v, -20.0..=20.0, 56.0).changed();
                    changed |= ui.add(egui::DragValue::new(&mut v).speed(0.1).suffix(" dB").max_decimals(1)).changed();
                    if changed {
                        cmds.push(make(v as f32));
                    }
                });
            }
        });
    });
}

pub fn dsp_meter(ui: &mut egui::Ui, load: &VecDeque<f32>) {
    card(ui, "DSP load", |ui| {
        let Some(&now) = load.back() else {
            ui.weak("waiting for the amp…");
            return;
        };
        let peak = load.iter().copied().fold(0.0, f32::max);
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{now:.1}%")).size(26.0).color(knob::accent(ui)));
            ui.weak(format!("peak {peak:.1}%"));
        });

        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 56.0), Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, CornerRadius::same(4), ui.visuals().extreme_bg_color);
        let n = 120.0;
        let points: Vec<Pos2> = load
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let x = rect.max.x - (load.len() - 1 - i) as f32 / (n - 1.0) * rect.width();
                Pos2::new(x, rect.max.y - v.clamp(0.0, 100.0) / 100.0 * rect.height())
            })
            .collect();
        if points.len() >= 2 {
            let mut area = points.clone();
            area.push(Pos2::new(points[points.len() - 1].x, rect.max.y));
            area.push(Pos2::new(points[0].x, rect.max.y));
            p.add(Shape::convex_polygon(area, knob::accent(ui).gamma_multiply(0.15), Stroke::NONE));
            p.add(Shape::line(points, Stroke::new(2.0, knob::accent(ui))));
        }
    });
}

fn card(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style())
        .fill(ui.visuals().faint_bg_color)
        .corner_radius(8.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(title.to_uppercase()).strong().color(knob::accent(ui)));
            ui.add_space(4.0);
            add(ui);
        });
    ui.add_space(6.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tap_tempo_from_intervals() {
        let mut taps = Vec::new();
        assert_eq!(tap(&mut taps, 10.0), None);
        assert_eq!(tap(&mut taps, 10.5), Some(120.0));
        assert_eq!(tap(&mut taps, 11.0), Some(120.0));
        assert_eq!(tap(&mut taps, 20.0), None);
        assert_eq!(tap(&mut taps, 20.4).map(f64::round), Some(150.0));
    }

    #[test]
    fn chain_follows_connections() {
        let json = r#"{"connections":[
            {"input":{"index":0,"nodeId":"preset"},"output":{"index":0,"nodeId":"stomp"}},
            {"input":{"index":0,"nodeId":"amp"},"output":{"index":0,"nodeId":"reverb"}},
            {"input":{"index":0,"nodeId":"stomp"},"output":{"index":0,"nodeId":"amp"}},
            {"input":{"index":0,"nodeId":"reverb"},"output":{"index":0,"nodeId":"preset"}}],
            "nodes":[
            {"FenderId":"a","dspUnitParameters":{},"nodeId":"reverb","nodeType":"dspUnit"},
            {"FenderId":"b","dspUnitParameters":{},"nodeId":"stomp","nodeType":"dspUnit"},
            {"FenderId":"c","dspUnitParameters":{},"nodeId":"amp","nodeType":"dspUnit"}]}"#;
        let graph: AudioGraph = serde_json::from_str(json).unwrap();
        assert_eq!(chain_order(&graph), ["stomp", "amp", "reverb"]);
    }
}
