use crate::animation;
use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId, Visuals};
use std::sync::Arc;

pub struct Palette {
    pub bg: Color32,
    pub bg_top: Color32,
    pub bg_bottom: Color32,
    pub panel: Color32,
    pub panel_alt: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub accent: Color32,
    pub accent_dim: Color32,
    pub accent_glow: Color32,
    pub border: Color32,
    pub danger: Color32,
    pub warn: Color32,
}

pub fn palette() -> Palette {
    Palette {
        bg: Color32::from_rgb(0x0C, 0x0E, 0x12),
        bg_top: Color32::from_rgb(0x1B, 0x20, 0x28),
        bg_bottom: Color32::from_rgb(0x0A, 0x0C, 0x10),
        panel: Color32::from_rgb(0x14, 0x17, 0x1D),
        panel_alt: Color32::from_rgb(0x1E, 0x23, 0x2C),
        text: Color32::from_rgb(0xE9, 0xEB, 0xEE),
        text_dim: Color32::from_rgb(0x9A, 0xA3, 0xAD),
        accent: Color32::from_rgb(0xE8, 0xB4, 0x3C),
        accent_dim: Color32::from_rgb(0x3A, 0x2E, 0x14),
        accent_glow: Color32::from_rgba_premultiplied(0xE8, 0xB4, 0x3C, 0x2E),
        border: Color32::from_rgb(0x2A, 0x30, 0x38),
        danger: Color32::from_rgb(0xE0, 0x5A, 0x4E),
        warn: Color32::from_rgb(0xE8, 0xB4, 0x3C),
    }
}

/// The display face for headings and the logo. Space Grotesk Bold.
pub fn heading_family() -> FontFamily {
    FontFamily::Name("heading".into())
}

pub fn heading_font(size: f32) -> FontId {
    FontId::new(size, heading_family())
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "SpaceGrotesk-Bold".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/SpaceGrotesk-Bold.ttf"
        ))),
    );
    fonts.font_data.insert(
        "SpaceGrotesk-SemiBold".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/SpaceGrotesk-SemiBold.ttf"
        ))),
    );
    fonts.font_data.insert(
        "Inter".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/Inter-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "Inter-Medium".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/Inter-Medium.ttf"
        ))),
    );
    fonts.font_data.insert(
        "JetBrainsMono".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/JetBrainsMono-Regular.ttf"
        ))),
    );
    // Primary faces first, egui's embedded fonts last so symbols and
    // glyphs Inter does not cover still render instead of showing boxes.
    fonts.families.insert(
        FontFamily::Proportional,
        vec![
            "Inter".to_owned(),
            "Inter-Medium".to_owned(),
            "Ubuntu-Light".to_owned(),
            "NotoEmoji-Regular".to_owned(),
            "emoji-icon-font".to_owned(),
        ],
    );
    fonts.families.insert(
        heading_family(),
        vec![
            "SpaceGrotesk-Bold".to_owned(),
            "SpaceGrotesk-SemiBold".to_owned(),
            "Inter-Medium".to_owned(),
        ],
    );
    fonts.families.insert(
        FontFamily::Monospace,
        vec![
            "JetBrainsMono".to_owned(),
            "Hack".to_owned(),
            "NotoEmoji-Regular".to_owned(),
        ],
    );
    ctx.set_fonts(fonts);
}

pub fn apply_theme(ctx: &egui::Context) {
    install_fonts(ctx);
    let p = palette();
    let mut visuals = Visuals::dark();
    visuals.panel_fill = p.panel;
    visuals.window_fill = p.panel_alt;
    visuals.extreme_bg_color = p.bg;
    visuals.window_corner_radius = egui::CornerRadius::same(12);
    visuals.menu_corner_radius = egui::CornerRadius::same(10);
    visuals.widgets.noninteractive.bg_fill = p.panel;
    visuals.widgets.inactive.bg_fill = p.panel_alt;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x27, 0x2E, 0x38);
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
    for w in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
    ] {
        w.corner_radius = egui::CornerRadius::same(8);
    }
    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style.text_styles.insert(
            egui::TextStyle::Body,
            FontId::new(15.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            FontId::new(15.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            FontId::new(22.0, heading_family()),
        );
        style.text_styles.insert(
            egui::TextStyle::Monospace,
            FontId::new(13.0, FontFamily::Monospace),
        );
        style.text_styles.insert(
            egui::TextStyle::Small,
            FontId::new(12.5, FontFamily::Proportional),
        );
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style.spacing.indent = 18.0;
    });
}

/// A 1x256 vertical gradient, generated once and cached in the egui data
/// store. The top rows carry a faint accent tint; the rest fades from the
/// top color to the bottom color. Dark only, never purple-blue.
pub fn gradient_texture(
    ctx: &egui::Context,
    top: Color32,
    bottom: Color32,
    accent: Color32,
) -> egui::TextureHandle {
    let id = egui::Id::new("bg-gradient");
    if let Some(tex) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return tex;
    }
    let height = 256usize;
    let mut pixels = Vec::with_capacity(height);
    for y in 0..height {
        let t = y as f32 / (height - 1) as f32;
        let mut c = lerp_color(top, bottom, t);
        // Faint accent tint along the very top edge, fading out quickly.
        let tint = (1.0 - t * 12.0).clamp(0.0, 1.0) * 0.16;
        if tint > 0.0 {
            c = lerp_color(c, accent, tint);
        }
        pixels.push(c);
    }
    let image = egui::ColorImage::new([1, height], pixels);
    let tex = ctx.load_texture("bg-gradient", image, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    tex
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
    )
}

/// Paint the vertical gradient over the whole area of `ui`, behind the
/// widgets added after this call.
pub fn paint_gradient_background(ui: &mut egui::Ui) {
    let p = palette();
    let tex = gradient_texture(ui.ctx(), p.bg_top, p.bg_bottom, p.accent);
    let rect = ui.max_rect();
    ui.painter().image(
        tex.id(),
        rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

/// The standard card: rounded, bordered, slightly lifted with a soft
/// shadow. Use for game rows, snapshot rows, and stat blocks.
pub fn card_frame(p: &Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(p.panel_alt)
        .stroke(egui::Stroke::new(1.0f32, p.border))
        .corner_radius(12)
        .shadow(egui::Shadow {
            offset: [0, 2],
            blur: 10,
            spread: 0,
            color: Color32::from_black_alpha(70),
        })
        .inner_margin(egui::Margin::symmetric(14, 12))
}

/// A sidebar navigation item: icon plus label, accent-tinted when active
/// with an accent bar on its left edge.
pub fn nav_button(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    selected: bool,
    collapsed: bool,
) -> egui::Response {
    let p = palette();
    let text = if collapsed {
        icon.to_string()
    } else {
        format!("{}  {}", icon, label)
    };
    let rich = if selected {
        egui::RichText::new(text).color(p.accent)
    } else {
        egui::RichText::new(text).color(p.text)
    };
    let mut button = egui::Button::new(rich).min_size(egui::vec2(0.0, 36.0));
    if selected {
        button = button.fill(p.accent_dim);
    }
    let resp = ui.add(button);
    if selected {
        let rect = resp.rect;
        ui.painter().rect_filled(
            egui::Rect::from_min_max(
                rect.min + egui::vec2(0.0, 7.0),
                rect.min + egui::vec2(3.0, rect.height() - 7.0),
            ),
            1.5,
            p.accent,
        );
    }
    resp
}

/// A primary action button: accent fill, dark text, rounded.
pub fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let p = palette();
    ui.add(
        egui::Button::new(
            egui::RichText::new(label)
                .color(Color32::from_rgb(0x0C, 0x0E, 0x12))
                .font(FontId::new(15.0, FontFamily::Proportional)),
        )
        .fill(p.accent),
    )
}

/// A small dashboard stat card: big value in the display face, dim label
/// underneath.
pub fn stat_card(ui: &mut egui::Ui, label: &str, value: &str) {
    let p = palette();
    card_frame(&p).show(ui, |ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(value).font(heading_font(22.0)));
            ui.label(egui::RichText::new(label).small().color(p.text_dim));
        });
    });
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
    card_frame(&p)
        .stroke(egui::Stroke::new(1.0f32, color))
        .show(ui, |ui| {
            ui.colored_label(color, text);
        });
}

pub fn ease_out_cubic(t: f32) -> f32 {
    animation::ease_out_cubic(t)
}

pub fn section_title(ui: &mut egui::Ui, title: &str) {
    let p = palette();
    ui.label(egui::RichText::new(title).font(heading_font(22.0)));
    ui.add_space(6.0);
    let (x, y) = (ui.min_rect().min.x, ui.cursor().top());
    ui.painter().rect_filled(
        egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(36.0, 3.0)),
        1.5,
        p.accent,
    );
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(8.0);
}
