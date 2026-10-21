use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use eframe::egui::{self, ViewportCommand};
use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::{CheckmarkItem, StandardItem};
use ksni::{MenuItem, ToolTip};

#[derive(Clone, Copy, Debug)]
pub enum TrayCommand {
    Show,
    Compose,
    Sync,
    Settings,
    Quit,
}

pub struct Tray {
    handle: Option<Handle<LettersTray>>,
    rx: Receiver<TrayCommand>,
}

pub struct LettersTray {
    pub unread: usize,
    pub syncing: bool,
    pub account: String,
    pub notifications: bool,
    ctx: egui::Context,
    tx: Sender<TrayCommand>,
}

impl Tray {
    pub fn spawn(ctx: egui::Context) -> Self {
        let (tx, rx) = channel();
        let tray = LettersTray {
            unread: 0,
            syncing: false,
            account: String::new(),
            notifications: true,
            ctx,
            tx,
        };
        let handle = match tray.spawn() {
            Ok(handle) => {
                log::info!("Tray service is active");
                Some(handle)
            }
            Err(err) => {
                log::info!("Tray is unavailable: {err}");
                None
            }
        };
        Self { handle, rx }
    }

    pub fn available(&self) -> bool {
        self.handle
            .as_ref()
            .is_some_and(|handle| !handle.is_closed())
    }

    pub fn update(&self, unread: usize, syncing: bool, account: &str, notifications: bool) {
        let Some(handle) = &self.handle else {
            return;
        };
        handle.update(|tray| {
            tray.unread = unread;
            tray.syncing = syncing;
            tray.account = account.to_owned();
            tray.notifications = notifications;
        });
    }

    pub fn updater(&self) -> Option<Arc<dyn Fn(usize, bool) + Send + Sync>> {
        let handle = self.handle.clone()?;
        Some(Arc::new(move |unread, syncing| {
            let _ = handle.update(|tray| {
                tray.unread = unread;
                tray.syncing = syncing;
            });
        }))
    }

    pub fn try_recv(&self) -> Option<TrayCommand> {
        self.rx.try_recv().ok()
    }
}

impl LettersTray {
    fn emit(&self, command: TrayCommand) {
        let _ = self.tx.send(command);
        self.ctx.request_repaint();
    }

    fn show_window(&self) {
        self.ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        self.ctx.send_viewport_cmd(ViewportCommand::Focus);
        self.ctx.request_repaint();
    }
}

impl ksni::Tray for LettersTray {
    fn id(&self) -> String {
        "letters".to_owned()
    }

    fn title(&self) -> String {
        if self.unread > 0 {
            format!("Letters · {}", self.unread)
        } else {
            "Letters".to_owned()
        }
    }

    fn icon_name(&self) -> String {
        "mail-unread".to_owned()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![tray_icon()]
    }

    fn tool_tip(&self) -> ToolTip {
        let description = if self.account.is_empty() {
            "Letters email client".to_owned()
        } else {
            format!("{} · {} unread", self.account, self.unread)
        };
        ToolTip {
            title: self.title(),
            description,
            icon_name: "mail-unread".to_owned(),
            icon_pixmap: vec![tray_icon()],
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.show_window();
        self.emit(TrayCommand::Show);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: "Open Letters".to_owned(),
                icon_name: "mail-read".to_owned(),
                activate: Box::new(|tray: &mut LettersTray| {
                    tray.show_window();
                    tray.emit(TrayCommand::Show);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "New message".to_owned(),
                icon_name: "document-new".to_owned(),
                activate: Box::new(|tray: &mut LettersTray| {
                    tray.show_window();
                    tray.emit(TrayCommand::Compose);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: if self.syncing {
                    "Refreshing…".to_owned()
                } else {
                    "Refresh now".to_owned()
                },
                enabled: !self.syncing,
                icon_name: "view-refresh".to_owned(),
                activate: Box::new(|tray: &mut LettersTray| {
                    tray.show_window();
                    tray.emit(TrayCommand::Sync);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            CheckmarkItem {
                label: "Desktop notifications".to_owned(),
                checked: self.notifications,
                enabled: false,
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Settings".to_owned(),
                icon_name: "preferences-system".to_owned(),
                activate: Box::new(|tray: &mut LettersTray| {
                    tray.show_window();
                    tray.emit(TrayCommand::Settings);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Quit".to_owned(),
                icon_name: "application-exit".to_owned(),
                activate: Box::new(|tray: &mut LettersTray| {
                    tray.emit(TrayCommand::Quit);
                    std::thread::spawn(|| {
                        std::thread::sleep(Duration::from_secs(1));
                        std::process::exit(0);
                    });
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

fn tray_icon() -> ksni::Icon {
    const SIZE: usize = 64;
    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    let coral = (255u8, 138u8, 92u8);
    let ink = (20u8, 14u8, 11u8);
    let mint = (159u8, 227u8, 192u8);

    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = (x as f32 - 31.5).abs();
            let dy = (y as f32 - 31.5).abs();
            let inside = dx <= 26.0 && dy <= 26.0
                || (dx <= 31.5
                    && dy <= 31.5
                    && (dx - 26.0).max(0.0).hypot((dy - 26.0).max(0.0)) <= 5.5);
            if inside {
                put(&mut rgba, SIZE, x, y, coral, 255);
            }
        }
    }

    line(&mut rgba, SIZE, (17, 22), (47, 22), ink, 3);
    line(&mut rgba, SIZE, (17, 42), (47, 42), ink, 3);
    line(&mut rgba, SIZE, (17, 22), (17, 42), ink, 3);
    line(&mut rgba, SIZE, (47, 22), (47, 42), ink, 3);
    line(&mut rgba, SIZE, (19, 24), (32, 33), ink, 3);
    line(&mut rgba, SIZE, (45, 24), (32, 33), ink, 3);

    for y in 12..21 {
        for x in 42..51 {
            let d = ((x as f32 - 46.0).powi(2) + (y as f32 - 16.0).powi(2)).sqrt();
            if d <= 4.2 {
                put(&mut rgba, SIZE, x, y, mint, 255);
            }
        }
    }

    for pixel in rgba.chunks_exact_mut(4) {
        pixel.rotate_right(1);
    }
    ksni::Icon {
        width: SIZE as i32,
        height: SIZE as i32,
        data: rgba,
    }
}

fn put(data: &mut [u8], size: usize, x: usize, y: usize, color: (u8, u8, u8), alpha: u8) {
    if x >= size || y >= size {
        return;
    }
    let index = (y * size + x) * 4;
    let src = [color.0 as f32, color.1 as f32, color.2 as f32];
    let dst = [
        data[index] as f32,
        data[index + 1] as f32,
        data[index + 2] as f32,
    ];
    let a = alpha as f32 / 255.0;
    for channel in 0..3 {
        data[index + channel] = (src[channel] * a + dst[channel] * (1.0 - a)) as u8;
    }
    data[index + 3] =
        ((data[index + 3] as f32) + alpha as f32 * (1.0 - data[index + 3] as f32 / 255.0)) as u8;
}

fn line(
    data: &mut [u8],
    size: usize,
    from: (i32, i32),
    to: (i32, i32),
    color: (u8, u8, u8),
    width: i32,
) {
    let steps = ((to.0 - from.0).abs().max((to.1 - from.1).abs())).max(1);
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let x = from.0 as f32 + (to.0 - from.0) as f32 * t;
        let y = from.1 as f32 + (to.1 - from.1) as f32 * t;
        for oy in -width..=width {
            for ox in -width..=width {
                if ox * ox + oy * oy <= width * width {
                    let px = (x.round() as i32 + ox).max(0) as usize;
                    let py = (y.round() as i32 + oy).max(0) as usize;
                    put(data, size, px, py, color, 255);
                }
            }
        }
    }
}
