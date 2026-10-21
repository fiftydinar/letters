use eframe::egui::{
    self, Align, Color32, CornerRadius, CursorIcon, FontId, Layout, Pos2, Rect, Response, Sense,
    Stroke, StrokeKind, Ui, UiBuilder, Vec2, pos2, text::LayoutJob, vec2,
};

use crate::icons::{self, Icon};
use crate::theme;

pub fn panel(
    parent: &mut Ui,
    id: impl std::hash::Hash,
    rect: Rect,
    fill: Color32,
    margin: f32,
) -> Ui {
    let radius = CornerRadius::same(16);
    parent.painter().rect_filled(rect, radius, fill);
    parent.painter().rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, theme::BORDER),
        StrokeKind::Inside,
    );
    let content = rect.shrink(margin);
    let mut ui = parent.new_child(
        UiBuilder::new()
            .id_salt(id)
            .max_rect(content)
            .layout(Layout::top_down(Align::Min)),
    );
    ui.set_clip_rect(rect.shrink(1.0));
    ui.set_min_size(content.size());
    ui
}

pub fn icon_button(ui: &mut Ui, icon: Icon, selected: bool, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let hovered = response.hovered();
    if selected {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(10), theme::ACCENT_SOFT);
    } else if hovered {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(10), theme::SURFACE_2);
    }
    let color = if selected {
        theme::ACCENT
    } else if hovered {
        theme::TEXT
    } else {
        theme::MUTED
    };
    icons::paint(ui.painter(), icon, rect.shrink(size * 0.28), color, 1.65);
    if hovered {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

pub fn primary_button(ui: &mut Ui, label: &str, icon: Option<Icon>, width: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 40.0), Sense::click());
    let hovered = response.hovered();
    let fill = if hovered {
        theme::ACCENT_HI
    } else {
        theme::ACCENT
    };
    ui.painter().rect_filled(rect, CornerRadius::same(11), fill);
    let icon_space = if icon.is_some() { 22.0 } else { 0.0 };
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_owned(), theme::font_semibold(13.5), theme::INK);
    let total = galley.size().x + icon_space;
    let start = rect.center().x - total * 0.5;
    if let Some(icon) = icon {
        icons::paint(
            ui.painter(),
            icon,
            Rect::from_center_size(pos2(start + 8.0, rect.center().y), Vec2::splat(17.0)),
            theme::INK,
            1.8,
        );
    }
    ui.painter().galley(
        pos2(start + icon_space, rect.center().y - galley.size().y * 0.5),
        galley,
        theme::INK,
    );
    if hovered {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

pub fn secondary_button(ui: &mut Ui, label: &str, icon: Option<Icon>, width: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 38.0), Sense::click());
    let hovered = response.hovered();
    let fill = if hovered {
        theme::SURFACE_3
    } else {
        theme::SURFACE_2
    };
    ui.painter().rect_filled(rect, CornerRadius::same(10), fill);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(10),
        Stroke::new(1.0, theme::BORDER),
        StrokeKind::Inside,
    );
    let icon_space = if icon.is_some() { 22.0 } else { 0.0 };
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_owned(), theme::font_medium(13.0), theme::TEXT);
    let total = galley.size().x + icon_space;
    let start = rect.center().x - total * 0.5;
    if let Some(icon) = icon {
        icons::paint(
            ui.painter(),
            icon,
            Rect::from_center_size(pos2(start + 8.0, rect.center().y), Vec2::splat(16.0)),
            theme::MUTED,
            1.6,
        );
    }
    ui.painter().galley(
        pos2(start + icon_space, rect.center().y - galley.size().y * 0.5),
        galley,
        theme::TEXT,
    );
    if hovered {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

pub fn chip(ui: &mut Ui, label: &str, selected: bool) -> Response {
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_owned(), theme::font_medium(11.5), theme::TEXT);
    let width = galley.size().x + 22.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, 26.0), Sense::click());
    let hovered = response.hovered();
    let fill = if selected {
        theme::ACCENT_SOFT
    } else if hovered {
        theme::SURFACE_3
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, CornerRadius::same(13), fill);
    if selected {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(13),
            Stroke::new(1.0, theme::with_alpha(theme::ACCENT, 130)),
            StrokeKind::Inside,
        );
    } else {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(13),
            Stroke::new(1.0, theme::with_alpha(theme::BORDER, 170)),
            StrokeKind::Inside,
        );
    }
    let color = if selected {
        theme::ACCENT
    } else if hovered {
        theme::TEXT
    } else {
        theme::MUTED
    };
    ui.painter().galley(
        pos2(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        color,
    );
    if hovered {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

pub fn text(ui: &Ui, pos: Pos2, value: &str, font: FontId, color: Color32, max_width: f32) -> Rect {
    let mut job = LayoutJob::single_section(
        value.to_owned(),
        egui::TextFormat {
            font_id: font,
            color,
            ..Default::default()
        },
    );
    job.wrap.max_width = max_width;
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    let galley = ui.painter().layout_job(job);
    let size = galley.size();
    ui.painter().galley(pos, galley, color);
    Rect::from_min_size(pos, size)
}

pub fn text_width(ui: &Ui, value: &str, font: FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(value.to_owned(), font, theme::TEXT)
        .size()
        .x
}

pub fn divider(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0, theme::BORDER);
}

pub fn small_label(ui: &mut Ui, value: &str, color: Color32) {
    ui.label(
        egui::RichText::new(value)
            .font(theme::font_semibold(10.5))
            .color(color),
    );
}

pub fn spinner(ui: &mut Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let time = ui.input(|input| input.time) as f32;
    icons::paint_spinner(ui.painter(), rect.center(), size * 0.36, time);
    ui.ctx().request_repaint();
}

pub fn field(ui: &mut Ui, width: f32, content: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .fill(theme::SURFACE_2)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(12, 7))
        .show(ui, |ui| {
            ui.set_width((width - 24.0).max(20.0));
            content(ui);
        });
}

pub fn text_field<'a>(value: &'a mut String, hint: &str, password: bool) -> egui::TextEdit<'a> {
    egui::TextEdit::singleline(value)
        .hint_text(hint)
        .password(password)
        .frame(false)
        .font(theme::font_regular(14.0))
        .desired_width(f32::INFINITY)
        .text_color(theme::TEXT)
}

pub fn draw_empty_illustration(ui: &Ui, center: Pos2, scale: f32) {
    let painter = ui.painter();
    let w = 220.0 * scale;
    let card = Rect::from_center_size(center, vec2(w, w * 0.68));
    painter.rect_filled(
        card.translate(vec2(8.0, 12.0)),
        CornerRadius::same((26.0 * scale) as u8),
        Color32::from_black_alpha(60),
    );
    painter.rect_filled(
        card,
        CornerRadius::same((26.0 * scale) as u8),
        theme::SURFACE_2,
    );
    painter.rect_stroke(
        card,
        CornerRadius::same((26.0 * scale) as u8),
        Stroke::new(1.0, theme::BORDER_HI),
        StrokeKind::Inside,
    );
    let flap = Stroke::new(2.0, theme::ACCENT);
    let left = card.left() + w * 0.16;
    let right = card.right() - w * 0.16;
    let top = card.top() + w * 0.2;
    let bottom = card.bottom() - w * 0.18;
    painter.line_segment(
        [
            pos2(left, top),
            pos2(card.center().x, card.center().y + 2.0),
        ],
        flap,
    );
    painter.line_segment(
        [
            pos2(right, top),
            pos2(card.center().x, card.center().y + 2.0),
        ],
        flap,
    );
    painter.line_segment(
        [pos2(left, top), pos2(left, bottom)],
        Stroke::new(1.5, theme::with_alpha(theme::MUTED, 120)),
    );
    painter.line_segment(
        [pos2(right, top), pos2(right, bottom)],
        Stroke::new(1.5, theme::with_alpha(theme::MUTED, 120)),
    );
    painter.line_segment(
        [pos2(left, bottom), pos2(right, bottom)],
        Stroke::new(1.5, theme::with_alpha(theme::MUTED, 120)),
    );
    let star_center = pos2(card.right() - w * 0.08, card.top() + w * 0.06);
    icons::star(
        painter,
        Rect::from_center_size(star_center, Vec2::splat(w * 0.13)),
        theme::YELLOW,
        1.5,
        true,
    );
    painter.circle_filled(
        pos2(card.left() + w * 0.06, card.bottom() - w * 0.1),
        4.0 * scale,
        theme::MINT,
    );
}

pub fn storage_bar(ui: &mut Ui, remote_share: f32, local_share: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 7.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(4), theme::SURFACE_3);
    let remote = (remote_share.clamp(0.0, 1.0) * rect.width()).round();
    if remote > 0.0 {
        let remote_rect = Rect::from_min_size(rect.min, vec2(remote, rect.height()));
        ui.painter()
            .rect_filled(remote_rect, CornerRadius::same(4), theme::ACCENT);
    }
    if local_share > 0.0 {
        let width = (local_share.clamp(0.0, 1.0) * rect.width()).round();
        let local_rect = Rect::from_min_size(
            pos2(rect.left() + remote, rect.top()),
            vec2(width, rect.height()),
        );
        ui.painter()
            .rect_filled(local_rect, CornerRadius::same(4), theme::MINT);
    }
}
