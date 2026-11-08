use std::f32::consts::PI;

use eframe::egui::{self, Align2, FontId, Pos2, Shape, Stroke, Vec2};

use crate::knob;

pub const MIN_SECS: f64 = 2.4;

fn ease_out(x: f32) -> f32 {
    1.0 - (1.0 - x.clamp(0.0, 1.0)).powi(3)
}

pub fn show(ui: &mut egui::Ui, t: f64, status: &str, failed: bool) -> bool {
    let t = t as f32;
    let accent = knob::accent(ui);
    let center = ui.max_rect().center() - Vec2::new(0.0, 60.0);
    let painter = ui.painter();

    let r = 64.0;
    let sweep = ease_out(t / 1.3);
    let angle = knob::START + knob::SWEEP * sweep;
    let pulse = 0.5 + 0.5 * (t * 2.5).sin();
    for i in 1..=4 {
        let alpha = sweep * (0.10 - i as f32 * 0.02) * (0.6 + 0.4 * pulse);
        painter.circle_filled(center, r + i as f32 * 7.0, accent.gamma_multiply(alpha));
    }
    painter.add(Shape::line(knob::arc(center, r, knob::START, knob::START + knob::SWEEP), Stroke::new(7.0, ui.visuals().extreme_bg_color)));
    painter.add(Shape::line(knob::arc(center, r, knob::START, angle), Stroke::new(7.0, accent)));
    painter.circle(center, r - 12.0, ui.visuals().widgets.inactive.bg_fill, Stroke::new(1.5, accent.gamma_multiply(0.4)));
    painter.line_segment(
        [center + 8.0 * Vec2::angled(angle), center + (r - 18.0) * Vec2::angled(angle)],
        Stroke::new(4.0, ui.visuals().strong_text_color()),
    );

    let fade = ease_out((t - 0.5) / 0.8);
    let text = |a: f32| ui.visuals().strong_text_color().gamma_multiply(a);
    painter.text(
        center + Vec2::new(0.0, r + 46.0 + 12.0 * (1.0 - fade)),
        Align2::CENTER_CENTER,
        "guit2",
        FontId::proportional(44.0),
        accent.gamma_multiply(fade),
    );
    painter.text(
        center + Vec2::new(0.0, r + 80.0),
        Align2::CENTER_CENTER,
        "Mustang LT25 editor",
        FontId::proportional(14.0),
        text(fade * 0.6),
    );

    let width = 320.0;
    let y = center.y + r + 120.0;
    let pluck = (t - 1.0).max(0.0) % 1.6;
    let amp = if failed { 0.0 } else { 24.0 * (-pluck * 1.8).exp() * ease_out(t - 1.0) };
    let points: Vec<Pos2> = (0..=120)
        .map(|i| {
            let x = i as f32 / 120.0;
            let wave = (x * PI).sin() * (x * 6.0 * PI - t * 30.0).sin();
            Pos2::new(center.x - width / 2.0 + x * width, y + amp * wave)
        })
        .collect();
    let string = if failed { ui.visuals().weak_text_color() } else { accent.gamma_multiply(0.4 + 0.6 * fade) };
    painter.add(Shape::line(points, Stroke::new(2.0, string)));

    painter.text(
        Pos2::new(center.x, y + 40.0),
        Align2::CENTER_CENTER,
        status,
        FontId::proportional(13.0),
        if failed { ui.visuals().warn_fg_color } else { text(0.5) },
    );

    if !failed {
        return false;
    }
    let button = egui::Rect::from_center_size(Pos2::new(center.x, y + 80.0), Vec2::new(120.0, 30.0));
    ui.put(button, egui::Button::new("Connect")).clicked()
}
