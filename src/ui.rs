use egui::{Color32, Visuals};

pub struct Palette {
    pub bg: Color32,
    pub panel: Color32,
    pub panel_alt: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub accent: Color32,
    pub accent_dim: Color32,
    pub border: Color32,
    pub danger: Color32,
    pub warn: Color32,
}

pub fn palette() -> Palette {
    Palette {
        bg: Color32::from_rgb(0x0C, 0x0E, 0x12),
        panel: Color32::from_rgb(0x14, 0x17, 0x1D),
        panel_alt: Color32::from_rgb(0x1B, 0x1F, 0x27),
        text: Color32::from_rgb(0xE9, 0xEB, 0xEE),
        text_dim: Color32::from_rgb(0x9A, 0xA3, 0xAD),
        accent: Color32::from_rgb(0xE8, 0xB4, 0x3C),
        accent_dim: Color32::from_rgb(0x3A, 0x2E, 0x14),
        border: Color32::from_rgb(0x2A, 0x30, 0x38),
        danger: Color32::from_rgb(0xE0, 0x5A, 0x4E),
        warn: Color32::from_rgb(0xE8, 0xB4, 0x3C),
    }
}

pub fn apply_theme(ctx: &egui::Context) {
    let p = palette();
    let mut visuals = Visuals::dark();
    visuals.panel_fill = p.panel;
    visuals.window_fill = p.panel_alt;
    visuals.extreme_bg_color = p.bg;
    visuals.widgets.noninteractive.bg_fill = p.panel;
    visuals.widgets.inactive.bg_fill = p.panel_alt;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x24, 0x29, 0x33);
    visuals.widgets.active.bg_fill = p.accent_dim;
    visuals.selection.bg_fill = p.accent_dim;
    visuals.selection.stroke.color = p.accent;
    visuals.widgets.inactive.fg_stroke.color = p.text;
    visuals.widgets.hovered.fg_stroke.color = p.text;
    visuals.widgets.active.fg_stroke.color = p.text;
    visuals.widgets.noninteractive.fg_stroke.color = p.text_dim;
    visuals.window_stroke.color = p.border;
    visuals.widgets.noninteractive.bg_stroke.color = p.border;
    visuals.widgets.inactive.bg_stroke.color = p.border;
    ctx.set_visuals(visuals);
}

#[derive(Clone, Copy, PartialEq)]
pub enum ToastKind {
    Ok,
    Error,
    Info,
}

pub fn toast(ui: &mut egui::Ui, text: &str, kind: ToastKind) {
    let p = palette();
    let color = match kind {
        ToastKind::Ok => p.accent,
        ToastKind::Error => p.danger,
        ToastKind::Info => p.text_dim,
    };
    egui::Frame::new()
        .fill(p.panel_alt)
        .stroke(egui::Stroke::new(1.0f32, color))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.colored_label(color, text);
        });
}

pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

pub fn section_title(ui: &mut egui::Ui, title: &str) {
    ui.heading(title);
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(8.0);
}
