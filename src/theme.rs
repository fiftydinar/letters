use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Pos2,
    Stroke, TextStyle, Theme, ThemePreference, Vec2, Visuals,
};

pub const BG: Color32 = Color32::from_rgb(10, 11, 15);
pub const SURFACE: Color32 = Color32::from_rgb(18, 20, 26);
pub const SURFACE_2: Color32 = Color32::from_rgb(23, 26, 33);
pub const SURFACE_3: Color32 = Color32::from_rgb(29, 33, 42);
pub const BORDER: Color32 = Color32::from_rgb(39, 43, 54);
pub const BORDER_HI: Color32 = Color32::from_rgb(55, 60, 74);
pub const TEXT: Color32 = Color32::from_rgb(244, 241, 234);
pub const MUTED: Color32 = Color32::from_rgb(166, 169, 181);
pub const FAINT: Color32 = Color32::from_rgb(112, 116, 129);
pub const ACCENT: Color32 = Color32::from_rgb(255, 138, 92);
pub const ACCENT_HI: Color32 = Color32::from_rgb(255, 160, 120);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(48, 30, 25);
pub const MINT: Color32 = Color32::from_rgb(159, 227, 192);
pub const LAVENDER: Color32 = Color32::from_rgb(184, 167, 255);
pub const YELLOW: Color32 = Color32::from_rgb(255, 209, 102);
pub const RED: Color32 = Color32::from_rgb(255, 107, 107);
pub const INK: Color32 = Color32::from_rgb(20, 14, 11);

pub fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

pub fn font_regular(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn font_medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Letters Medium".into()))
}

pub fn font_semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Letters SemiBold".into()))
}

pub fn font_bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Letters Bold".into()))
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let font_files = [
        (
            "montserrat-regular",
            include_bytes!("../assets/fonts/Montserrat-Regular.otf").as_slice(),
        ),
        (
            "montserrat-medium",
            include_bytes!("../assets/fonts/Montserrat-Medium.otf").as_slice(),
        ),
        (
            "montserrat-semibold",
            include_bytes!("../assets/fonts/Montserrat-SemiBold.otf").as_slice(),
        ),
        (
            "montserrat-bold",
            include_bytes!("../assets/fonts/Montserrat-Bold.otf").as_slice(),
        ),
    ];
    for (name, bytes) in font_files {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    }

    let default_proportional = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let default_monospace = fonts
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();

    fonts.families.insert(
        FontFamily::Proportional,
        family("montserrat-regular", &default_proportional),
    );
    fonts.families.insert(
        FontFamily::Name("Letters Medium".into()),
        family("montserrat-medium", &default_proportional),
    );
    fonts.families.insert(
        FontFamily::Name("Letters SemiBold".into()),
        family("montserrat-semibold", &default_proportional),
    );
    fonts.families.insert(
        FontFamily::Name("Letters Bold".into()),
        family("montserrat-bold", &default_proportional),
    );
    fonts.families.insert(
        FontFamily::Name("Letters Display".into()),
        family("montserrat-bold", &default_proportional),
    );
    fonts
        .families
        .insert(FontFamily::Monospace, default_monospace);
    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();
    style.visuals = dark_visuals();
    style.text_styles = [
        (TextStyle::Heading, font_bold(27.0)),
        (TextStyle::Body, font_regular(14.0)),
        (TextStyle::Button, font_medium(13.5)),
        (TextStyle::Small, font_regular(11.5)),
        (TextStyle::Monospace, FontId::monospace(12.5)),
    ]
    .into();
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(12.0, 7.0);
    style.spacing.interact_size = Vec2::new(40.0, 34.0);
    style.spacing.window_margin = Margin::same(18);
    style.spacing.menu_margin = Margin::same(8);
    style.spacing.indent = 18.0;
    style.spacing.scroll.floating = true;
    style.spacing.scroll.bar_width = 8.0;
    style.spacing.scroll.floating_width = 4.0;
    style.spacing.scroll.handle_min_length = 28.0;
    style.spacing.scroll.bar_inner_margin = 2.0;
    style.spacing.scroll.bar_outer_margin = 2.0;
    style.spacing.scroll.floating_allocated_width = 4.0;
    style.spacing.scroll.dormant_background_opacity = 0.0;
    style.spacing.scroll.dormant_handle_opacity = 0.35;
    style.animation_time = 0.12;

    ctx.set_theme(ThemePreference::Dark);
    ctx.set_style_of(Theme::Dark, style.clone());
    ctx.set_style(style);
}

fn family(primary: &str, fallback: &[String]) -> Vec<String> {
    let mut families = Vec::with_capacity(fallback.len() + 1);
    families.push(primary.to_owned());
    families.extend(fallback.iter().cloned());
    families
}

pub fn dark_visuals() -> Visuals {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = SURFACE;
    visuals.extreme_bg_color = SURFACE_2;
    visuals.faint_bg_color = SURFACE;
    visuals.code_bg_color = SURFACE_2;
    visuals.window_corner_radius = CornerRadius::same(18);
    visuals.menu_corner_radius = CornerRadius::same(12);
    visuals.window_stroke = Stroke::new(1.0, BORDER);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: [0, 16],
        blur: 48,
        spread: 0,
        color: Color32::from_black_alpha(120),
    };
    visuals.popup_shadow = egui::epaint::Shadow {
        offset: [0, 8],
        blur: 28,
        spread: 0,
        color: Color32::from_black_alpha(120),
    };
    visuals.selection.bg_fill = with_alpha(ACCENT, 64);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.hyperlink_color = ACCENT;
    visuals.warn_fg_color = YELLOW;
    visuals.error_fg_color = RED;
    visuals.weak_text_color = Some(MUTED);
    visuals.slider_trailing_fill = true;

    let w = &mut visuals.widgets;
    w.noninteractive.bg_fill = SURFACE;
    w.noninteractive.weak_bg_fill = SURFACE;
    w.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    w.noninteractive.corner_radius = CornerRadius::same(10);

    w.inactive.bg_fill = SURFACE_2;
    w.inactive.weak_bg_fill = SURFACE_2;
    w.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    w.inactive.corner_radius = CornerRadius::same(10);

    w.hovered.bg_fill = SURFACE_3;
    w.hovered.weak_bg_fill = SURFACE_3;
    w.hovered.bg_stroke = Stroke::new(1.0, BORDER_HI);
    w.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    w.hovered.corner_radius = CornerRadius::same(10);

    w.active.bg_fill = ACCENT_SOFT;
    w.active.weak_bg_fill = ACCENT_SOFT;
    w.active.bg_stroke = Stroke::new(1.0, with_alpha(ACCENT, 180));
    w.active.fg_stroke = Stroke::new(1.0, TEXT);
    w.active.corner_radius = CornerRadius::same(10);

    w.open.bg_fill = SURFACE_3;
    w.open.weak_bg_fill = SURFACE_3;
    w.open.bg_stroke = Stroke::new(1.0, BORDER_HI);
    w.open.fg_stroke = Stroke::new(1.0, TEXT);
    w.open.corner_radius = CornerRadius::same(10);
    visuals
}

pub fn paint_ambient(ui: &egui::Ui) {
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, BG);
    let w = rect.width();
    let h = rect.height();
    glow(
        ui,
        Pos2::new(rect.left() + w * 0.18, rect.top() - 40.0),
        360.0,
        ACCENT,
        16,
    );
    glow(
        ui,
        Pos2::new(rect.right() - w * 0.08, rect.top() + h * 0.22),
        300.0,
        LAVENDER,
        10,
    );
    glow(
        ui,
        Pos2::new(rect.left() + w * 0.42, rect.bottom() + 90.0),
        380.0,
        MINT,
        7,
    );
}

fn glow(ui: &egui::Ui, center: Pos2, radius: f32, color: Color32, steps: u32) {
    let painter = ui.painter();
    for i in (1..=steps).rev() {
        let t = i as f32 / steps as f32;
        let r = radius * t;
        let alpha = (2.0 * (1.0 - t) + 0.6) as u8;
        painter.circle_filled(center, r, with_alpha(color, alpha));
    }
}
