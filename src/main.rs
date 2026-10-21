mod app;
mod body;
mod cache;
mod config;
mod demo;
mod html;
mod icons;
mod local;
mod mail;
mod model;
mod notify;
mod theme;
mod tray;
mod ui;

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("letters=info,warn"),
    )
    .init();
    let config = config::Config::load();
    force_x11_for_close_to_tray(&config);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Letters")
            .with_app_id("letters")
            .with_inner_size([1280.0, 840.0])
            .with_min_inner_size([760.0, 560.0])
            .with_icon(window_icon()),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "Letters",
        options,
        Box::new(|cc| Ok(Box::new(app::LettersApp::new(cc)))),
    )
}

fn window_icon() -> egui::IconData {
    const SIZE: usize = 64;
    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    let coral = [255u8, 138, 92];
    let ink = [20u8, 14, 11];
    let mint = [159u8, 227, 192];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = (x as f32 - 31.5).abs();
            let dy = (y as f32 - 31.5).abs();
            let inside = dx <= 26.0 && dy <= 26.0
                || (dx <= 31.5
                    && dy <= 31.5
                    && (dx - 26.0).max(0.0).hypot((dy - 26.0).max(0.0)) <= 5.5);
            if inside {
                let index = (y * SIZE + x) * 4;
                rgba[index..index + 4].copy_from_slice(&[coral[0], coral[1], coral[2], 255]);
            }
        }
    }
    let mut rect = |x0: usize, y0: usize, x1: usize, y1: usize| {
        for y in y0..y1 {
            for x in x0..x1 {
                if x < SIZE && y < SIZE {
                    let index = (y * SIZE + x) * 4;
                    rgba[index..index + 4].copy_from_slice(&[ink[0], ink[1], ink[2], 255]);
                }
            }
        }
    };
    rect(17, 22, 47, 25);
    rect(17, 39, 47, 42);
    rect(17, 22, 20, 42);
    rect(44, 22, 47, 42);
    for step in 0..=13 {
        let y = 24 + (step * 9) / 13;
        for (x, _) in [(19 + step, 0), (45 - step, 0)] {
            for oy in 0..3 {
                for ox in 0..2 {
                    let px = x + ox;
                    let py = y + oy;
                    if px < SIZE && py < SIZE {
                        let index = (py * SIZE + px) * 4;
                        rgba[index..index + 4].copy_from_slice(&[ink[0], ink[1], ink[2], 255]);
                    }
                }
            }
        }
    }
    for y in 12..21 {
        for x in 42..51 {
            let distance = ((x as f32 - 46.0).powi(2) + (y as f32 - 16.0).powi(2)).sqrt();
            if distance <= 4.0 {
                let index = (y * SIZE + x) * 4;
                rgba[index..index + 4].copy_from_slice(&[mint[0], mint[1], mint[2], 255]);
            }
        }
    }
    egui::IconData {
        rgba,
        width: SIZE as u32,
        height: SIZE as u32,
    }
}

#[cfg(target_os = "linux")]
fn force_x11_for_close_to_tray(config: &config::Config) {
    if !config.close_to_tray {
        return;
    }
    let has_wayland = std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var_os("WAYLAND_SOCKET").is_some();
    let has_x11 = std::env::var_os("DISPLAY").is_some();
    if !has_wayland || !has_x11 {
        return;
    }
    // winit cannot hide a Wayland window, so close-to-tray uses XWayland on Linux.
    unsafe {
        std::env::remove_var("WAYLAND_DISPLAY");
        std::env::remove_var("WAYLAND_SOCKET");
    }
    log::info!("Close to tray is enabled; starting with the X11 backend (XWayland)");
}

#[cfg(not(target_os = "linux"))]
fn force_x11_for_close_to_tray(_: &config::Config) {}
