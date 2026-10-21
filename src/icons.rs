use std::f32::consts::TAU;

use eframe::egui::{
    Align2, Color32, CornerRadius, Painter, Pos2, Rect, Shape, Stroke, Vec2,
    epaint::CubicBezierShape,
};

use crate::theme::{self, INK, with_alpha};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Mail,
    Inbox,
    Star,
    NewMail,
    Draft,
    Settings,
    Send,
    Reply,
    Archive,
    Trash,
    Attach,
    Refresh,
    Back,
    Bell,
    Check,
    Close,
    Plus,
    Download,
    Wifi,
}

pub fn paint(painter: &Painter, icon: Icon, rect: Rect, color: Color32, width: f32) {
    match icon {
        Icon::Mail => mail(painter, rect, color, width),
        Icon::Inbox => inbox(painter, rect, color, width),
        Icon::Star => star(painter, rect, color, width, false),
        Icon::NewMail => new_mail(painter, rect, color, width),
        Icon::Draft => draft(painter, rect, color, width),
        Icon::Settings => settings(painter, rect, color, width),
        Icon::Send => send(painter, rect, color, width),
        Icon::Reply => reply(painter, rect, color, width),
        Icon::Archive => archive(painter, rect, color, width),
        Icon::Trash => trash(painter, rect, color, width),
        Icon::Attach => attach(painter, rect, color, width),
        Icon::Refresh => refresh(painter, rect, color, width),
        Icon::Back => back(painter, rect, color, width),
        Icon::Bell => bell(painter, rect, color, width),
        Icon::Check => check(painter, rect, color, width),
        Icon::Close => close(painter, rect, color, width),
        Icon::Plus => plus(painter, rect, color, width),
        Icon::Download => download(painter, rect, color, width),
        Icon::Wifi => wifi(painter, rect, color, width),
    }
}

pub fn star(painter: &Painter, rect: Rect, color: Color32, width: f32, filled: bool) {
    let points = star_points(rect, 0.44, 0.19);
    if filled {
        painter.add(Shape::convex_polygon(points.to_vec(), color, Stroke::NONE));
    } else {
        painter.add(Shape::closed_line(
            points.to_vec(),
            Stroke::new(width, color),
        ));
    }
}

fn star_points(rect: Rect, outer: f32, inner: f32) -> [Pos2; 10] {
    let c = rect.center();
    let s = rect.width().min(rect.height());
    let mut points = [Pos2::ZERO; 10];
    for (i, point) in points.iter_mut().enumerate() {
        let angle = -TAU / 4.0 + i as f32 * TAU / 10.0;
        let radius = if i % 2 == 0 { outer } else { inner } * s;
        *point = c + Vec2::angled(angle) * radius;
    }
    points
}

fn stroke(rect: Rect, width: f32, color: Color32) -> Stroke {
    let scale = (rect.width().min(rect.height()) / 24.0).clamp(0.8, 1.6);
    Stroke::new((width * scale).max(1.0), color)
}

fn at(rect: Rect, x: f32, y: f32) -> Pos2 {
    let s = rect.width().min(rect.height());
    rect.center() + Vec2::new(x * s, y * s)
}

fn mail(painter: &Painter, rect: Rect, color: Color32, width: f32) {
    let stroke = stroke(rect, width, color);
    let body = Rect::from_min_max(at(rect, -0.4, -0.28), at(rect, 0.4, 0.3));
    painter.rect_stroke(
        body,
        CornerRadius::same(2),
        stroke,
        eframe::egui::StrokeKind::Middle,
    );
    painter.line_segment([at(rect, -0.38, -0.25), at(rect, 0.0, 0.05)], stroke);
    painter.line_segment([at(rect, 0.38, -0.25), at(rect, 0.0, 0.05)], stroke);
}

fn inbox(painter: &Painter, rect: Rect, color: Color32, width: f32) {
    let stroke = stroke(rect, width, color);
    let points = vec![
        at(rect, -0.42, -0.28),
        at(rect, 0.42, -0.28),
        at(rect, 0.42, 0.3),
        at(rect, 0.16, 0.3),
        at(rect, 0.08, 0.16),
        at(rect, -0.08, 0.16),
        at(rect, -0.16, 0.3),
        at(rect, -0.42, 0.3),
    ];
    painter.add(Shape::closed_line(points, stroke));
}

fn new_mail(painter: &Painter, rect: Rect, color: Color32, width: f32) {
    let stroke = stroke(rect, width, color);
    let body = Rect::from_min_max(at(rect, -0.4, -0.18), at(rect, 0.38, 0.32));
    painter.rect_stroke(
        body,
        CornerRadius::same(2),
        stroke,
        eframe::egui::StrokeKind::Middle,
    );
    painter.line_segment([at(rect, -0.38, -0.15), at(rect, 0.0, 0.08)], stroke);
    painter.line_segment([at(rect, 0.36, -0.15), at(rect, 0.08, 0.02)], stroke);
    painter.line_segment([at(rect, 0.25, -0.42), at(rect, 0.25, -0.18)], stroke);
    painter.line_segment([at(rect, 0.13, -0.3), at(rect, 0.37, -0.3)], stroke);
}

fn draft(painter: &Painter, rect: Rect, color: Color32, width: f32) {
    let stroke = stroke(rect, width, color);
    let outline = vec![
        at(rect, -0.34, -0.36),
        at(rect, 0.04, -0.36),
        at(rect, 0.24, -0.16),
        at(rect, 0.24, 0.3),
        at(rect, -0.34, 0.3),
    ];
    painter.add(Shape::closed_line(outline, stroke));
    painter.line_segment([at(rect, 0.04, -0.36), at(rect, 0.04, -0.16)], stroke);
    painter.line_segment([at(rect, 0.04, -0.16), at(rect, 0.24, -0.16)], stroke);
    painter.line_segment([at(rect, -0.2, -0.04), at(rect, 0.1, -0.04)], stroke);
    painter.line_segment([at(rect, -0.2, 0.1), at(rect, 0.1, 0.1)], stroke);
    painter.line_segment([at(rect, 0.02, 0.27), at(rect, 0.34, -0.05)], stroke);
    painter.line_segment([at(rect, 0.02, 0.27), at(rect, 0.04, 0.16)], stroke);
    painter.line_segment([at(rect, 0.02, 0.27), at(rect, 0.13, 0.25)], stroke);
}

fn settings(painter: &Painter, rect: Rect, color: Color32, width: f32) {
    let stroke = stroke(rect, width, color);
    let knob = stroke.width * 1.7;
    for (y, x) in [(-0.25, 0.12), (0.0, -0.16), (0.25, 0.05)] {
        painter.line_segment([at(rect, -0.36, y), at(rect, 0.36, y)], stroke);
        painter.circle_filled(at(rect, x, y), knob, color);
    }
}

fn send(painter: &Painter, rect: Rect, color: Color32, width: f32) {
    let stroke = stroke(rect, width, color);
    let plane = vec![
        at(rect, -0.38, -0.32),
        at(rect, 0.4, 0.0),
        at(rect, -0.38, 0.32),
        at(rect, -0.26, 0.0),
    ];
    painter.add(Shape::closed_line(plane, stroke));
    painter.line_segment([at(rect, -0.26, 0.0), at(rect, 0.2, 0.0)], stroke);
}

fn reply(painter: &Painter, rect: Rect, color: Color32, width: f32) {
    let stroke = stroke(rect, width, color);
    let points = [
        at(rect, 0.34, 0.24),
        at(rect, 0.22, -0.06),
        at(rect, -0.12, -0.08),
        at(rect, -0.3, 0.03),
    ];
    painter.add(Shape::CubicBezier(CubicBezierShape::from_points_stroke(
        points,
        false,
        Color32::TRANSPARENT,
        stroke,
    )));
    painter.line_segment([at(rect, -0.3, 0.03), at(rect, -0.12, -0.1)], stroke);
    painter.line_segment([at(rect, -0.3, 0.03), at(rect, -0.12, 0.17)], stroke);
}

fn archive(painter: &Painter, rect: Rect, color: Color32, width: f32) {
    let stroke = stroke(rect, width, color);
    let lid = Rect::from_min_max(at(rect, -0.42, -0.32), at(rect, 0.42, -0.16));
    painter.rect_stroke(
        lid,
        CornerRadius::same(1),
        stroke,
        eframe::egui::StrokeKind::Middle,
    );
    let body = Rect::from_min_max(at(rect, -0.34, -0.1), at(rect, 0.34, 0.34));
    painter.rect_stroke(
        body,
        CornerRadius::same(2),
        stroke,
        eframe::egui::StrokeKind::Middle,
    );
    painter.line_segment([at(rect, -0.12, 0.05), at(rect, 0.12, 0.05)], stroke);
}

fn trash(painter: &Painter, rect: Rect, color: Color32, width: f32) {
    let stroke = stroke(rect, width, color);
    painter.line_segment([at(rect, -0.34, -0.24), at(rect, 0.34, -0.24)], stroke);
    painter.line_segment([at(rect, -0.12, -0.32), at(rect, 0.12, -0.32)], stroke);
    let can = vec![
        at(rect, -0.24, -0.16),
        at(rect, -0.19, 0.33),
        at(rect, 0.19, 0.33),
        at(rect, 0.24, -0.16),
    ];
    painter.add(Shape::closed_line(can, stroke));
    painter.line_segment([at(rect, -0.05, -0.05), at(rect, -0.03, 0.22)], stroke);
    painter.line_segment([at(rect, 0.05, -0.05), at(rect, 0.03, 0.22)], stroke);
}

fn attach(painter: &Painter, rect: Rect, color: Color32, width: f32) {
    let stroke = stroke(rect, width, color);
    let outline = vec![
        at(rect, -0.26, -0.36),
        at(rect, 0.08, -0.36),
        at(rect, 0.28, -0.16),
        at(rect, 0.28, 0.36),
        at(rect, -0.26, 0.36),
    ];
    painter.add(Shape::closed_line(outline, stroke));
    painter.line_segment([at(rect, 0.08, -0.36), at(rect, 0.08, -0.16)], stroke);
    painter.line_segment([at(rect, 0.08, -0.16), at(rect, 0.28, -0.16)], stroke);
    painter.line_segment([at(rect, -0.14, 0.02), at(rect, 0.14, 0.02)], stroke);
    painter.line_segment([at(rect, -0.14, 0.17), at(rect, 0.14, 0.17)], stroke);
}

fn refresh(painter: &Painter, rect: Rect, color: Color32, width: f32) {
    let stroke = stroke(rect, width, color);
    let c = rect.center();
    let radius = 0.3 * rect.width().min(rect.height());
    let mut points = Vec::new();
    for i in 0..32 {
        let angle = -TAU * 0.32 + i as f32 * TAU * 0.78 / 31.0;
        points.push(c + Vec2::angled(angle) * radius);
    }
    painter.add(Shape::line(points, stroke));
    let tip = at(rect, 0.31, -0.18);
    painter.line_segment([tip, at(rect, 0.18, -0.32)], stroke);
    painter.line_segment([tip, at(rect, 0.36, -0.36)], stroke);
}

fn back(painter: &Painter, rect: Rect, color: Color32, width: f32) {
    let stroke = stroke(rect, width, color);
    painter.line_segment([at(rect, 0.3, 0.0), at(rect, -0.3, 0.0)], stroke);
    painter.line_segment([at(rect, -0.3, 0.0), at(rect, -0.08, -0.2)], stroke);
    painter.line_segment([at(rect, -0.3, 0.0), at(rect, -0.08, 0.2)], stroke);
}

fn bell(painter: &Painter, rect: Rect, color: Color32, width: f32) {
    let stroke = stroke(rect, width, color);
    let points = vec![
        at(rect, -0.3, 0.16),
        at(rect, -0.22, 0.08),
        at(rect, -0.22, -0.08),
        at(rect, -0.14, -0.24),
        at(rect, 0.0, -0.3),
        at(rect, 0.14, -0.24),
        at(rect, 0.22, -0.08),
        at(rect, 0.22, 0.08),
        at(rect, 0.3, 0.16),
    ];
    painter.add(Shape::line(points, stroke));
    painter.circle_filled(at(rect, 0.0, 0.29), stroke.width * 1.4, color);
}

fn check(painter: &Painter, rect: Rect, color: Color32, width: f32) {
    let stroke = stroke(rect, width, color);
    painter.line_segment([at(rect, -0.32, 0.02), at(rect, -0.08, 0.26)], stroke);
    painter.line_segment([at(rect, -0.08, 0.26), at(rect, 0.34, -0.24)], stroke);
}

fn close(painter: &Painter, rect: Rect, color: Color32, width: f32) {
    let stroke = stroke(rect, width, color);
    painter.line_segment([at(rect, -0.26, -0.26), at(rect, 0.26, 0.26)], stroke);
    painter.line_segment([at(rect, 0.26, -0.26), at(rect, -0.26, 0.26)], stroke);
}

fn plus(painter: &Painter, rect: Rect, color: Color32, width: f32) {
    let stroke = stroke(rect, width, color);
    painter.line_segment([at(rect, 0.0, -0.3), at(rect, 0.0, 0.3)], stroke);
    painter.line_segment([at(rect, -0.3, 0.0), at(rect, 0.3, 0.0)], stroke);
}

fn download(painter: &Painter, rect: Rect, color: Color32, width: f32) {
    let stroke = stroke(rect, width, color);
    painter.line_segment([at(rect, 0.0, -0.3), at(rect, 0.0, 0.16)], stroke);
    painter.line_segment([at(rect, 0.0, 0.16), at(rect, -0.18, -0.02)], stroke);
    painter.line_segment([at(rect, 0.0, 0.16), at(rect, 0.18, -0.02)], stroke);
    painter.line_segment([at(rect, -0.3, 0.32), at(rect, 0.3, 0.32)], stroke);
}

fn wifi(painter: &Painter, rect: Rect, color: Color32, width: f32) {
    let stroke = stroke(rect, width, color);
    let c = at(rect, 0.0, 0.27);
    let s = rect.width().min(rect.height());
    for radius in [0.13, 0.25, 0.37] {
        let mut points = Vec::new();
        for i in 0..24 {
            let t = i as f32 / 23.0;
            let angle = -TAU * 0.25 - 0.72 + 1.44 * t;
            points.push(c + Vec2::angled(angle) * radius * s);
        }
        painter.add(Shape::line(points, stroke));
    }
    painter.circle_filled(c, stroke.width * 1.15, color);
}

pub fn paint_logo(painter: &Painter, rect: Rect) {
    let radius = CornerRadius::same((rect.width() * 0.25) as u8);
    painter.rect_filled(rect, radius, theme::ACCENT);
    let body = Rect::from_center_size(
        rect.center(),
        Vec2::new(rect.width() * 0.48, rect.height() * 0.35),
    );
    let ink = Stroke::new((rect.width() * 0.05).max(1.5), INK);
    painter.rect_stroke(
        body,
        CornerRadius::same(3),
        ink,
        eframe::egui::StrokeKind::Middle,
    );
    let flap = body.center() + Vec2::new(0.0, body.height() * 0.09);
    painter.line_segment([body.left_top(), flap], ink);
    painter.line_segment([body.right_top(), flap], ink);
    let dot = rect.left_top() + Vec2::new(rect.width() * 0.77, rect.height() * 0.26);
    painter.circle_filled(dot, rect.width() * 0.05, theme::MINT);
}

pub fn paint_avatar(
    painter: &Painter,
    center: Pos2,
    radius: f32,
    name: &str,
    email: &str,
    selected: bool,
) {
    let color = avatar_color(email);
    painter.circle_filled(center, radius, with_alpha(color, 230));
    painter.circle_filled(
        center - Vec2::new(radius * 0.24, radius * 0.28),
        radius * 0.72,
        with_alpha(Color32::WHITE, 12),
    );
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(1.0, with_alpha(color, if selected { 255 } else { 130 })),
    );
    let text = crate::model::initials(name);
    painter.text(
        center + Vec2::new(0.0, -0.5),
        Align2::CENTER_CENTER,
        text,
        theme::font_semibold(radius * 0.78),
        INK,
    );
}

pub fn avatar_color(key: &str) -> Color32 {
    const COLORS: [Color32; 6] = [
        theme::ACCENT,
        theme::MINT,
        theme::LAVENDER,
        theme::YELLOW,
        Color32::from_rgb(127, 184, 255),
        Color32::from_rgb(255, 159, 178),
    ];
    let mut hash = 0u32;
    for byte in key.as_bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(u32::from(*byte));
    }
    COLORS[(hash as usize) % COLORS.len()]
}

pub fn paint_spinner(painter: &Painter, center: Pos2, radius: f32, time: f32) {
    let color = theme::ACCENT;
    let mut points = Vec::new();
    for i in 0..18 {
        let t = i as f32 / 17.0;
        let angle = time * 5.0 + t * TAU * 0.72;
        points.push(center + Vec2::angled(angle) * radius);
    }
    painter.add(Shape::line(points, Stroke::new(1.8, color)));
}
