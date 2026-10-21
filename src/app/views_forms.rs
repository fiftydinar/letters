use eframe::egui::{
    self, Align, Align2, Layout, RichText, ScrollArea, Sense, Stroke, TextEdit, Ui, Vec2, vec2,
};

use super::{LettersApp, ToastKind};
use crate::config::{DefaultStore, PRESETS};
use crate::icons::{self, Icon};
use crate::model::{DEMO_EMAIL, Security};
use crate::theme::{self, ACCENT, BORDER, BORDER_HI, FAINT, MUTED, RED, SURFACE, SURFACE_2, TEXT};
use crate::ui;

impl LettersApp {
    pub(super) fn ui_welcome(&mut self, ui: &mut Ui, _ctx: &egui::Context) {
        let mut submit = false;
        let mut demo = false;
        let mut local = false;
        let outer = ui.max_rect().shrink(30.0);
        let wide = outer.width() >= 1010.0 && outer.height() >= 620.0;

        if wide {
            let card_width = 450.0;
            let card_rect = egui::Rect::from_min_max(
                egui::pos2(outer.right() - card_width, outer.top() + 20.0),
                egui::pos2(outer.right(), outer.bottom() - 20.0),
            );
            let left_rect = egui::Rect::from_min_max(
                egui::pos2(outer.left(), outer.top()),
                egui::pos2(card_rect.left() - 42.0, outer.bottom()),
            );
            self.ui_welcome_editorial(ui, left_rect);
            self.ui_login_card(ui, card_rect, &mut submit, &mut demo, &mut local);
        } else {
            let card_width = outer.width().min(480.0);
            let card_height = (outer.height() - 40.0).max(400.0);
            let card_rect = egui::Rect::from_center_size(
                outer.center() + vec2(0.0, 20.0),
                vec2(card_width, card_height),
            );
            let mut card = ui::panel(ui, "login_card_compact", card_rect, SURFACE, 20.0);
            let form_width = card.available_width();
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(&mut card, |ui| {
                    ui.set_width(form_width);
                    ui.horizontal(|ui| {
                        let (logo_rect, _) =
                            ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                        icons::paint_logo(ui.painter(), logo_rect);
                        ui.label(
                            RichText::new("Letters")
                                .font(theme::font_bold(19.0))
                                .color(TEXT),
                        );
                    });
                    ui.add_space(14.0);
                    self.ui_login_form(ui, &mut submit, &mut demo, &mut local);
                });
        }

        if submit {
            self.submit_login();
        }
        if demo {
            self.load_demo(true);
        }
        if local {
            self.open_local_only(true);
        }
    }

    fn ui_welcome_editorial(&self, ui: &mut Ui, rect: egui::Rect) {
        ui.painter().circle_filled(
            egui::pos2(rect.right() - 30.0, rect.top() + 70.0),
            3.0,
            ACCENT,
        );
        let mut left = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("welcome_editorial")
                .max_rect(rect)
                .layout(Layout::top_down(Align::Min)),
        );
        left.spacing_mut().item_spacing = vec2(8.0, 10.0);
        left.horizontal(|ui| {
            let (logo_rect, _) = ui.allocate_exact_size(vec2(38.0, 38.0), Sense::hover());
            icons::paint_logo(ui.painter(), logo_rect);
            ui.add_space(4.0);
            ui.label(
                RichText::new("LETTERS")
                    .font(theme::font_semibold(11.0))
                    .color(TEXT),
            );
        });
        left.add_space((rect.height() * 0.14).clamp(30.0, 120.0));
        left.label(
            RichText::new("Mail that")
                .font(theme::font_bold(52.0))
                .color(TEXT)
                .line_height(Some(58.0)),
        );
        left.label(
            RichText::new("breathes.")
                .font(theme::font_bold(52.0))
                .color(ACCENT)
                .line_height(Some(58.0)),
        );
        left.add_space(14.0);
        left.add(
            egui::Label::new(
                RichText::new(
                    "A fast, quiet email client built in Rust. Your inbox, without the weight.",
                )
                .font(theme::font_regular(15.0))
                .color(MUTED)
                .line_height(Some(23.0)),
            )
            .wrap(),
        );
        left.add_space(10.0);
        for feature in [
            "IMAP and SMTP, with app-password support",
            "Background sync, desktop notifications and tray",
            "A native egui interface that opens in a blink",
        ] {
            left.horizontal(|ui| {
                ui.painter()
                    .circle_filled(ui.cursor().min + vec2(3.0, 8.0), 2.5, ACCENT);
                ui.add_space(14.0);
                ui.label(
                    RichText::new(feature)
                        .font(theme::font_regular(12.0))
                        .color(FAINT),
                );
            });
        }
        left.add_space((left.available_height() - 160.0).max(10.0));
        ui::draw_empty_illustration(
            &left,
            egui::pos2(rect.right() - 120.0, rect.bottom() - 115.0),
            0.72,
        );
        left.label(
            RichText::new("Open source · Rust + egui · MIT")
                .font(theme::font_regular(10.5))
                .color(theme::with_alpha(FAINT, 170)),
        );
    }

    fn ui_login_card(
        &mut self,
        ui: &mut Ui,
        rect: egui::Rect,
        submit: &mut bool,
        demo: &mut bool,
        local: &mut bool,
    ) {
        let mut card = ui::panel(ui, "login_card", rect, SURFACE, 22.0);
        let form_width = card.available_width();
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(&mut card, |ui| {
                ui.set_width(form_width);
                self.ui_login_form(ui, submit, demo, local);
            });
    }

    fn ui_login_form(&mut self, ui: &mut Ui, submit: &mut bool, demo: &mut bool, local: &mut bool) {
        ui::small_label(ui, "WELCOME", ACCENT);
        ui.add_space(2.0);
        ui.label(
            RichText::new("Connect your mailbox")
                .font(theme::font_bold(23.0))
                .color(TEXT),
        );
        ui.label(
            RichText::new("Bring your existing email. No migration, no lock-in.")
                .font(theme::font_regular(12.0))
                .color(FAINT),
        );
        ui.add_space(14.0);

        ui.horizontal_wrapped(|ui| {
            for (index, preset) in PRESETS.iter().enumerate() {
                if ui::chip(ui, preset.name, self.form.preset == index).clicked() {
                    self.form.apply_preset(index);
                }
            }
        });
        ui.add_space(10.0);

        field_label(ui, "Email address");
        ui::field(ui, ui.available_width(), |ui| {
            ui.add(ui::text_field(
                &mut self.form.email,
                "you@example.com",
                false,
            ));
        });
        ui.add_space(6.0);
        field_label(ui, "Name");
        ui::field(ui, ui.available_width(), |ui| {
            ui.add(ui::text_field(
                &mut self.form.name,
                "How should we address you?",
                false,
            ));
        });
        ui.add_space(6.0);
        field_label(ui, "Password");
        ui::field(ui, ui.available_width(), |ui| {
            ui.add(ui::text_field(
                &mut self.form.password,
                "Password or app password",
                true,
            ));
        });
        ui.add_space(8.0);

        if ui::chip(
            ui,
            if self.form.advanced {
                "Hide server settings"
            } else {
                "Server settings"
            },
            self.form.advanced,
        )
        .clicked()
        {
            self.form.advanced = !self.form.advanced;
        }

        if self.form.advanced {
            ui.add_space(6.0);
            field_label(ui, "Username");
            ui::field(ui, ui.available_width(), |ui| {
                ui.add(ui::text_field(
                    &mut self.form.username,
                    "Defaults to your email address",
                    false,
                ));
            });
            ui.add_space(6.0);
            field_label(ui, "IMAP host");
            ui::field(ui, ui.available_width(), |ui| {
                ui.add(ui::text_field(
                    &mut self.form.imap_host,
                    "imap.example.com",
                    false,
                ));
            });
            ui.horizontal(|ui| {
                field_label(ui, "IMAP port");
                ui.add_space(4.0);
                width_text(ui, &mut self.form.imap_port, 58.0);
                ui.add_space(8.0);
                security_combo(ui, "imap_security", &mut self.form.imap_security);
            });
            ui.add_space(6.0);
            field_label(ui, "SMTP host");
            ui::field(ui, ui.available_width(), |ui| {
                ui.add(ui::text_field(
                    &mut self.form.smtp_host,
                    "smtp.example.com",
                    false,
                ));
            });
            ui.horizontal(|ui| {
                field_label(ui, "SMTP port");
                ui.add_space(4.0);
                width_text(ui, &mut self.form.smtp_port, 58.0);
                ui.add_space(8.0);
                security_combo(ui, "smtp_security", &mut self.form.smtp_security);
            });
        }

        ui.add_space(10.0);
        let mut remember = self.form.remember;
        if switch_row(
            ui,
            "Remember password",
            "Stored in the system keyring",
            &mut remember,
        )
        .changed()
        {
            self.form.remember = remember;
        }

        if let Some(error) = &self.form_error {
            ui.add_space(6.0);
            ui.add(
                egui::Label::new(
                    RichText::new(error)
                        .font(theme::font_regular(11.5))
                        .color(RED),
                )
                .wrap(),
            );
        }

        ui.add_space(12.0);
        let width = ui.available_width();
        if self.connecting {
            let _ = ui::primary_button(ui, "Connecting…", None, width);
            ui.ctx().request_repaint();
        } else if ui::primary_button(ui, "Connect mailbox", Some(Icon::Mail), width).clicked() {
            *submit = true;
        }
        ui.add_space(6.0);
        if ui::secondary_button(ui, "Try the demo", None, width).clicked() {
            *demo = true;
        }
        ui.add_space(6.0);
        if ui::secondary_button(ui, "Open local folders", Some(Icon::Inbox), width).clicked() {
            *local = true;
        }
        ui.add_space(10.0);
        ui.label(
            RichText::new(self.form.preset_hint())
                .font(theme::font_regular(10.5))
                .color(FAINT),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "Letters only talks to your mail server. Passwords are kept in the system keyring.",
            )
            .font(theme::font_regular(10.0))
            .color(theme::with_alpha(FAINT, 170)),
        );
    }

    pub(super) fn ui_settings(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let mut close = false;
        let mut disconnect = false;
        let mut connect = false;
        let mut test_notification = false;
        let modal = egui::Modal::new(egui::Id::new("letters_settings")).frame(
            egui::Frame::new()
                .fill(SURFACE)
                .stroke(Stroke::new(1.0, BORDER_HI))
                .corner_radius(18)
                .inner_margin(22),
        );
        let response = modal.show(ctx, |ui| {
            let max_height = (ctx.content_rect().height() - 170.0).clamp(280.0, 700.0);
            ScrollArea::vertical()
                .auto_shrink([false, true])
                .min_scrolled_height(max_height)
                .max_height(max_height)
                .show(ui, |ui| {
            ui.set_width(540.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui::small_label(ui, "PREFERENCES", ACCENT);
                    ui.label(
                        RichText::new("Settings")
                            .font(theme::font_bold(24.0))
                            .color(TEXT),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if ui::icon_button(ui, Icon::Close, false, 34.0).clicked() {
                        close = true;
                    }
                });
            });
            ui.add_space(12.0);
            ui::divider(ui);
            ui.add_space(12.0);

            ui::small_label(ui, "ACCOUNT", FAINT);
            let (name, email) = if self.demo {
                ("Demo mailbox".to_owned(), DEMO_EMAIL.to_owned())
            } else if let Some(account) = &self.config.account {
                (account.name.clone(), account.email.clone())
            } else if self.config.local_mode {
                ("Local folders".to_owned(), String::new())
            } else {
                ("Not connected".to_owned(), String::new())
            };
            ui.horizontal(|ui| {
                let (avatar_rect, _) = ui.allocate_exact_size(vec2(42.0, 42.0), Sense::hover());
                icons::paint_avatar(
                    ui.painter(),
                    avatar_rect.center(),
                    21.0,
                    &name,
                    &email,
                    false,
                );
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(name)
                            .font(theme::font_semibold(14.0))
                            .color(TEXT),
                    );
                    ui.label(
                        RichText::new(if email.is_empty() {
                            "Connect an account from the welcome screen."
                        } else {
                            &email
                        })
                        .font(theme::font_regular(11.5))
                        .color(FAINT),
                    );
                });
            });
            if let Some(account) = &self.config.account {
                ui.add_space(8.0);
                server_line(
                    ui,
                    "IMAP",
                    &account.imap.host,
                    account.imap.port,
                    account.imap.security,
                );
                server_line(
                    ui,
                    "SMTP",
                    &account.smtp.host,
                    account.smtp.port,
                    account.smtp.security,
                );
            }
            ui.add_space(14.0);
            ui::divider(ui);
            ui.add_space(12.0);

            ui::small_label(ui, "BACKGROUND", FAINT);
            ui.add_space(4.0);
            let mut notifications = self.config.notifications;
            if switch_row(
                ui,
                "Desktop notifications",
                "Show a notification for new mail",
                &mut notifications,
            )
            .changed()
            {
                self.config.notifications = notifications;
            }
            let mut save_sent_copy = self.config.save_sent_copy;
            if switch_row(
                ui,
                "Save sent copy",
                "Append sent messages to the Sent folder",
                &mut save_sent_copy,
            )
            .changed()
            {
                self.config.save_sent_copy = save_sent_copy;
            }
            if self.tray.available() {
                let mut close_to_tray = self.config.close_to_tray;
                if switch_row(
                    ui,
                    "Close to tray",
                    "Keep Letters running in the background",
                    &mut close_to_tray,
                )
                .changed()
                {
                    self.config.close_to_tray = close_to_tray;
                }
            } else {
                let mut off = false;
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Close to tray")
                                .font(theme::font_medium(12.0))
                                .color(FAINT),
                        );
                        ui.label(
                            RichText::new("Tray is unavailable in this session")
                                .font(theme::font_regular(10.0))
                                .color(FAINT),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let _ = switch(ui, &mut off);
                    });
                });
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Background sync")
                        .font(theme::font_medium(12.0))
                        .color(TEXT),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.scope(|ui| {
                        ui.spacing_mut().slider_width = 190.0;
                        ui.add(
                            egui::Slider::new(
                                &mut self.config.sync_secs,
                                super::MIN_SYNC_SECS..=super::MAX_SYNC_SECS,
                            )
                            .step_by(15.0)
                            .suffix(" s"),
                        );
                    });
                });
            });
            ui.label(
                RichText::new("How often Letters checks the inbox in the background.")
                    .font(theme::font_regular(10.5))
                    .color(FAINT),
            );
            ui.add_space(8.0);
            if ui::secondary_button(ui, "Send test notification", Some(Icon::Bell), 196.0)
                .clicked()
            {
                test_notification = true;
            }
            ui.add_space(14.0);
            ui::divider(ui);
            ui.add_space(12.0);

            ui::small_label(ui, "STORAGE", FAINT);
            let summary = self.storage_summary();
            ui.add_space(4.0);
            if let Some((used, total)) = self.remote_quota {
                let share = used as f32 / total as f32;
                ui::storage_bar(ui, share, 0.0);
                ui.add_space(3.0);
                ui.label(
                    RichText::new(format!(
                        "Server storage: {:.0}% used · {} of {}",
                        share * 100.0,
                        crate::model::format_size(used as usize),
                        crate::model::format_size(total as usize)
                    ))
                    .font(theme::font_regular(10.5))
                    .color(FAINT),
                );
            } else if self.config.account.is_some() {
                ui.label(
                    RichText::new(if self.quota_checked {
                        format!(
                            "Server storage: the server does not report a quota. {} known locally.",
                            crate::model::format_size(summary.remote_bytes)
                        )
                    } else {
                        "Server storage: checking the server quota…".to_owned()
                    })
                    .font(theme::font_regular(10.5))
                    .color(FAINT),
                );
            } else {
                ui.label(
                    RichText::new("Server storage: no account connected.")
                        .font(theme::font_regular(10.5))
                        .color(FAINT),
                );
            }
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "Local folders: {} messages · {} stored · no server quota",
                    summary.local_count,
                    crate::model::format_size(summary.local_bytes)
                ))
                .font(theme::font_regular(10.5))
                .color(FAINT),
            );
            ui.add_space(8.0);
            ui::small_label(ui, "DEFAULT STORAGE", FAINT);
            ui.add_space(3.0);
            ui.horizontal(|ui| {
                if ui::chip(
                    ui,
                    DefaultStore::Remote.label(),
                    self.config.default_store == DefaultStore::Remote,
                )
                .clicked()
                {
                    self.config.default_store = DefaultStore::Remote;
                }
                if ui::chip(
                    ui,
                    DefaultStore::Local.label(),
                    self.config.default_store == DefaultStore::Local,
                )
                .clicked()
                {
                    self.config.default_store = DefaultStore::Local;
                }
            });
            ui.label(
                RichText::new("Remote inbox keeps new mail on the server. Local folders also copies new Inbox mail into your local mailbox.")
                    .font(theme::font_regular(10.0))
                    .color(FAINT),
            );
            ui.add_space(14.0);
            ui::divider(ui);
            ui.add_space(12.0);

            ui::small_label(ui, "PUSH & TRAY", FAINT);
            ui.add_space(6.0);
            egui::Frame::new()
                .fill(SURFACE_2)
                .corner_radius(12)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                        icons::paint(ui.painter(), Icon::Wifi, icon_rect, ACCENT, 1.4);
                        ui.add_space(20.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(if self.tray.available() {
                                    "System tray is active"
                                } else {
                                    "System tray is not available"
                                })
                                .font(theme::font_semibold(11.5))
                                .color(TEXT),
                            );
                            ui.label(
                                RichText::new("UnifiedPush is not connected in this build. Background IMAP sync and desktop notifications are used instead.")
                                    .font(theme::font_regular(10.5))
                                    .color(FAINT),
                            );
                        });
                    });
                });
            });
            ui.add_space(8.0);
            ui::divider(ui);
            ui.add_space(8.0);
            ui.set_width(540.0);
            ui.horizontal(|ui| {
                if self.config.account.is_some() {
                    if ui::secondary_button(ui, "Disconnect", Some(Icon::Close), 124.0).clicked() {
                        disconnect = true;
                    }
                } else if ui::secondary_button(ui, "Connect mailbox", Some(Icon::Mail), 160.0)
                    .clicked()
                {
                    connect = true;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui::primary_button(ui, "Done", Some(Icon::Check), 108.0).clicked() {
                        close = true;
                    }
                });
            });
        });

        if test_notification {
            crate::notify::show_test();
        }
        if connect {
            self.settings_open = false;
            self.open_login();
            return;
        }
        if disconnect {
            self.logout();
            return;
        }
        if close || response.should_close() {
            self.settings_open = false;
            self.apply_settings();
        }
    }

    pub(super) fn ui_local_transfer(&mut self, ctx: &egui::Context) {
        let Some(transfer) = self.local_transfer.as_mut() else {
            return;
        };
        let folders = self.local.folders();
        let mut close = false;
        let mut submit: Option<bool> = None;
        let modal = egui::Modal::new(egui::Id::new("letters_local_transfer")).frame(
            egui::Frame::new()
                .fill(SURFACE)
                .stroke(Stroke::new(1.0, BORDER_HI))
                .corner_radius(18)
                .inner_margin(22),
        );
        let response = modal.show(ctx, |ui| {
            ui.set_width(430.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui::small_label(ui, "LOCAL FOLDERS", ACCENT);
                    ui.label(
                        RichText::new("Store this message")
                            .font(theme::font_bold(23.0))
                            .color(TEXT),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if ui::icon_button(ui, Icon::Close, false, 34.0).clicked() {
                        close = true;
                    }
                });
            });
            ui.add_space(10.0);
            ui::divider(ui);
            ui.add_space(10.0);
            ui.label(
                RichText::new("Choose a local folder")
                    .font(theme::font_semibold(10.5))
                    .color(FAINT),
            );
            ui.add_space(4.0);
            ScrollArea::vertical()
                .auto_shrink([false, true])
                .max_height(220.0)
                .show(ui, |ui| {
                    for folder in &folders {
                        if ui
                            .selectable_label(transfer.target == folder.id, &folder.name)
                            .clicked()
                        {
                            transfer.target = folder.id.clone();
                        }
                    }
                });
            ui.add_space(8.0);
            ui.label(
                RichText::new("Move removes the message from the remote account by putting it in the remote Trash. Copy keeps both.")
                    .font(theme::font_regular(10.5))
                    .color(FAINT),
            );
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui::secondary_button(ui, "Cancel", None, 100.0).clicked() {
                    close = true;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui::primary_button(ui, "Move here", Some(Icon::Download), 128.0).clicked() {
                        submit = Some(true);
                    }
                    ui.add_space(4.0);
                    if ui::secondary_button(ui, "Copy", Some(Icon::Download), 100.0).clicked() {
                        submit = Some(false);
                    }
                });
            });
        });
        if let Some(remove_remote) = submit
            && let Some(transfer) = self.local_transfer.take()
        {
            let source = self
                .folders
                .iter()
                .find(|folder| folder.id == transfer.source_folder_id && !folder.local)
                .cloned();
            if let Some(source) = source
                && let Some(worker) = &self.worker
            {
                worker.fetch_local_copy(
                    &source,
                    transfer.uid,
                    transfer.target,
                    remove_remote,
                    false,
                );
                self.toast("Storing the message locally…", ToastKind::Info);
            } else {
                self.toast("The remote account is not connected.", ToastKind::Error);
            }
            return;
        }
        if close || response.should_close() {
            self.local_transfer = None;
        }
    }

    pub(super) fn ui_new_folder(&mut self, ctx: &egui::Context) {
        if !self.new_folder_open {
            return;
        }
        let mut close = false;
        let mut submit = false;
        let modal = egui::Modal::new(egui::Id::new("letters_new_folder")).frame(
            egui::Frame::new()
                .fill(SURFACE)
                .stroke(Stroke::new(1.0, BORDER_HI))
                .corner_radius(18)
                .inner_margin(22),
        );
        let local_folder = self.new_folder_local;
        let response = modal.show(ctx, |ui| {
            ui.set_width(430.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui::small_label(
                        ui,
                        if local_folder {
                            "LOCAL FOLDERS"
                        } else {
                            "MAILBOXES"
                        },
                        ACCENT,
                    );
                    ui.label(
                        RichText::new("New folder")
                            .font(theme::font_bold(23.0))
                            .color(TEXT),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if ui::icon_button(ui, Icon::Close, false, 34.0).clicked() {
                        close = true;
                    }
                });
            });
            ui.add_space(10.0);
            ui::divider(ui);
            ui.add_space(10.0);
            field_label(ui, "Folder name");
            ui::field(ui, ui.available_width(), |ui| {
                ui.add(ui::text_field(
                    &mut self.new_folder_name,
                    "Projects",
                    false,
                ));
            });
            ui.add_space(6.0);
            ui.label(
                RichText::new(if local_folder {
                    "This folder is stored locally on this device."
                } else {
                    "Use a name like INBOX/Projects for a nested folder. The folder is created on the mail server."
                })
                .font(theme::font_regular(10.5))
                .color(FAINT),
            );
            if let Some(error) = &self.new_folder_error {
                ui.add_space(6.0);
                ui.add(
                    egui::Label::new(
                        RichText::new(error)
                            .font(theme::font_regular(11.5))
                            .color(RED),
                    )
                    .wrap(),
                );
            }
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui::secondary_button(ui, "Cancel", None, 100.0).clicked() {
                    close = true;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let label = if local_folder {
                        "Create local folder"
                    } else {
                        "Create folder"
                    };
                    if ui::primary_button(ui, label, Some(Icon::Plus), 180.0).clicked() {
                        submit = true;
                    }
                });
            });
            let enter = ui.input(|input| input.key_pressed(egui::Key::Enter));
            if enter {
                submit = true;
            }
        });
        if submit {
            self.submit_new_folder();
        }
        if close || (response.should_close() && !submit) {
            self.new_folder_open = false;
        }
    }

    pub(super) fn ui_compose(&mut self, ctx: &egui::Context) {
        let Some(mut compose) = self.compose.take() else {
            return;
        };
        let mut close = false;
        let mut send = false;
        let account = self.config.account.clone();
        let demo = self.demo;
        let modal = egui::Modal::new(egui::Id::new("letters_compose")).frame(
            egui::Frame::new()
                .fill(SURFACE)
                .stroke(Stroke::new(1.0, BORDER_HI))
                .corner_radius(18)
                .inner_margin(22),
        );
        let response = modal.show(ctx, |ui| {
            let max_height = (ctx.content_rect().height() - 120.0).clamp(280.0, 620.0);
            ScrollArea::vertical()
                .auto_shrink([false, true])
                .min_scrolled_height(max_height)
                .max_height(max_height)
                .show(ui, |ui| {
                    ui.set_width(560.0);
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui::small_label(ui, "NEW MESSAGE", ACCENT);
                            ui.label(
                                RichText::new("Compose")
                                    .font(theme::font_bold(23.0))
                                    .color(TEXT),
                            );
                        });
                        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                            if ui::icon_button(ui, Icon::Close, false, 34.0).clicked() {
                                close = true;
                            }
                        });
                    });
                    ui.add_space(10.0);
                    ui::divider(ui);
                    ui.add_space(10.0);

                    let from = if demo {
                        format!("Letters Studio <{DEMO_EMAIL}>")
                    } else {
                        account
                            .as_ref()
                            .map(|account| format!("{} <{}>", account.name, account.email))
                            .unwrap_or_else(|| "Not connected".to_owned())
                    };
                    ui.label(
                        RichText::new(format!("From  {from}"))
                            .font(theme::font_regular(11.0))
                            .color(FAINT),
                    );
                    ui.add_space(6.0);
                    field_label(ui, "To");
                    ui::field(ui, ui.available_width(), |ui| {
                        ui.add(ui::text_field(
                            &mut compose.draft.to,
                            "name@example.com",
                            false,
                        ));
                    });
                    ui.add_space(6.0);
                    field_label(ui, "Subject");
                    ui::field(ui, ui.available_width(), |ui| {
                        ui.add(ui::text_field(
                            &mut compose.draft.subject,
                            "What is this about?",
                            false,
                        ));
                    });
                    ui.add_space(6.0);
                    field_label(ui, "Message");
                    egui::Frame::new()
                        .fill(SURFACE_2)
                        .stroke(Stroke::new(1.0, BORDER))
                        .corner_radius(10)
                        .inner_margin(egui::Margin::symmetric(12, 9))
                        .show(ui, |ui| {
                            ui.add_sized(
                                vec2(ui.available_width(), 190.0),
                                TextEdit::multiline(&mut compose.draft.body)
                                    .hint_text("Write something thoughtful…")
                                    .font(theme::font_regular(13.5))
                                    .frame(false)
                                    .desired_width(f32::INFINITY),
                            );
                        });

                    if let Some(error) = &compose.error {
                        ui.add_space(6.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(error)
                                    .font(theme::font_regular(11.5))
                                    .color(RED),
                            )
                            .wrap(),
                        );
                    }

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui::secondary_button(ui, "Cancel", None, 100.0).clicked() {
                            close = true;
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let label = if compose.sending {
                                "Sending…"
                            } else {
                                "Send message"
                            };
                            let send_response =
                                ui::primary_button(ui, label, Some(Icon::Send), 170.0);
                            if send_response.clicked() && !compose.sending {
                                send = true;
                            }
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Ctrl+Enter to send")
                            .font(theme::font_regular(10.0))
                            .color(theme::with_alpha(FAINT, 170)),
                    );
                });
        });

        let sending = compose.sending;
        let ctrl_enter =
            ctx.input(|input| input.modifiers.command && input.key_pressed(egui::Key::Enter));
        if (ctrl_enter || send) && !sending {
            self.compose = Some(compose);
            self.submit_compose();
            return;
        }
        let escape = ctx.input(|input| input.key_pressed(egui::Key::Escape));
        let _ = response;
        if close || escape {
            self.compose = None;
        } else {
            self.compose = Some(compose);
        }
        if sending {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
    }

    pub(super) fn ui_toast(&mut self, ctx: &egui::Context) {
        let Some(toast) = &self.toast else {
            return;
        };
        let age = toast.created.elapsed().as_secs_f32();
        let opacity = if age > 4.4 {
            ((5.0 - age) / 0.6).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let color = match toast.kind {
            ToastKind::Info => MUTED,
            ToastKind::Success => crate::theme::MINT,
            ToastKind::Error => RED,
        };
        egui::Area::new(egui::Id::new("letters_toast"))
            .order(egui::Order::Foreground)
            .anchor(Align2::RIGHT_BOTTOM, vec2(-24.0, -22.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(crate::theme::SURFACE_3)
                    .stroke(Stroke::new(1.0, theme::with_alpha(color, 130)))
                    .corner_radius(12)
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.set_max_width(380.0);
                        ui.horizontal(|ui| {
                            let (dot_rect, _) =
                                ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                            ui.painter().circle_filled(
                                dot_rect.center(),
                                3.5,
                                theme::with_alpha(color, (opacity * 255.0) as u8),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(&toast.text)
                                    .font(theme::font_medium(12.0))
                                    .color(theme::with_alpha(TEXT, (opacity * 255.0) as u8)),
                            );
                        });
                    });
            });
    }
}

fn field_label(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .font(theme::font_semibold(10.5))
            .color(FAINT),
    );
}

fn width_text(ui: &mut Ui, value: &mut String, width: f32) {
    ui.add_sized(
        vec2(width, 30.0),
        TextEdit::singleline(value)
            .font(theme::font_regular(12.0))
            .desired_width(width),
    );
}

fn security_combo(ui: &mut Ui, id: &str, value: &mut Security) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(
            RichText::new(value.label())
                .font(theme::font_medium(11.0))
                .color(TEXT),
        )
        .width(110.0)
        .show_ui(ui, |ui| {
            for option in Security::options() {
                ui.selectable_value(value, option, option.label());
            }
        });
}

fn switch(ui: &mut Ui, value: &mut bool) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(vec2(40.0, 22.0), Sense::click());
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    paint_switch(ui, rect, *value, response.hovered());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn switch_row(ui: &mut Ui, label: &str, subtitle: &str, value: &mut bool) -> egui::Response {
    let (rect, mut response) =
        ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    let text_width = (rect.width() - 62.0).max(20.0);
    ui::text(
        ui,
        egui::pos2(rect.left(), rect.top() + 4.0),
        label,
        theme::font_medium(12.0),
        TEXT,
        text_width,
    );
    ui::text(
        ui,
        egui::pos2(rect.left(), rect.top() + 23.0),
        subtitle,
        theme::font_regular(10.0),
        FAINT,
        text_width,
    );
    let switch_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 20.0, rect.center().y),
        vec2(40.0, 22.0),
    );
    paint_switch(ui, switch_rect, *value, response.hovered());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn paint_switch(ui: &Ui, rect: egui::Rect, value: bool, hovered: bool) {
    let fill = if value {
        if hovered { theme::ACCENT_HI } else { ACCENT }
    } else if hovered {
        crate::theme::SURFACE_3
    } else {
        SURFACE_2
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(11), fill);
    let knob_x = if value {
        rect.right() - 11.0
    } else {
        rect.left() + 11.0
    };
    ui.painter().circle_filled(
        egui::pos2(knob_x, rect.center().y),
        7.5,
        if value { crate::theme::INK } else { MUTED },
    );
}

fn server_line(ui: &mut Ui, label: &str, host: &str, port: u16, security: Security) {
    ui.label(
        RichText::new(format!("{label}  {host}:{port} · {}", security.label()))
            .font(theme::font_regular(10.5))
            .color(FAINT),
    );
}
