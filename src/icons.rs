use std::f32::consts::PI;

use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Vec2};

pub const SAVE: &str = "\u{e14d}";
pub const SAVE_PEN: &str = "\u{e705}";
pub const FILE_INPUT: &str = "\u{e0c5}";
pub const FILE_OUTPUT: &str = "\u{e0c8}";

pub fn install_lucide(ctx: &egui::Context) {
    use egui::epaint::text::{FontData, FontInsert, FontPriority, InsertFontFamily};
    ctx.add_font(FontInsert::new(
        "lucide",
        FontData::from_static(include_bytes!("../assets/lucide-subset.ttf")),
        vec![InsertFontFamily { family: egui::FontFamily::Proportional, priority: FontPriority::Lowest }],
    ));
}

pub fn block(ui: &mut egui::Ui, node_id: &str, color: Color32, active: bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::hover());
    let color = if active { color } else { ui.visuals().weak_text_color() };
    paint(ui.painter(), rect, node_id, color, active);
}

pub fn paint(p: &egui::Painter, rect: Rect, node_id: &str, color: Color32, active: bool) {
    let stroke = Stroke::new(2.0, color);
    let faint = color.gamma_multiply(0.35);
    let c = rect.center();
    let r = rect.shrink(4.0);

    match node_id {
        "amp" => {
            p.rect_stroke(r, CornerRadius::same(4), stroke, StrokeKind::Inside);
            let strip = Rect::from_min_max(r.min, Pos2::new(r.max.x, r.min.y + 10.0));
            p.line_segment([strip.left_bottom(), strip.right_bottom()], stroke);
            for i in 0..4 {
                p.circle_filled(Pos2::new(r.min.x + 7.0 + i as f32 * 6.5, strip.center().y), 1.8, color);
            }
            let grille = Rect::from_min_max(Pos2::new(r.min.x + 4.0, strip.max.y + 3.0), r.max - Vec2::splat(4.0));
            for i in 0..4 {
                let y = grille.min.y + i as f32 * grille.height() / 3.0;
                p.line_segment([Pos2::new(grille.min.x, y), Pos2::new(grille.max.x, y)], Stroke::new(1.0, faint));
            }
        }

        "stomp" => {
            let body = Rect::from_center_size(c, Vec2::new(24.0, 32.0));
            p.rect_stroke(body, CornerRadius::same(4), stroke, StrokeKind::Inside);
            p.circle_stroke(Pos2::new(body.min.x + 7.0, body.min.y + 8.0), 3.0, stroke);
            p.circle_stroke(Pos2::new(body.max.x - 7.0, body.min.y + 8.0), 3.0, stroke);
            p.circle_filled(Pos2::new(c.x, body.min.y + 15.0), 1.8, if active { Color32::from_rgb(0xff, 0x4d, 0x4d) } else { faint });
            p.circle_filled(Pos2::new(c.x, body.max.y - 7.0), 4.0, color);
        }

        "mod" => {
            let points = (0..=40)
                .map(|i| {
                    let x = i as f32 / 40.0;
                    let y = (x * 4.0 * PI).sin() * (0.5 + 0.5 * (x * PI).sin());
                    Pos2::new(r.min.x + x * r.width(), c.y - y * 11.0)
                })
                .collect();
            p.add(Shape::line(points, stroke));
        }

        "delay" => {
            for i in 0..4 {
                let x = r.min.x + 3.0 + i as f32 * 9.0;
                let h = 26.0 * 0.7f32.powi(i);
                let a = 1.0 - i as f32 * 0.22;
                p.line_segment([Pos2::new(x, c.y - h / 2.0), Pos2::new(x, c.y + h / 2.0)], Stroke::new(3.0, color.gamma_multiply(a)));
            }
        }

        "reverb" => {
            let src = Pos2::new(r.min.x + 4.0, c.y);
            p.circle_filled(src, 3.0, color);
            for i in 1..=3 {
                let rad = i as f32 * 9.0;
                let points = (0..=16)
                    .map(|j| src + rad * Vec2::angled(-PI / 3.0 + j as f32 / 16.0 * 2.0 * PI / 3.0))
                    .collect();
                p.add(Shape::line(points, Stroke::new(2.0, color.gamma_multiply(1.0 - (i - 1) as f32 * 0.3))));
            }
        }

        _ => {
            p.circle_stroke(c, 12.0, stroke);
        }
    }
}
