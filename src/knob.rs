use std::{f32::consts::PI, ops::RangeInclusive};

use eframe::egui::{self, Color32, Pos2, Sense, Shape, Stroke, Vec2};

pub fn accent(ui: &egui::Ui) -> Color32 {
    ui.visuals().selection.stroke.color
}

pub const START: f32 = 0.75 * PI;
pub const SWEEP: f32 = 1.5 * PI;

pub fn arc(center: Pos2, r: f32, from: f32, to: f32) -> Vec<Pos2> {
    let steps = 32;
    (0..=steps)
        .map(|i| {
            let a = from + (to - from) * i as f32 / steps as f32;
            center + r * Vec2::angled(a)
        })
        .collect()
}

pub fn knob(ui: &mut egui::Ui, value: &mut f64, range: RangeInclusive<f64>, size: f32) -> egui::Response {
    let (lo, hi) = (*range.start(), *range.end());
    let (rect, mut response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click_and_drag());

    if response.dragged() {
        let fine = if ui.input(|i| i.modifiers.shift) { 0.1 } else { 1.0 };
        let delta = (response.drag_delta().x - response.drag_delta().y) as f64;
        let new = (*value + delta / 150.0 * (hi - lo) * fine).clamp(lo, hi);
        if new != *value {
            *value = new;
            response.mark_changed();
        }
    }
    if response.double_clicked() {
        *value = (lo + hi) / 2.0;
        response.mark_changed();
    }

    let t = (((*value - lo) / (hi - lo)) as f32).clamp(0.0, 1.0);
    let angle = START + SWEEP * t;
    let center = rect.center();
    let r = size / 2.0 - 4.0;
    let accent = accent(ui);
    let visuals = ui.style().interact(&response);
    let painter = ui.painter();

    painter.add(Shape::line(arc(center, r, START, START + SWEEP), Stroke::new(size / 13.0, ui.visuals().extreme_bg_color)));
    painter.add(Shape::line(arc(center, r, START, angle), Stroke::new(size / 13.0, accent)));
    painter.circle(center, r - 6.0, visuals.bg_fill, visuals.fg_stroke);
    painter.line_segment(
        [center + 4.0 * Vec2::angled(angle), center + (r - 9.0) * Vec2::angled(angle)],
        Stroke::new(2.5, ui.visuals().strong_text_color()),
    );

    response
}
