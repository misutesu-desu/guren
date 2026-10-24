use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke,
    TextStyle, Theme, ThemePreference, style::ScrollAnimation,
};

#[derive(Clone, Copy)]
pub struct Pal {
    pub strip: Color32,
    pub content: Color32,
    pub status: Color32,
    pub nav: Color32,
    pub field: Color32,
    pub hover: Color32,
    pub divider: Color32,
    pub popup_stroke: Color32,
    pub sel: Color32,
    pub pressed: Color32,
    pub button: Color32,
    pub button_stroke: Color32,
    pub text: Color32,
    pub weak: Color32,
    pub accent_text: Color32,
    pub on_accent: Color32,
    pub error_text: Color32,
}

const fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

pub const DARK: Pal = Pal {
    strip: hex(0x0A0A0A),
    content: hex(0x191919),
    status: hex(0x1C1C1C),
    nav: hex(0x1F1F1F),
    field: hex(0x2C2C2C),
    hover: hex(0x2D2D2D),
    divider: hex(0x303030),
    popup_stroke: hex(0x3C3C3C),
    sel: hex(0x414141),
    pressed: hex(0x424242),
    button: hex(0x2D2D2D),
    button_stroke: hex(0x3A3A3A),
    text: hex(0xF0F0F0),
    weak: hex(0xC8C8C8),
    accent_text: hex(0x60CDFF),
    on_accent: hex(0x000000),
    error_text: hex(0xFF99A4),
};

pub const LIGHT: Pal = Pal {
    strip: hex(0xE8E8E8),
    content: hex(0xFFFFFF),
    status: hex(0xFFFFFF),
    nav: hex(0xF9F9F9),
    field: hex(0xFFFFFF),
    hover: hex(0xF0F0F0),
    divider: hex(0xE5E5E5),
    popup_stroke: hex(0xD7D7D7),
    sel: hex(0xCCE8FF),
    pressed: hex(0xDEDEDE),
    button: hex(0xFBFBFB),
    button_stroke: hex(0xDEDEDE),
    text: hex(0x1A1A1A),
    weak: hex(0x606060),
    accent_text: hex(0x005FB8),
    on_accent: hex(0xFFFFFF),
    error_text: hex(0xC42B1C),
};

pub fn pal(theme: Theme) -> &'static Pal {
    match theme {
        Theme::Dark => &DARK,
        Theme::Light => &LIGHT,
    }
}

pub const ICONS: &str = "icons";

/// Segoe UI and Segoe Fluent Icons are read from the Windows font folder.
/// Returns false when the icon font is missing so the toolbar can fall back to text.
pub fn install_fonts(ctx: &egui::Context) -> bool {
    let mut fonts = FontDefinitions::default();
    let dir = std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "C:\\Windows".into())
        .join("Fonts");

    if let Ok(bytes) = std::fs::read(dir.join("segoeui.ttf")) {
        fonts
            .font_data
            .insert("segoe".into(), Arc::new(FontData::from_owned(bytes)));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "segoe".into());
    }

    let icon_file = ["SegoeIcons.ttf", "segmdl2.ttf"]
        .iter()
        .find_map(|name| std::fs::read(dir.join(name)).ok());
    let has_icons = icon_file.is_some();
    if let Some(bytes) = icon_file {
        fonts
            .font_data
            .insert(ICONS.into(), Arc::new(FontData::from_owned(bytes)));
        fonts
            .families
            .insert(FontFamily::Name(ICONS.into()), vec![ICONS.into()]);
    }

    ctx.set_fonts(fonts);
    has_icons
}

pub fn apply(ctx: &egui::Context, pref: ThemePreference) {
    for theme in [Theme::Dark, Theme::Light] {
        let p = pal(theme);
        ctx.style_mut_of(theme, |s| {
            s.text_styles = [
                (TextStyle::Body, FontId::proportional(14.0)),
                (TextStyle::Button, FontId::proportional(14.0)),
                (TextStyle::Small, FontId::proportional(12.0)),
                (TextStyle::Heading, FontId::proportional(20.0)),
                (TextStyle::Monospace, FontId::monospace(13.0)),
            ]
            .into();

            s.spacing.item_spacing = egui::vec2(6.0, 4.0);
            s.spacing.button_padding = egui::vec2(8.0, 4.0);
            s.spacing.interact_size.y = 26.0;
            s.spacing.menu_margin = Margin::same(6);
            s.animation_time = 0.0;
            s.scroll_animation = ScrollAnimation::none();

            let v = &mut s.visuals;
            v.dark_mode = theme == Theme::Dark;
            v.override_text_color = Some(p.text);
            v.weak_text_color = Some(p.weak);
            v.hyperlink_color = p.accent_text;
            v.error_fg_color = p.error_text;
            v.panel_fill = p.content;
            v.window_fill = p.field;
            v.extreme_bg_color = p.field;
            v.window_stroke = Stroke::new(1.0, p.popup_stroke);
            v.window_corner_radius = CornerRadius::same(8);
            v.menu_corner_radius = CornerRadius::same(8);
            v.window_shadow = egui::Shadow {
                offset: [0, 8],
                blur: 16,
                spread: 0,
                color: Color32::from_black_alpha(71),
            };
            v.popup_shadow = v.window_shadow;
            v.selection.bg_fill = p.sel;
            v.selection.stroke = Stroke::new(1.0, p.accent_text);

            let w = &mut v.widgets;
            for wv in [
                &mut w.noninteractive,
                &mut w.inactive,
                &mut w.hovered,
                &mut w.active,
                &mut w.open,
            ] {
                wv.corner_radius = CornerRadius::same(5);
                wv.expansion = 0.0;
                wv.fg_stroke = Stroke::new(1.0, p.text);
            }
            w.noninteractive.bg_stroke = Stroke::new(1.0, p.divider);
            w.noninteractive.bg_fill = p.content;
            w.noninteractive.weak_bg_fill = p.content;
            w.inactive.bg_fill = p.button;
            w.inactive.weak_bg_fill = p.button;
            w.inactive.bg_stroke = Stroke::new(1.0, p.button_stroke);
            w.hovered.bg_fill = p.hover;
            w.hovered.weak_bg_fill = p.hover;
            w.hovered.bg_stroke = Stroke::NONE;
            w.active.bg_fill = p.pressed;
            w.active.weak_bg_fill = p.pressed;
            w.active.bg_stroke = Stroke::NONE;
            w.open.bg_fill = p.pressed;
            w.open.weak_bg_fill = p.pressed;
        });
    }
    ctx.set_theme(pref);
}
