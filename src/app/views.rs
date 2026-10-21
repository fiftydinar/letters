use eframe::egui::{
    self, Align, Align2, Color32, ComboBox, CornerRadius, Id, Layout, Rect, RichText, ScrollArea,
    Sense, Stroke, TextEdit, Ui, Vec2, pos2, vec2,
};

use super::{Action, LettersApp, RowData, ToastKind};
use crate::icons::{self, Icon};
use crate::model::{
    BodyBlock, DEMO_EMAIL, Folder, FolderKind, InlineImage, MailFilter, inline_image_key,
};
use crate::theme::{
    self, ACCENT, BORDER, FAINT, INK, MINT, MUTED, RED, SURFACE, SURFACE_2, SURFACE_3, TEXT, YELLOW,
};
use crate::ui;

#[derive(Clone, Copy)]
enum AttachmentAction {
    Save,
    Open,
}

impl LettersApp {
    pub(super) fn ui_mail(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let mut actions = Vec::new();
        let outer = ui.max_rect().shrink(12.0);
        let width = outer.width();
        let compact = width < 916.0;
        let show_folders = width >= 1116.0;
        let gap = 12.0;
        let rail_width = 76.0;

        let mut x = outer.left();
        let rail_rect = Rect::from_min_size(pos2(x, outer.top()), vec2(rail_width, outer.height()));
        x += rail_width + gap;

        let folders_rect = if show_folders {
            let rect = Rect::from_min_size(pos2(x, outer.top()), vec2(220.0, outer.height()));
            x += 220.0 + gap;
            Some(rect)
        } else {
            None
        };

        let available = (outer.right() - x).max(0.0);
        let reader_only = compact && self.compact_reader && self.selected.is_some();

        if reader_only {
            let reader_rect =
                Rect::from_min_size(pos2(x, outer.top()), vec2(available, outer.height()));
            self.ui_reader(ui, ctx, reader_rect, &mut actions, true);
        } else if compact {
            let list_rect =
                Rect::from_min_size(pos2(x, outer.top()), vec2(available, outer.height()));
            self.ui_list(ui, list_rect, &mut actions, false);
        } else {
            let list_width = if show_folders {
                380.0
            } else {
                (available * 0.34).clamp(340.0, 390.0)
            };
            let list_rect =
                Rect::from_min_size(pos2(x, outer.top()), vec2(list_width, outer.height()));
            let reader_rect = Rect::from_min_size(
                pos2(x + list_width + gap, outer.top()),
                vec2((available - list_width - gap).max(0.0), outer.height()),
            );
            self.ui_list(ui, list_rect, &mut actions, show_folders);
            self.ui_reader(ui, ctx, reader_rect, &mut actions, false);
        }

        self.ui_rail(ui, rail_rect, ctx, &mut actions);
        if let Some(folders_rect) = folders_rect {
            self.ui_folders(ui, folders_rect, &mut actions);
        }

        self.apply_actions(ctx, actions);
    }

    fn ui_rail(&mut self, ui: &mut Ui, rect: Rect, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let mut rail = ui::panel(ui, "rail_panel", rect, SURFACE, 8.0);
        rail.vertical_centered(|ui| {
            ui.add_space(6.0);
            let (logo_rect, _) = ui.allocate_exact_size(vec2(38.0, 38.0), Sense::hover());
            icons::paint_logo(ui.painter(), logo_rect);
            ui.add_space(20.0);

            let mail_selected = self.filter != MailFilter::Starred
                && self.current_folder_kind() == FolderKind::Inbox;
            let response =
                ui::icon_button(ui, Icon::Mail, mail_selected, 44.0).on_hover_text("Inbox");
            if response.clicked() {
                let inbox = self
                    .folders
                    .iter()
                    .find(|folder| folder.kind == FolderKind::Inbox)
                    .map(|folder| folder.id.clone())
                    .unwrap_or_else(|| "INBOX".to_owned());
                actions.push(Action::OpenFolder(inbox));
                actions.push(Action::Filter(MailFilter::All));
            }
            if self.unread_inbox > 0 {
                let center = response.rect.right_top() + vec2(-4.0, 4.0);
                ui.painter().circle_filled(center, 4.5, ACCENT);
            }

            let star_response =
                ui::icon_button(ui, Icon::Star, self.filter == MailFilter::Starred, 44.0)
                    .on_hover_text("Starred messages");
            if star_response.clicked() {
                actions.push(Action::Starred);
            }

            ui.add_space(4.0);
            let (compose_rect, compose_response) =
                ui.allocate_exact_size(Vec2::splat(44.0), Sense::click());
            let hovered = compose_response.hovered();
            ui.painter().rect_filled(
                compose_rect,
                CornerRadius::same(12),
                if hovered { theme::ACCENT_HI } else { ACCENT },
            );
            icons::paint(
                ui.painter(),
                Icon::NewMail,
                compose_rect.shrink(12.0),
                INK,
                1.9,
            );
            if hovered {
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            let compose_response = compose_response.on_hover_text("New message (Ctrl+N)");
            if compose_response.clicked() {
                actions.push(Action::Compose);
            }

            let bottom = 152.0;
            ui.add_space((ui.available_height() - bottom).max(10.0));

            let bell = ui::icon_button(ui, Icon::Bell, self.config.notifications, 40.0)
                .on_hover_text(if self.config.notifications {
                    "Desktop notifications are on"
                } else {
                    "Desktop notifications are off"
                });
            if bell.clicked() {
                self.config.notifications = !self.config.notifications;
                self.apply_settings();
                self.toast(
                    if self.config.notifications {
                        "Desktop notifications enabled."
                    } else {
                        "Desktop notifications disabled."
                    },
                    ToastKind::Info,
                );
            }

            let settings =
                ui::icon_button(ui, Icon::Settings, false, 40.0).on_hover_text("Settings");
            if settings.clicked() {
                actions.push(Action::Settings);
            }

            ui.add_space(6.0);
            let (avatar_rect, avatar_response) =
                ui.allocate_exact_size(vec2(34.0, 34.0), Sense::click());
            let (name, email) = if self.demo {
                ("Letters Studio", DEMO_EMAIL)
            } else {
                (
                    self.config
                        .account
                        .as_ref()
                        .map(|account| account.display_name())
                        .unwrap_or("Letters"),
                    self.config
                        .account
                        .as_ref()
                        .map(|account| account.email.as_str())
                        .unwrap_or(""),
                )
            };
            icons::paint_avatar(ui.painter(), avatar_rect.center(), 17.0, name, email, false);
            if avatar_response.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            let avatar_response = avatar_response.on_hover_text("Account settings");
            if avatar_response.clicked() {
                actions.push(Action::Settings);
            }
        });
    }

    fn ui_folders(&mut self, ui: &mut Ui, rect: Rect, actions: &mut Vec<Action>) {
        let mut panel = ui::panel(ui, "folders_panel", rect, SURFACE, 16.0);
        ui::small_label(&mut panel, "LETTERS", FAINT);
        panel.add_space(6.0);
        let title = if self.demo {
            "Demo mail"
        } else if self.config.local_mode && self.config.account.is_none() {
            "Local mail"
        } else {
            "Mail"
        };
        panel.label(
            RichText::new(title)
                .font(theme::font_bold(24.0))
                .color(TEXT),
        );
        panel.add_space(4.0);
        panel.label(
            RichText::new(if self.demo {
                "A calm place to explore Letters."
            } else {
                "Your quiet corner of the internet."
            })
            .font(theme::font_regular(11.5))
            .color(FAINT),
        );
        panel.add_space(14.0);
        let compose_width = panel.available_width();
        let can_send = self.demo || self.config.account.is_some();
        let (label, icon, action) = if can_send {
            ("New message", Icon::NewMail, Action::Compose)
        } else {
            ("Connect mailbox", Icon::Mail, Action::ConnectAccount)
        };
        if ui::primary_button(&mut panel, label, Some(icon), compose_width).clicked() {
            actions.push(action);
        }
        panel.add_space(16.0);
        panel.horizontal(|ui| {
            ui::small_label(ui, "MAILBOXES", FAINT);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let add = ui::icon_button(ui, Icon::Plus, false, 24.0).on_hover_text("New folder");
                if add.clicked() {
                    actions.push(Action::NewFolder);
                }
            });
        });
        panel.add_space(4.0);

        let footer_height = 150.0;
        let scroll_height = (panel.available_height() - footer_height - 8.0).max(80.0);
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(scroll_height)
            .show(&mut panel, |ui| {
                let folders = self.folders.clone();
                let remote = folders
                    .iter()
                    .filter(|folder| !folder.local)
                    .cloned()
                    .collect::<Vec<_>>();
                let local = folders
                    .iter()
                    .filter(|folder| folder.local)
                    .cloned()
                    .collect::<Vec<_>>();
                for folder in remote {
                    let selected = folder.id == self.current_folder;
                    if folder_row(ui, &folder, selected).clicked() {
                        actions.push(Action::OpenFolder(folder.id.clone()));
                        actions.push(Action::Filter(MailFilter::All));
                    }
                }
                if self.folders.iter().all(|folder| folder.local) {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(if self.connecting {
                            "Connecting to your mailbox…"
                        } else {
                            "No remote mailboxes yet."
                        })
                        .font(theme::font_regular(12.0))
                        .color(FAINT),
                    );
                }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui::small_label(ui, "LOCAL FOLDERS", FAINT);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let add = ui::icon_button(ui, Icon::Plus, false, 24.0)
                            .on_hover_text("New local folder");
                        if add.clicked() {
                            actions.push(Action::NewLocalFolder);
                        }
                    });
                });
                ui.add_space(4.0);
                for folder in local {
                    let selected = folder.id == self.current_folder;
                    if folder_row(ui, &folder, selected).clicked() {
                        actions.push(Action::OpenFolder(folder.id.clone()));
                        actions.push(Action::Filter(MailFilter::All));
                    }
                }
            });

        let remaining = (panel.available_height() - footer_height).max(0.0);
        panel.add_space(remaining);
        egui::Frame::new()
            .fill(SURFACE_2)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(12)
            .inner_margin(egui::Margin::same(11))
            .show(&mut panel, |ui| {
                ui.label(
                    RichText::new(if self.config.local_mode && self.config.account.is_none() {
                        "Local mailbox"
                    } else {
                        "Background sync"
                    })
                    .font(theme::font_semibold(11.5))
                    .color(TEXT),
                );
                ui.add_space(3.0);
                ui.horizontal(|ui| {
                    let (dot_rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                    ui.painter().circle_filled(
                        dot_rect.center(),
                        3.0,
                        if self.sync_error.is_some() {
                            RED
                        } else if self.syncing {
                            YELLOW
                        } else {
                            MINT
                        },
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(if self.config.local_mode && self.config.account.is_none() {
                            "Stored on this device".to_owned()
                        } else {
                            format!(
                                "Every {} s · {}",
                                self.config.sync_secs,
                                if self.tray.available() {
                                    "tray on"
                                } else {
                                    "tray off"
                                }
                            )
                        })
                        .font(theme::font_regular(10.5))
                        .color(FAINT),
                    );
                });
                let summary = self.storage_summary();
                if let Some((used, total)) = self.remote_quota {
                    let share = used as f32 / total as f32;
                    ui.add_space(5.0);
                    ui::storage_bar(ui, share, 0.0);
                    ui.add_space(3.0);
                    ui.label(
                        RichText::new(format!(
                            "Server storage {:.0}% used · {} of {}",
                            share * 100.0,
                            crate::model::format_size(used as usize),
                            crate::model::format_size(total as usize)
                        ))
                        .font(theme::font_regular(10.0))
                        .color(FAINT),
                    );
                } else if self.config.account.is_some() {
                    ui.add_space(5.0);
                    ui.label(
                        RichText::new(if self.quota_checked {
                            format!(
                                "Server storage quota not reported · {} known",
                                crate::model::format_size(summary.remote_bytes)
                            )
                        } else {
                            "Server storage: checking…".to_owned()
                        })
                        .font(theme::font_regular(10.0))
                        .color(FAINT),
                    );
                } else {
                    ui.add_space(5.0);
                    ui.label(
                        RichText::new(format!(
                            "Server storage: not connected · {} known",
                            crate::model::format_size(summary.remote_bytes)
                        ))
                        .font(theme::font_regular(10.0))
                        .color(FAINT),
                    );
                }
                ui.label(
                    RichText::new(format!(
                        "Local folders: {} messages · {}",
                        summary.local_count,
                        crate::model::format_size(summary.local_bytes)
                    ))
                    .font(theme::font_regular(10.0))
                    .color(FAINT),
                );
                ui.label(
                    RichText::new(if self.tray.available() {
                        "Close Letters to the tray anytime."
                    } else {
                        "Install a tray extension to run hidden."
                    })
                    .font(theme::font_regular(9.5))
                    .color(FAINT),
                );
            });
    }

    fn ui_list(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        actions: &mut Vec<Action>,
        show_folder_title: bool,
    ) {
        let mut panel = ui::panel(ui, "list_panel", rect, SURFACE, 14.0);
        panel.horizontal(|ui| {
            if show_folder_title {
                ui.label(
                    RichText::new(self.current_folder_name())
                        .font(theme::font_bold(24.0))
                        .color(TEXT),
                );
            } else {
                let folders = self.folders.clone();
                let current = self.current_folder_name();
                ComboBox::from_id_salt("folder_combo")
                    .selected_text(
                        RichText::new(current)
                            .font(theme::font_semibold(18.0))
                            .color(TEXT),
                    )
                    .width(180.0)
                    .show_ui(ui, |ui| {
                        for folder in folders.iter().filter(|folder| !folder.local) {
                            if ui
                                .selectable_label(self.current_folder == folder.id, &folder.name)
                                .clicked()
                            {
                                actions.push(Action::OpenFolder(folder.id.clone()));
                            }
                        }
                        if ui.selectable_label(false, "+ New remote folder").clicked() {
                            actions.push(Action::NewFolder);
                        }
                        ui.separator();
                        ui.label(
                            RichText::new("LOCAL FOLDERS")
                                .font(theme::font_semibold(9.5))
                                .color(FAINT),
                        );
                        for folder in folders.iter().filter(|folder| folder.local) {
                            if ui
                                .selectable_label(self.current_folder == folder.id, &folder.name)
                                .clicked()
                            {
                                actions.push(Action::OpenFolder(folder.id.clone()));
                            }
                        }
                        if ui.selectable_label(false, "+ New local folder").clicked() {
                            actions.push(Action::NewLocalFolder);
                        }
                    });
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let compose =
                    ui::icon_button(ui, Icon::NewMail, false, 34.0).on_hover_text("New message");
                if compose.clicked() {
                    actions.push(Action::Compose);
                }
                if self.syncing {
                    ui::spinner(ui, 30.0);
                } else {
                    let refresh = ui::icon_button(ui, Icon::Refresh, false, 34.0)
                        .on_hover_text("Refresh (Ctrl+R)");
                    if refresh.clicked() {
                        actions.push(Action::Sync);
                    }
                }
            });
        });
        let unread = self
            .current_folder()
            .map(|folder| folder.unread)
            .unwrap_or(0);
        if let Some(error) = &self.sync_error {
            panel
                .label(
                    RichText::new("Last refresh failed")
                        .font(theme::font_medium(11.0))
                        .color(RED),
                )
                .on_hover_text(error);
        } else {
            panel.label(
                RichText::new(if self.connecting {
                    "Connecting…".to_owned()
                } else if self.syncing {
                    "Checking for new mail…".to_owned()
                } else {
                    format!("{} · {}", unread_label(unread), sync_label(self.last_sync))
                })
                .font(theme::font_regular(11.0))
                .color(FAINT),
            );
        }
        panel.add_space(6.0);

        let search_width = panel.available_width().max(80.0);
        let search = TextEdit::singleline(&mut self.search)
            .id(Id::new("letters_search"))
            .hint_text("Search mail")
            .font(theme::font_regular(13.5))
            .desired_width(search_width)
            .margin(egui::Margin::symmetric(12, 8));
        panel.add(search);
        panel.add_space(2.0);
        panel.horizontal(|ui| {
            if ui::chip(ui, MailFilter::All.label(), self.filter == MailFilter::All).clicked() {
                actions.push(Action::Filter(MailFilter::All));
            }
            if ui::chip(
                ui,
                MailFilter::Unread.label(),
                self.filter == MailFilter::Unread,
            )
            .clicked()
            {
                actions.push(Action::Filter(MailFilter::Unread));
            }
            if ui::chip(
                ui,
                MailFilter::Starred.label(),
                self.filter == MailFilter::Starred,
            )
            .clicked()
            {
                actions.push(Action::Filter(MailFilter::Starred));
            }
        });
        panel.add_space(6.0);

        let rows = self.visible_rows();
        let current_loading = self.loading.contains(&self.current_folder);
        let empty_cache = !self.cache.contains_key(&self.current_folder);
        let scroll_height = panel.available_height().max(80.0);
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(scroll_height)
            .show(&mut panel, |ui| {
                if empty_cache && (current_loading || self.connecting) {
                    loading_rows(ui, 7);
                    return;
                }
                if rows.is_empty() {
                    let rect = ui.available_rect_before_wrap();
                    ui.painter().text(
                        rect.center() - vec2(0.0, 8.0),
                        Align2::CENTER_CENTER,
                        if self.search.trim().is_empty() && self.filter == MailFilter::All {
                            "This folder is empty."
                        } else {
                            "No messages match your filters."
                        },
                        theme::font_medium(13.0),
                        FAINT,
                    );
                    ui.painter().text(
                        rect.center() + vec2(0.0, 16.0),
                        Align2::CENTER_CENTER,
                        "A little quiet is a good thing.",
                        theme::font_regular(11.0),
                        theme::with_alpha(FAINT, 170),
                    );
                    return;
                }
                for row in &rows {
                    let selected = self.selected.as_ref().is_some_and(|(folder, uid)| {
                        folder == &self.current_folder && *uid == row.uid
                    });
                    let response = mail_row(ui, row, selected);
                    if response.clicked() {
                        actions.push(Action::SelectMessage(self.current_folder.clone(), row.uid));
                    }
                }
            });
    }

    fn ui_reader(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        rect: Rect,
        actions: &mut Vec<Action>,
        compact: bool,
    ) {
        let mut panel = ui::panel(ui, "reader_panel", rect, SURFACE, 18.0);
        if self.folders.is_empty() && self.connecting {
            let area = panel.available_rect_before_wrap();
            let time = ctx.input(|input| input.time) as f32;
            icons::paint_spinner(panel.painter(), area.center() - vec2(0.0, 16.0), 18.0, time);
            panel.painter().text(
                area.center() + vec2(0.0, 28.0),
                Align2::CENTER_CENTER,
                "Connecting to your mailbox…",
                theme::font_medium(13.0),
                MUTED,
            );
            ctx.request_repaint();
            return;
        }

        let signature_data = self
            .selected_message()
            .map(|message| collect_block_images(&message.body_blocks))
            .unwrap_or_default();
        let signature_textures = self.signature_textures(ctx, &signature_data);
        let Some(message) = self.selected_message() else {
            let area = panel.available_rect_before_wrap();
            ui::draw_empty_illustration(&panel, area.center() - vec2(0.0, 46.0), 0.9);
            panel.painter().text(
                area.center() + vec2(0.0, 82.0),
                Align2::CENTER_CENTER,
                "Select a message",
                theme::font_semibold(18.0),
                TEXT,
            );
            panel.painter().text(
                area.center() + vec2(0.0, 106.0),
                Align2::CENTER_CENTER,
                "A quiet place for your mail.",
                theme::font_regular(12.0),
                FAINT,
            );
            return;
        };

        let folder_id = self
            .selected
            .as_ref()
            .map(|(folder, _)| folder.clone())
            .unwrap_or_default();
        let folder_info = self
            .folders
            .iter()
            .find(|folder| folder.id == folder_id)
            .map(|folder| (folder.kind, folder.local));
        let folder_kind = folder_info
            .map(|(kind, _)| kind)
            .unwrap_or(FolderKind::Custom);
        let folder_is_local = folder_info.map(|(_, local)| local).unwrap_or(false);

        panel.horizontal(|ui| {
            if compact {
                let back =
                    ui::icon_button(ui, Icon::Back, false, 34.0).on_hover_text("Back to messages");
                if back.clicked() {
                    actions.push(Action::Back);
                }
            }
            ui.label(
                RichText::new(self.current_folder_name())
                    .font(theme::font_medium(11.5))
                    .color(FAINT),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let reply = ui::icon_button(ui, Icon::Reply, false, 34.0).on_hover_text("Reply");
                if reply.clicked() {
                    actions.push(Action::Reply);
                }
                if folder_kind != FolderKind::Trash {
                    let trash = ui::icon_button(ui, Icon::Trash, false, 34.0)
                        .on_hover_text("Move to Trash");
                    if trash.clicked() {
                        actions.push(Action::Trash(folder_id.clone(), message.uid));
                    }
                }
                if folder_kind != FolderKind::Archive {
                    let archive =
                        ui::icon_button(ui, Icon::Archive, false, 34.0).on_hover_text("Archive");
                    if archive.clicked() {
                        actions.push(Action::Archive(folder_id.clone(), message.uid));
                    }
                }
                let (star_rect, star_response) =
                    ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
                if star_response.hovered() {
                    ui.painter()
                        .rect_filled(star_rect, CornerRadius::same(9), SURFACE_2);
                }
                icons::star(
                    ui.painter(),
                    star_rect.shrink(9.0),
                    if message.starred { YELLOW } else { MUTED },
                    1.7,
                    message.starred,
                );
                let star_response = star_response.on_hover_text(if message.starred {
                    "Remove star"
                } else {
                    "Star message"
                });
                if star_response.clicked() {
                    actions.push(Action::ToggleStar(folder_id.clone(), message.uid));
                }
                if !folder_is_local {
                    let store = ui::icon_button(ui, Icon::Download, false, 34.0)
                        .on_hover_text("Store in local folders");
                    if store.clicked() {
                        actions.push(Action::StoreLocally(folder_id.clone(), message.uid));
                    }
                }
            });
        });
        panel.add_space(2.0);
        ui::divider(&mut panel);
        panel.add_space(10.0);

        let scroll_height = panel.available_height();
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(scroll_height)
            .show(&mut panel, |ui| {
                let content_width = ui.available_width().min(760.0);
                let left_pad = ((ui.available_width() - content_width) * 0.5).max(0.0);
                ui.horizontal_top(|ui| {
                    ui.add_space(left_pad);
                    ui.vertical(|ui| {
                        ui.set_width(content_width);
                        ui.label(
                            RichText::new(&message.subject)
                                .font(theme::font_bold(25.0))
                                .color(TEXT)
                                .line_height(Some(32.0)),
                        );
                        ui.add_space(14.0);
                        ui.horizontal(|ui| {
                            let (avatar_rect, _) =
                                ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                            icons::paint_avatar(
                                ui.painter(),
                                avatar_rect.center(),
                                20.0,
                                message.sender_label(),
                                &message.from_email,
                                false,
                            );
                            ui.add_space(4.0);
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(message.sender_label())
                                        .font(theme::font_semibold(14.0))
                                        .color(TEXT),
                                );
                                ui.label(
                                    RichText::new(&message.from_email)
                                        .font(theme::font_regular(11.5))
                                        .color(FAINT),
                                );
                            });
                            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(message.long_date())
                                            .font(theme::font_regular(10.5))
                                            .color(FAINT),
                                    );
                                    if !message.to.is_empty() {
                                        ui.label(
                                            RichText::new(format!("to {}", message.to))
                                                .font(theme::font_regular(10.5))
                                                .color(FAINT),
                                        );
                                    }
                                });
                            });
                        });
                        ui.add_space(22.0);
                        if message.body_blocks.is_empty() {
                            reader_body(ui, &message.body);
                        } else {
                            render_blocks(ui, &message.body_blocks, &signature_textures, 0);
                        }
                        if !message.attachments.is_empty() {
                            let saved_paths = message
                                .attachments
                                .iter()
                                .enumerate()
                                .map(|(index, attachment)| {
                                    let key = (folder_id.clone(), message.uid, index);
                                    self.saved_attachments
                                        .get(&key)
                                        .cloned()
                                        .filter(|path| path.exists())
                                        .or_else(|| {
                                            super::find_saved_attachment(
                                                &attachment.name,
                                                attachment.size,
                                            )
                                        })
                                })
                                .collect::<Vec<_>>();
                            ui.add_space(14.0);
                            ui::small_label(ui, "ATTACHMENTS", FAINT);
                            ui.add_space(6.0);
                            for (index, attachment) in message.attachments.iter().enumerate() {
                                let saved = saved_paths[index].is_some();
                                match attachment_row(ui, attachment, saved) {
                                    Some(AttachmentAction::Save) => {
                                        actions.push(Action::SaveAttachment(
                                            folder_id.clone(),
                                            message.uid,
                                            index,
                                        ));
                                    }
                                    Some(AttachmentAction::Open) => {
                                        actions.push(Action::OpenAttachment(
                                            folder_id.clone(),
                                            message.uid,
                                            index,
                                        ));
                                    }
                                    None => {}
                                }
                            }
                        }
                        ui.add_space(26.0);
                        ui.horizontal(|ui| {
                            if ui::secondary_button(ui, "Reply", Some(Icon::Reply), 120.0).clicked()
                            {
                                actions.push(Action::Reply);
                            }
                            if !folder_is_local
                                && ui::secondary_button(
                                    ui,
                                    "Store locally",
                                    Some(Icon::Download),
                                    150.0,
                                )
                                .clicked()
                            {
                                actions.push(Action::StoreLocally(folder_id.clone(), message.uid));
                            }
                        });
                        ui.add_space(20.0);
                    });
                });
            });
    }
}

fn folder_row(ui: &mut Ui, folder: &Folder, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::click());
    let hovered = response.hovered();
    if selected {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(9), theme::ACCENT_SOFT);
    } else if hovered {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(9), SURFACE_2);
    }
    let icon = folder_icon(folder.kind);
    let icon_rect =
        Rect::from_center_size(pos2(rect.left() + 18.0, rect.center().y), Vec2::splat(16.0));
    let color = if selected {
        ACCENT
    } else if hovered {
        TEXT
    } else {
        MUTED
    };
    icons::paint(ui.painter(), icon, icon_rect, color, 1.5);
    let count_width = if folder.unread > 0 {
        let text = folder.unread.to_string();
        let width = ui::text_width(ui, &text, theme::font_semibold(10.0));
        ui.painter().text(
            pos2(rect.right() - 12.0, rect.center().y),
            Align2::RIGHT_CENTER,
            &text,
            theme::font_semibold(10.0),
            if selected { ACCENT } else { FAINT },
        );
        width + 12.0
    } else {
        0.0
    };
    let text_width = (rect.width() - 38.0 - count_width).max(20.0);
    ui::text(
        ui,
        pos2(rect.left() + 34.0, rect.center().y - 8.5),
        &folder.name,
        if selected {
            theme::font_semibold(12.5)
        } else {
            theme::font_medium(12.5)
        },
        if selected { TEXT } else { MUTED },
        text_width,
    );
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn folder_icon(kind: FolderKind) -> Icon {
    match kind {
        FolderKind::Inbox => Icon::Inbox,
        FolderKind::Sent => Icon::Send,
        FolderKind::Drafts => Icon::Draft,
        FolderKind::Archive => Icon::Archive,
        FolderKind::Trash => Icon::Trash,
        FolderKind::Custom => Icon::Mail,
    }
}

fn mail_row(ui: &mut Ui, row: &RowData, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 84.0), Sense::click());
    let hovered = response.hovered();
    if selected {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(12), theme::ACCENT_SOFT);
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(12),
            Stroke::new(1.0, theme::with_alpha(ACCENT, 90)),
            egui::StrokeKind::Inside,
        );
    } else if hovered {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(12), SURFACE_2);
    }
    let avatar_center = pos2(rect.left() + 32.0, rect.top() + 32.0);
    icons::paint_avatar(
        ui.painter(),
        avatar_center,
        18.0,
        &row.sender,
        &row.sender,
        selected,
    );
    if row.unread {
        ui.painter()
            .circle_filled(avatar_center + vec2(14.0, -14.0), 4.0, ACCENT);
    }
    let text_left = rect.left() + 62.0;
    let right_pad = 12.0;
    let time_width = ui::text_width(ui, &row.time, theme::font_regular(10.0));
    ui::text(
        ui,
        pos2(rect.right() - right_pad - time_width, rect.top() + 11.0),
        &row.time,
        theme::font_regular(10.0),
        FAINT,
        time_width + 2.0,
    );
    let sender_width = (rect.width() - 62.0 - time_width - 22.0).max(30.0);
    ui::text(
        ui,
        pos2(text_left, rect.top() + 10.0),
        &row.sender,
        if row.unread {
            theme::font_semibold(12.5)
        } else {
            theme::font_medium(12.5)
        },
        if row.unread { TEXT } else { MUTED },
        sender_width,
    );
    let subject_width = (rect.width() - 74.0).max(30.0);
    ui::text(
        ui,
        pos2(text_left, rect.top() + 31.0),
        &row.subject,
        if row.unread {
            theme::font_semibold(12.5)
        } else {
            theme::font_regular(12.5)
        },
        if row.unread {
            TEXT
        } else {
            theme::with_alpha(TEXT, 210)
        },
        subject_width,
    );
    let preview_width = (rect.width() - 94.0).max(30.0);
    ui::text(
        ui,
        pos2(text_left, rect.top() + 53.0),
        &row.preview,
        theme::font_regular(10.8),
        if row.unread {
            theme::with_alpha(MUTED, 230)
        } else {
            FAINT
        },
        preview_width,
    );
    if row.starred {
        icons::star(
            ui.painter(),
            Rect::from_center_size(
                pos2(rect.right() - 20.0, rect.top() + 61.0),
                Vec2::splat(12.0),
            ),
            YELLOW,
            1.2,
            true,
        );
    }
    if row.has_attachments {
        let attach_rect = Rect::from_center_size(
            pos2(
                rect.right() - if row.starred { 39.0 } else { 20.0 },
                rect.top() + 61.0,
            ),
            Vec2::splat(12.0),
        );
        icons::paint(ui.painter(), Icon::Attach, attach_rect, FAINT, 1.1);
    }
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn loading_rows(ui: &mut Ui, count: usize) {
    let time = ui.input(|input| input.time) as f32;
    let pulse = ((time * 2.0).sin() * 0.5 + 0.5) * 0.35 + 0.25;
    for index in 0..count {
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 76.0), Sense::hover());
        let fill = Color32::from_rgba_unmultiplied(29, 33, 42, (pulse * 255.0) as u8);
        ui.painter().rect_filled(rect, CornerRadius::same(10), fill);
        ui.painter()
            .circle_filled(pos2(rect.left() + 30.0, rect.center().y), 15.0, SURFACE_3);
        let line_color = theme::with_alpha(SURFACE_3, (pulse * 255.0) as u8);
        let y = rect.top() + 18.0;
        ui.painter().rect_filled(
            Rect::from_min_size(
                pos2(rect.left() + 58.0, y),
                vec2(rect.width() * (0.42 - index as f32 * 0.015), 8.0),
            ),
            CornerRadius::same(4),
            line_color,
        );
        ui.painter().rect_filled(
            Rect::from_min_size(
                pos2(rect.left() + 58.0, y + 18.0),
                vec2(rect.width() * 0.6, 6.0),
            ),
            CornerRadius::same(3),
            theme::with_alpha(SURFACE_3, (pulse * 190.0) as u8),
        );
        ui.painter().rect_filled(
            Rect::from_min_size(
                pos2(rect.left() + 58.0, y + 34.0),
                vec2(rect.width() * 0.75, 5.0),
            ),
            CornerRadius::same(3),
            theme::with_alpha(SURFACE_3, (pulse * 150.0) as u8),
        );
    }
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(80));
}

fn reader_body(ui: &mut Ui, body: &str) {
    let mut normal = String::new();
    let mut quote = String::new();
    for line in body.lines() {
        let trimmed = line.trim_start();
        if let Some(quoted) = trimmed.strip_prefix('>') {
            flush_normal(ui, &mut normal);
            if !quote.is_empty() {
                quote.push('\n');
            }
            quote.push_str(quoted.strip_prefix(' ').unwrap_or(quoted));
        } else {
            flush_quote(ui, &mut quote);
            if line.trim().is_empty() {
                flush_normal(ui, &mut normal);
            } else {
                if !normal.is_empty() {
                    normal.push('\n');
                }
                normal.push_str(line);
            }
        }
    }
    flush_quote(ui, &mut quote);
    flush_normal(ui, &mut normal);
    if body.trim().is_empty() {
        ui.label(
            RichText::new("This message has no readable body.")
                .font(theme::font_regular(13.0))
                .color(FAINT),
        );
    }
}

fn flush_normal(ui: &mut Ui, normal: &mut String) {
    let text = normal.trim();
    if text.is_empty() {
        normal.clear();
        return;
    }
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .font(theme::font_regular(14.0))
                .color(TEXT)
                .line_height(Some(22.0)),
        )
        .wrap()
        .selectable(true),
    );
    ui.add_space(12.0);
    normal.clear();
}

fn flush_quote(ui: &mut Ui, quote: &mut String) {
    let text = quote.trim_end();
    if text.trim().is_empty() {
        quote.clear();
        return;
    }
    let frame = egui::Frame::new()
        .fill(SURFACE_2)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(8)
        .inner_margin(egui::Margin {
            left: 16,
            right: 10,
            top: 8,
            bottom: 8,
        })
        .show(ui, |ui| {
            ui::small_label(ui, "QUOTED", FAINT);
            ui.add(
                egui::Label::new(
                    RichText::new(text)
                        .font(theme::font_regular(12.5))
                        .color(MUTED)
                        .line_height(Some(19.0))
                        .italics(),
                )
                .wrap()
                .selectable(true),
            );
        });
    let rect = frame.response.rect;
    ui.painter().rect_filled(
        egui::Rect::from_min_size(
            egui::pos2(rect.left() + 1.0, rect.top() + 7.0),
            vec2(3.0, (rect.height() - 14.0).max(4.0)),
        ),
        CornerRadius::same(2),
        ACCENT,
    );
    ui.add_space(12.0);
    quote.clear();
}

fn collect_block_images(blocks: &[BodyBlock]) -> Vec<InlineImage> {
    let mut images = Vec::new();
    let mut seen = std::collections::HashSet::new();
    fn visit(
        blocks: &[BodyBlock],
        images: &mut Vec<InlineImage>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        for block in blocks {
            match block {
                BodyBlock::Quote { blocks, .. } => visit(blocks, images, seen),
                BodyBlock::Signature {
                    images: block_images,
                    ..
                } => {
                    for image in block_images {
                        if seen.insert(image.data.clone()) {
                            images.push(image.clone());
                        }
                    }
                }
                BodyBlock::Text(_) => {}
            }
        }
    }
    visit(blocks, &mut images, &mut seen);
    images
}

fn render_blocks(
    ui: &mut Ui,
    blocks: &[BodyBlock],
    textures: &std::collections::HashMap<u64, egui::TextureHandle>,
    depth: usize,
) {
    for block in blocks {
        match block {
            BodyBlock::Text(text) => {
                let size = (14.0 - depth as f32 * 0.6).max(11.5);
                ui.add(
                    egui::Label::new(
                        RichText::new(text)
                            .font(theme::font_regular(size))
                            .color(if depth == 0 { TEXT } else { MUTED })
                            .line_height(Some(size + 8.0)),
                    )
                    .wrap()
                    .selectable(true),
                );
                ui.add_space(10.0);
            }
            BodyBlock::Quote { header, blocks } => {
                let frame = egui::Frame::new()
                    .fill(SURFACE_2)
                    .stroke(Stroke::new(1.0, BORDER))
                    .corner_radius(8)
                    .inner_margin(egui::Margin {
                        left: 16,
                        right: 10,
                        top: 8,
                        bottom: 8,
                    })
                    .show(ui, |ui| {
                        if let Some(header) = header {
                            ui.label(
                                RichText::new(header)
                                    .font(theme::font_medium(10.5))
                                    .color(ACCENT),
                            );
                        } else {
                            ui::small_label(ui, "QUOTED MESSAGE", FAINT);
                        }
                        ui.add_space(4.0);
                        render_blocks(ui, blocks, textures, depth + 1);
                    });
                let rect = frame.response.rect;
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.left() + 1.0, rect.top() + 7.0),
                        vec2(3.0, (rect.height() - 14.0).max(4.0)),
                    ),
                    CornerRadius::same(2),
                    ACCENT,
                );
                ui.add_space(10.0);
            }
            BodyBlock::Signature { text, images } => {
                egui::Frame::new()
                    .fill(SURFACE_2)
                    .stroke(Stroke::new(1.0, BORDER))
                    .corner_radius(9)
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui::small_label(ui, "SIGNATURE", FAINT);
                        if let Some(text) = text {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(text)
                                        .font(theme::font_regular(11.5))
                                        .color(FAINT)
                                        .line_height(Some(17.0)),
                                )
                                .wrap()
                                .selectable(true),
                            );
                        }
                        if !images.is_empty() {
                            ui.add_space(6.0);
                            let max_width = if depth == 0 { 220.0 } else { 170.0 };
                            ui.horizontal_wrapped(|ui| {
                                for image in images {
                                    if let Some(texture) = textures.get(&inline_image_key(image)) {
                                        ui.add(
                                            egui::Image::new(
                                                egui::load::SizedTexture::from_handle(texture),
                                            )
                                            .max_width(max_width)
                                            .max_height(140.0)
                                            .maintain_aspect_ratio(true),
                                        );
                                    }
                                }
                            });
                        }
                    });
                ui.add_space(10.0);
            }
        }
    }
}

fn attachment_row(
    ui: &mut Ui,
    attachment: &crate::model::Attachment,
    saved: bool,
) -> Option<AttachmentAction> {
    let kind = attachment
        .mime
        .rsplit('/')
        .next()
        .filter(|kind| !kind.is_empty())
        .map(str::to_uppercase)
        .unwrap_or_else(|| "FILE".to_owned());
    let mut action = None;
    let available_width = ui.available_width();
    let buttons_width = if saved { 72.0 } else { 140.0 };
    let text_width = (available_width - 32.0 - buttons_width).max(60.0);
    egui::Frame::new()
        .fill(SURFACE_2)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(9)
        .inner_margin(egui::Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                icons::paint(ui.painter(), Icon::Attach, icon_rect, ACCENT, 1.3);
                ui.add_space(6.0);
                ui.allocate_ui_with_layout(
                    vec2(text_width, 32.0),
                    Layout::top_down(Align::Min),
                    |ui| {
                        ui.set_width(text_width);
                        ui.add(
                            egui::Label::new(
                                RichText::new(&attachment.name)
                                    .font(theme::font_medium(11.5))
                                    .color(TEXT),
                            )
                            .truncate(),
                        )
                        .on_hover_text(&attachment.name);
                        ui.label(
                            RichText::new(format!(
                                "{kind} · {}",
                                crate::model::format_size(attachment.size)
                            ))
                            .font(theme::font_regular(9.5))
                            .color(FAINT),
                        );
                    },
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui::chip(ui, "Open", true)
                        .on_hover_text("Save if needed and open with the system application")
                        .clicked()
                    {
                        action = Some(AttachmentAction::Open);
                    }
                    if !saved
                        && ui::chip(ui, "Save", false)
                            .on_hover_text("Save to Downloads/Letters")
                            .clicked()
                    {
                        action = Some(AttachmentAction::Save);
                    }
                });
            });
        });
    ui.add_space(5.0);
    action
}

fn unread_label(unread: usize) -> String {
    match unread {
        0 => "All caught up".to_owned(),
        1 => "1 unread".to_owned(),
        count => format!("{count} unread"),
    }
}

fn sync_label(last_sync: Option<std::time::Instant>) -> String {
    let Some(last_sync) = last_sync else {
        return "Not synced yet".to_owned();
    };
    let seconds = last_sync.elapsed().as_secs();
    if seconds < 10 {
        "Updated just now".to_owned()
    } else if seconds < 60 {
        format!("Updated {seconds}s ago")
    } else if seconds < 3600 {
        format!("Updated {}m ago", seconds / 60)
    } else {
        format!("Updated {}h ago", seconds / 3600)
    }
}
