use eframe::egui::{self, Color32, RichText, Theme, ThemePreference};
use serde::{Deserialize, Serialize};

const ACCENTS: [(&str, [u8; 3]); 5] = [
    ("Amber", [0xf2, 0xa5, 0x3a]),
    ("Tweed", [0xd8, 0xc0, 0x8a]),
    ("Surf", [0x3f, 0xc1, 0xc9]),
    ("Candy", [0xe0, 0x4a, 0x5f]),
    ("Violet", [0x9b, 0x7b, 0xf0]),
];

const ZOOMS: [f32; 5] = [0.9, 1.0, 1.1, 1.25, 1.5];

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Prefs {
    pub theme: ThemePreference,
    pub accent: [u8; 3],
    pub zoom: f32,
    pub splash: bool,
    pub preset_dir: String,
}

impl Default for Prefs {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        Self {
            theme: ThemePreference::Dark,
            accent: ACCENTS[0].1,
            zoom: 1.0,
            splash: true,
            preset_dir: format!("{home}/guit2-presets"),
        }
    }
}

impl Prefs {
    pub fn apply(&self, ctx: &egui::Context) {
        let accent = Color32::from_rgb(self.accent[0], self.accent[1], self.accent[2]);

        let mut dark = egui::Visuals::dark();
        dark.panel_fill = Color32::from_rgb(0x1b, 0x1c, 0x20);
        dark.window_fill = Color32::from_rgb(0x23, 0x24, 0x29);
        dark.faint_bg_color = Color32::from_rgb(0x26, 0x28, 0x2e);
        dark.extreme_bg_color = Color32::from_rgb(0x12, 0x12, 0x15);

        let mut light = egui::Visuals::light();
        light.panel_fill = Color32::from_rgb(0xf3, 0xf1, 0xec);
        light.faint_bg_color = Color32::from_rgb(0xe8, 0xe5, 0xde);

        for (theme, mut visuals) in [(Theme::Dark, dark), (Theme::Light, light)] {
            visuals.selection.bg_fill = accent.gamma_multiply(0.45);
            visuals.selection.stroke.color = accent;
            ctx.set_visuals_of(theme, visuals);
        }
        ctx.set_theme(self.theme);
        ctx.set_zoom_factor(self.zoom);
        ctx.all_styles_mut(|s| s.spacing.item_spacing = egui::vec2(8.0, 6.0));
    }
}

fn section(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style())
        .fill(ui.visuals().faint_bg_color)
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(title).strong().color(crate::knob::accent(ui)));
            ui.add_space(4.0);
            add(ui);
        });
    ui.add_space(8.0);
}

pub fn ui(ui: &mut egui::Ui, prefs: &mut Prefs) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        let width = ui.available_width().min(1000.0);
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.add_space((ui.available_width() - width) / 2.0);
            ui.vertical(|ui| {
                ui.set_width(width);
                if width > 760.0 {
                    ui.columns(2, |c| {
                        appearance(&mut c[0], prefs);
                        behavior(&mut c[0], prefs);
                        about(&mut c[1]);
                        disclaimer(&mut c[1]);
                    });
                } else {
                    appearance(ui, prefs);
                    behavior(ui, prefs);
                    about(ui);
                    disclaimer(ui);
                }
            });
        });
    });
}

fn appearance(ui: &mut egui::Ui, prefs: &mut Prefs) {
    section(ui, "Appearance", |ui| {
        egui::Grid::new("appearance").num_columns(2).spacing([16.0, 10.0]).show(ui, |ui| {
            ui.label("Theme");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut prefs.theme, ThemePreference::Dark, "Dark");
                ui.selectable_value(&mut prefs.theme, ThemePreference::Light, "Light");
                ui.selectable_value(&mut prefs.theme, ThemePreference::System, "System");
            });
            ui.end_row();

            ui.label("Accent");
            ui.horizontal(|ui| {
                for (name, rgb) in ACCENTS {
                    let color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                    let swatch = egui::Button::new(RichText::new("●").color(color).size(22.0)).selected(prefs.accent == rgb);
                    if ui.add(swatch).on_hover_text(name).clicked() {
                        prefs.accent = rgb;
                    }
                }
                egui::color_picker::color_edit_button_srgb(ui, &mut prefs.accent).on_hover_text("custom");
            });
            ui.end_row();

            ui.label("UI scale");
            ui.horizontal(|ui| {
                for z in ZOOMS {
                    ui.selectable_value(&mut prefs.zoom, z, format!("{:.0}%", z * 100.0));
                }
            });
            ui.end_row();
        });
    });
}

fn behavior(ui: &mut egui::Ui, prefs: &mut Prefs) {
    section(ui, "Behavior", |ui| {
        egui::Grid::new("behavior").num_columns(2).spacing([16.0, 10.0]).show(ui, |ui| {
            ui.label("Startup");
            ui.checkbox(&mut prefs.splash, "play the startup animation");
            ui.end_row();

            ui.label("Preset folder");
            ui.vertical(|ui| {
                ui.add(egui::TextEdit::singleline(&mut prefs.preset_dir).desired_width(260.0));
                ui.weak("export saves here, import lists .json files from here");
            });
            ui.end_row();
        });
    });
}

fn about(ui: &mut egui::Ui) {
    section(ui, "About", |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("guit2").size(32.0).color(crate::knob::accent(ui)));
            ui.label(RichText::new(env!("CARGO_PKG_VERSION")).weak());
        });
        ui.label("A cross-platform editor for the Fender Mustang LT25: live parameter editing, model swapping and preset management over USB.");
        ui.add_space(8.0);
        egui::Grid::new("about").num_columns(2).spacing([16.0, 6.0]).show(ui, |ui| {
            ui.weak("License");
            ui.label("GPL-3.0");
            ui.end_row();
            ui.weak("Protocol");
            ui.vertical(|ui| {
                ui.label("protobufs + notes from Brent Maxwell's LtAmp");
                ui.hyperlink_to("github.com/brentmaxwell/LtAmp", "https://github.com/brentmaxwell/LtAmp");
            });
            ui.end_row();
            ui.weak("Built with");
            ui.label("egui, prost, hidapi");
            ui.end_row();
            ui.weak("Icons");
            ui.hyperlink_to("Lucide (ISC)", "https://lucide.dev");
            ui.end_row();
        });
    });
}

fn disclaimer(ui: &mut egui::Ui) {
    section(ui, "Disclaimer", |ui| {
        ui.label(
            "guit2 is an independent, unofficial project. It is not made, endorsed, sponsored or \
             supported by Fender Musical Instruments Corporation. Fender and Mustang are trademarks \
             of Fender Musical Instruments Corporation.",
        );
        ui.add_space(4.0);
        ui.colored_label(
            ui.visuals().warn_fg_color,
            "The Fender Mustang LT25 protocol is reverse-engineered. Saving writes presets to your amp; export them first \
             if you care about them. Use at your own risk.",
        );
    });
}
