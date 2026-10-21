use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use eframe::egui::{self, Id, Key, ViewportCommand};

use crate::cache;
use crate::config::{self, AccountForm, Config, DefaultStore};
use crate::demo::demo_data;
use crate::local::{self as local_store, LocalStore};
use crate::mail::{ErrorKind, MailEvent, MailWorker, Outgoing, WorkerOptions};
use crate::model::{
    Account, ComposeDraft, DEMO_EMAIL, Folder, FolderKind, InlineImage, MailFilter, Message,
    inline_image_key,
};
use crate::theme;
use crate::tray::{Tray, TrayCommand};

mod views;
mod views_forms;

const MIN_SYNC_SECS: u64 = 15;
const MAX_SYNC_SECS: u64 = 300;

pub struct LettersApp {
    config: Config,
    tray: Tray,
    worker: Option<MailWorker>,
    app_active: Arc<AtomicBool>,
    ui_live: Arc<AtomicBool>,
    pending: Option<PendingLogin>,

    welcome: bool,
    connecting: bool,
    form: AccountForm,
    form_error: Option<String>,

    folders: Vec<Folder>,
    cache: HashMap<String, Vec<Message>>,
    local: LocalStore,
    current_folder: String,
    selected: Option<(String, u32)>,
    search: String,
    filter: MailFilter,
    compact_reader: bool,

    loading: HashSet<String>,
    syncing: bool,
    last_sync: Option<Instant>,
    sync_error: Option<String>,
    remote_quota: Option<(u64, u64)>,
    quota_checked: bool,
    unread_inbox: usize,
    saved_attachments: HashMap<(String, u32, usize), PathBuf>,
    signature_textures: HashMap<u64, egui::TextureHandle>,

    compose: Option<ComposeState>,
    local_transfer: Option<LocalTransfer>,
    settings_open: bool,
    new_folder_open: bool,
    new_folder_local: bool,
    new_folder_name: String,
    new_folder_error: Option<String>,
    pending_new_folder: Option<String>,
    toast: Option<Toast>,
    demo: bool,
    hidden: bool,
    allow_close: bool,
    pending_hide: bool,
    tray_notice_shown: bool,
}

struct PendingLogin {
    account: Account,
    password: String,
    remember: bool,
}

struct ComposeState {
    draft: ComposeDraft,
    sending: bool,
    error: Option<String>,
}

struct LocalTransfer {
    source_folder_id: String,
    uid: u32,
    target: String,
}

pub(super) struct StorageSummary {
    pub remote_bytes: usize,
    pub local_bytes: usize,
    pub remote_count: usize,
    pub local_count: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToastKind {
    Info,
    Success,
    Error,
}

struct Toast {
    text: String,
    kind: ToastKind,
    created: Instant,
}

struct RowData {
    uid: u32,
    sender: String,
    subject: String,
    preview: String,
    time: String,
    unread: bool,
    starred: bool,
    has_attachments: bool,
}

enum Action {
    OpenFolder(String),
    SelectMessage(String, u32),
    ToggleStar(String, u32),
    Archive(String, u32),
    Trash(String, u32),
    Back,
    Compose,
    Reply,
    Sync,
    Settings,
    Starred,
    Filter(MailFilter),
    SaveAttachment(String, u32, usize),
    OpenAttachment(String, u32, usize),
    NewFolder,
    NewLocalFolder,
    StoreLocally(String, u32),
    ConnectAccount,
}

impl LettersApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);
        let config = Config::load();
        let tray = Tray::spawn(cc.egui_ctx.clone());
        let app_active = Arc::new(AtomicBool::new(true));
        let ui_live = Arc::new(AtomicBool::new(true));
        let local = LocalStore::load();
        let mut app = Self {
            config,
            tray,
            worker: None,
            app_active,
            ui_live,
            pending: None,
            welcome: true,
            connecting: false,
            form: AccountForm::default(),
            form_error: None,
            folders: Vec::new(),
            cache: HashMap::new(),
            local,
            current_folder: "INBOX".to_owned(),
            selected: None,
            search: String::new(),
            filter: MailFilter::All,
            compact_reader: false,
            loading: HashSet::new(),
            syncing: false,
            last_sync: None,
            sync_error: None,
            remote_quota: None,
            quota_checked: false,
            unread_inbox: 0,
            saved_attachments: HashMap::new(),
            signature_textures: HashMap::new(),
            compose: None,
            local_transfer: None,
            settings_open: false,
            new_folder_open: false,
            new_folder_local: false,
            new_folder_name: String::new(),
            new_folder_error: None,
            pending_new_folder: None,
            toast: None,
            demo: false,
            hidden: false,
            allow_close: false,
            pending_hide: false,
            tray_notice_shown: false,
        };

        if app.config.demo {
            app.load_demo(false);
            app.merge_local_folders();
        } else if let Some(account) = app.config.account.clone() {
            app.form = AccountForm::from_account(&account);
            app.welcome = false;
            app.restore_cache(&account.email);
            app.merge_local_folders();
            if let Some(password) = config::load_password(&account.email) {
                app.start_worker(account, password);
            } else {
                app.welcome = true;
                app.form_error = Some(
                    "Your saved password is unavailable. Enter it again to continue.".to_owned(),
                );
            }
        } else if app.config.local_mode {
            app.open_local_only(false);
        } else {
            app.merge_local_folders();
        }
        app.sync_tray();
        app
    }

    fn start_worker(&mut self, account: Account, password: String) {
        self.stop_worker();
        let options = WorkerOptions {
            notifications: self.config.notifications,
            sync_secs: self.config.sync_secs,
            save_sent_copy: self.config.save_sent_copy,
            app_active: self.app_active.clone(),
            ui_live: self.ui_live.clone(),
            tray_update: self.tray.updater(),
        };
        self.worker = Some(MailWorker::start(account.clone(), password, options));
        self.connecting = true;
        self.loading.clear();
        self.remote_quota = None;
        self.quota_checked = false;
        self.sync_tray();
    }

    fn restore_cache(&mut self, email: &str) -> bool {
        let Some(cached) = cache::load(email) else {
            return false;
        };
        log::info!("Loaded cached mail for {email}");
        self.folders = cached.folders;
        self.cache = cached.messages;
        if self
            .folders
            .iter()
            .any(|folder| folder.id == cached.current_folder)
        {
            self.current_folder = cached.current_folder;
        } else if let Some(inbox) = self
            .folders
            .iter()
            .find(|folder| folder.kind == FolderKind::Inbox)
        {
            self.current_folder = inbox.id.clone();
        }
        self.unread_inbox = self
            .folders
            .iter()
            .find(|folder| folder.kind == FolderKind::Inbox)
            .map(|folder| folder.unread)
            .unwrap_or(0);
        true
    }

    fn merge_local_folders(&mut self) {
        self.folders.retain(|folder| !folder.local);
        let local_folders = self.local.folders();
        for folder in &local_folders {
            self.cache
                .insert(folder.id.clone(), self.local.messages(&folder.id));
        }
        self.folders.extend(local_folders);
    }

    fn refresh_local_state(&mut self) {
        let local_folders = self.local.folders();
        for folder in &local_folders {
            if let Some(existing) = self.folders.iter_mut().find(|entry| entry.id == folder.id) {
                existing.unread = folder.unread;
            } else {
                self.folders.push(folder.clone());
            }
            self.cache
                .insert(folder.id.clone(), self.local.messages(&folder.id));
        }
        if self.current_folder_kind() == FolderKind::Inbox && self.current_folder_is_local() {
            self.unread_inbox = self
                .folders
                .iter()
                .find(|folder| folder.id == self.current_folder)
                .map(|folder| folder.unread)
                .unwrap_or(0);
        }
        self.sync_tray();
    }

    fn current_folder_is_local(&self) -> bool {
        self.folders
            .iter()
            .find(|folder| folder.id == self.current_folder)
            .is_some_and(|folder| folder.local)
    }

    fn open_login(&mut self) {
        self.stop_worker();
        self.config.local_mode = false;
        self.config.demo = false;
        self.demo = false;
        let _ = self.config.save();
        self.welcome = true;
        self.connecting = false;
        self.form_error = None;
        self.sync_error = None;
        self.sync_tray();
    }

    fn open_local_only(&mut self, persist: bool) {
        self.stop_worker();
        self.config.demo = false;
        self.config.local_mode = true;
        self.demo = false;
        self.welcome = false;
        self.connecting = false;
        self.pending = None;
        self.form_error = None;
        self.folders.clear();
        self.cache.clear();
        self.merge_local_folders();
        self.current_folder = self
            .folders
            .iter()
            .find(|folder| folder.kind == FolderKind::Inbox)
            .map(|folder| folder.id.clone())
            .unwrap_or_else(|| "local:inbox".to_owned());
        self.unread_inbox = self
            .folders
            .iter()
            .find(|folder| folder.id == self.current_folder)
            .map(|folder| folder.unread)
            .unwrap_or(0);
        self.selected = None;
        self.compact_reader = false;
        self.filter = MailFilter::All;
        self.search.clear();
        self.last_sync = Some(Instant::now());
        self.sync_error = None;
        self.remote_quota = None;
        self.quota_checked = false;
        if persist {
            let _ = self.config.save();
        }
        self.sync_tray();
    }

    fn storage_summary(&self) -> StorageSummary {
        let mut summary = StorageSummary {
            remote_bytes: 0,
            local_bytes: 0,
            remote_count: 0,
            local_count: 0,
        };
        for folder in &self.folders {
            let Some(messages) = self.cache.get(&folder.id) else {
                continue;
            };
            let bytes = messages.iter().map(message_size).sum::<usize>();
            let count = messages.len();
            if folder.local {
                summary.local_bytes += bytes;
                summary.local_count += count;
            } else {
                summary.remote_bytes += bytes;
                summary.remote_count += count;
            }
        }
        summary
    }

    fn save_cache(&self) {
        if self.demo || self.folders.is_empty() {
            return;
        }
        let Some(account) = &self.config.account else {
            return;
        };
        let remote_folders = self
            .folders
            .iter()
            .filter(|folder| !folder.local)
            .cloned()
            .collect::<Vec<_>>();
        let remote_messages = self
            .cache
            .iter()
            .filter(|(folder_id, _)| !local_store::is_local_folder(folder_id))
            .map(|(folder_id, messages)| (folder_id.clone(), messages.clone()))
            .collect::<HashMap<_, _>>();
        if let Err(error) = cache::save(
            &account.email,
            &remote_folders,
            &remote_messages,
            &self.current_folder,
        ) {
            log::warn!("Could not save the local mail cache: {error}");
        }
    }

    fn stop_worker(&mut self) {
        if let Some(mut worker) = self.worker.take() {
            worker.shutdown();
        }
        self.connecting = false;
    }

    fn load_demo(&mut self, notify_user: bool) {
        self.stop_worker();
        self.config.demo = true;
        self.demo = true;
        self.welcome = false;
        self.pending = None;
        self.selected = None;
        self.compact_reader = false;
        self.search.clear();
        self.filter = MailFilter::All;
        self.current_folder = "INBOX".to_owned();
        let data = demo_data();
        self.folders = data.folders;
        self.cache = data.cache;
        self.unread_inbox = self
            .folders
            .iter()
            .find(|folder| folder.kind == FolderKind::Inbox)
            .map(|folder| folder.unread)
            .unwrap_or(0);
        self.last_sync = Some(Instant::now());
        self.syncing = false;
        self.sync_error = None;
        let _ = self.config.save();
        self.sync_tray();
        if notify_user {
            self.toast("Demo mailbox loaded.", ToastKind::Success);
        }
    }

    fn logout(&mut self) {
        self.stop_worker();
        if let Some(account) = self.config.account.take() {
            config::delete_password(&account.email);
        }
        self.config.demo = false;
        self.config.local_mode = false;
        cache::clear();
        let _ = self.config.save();
        self.demo = false;
        self.welcome = true;
        self.connecting = false;
        self.pending = None;
        self.form = AccountForm::default();
        self.form_error = None;
        self.folders.clear();
        self.cache.clear();
        self.current_folder = "INBOX".to_owned();
        self.selected = None;
        self.search.clear();
        self.filter = MailFilter::All;
        self.settings_open = false;
        self.compose = None;
        self.unread_inbox = 0;
        self.last_sync = None;
        self.sync_error = None;
        self.remote_quota = None;
        self.quota_checked = false;
        self.sync_tray();
    }

    fn submit_login(&mut self) {
        if self.connecting {
            return;
        }
        let account = match self.form.to_account() {
            Ok(account) => account,
            Err(error) => {
                self.form_error = Some(error);
                return;
            }
        };
        if self.form.password.is_empty() {
            self.form_error = Some("Enter your password.".to_owned());
            return;
        }
        let password = self.form.password.clone();
        let remember = self.form.remember;
        let different_account = self
            .config
            .account
            .as_ref()
            .is_some_and(|existing| !existing.email.eq_ignore_ascii_case(&account.email));
        if different_account || self.config.account.is_none() {
            self.folders.clear();
            self.cache.clear();
            cache::clear();
            self.current_folder = "INBOX".to_owned();
            self.selected = None;
            self.compact_reader = false;
        }
        self.form_error = None;
        self.pending = Some(PendingLogin {
            account: account.clone(),
            password: password.clone(),
            remember,
        });
        self.start_worker(account, password);
    }

    fn finish_pending_login(&mut self) {
        let Some(pending) = self.pending.take() else {
            self.welcome = false;
            self.form_error = None;
            return;
        };
        self.config.account = Some(pending.account.clone());
        self.config.demo = false;
        self.config.local_mode = false;
        self.demo = false;
        self.form.password.clear();
        let saved = if pending.remember {
            config::save_password(&pending.account.email, &pending.password)
        } else {
            config::delete_password(&pending.account.email);
            true
        };
        if let Err(error) = self.config.save() {
            log::warn!("Could not save config: {error}");
        }
        self.welcome = false;
        self.form_error = None;
        if pending.remember && !saved {
            self.toast(
                "Connected, but the password could not be stored in the system keyring.",
                ToastKind::Info,
            );
        }
    }

    fn process_events(&mut self, ctx: &egui::Context) {
        let mut events = Vec::new();
        if let Some(worker) = &self.worker {
            while let Some(event) = worker.try_recv() {
                events.push(event);
            }
        }
        for event in events {
            self.handle_event(event, ctx);
        }
    }

    fn handle_event(&mut self, event: MailEvent, ctx: &egui::Context) {
        match event {
            MailEvent::Connected { folders } => {
                self.connecting = false;
                let cached_unread = self
                    .folders
                    .iter()
                    .map(|folder| (folder.id.clone(), folder.unread))
                    .collect::<HashMap<_, _>>();
                self.folders = folders;
                for folder in &mut self.folders {
                    if let Some(unread) = cached_unread.get(&folder.id) {
                        folder.unread = *unread;
                    }
                }
                self.merge_local_folders();
                if !self
                    .folders
                    .iter()
                    .any(|folder| folder.id == self.current_folder)
                {
                    let inbox = self
                        .folders
                        .iter()
                        .find(|folder| folder.kind == FolderKind::Inbox)
                        .map(|folder| folder.id.clone())
                        .or_else(|| self.folders.first().map(|folder| folder.id.clone()));
                    if let Some(inbox) = inbox {
                        self.current_folder = inbox;
                    }
                }
                self.unread_inbox = self
                    .folders
                    .iter()
                    .find(|folder| folder.kind == FolderKind::Inbox)
                    .map(|folder| folder.unread)
                    .unwrap_or(0);
                if self.pending.is_some() {
                    self.finish_pending_login();
                    self.toast("Mailbox connected.", ToastKind::Success);
                }
                self.welcome = false;
                self.sync_tray();
                self.save_cache();
            }
            MailEvent::SyncStarted { folder_id } => {
                self.loading.insert(folder_id);
                self.syncing = true;
                self.sync_tray();
            }
            MailEvent::Synced {
                folder_id,
                messages,
                unread,
                ..
            } => {
                self.loading.remove(&folder_id);
                self.syncing = false;
                self.last_sync = Some(Instant::now());
                if let Some(folder) = self
                    .folders
                    .iter_mut()
                    .find(|folder| folder.id == folder_id)
                {
                    folder.unread = unread;
                    if folder.kind == FolderKind::Inbox {
                        self.unread_inbox = unread;
                    }
                }
                let selected_message_gone = self
                    .selected
                    .as_ref()
                    .map(|(folder, uid)| {
                        folder == &folder_id && !messages.iter().any(|message| message.uid == *uid)
                    })
                    .unwrap_or(false);
                let source_folder = self
                    .folders
                    .iter()
                    .find(|folder| folder.id == folder_id)
                    .cloned();
                let previous_uids = self
                    .cache
                    .get(&folder_id)
                    .map(|old| {
                        old.iter()
                            .map(|message| message.uid)
                            .collect::<HashSet<_>>()
                    })
                    .unwrap_or_default();
                let automatic_copies = if self.config.default_store == DefaultStore::Local
                    && source_folder
                        .as_ref()
                        .is_some_and(|folder| !folder.local && folder.kind == FolderKind::Inbox)
                    && let Some(worker) = &self.worker
                    && let Some(source) = source_folder.as_ref()
                    && let Some(target) = self
                        .folders
                        .iter()
                        .find(|folder| folder.local && folder.kind == FolderKind::Inbox)
                        .map(|folder| folder.id.clone())
                {
                    messages
                        .iter()
                        .filter(|message| match message.message_id.as_deref() {
                            Some(message_id) => !self.local.has_message_id_any(message_id),
                            None => !previous_uids.contains(&message.uid),
                        })
                        .map(|message| (source.clone(), message.uid, target.clone()))
                        .collect::<Vec<_>>()
                        .into_iter()
                        .map(|(source, uid, target)| {
                            worker.fetch_local_copy(&source, uid, target, false, true);
                        })
                        .count();
                    true
                } else {
                    false
                };
                let _ = automatic_copies;
                self.cache.insert(folder_id.clone(), messages);
                if selected_message_gone {
                    self.selected = None;
                    self.compact_reader = false;
                }
                self.sync_tray();
                self.save_cache();
            }
            MailEvent::Sent { to } => {
                if let Some(compose) = &mut self.compose {
                    compose.sending = false;
                }
                self.compose = None;
                self.toast(
                    if to.trim().is_empty() {
                        "Message sent.".to_owned()
                    } else {
                        format!("Message sent to {to}.")
                    },
                    ToastKind::Success,
                );
                if let Some(sent) = self
                    .folders
                    .iter()
                    .find(|folder| folder.kind == FolderKind::Sent)
                    .cloned()
                    && let Some(worker) = &self.worker
                {
                    worker.sync(&sent, false);
                }
            }
            MailEvent::AttachmentSaved {
                folder_id,
                uid,
                attachment_index,
                path,
                open_after_save,
            } => {
                self.saved_attachments
                    .insert((folder_id, uid, attachment_index), path.clone());
                if open_after_save {
                    self.open_external(path);
                } else {
                    self.toast(
                        format!("Attachment saved to {}", path.display()),
                        ToastKind::Success,
                    );
                }
            }
            MailEvent::LocalCopyReady {
                source_folder_id,
                target_local,
                uid,
                raw,
                unread,
                starred,
                remove_remote,
                automatic,
            } => match self.local.store_raw(&target_local, &raw, unread, starred) {
                Ok(_) => {
                    self.refresh_local_state();
                    if !automatic {
                        self.toast("Saved to local folders.", ToastKind::Success);
                    }
                    if remove_remote {
                        let source = self
                            .folders
                            .iter()
                            .find(|folder| folder.id == source_folder_id && !folder.local)
                            .cloned();
                        let trash = self
                            .folders
                            .iter()
                            .find(|folder| folder.kind == FolderKind::Trash && !folder.local)
                            .cloned();
                        match (source, trash, self.worker.as_ref()) {
                            (Some(source), Some(trash), Some(worker)) => {
                                worker.move_to(&source, uid, &trash);
                            }
                            _ => {
                                self.toast(
                                        "Saved locally, but the remote copy could not be moved to Trash.",
                                        ToastKind::Info,
                                    );
                            }
                        }
                    }
                }
                Err(message) => self.toast(message, ToastKind::Error),
            },
            MailEvent::Quota { used, total } => {
                log::debug!("Remote quota: used={used:?}, total={total:?}");
                self.quota_checked = true;
                self.remote_quota = match (used, total) {
                    (Some(used), Some(total)) if total > 0 => Some((used, total)),
                    _ => None,
                };
            }
            MailEvent::FoldersUpdated { folders } => {
                let cached_unread = self
                    .folders
                    .iter()
                    .map(|folder| (folder.id.clone(), folder.unread))
                    .collect::<HashMap<_, _>>();
                self.folders = folders;
                for folder in &mut self.folders {
                    if let Some(unread) = cached_unread.get(&folder.id) {
                        folder.unread = *unread;
                    }
                }
                self.merge_local_folders();
                if !self
                    .folders
                    .iter()
                    .any(|folder| folder.id == self.current_folder)
                    && let Some(inbox) = self
                        .folders
                        .iter()
                        .find(|folder| folder.kind == FolderKind::Inbox)
                {
                    self.current_folder = inbox.id.clone();
                }
                if let Some(name) = self.pending_new_folder.take()
                    && let Some(folder) = self
                        .folders
                        .iter()
                        .find(|folder| {
                            folder.remote.eq_ignore_ascii_case(&name)
                                || folder.name.eq_ignore_ascii_case(&name)
                        })
                        .cloned()
                {
                    self.current_folder = folder.id.clone();
                    self.cache.entry(folder.id.clone()).or_default();
                    self.selected = None;
                    self.compact_reader = false;
                    self.filter = MailFilter::All;
                    if let Some(worker) = &self.worker {
                        worker.sync(&folder, false);
                    }
                }
                self.toast("Folder created.", ToastKind::Success);
                self.save_cache();
            }
            MailEvent::Moved {
                folder_id,
                uid,
                target,
            } => {
                let was_unread = self.remove_message(&folder_id, uid);
                if was_unread
                    && let Some(folder) = self
                        .folders
                        .iter_mut()
                        .find(|folder| folder.id == folder_id)
                {
                    folder.unread = folder.unread.saturating_sub(1);
                    if folder.kind == FolderKind::Inbox {
                        self.unread_inbox = self.unread_inbox.saturating_sub(1);
                    }
                }
                if self
                    .selected
                    .as_ref()
                    .is_some_and(|(folder, selected_uid)| {
                        folder == &folder_id && *selected_uid == uid
                    })
                {
                    self.selected = None;
                    self.compact_reader = false;
                }
                let target_name = self
                    .folders
                    .iter()
                    .find(|folder| folder.id == target)
                    .map(|folder| folder.name.clone())
                    .unwrap_or_else(|| "folder".to_owned());
                self.toast(
                    format!("Message moved to {target_name}."),
                    ToastKind::Success,
                );
                self.sync_tray();
                self.save_cache();
            }
            MailEvent::Error { kind, message } => match kind {
                ErrorKind::Connect => {
                    self.connecting = false;
                    self.syncing = false;
                    if !self.welcome && !self.folders.is_empty() {
                        log::warn!("{message}; showing cached mail");
                        self.sync_error = Some(message);
                        self.toast(
                            "Could not connect. Showing cached mail; press Refresh to retry.",
                            ToastKind::Error,
                        );
                    } else {
                        self.stop_worker();
                        if let Some(pending) = self.pending.take() {
                            self.form = AccountForm::from_account(&pending.account);
                        } else if let Some(account) = &self.config.account {
                            self.form = AccountForm::from_account(account);
                        }
                        self.welcome = true;
                        self.form_error = Some(message);
                    }
                    self.sync_tray();
                }
                ErrorKind::Send => {
                    if let Some(compose) = &mut self.compose {
                        compose.sending = false;
                        compose.error = Some(message);
                    }
                }
                ErrorKind::Sync => {
                    self.syncing = false;
                    self.toast("Could not refresh your mailbox.", ToastKind::Error);
                }
                ErrorKind::Action => {
                    self.toast(message, ToastKind::Error);
                }
            },
        }
        ctx.request_repaint();
    }

    fn process_tray(&mut self, ctx: &egui::Context) {
        while let Some(command) = self.tray.try_recv() {
            match command {
                TrayCommand::Show => {
                    self.hidden = false;
                    self.pending_hide = false;
                    self.app_active.store(true, Ordering::Relaxed);
                    self.ui_live.store(true, Ordering::Relaxed);
                }
                TrayCommand::Compose => {
                    self.hidden = false;
                    self.pending_hide = false;
                    if !self.welcome {
                        self.open_compose(ComposeDraft::empty());
                    }
                }
                TrayCommand::Sync => {
                    self.hidden = false;
                    self.pending_hide = false;
                    if !self.welcome {
                        self.sync_now(ctx);
                    }
                }
                TrayCommand::Settings => {
                    self.hidden = false;
                    self.pending_hide = false;
                    if !self.welcome {
                        self.settings_open = true;
                    }
                }
                TrayCommand::Quit => {
                    self.allow_close = true;
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        }
    }

    fn apply_pending_hide(&mut self, ctx: &egui::Context) {
        if !self.pending_hide {
            return;
        }
        self.pending_hide = false;
        ctx.send_viewport_cmd(ViewportCommand::Visible(false));
    }

    fn update_active_flag(&mut self, ctx: &egui::Context) {
        if self.hidden {
            self.app_active.store(false, Ordering::Relaxed);
            self.ui_live.store(false, Ordering::Relaxed);
            return;
        }
        let focused = ctx.input(|input| input.viewport().focused.unwrap_or(true));
        let minimized = ctx.input(|input| input.viewport().minimized.unwrap_or(false));
        self.app_active
            .store(focused && !minimized, Ordering::Relaxed);
        let live = !minimized;
        let was_live = self.ui_live.swap(live, Ordering::Relaxed);
        if live && !was_live && !self.welcome {
            self.sync_now(ctx);
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|input| input.modifiers.command && input.key_pressed(Key::Q)) {
            self.allow_close = true;
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }
        if self.welcome || self.settings_open || self.compose.is_some() {
            return;
        }
        let compose = ctx.input(|input| input.modifiers.command && input.key_pressed(Key::N));
        if compose {
            self.open_compose(ComposeDraft::empty());
        }
        let sync = ctx.input(|input| input.modifiers.command && input.key_pressed(Key::R));
        if sync {
            self.sync_now(ctx);
        }
        let focus_none = ctx.memory(|memory| memory.focused().is_none());
        let search = ctx.input(|input| input.key_pressed(Key::Slash));
        if focus_none && search {
            ctx.memory_mut(|memory| memory.request_focus(Id::new("letters_search")));
        }
    }

    fn handle_close(&mut self, ctx: &egui::Context) {
        let close_requested = ctx.input(|input| input.viewport().close_requested());
        if !close_requested || self.allow_close {
            return;
        }
        if self.config.close_to_tray && self.tray.available() {
            log::info!("Close requested; hiding the window to the tray");
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.pending_hide = true;
            self.hidden = true;
            self.app_active.store(false, Ordering::Relaxed);
            self.ui_live.store(false, Ordering::Relaxed);
            ctx.request_repaint();
            if !self.tray_notice_shown {
                self.tray_notice_shown = true;
                crate::notify::show_tray_notice();
            }
        } else {
            self.allow_close = true;
        }
    }

    fn sync_tray(&self) {
        let account = if self.demo {
            format!("Demo · {DEMO_EMAIL}")
        } else {
            self.config
                .account
                .as_ref()
                .map(|account| account.email.clone())
                .unwrap_or_else(|| {
                    if self.config.local_mode {
                        "Local folders".to_owned()
                    } else {
                        String::new()
                    }
                })
        };
        self.tray.update(
            self.unread_inbox,
            self.syncing,
            &account,
            self.config.notifications,
        );
    }

    fn toast(&mut self, text: impl Into<String>, kind: ToastKind) {
        let text = text.into();
        if text.is_empty() {
            return;
        }
        self.toast = Some(Toast {
            text,
            kind,
            created: Instant::now(),
        });
    }

    fn open_compose(&mut self, draft: ComposeDraft) {
        if self.welcome {
            return;
        }
        if self.config.account.is_none() && !self.demo {
            self.toast(
                "Connect a mail account to send messages. Local folders are available for storage.",
                ToastKind::Info,
            );
            return;
        }
        self.compose = Some(ComposeState {
            draft,
            sending: false,
            error: None,
        });
    }

    fn open_reply(&mut self) {
        let Some(message) = self.selected_message().cloned() else {
            return;
        };
        self.open_compose(ComposeDraft::reply_to(&message));
    }

    fn selected_message(&self) -> Option<&Message> {
        let (folder_id, uid) = self.selected.as_ref()?;
        self.cache
            .get(folder_id)?
            .iter()
            .find(|message| message.uid == *uid)
    }

    fn current_folder(&self) -> Option<&Folder> {
        self.folders
            .iter()
            .find(|folder| folder.id == self.current_folder)
    }

    fn current_folder_name(&self) -> String {
        self.current_folder()
            .map(|folder| folder.name.clone())
            .unwrap_or_else(|| "Mail".to_owned())
    }

    fn current_folder_kind(&self) -> FolderKind {
        self.current_folder()
            .map(|folder| folder.kind)
            .unwrap_or(FolderKind::Custom)
    }

    fn visible_rows(&self) -> Vec<RowData> {
        let Some(messages) = self.cache.get(&self.current_folder) else {
            return Vec::new();
        };
        let query = self.search.trim().to_lowercase();
        messages
            .iter()
            .filter(|message| match self.filter {
                MailFilter::All => true,
                MailFilter::Unread => message.unread,
                MailFilter::Starred => message.starred,
            })
            .filter(|message| {
                if query.is_empty() {
                    return true;
                }
                message.sender_label().to_lowercase().contains(&query)
                    || message.subject.to_lowercase().contains(&query)
                    || message.preview.to_lowercase().contains(&query)
            })
            .map(|message| RowData {
                uid: message.uid,
                sender: message.sender_label().to_owned(),
                subject: message.subject.clone(),
                preview: message.preview.clone(),
                time: message.short_time(),
                unread: message.unread,
                starred: message.starred,
                has_attachments: message.has_attachments(),
            })
            .collect()
    }

    fn sync_now(&mut self, ctx: &egui::Context) {
        if self.demo {
            self.last_sync = Some(Instant::now());
            self.toast("The demo mailbox is already up to date.", ToastKind::Info);
            return;
        }
        if self.connecting || self.welcome {
            return;
        }
        if self.current_folder_is_local() {
            self.toast("Local folders are stored on this device.", ToastKind::Info);
            return;
        }
        let Some(folder) = self.current_folder().cloned().or_else(|| {
            self.folders
                .iter()
                .find(|folder| folder.kind == FolderKind::Inbox)
                .cloned()
        }) else {
            return;
        };
        if let Some(worker) = &self.worker {
            self.loading.insert(folder.id.clone());
            worker.sync(&folder, false);
            ctx.request_repaint();
        }
    }

    fn mark_read(&mut self, folder_id: &str, uid: u32) {
        let folder = self
            .folders
            .iter()
            .find(|folder| folder.id == folder_id)
            .cloned();
        let mut changed = false;
        if let Some(message) = self
            .cache
            .get_mut(folder_id)
            .and_then(|messages| messages.iter_mut().find(|message| message.uid == uid))
            && message.unread
        {
            message.unread = false;
            changed = true;
        }
        if !changed {
            return;
        }
        if let Some(folder) = folder {
            if folder.local {
                let _ = self.local.set_read(folder_id, uid, false);
                self.refresh_local_state();
                return;
            }
            if let Some(state) = self.folders.iter_mut().find(|state| state.id == folder_id) {
                state.unread = state.unread.saturating_sub(1);
            }
            if folder.kind == FolderKind::Inbox {
                self.unread_inbox = self.unread_inbox.saturating_sub(1);
            }
            if !self.demo
                && let Some(worker) = &self.worker
            {
                worker.mark_seen(&folder, uid, true);
            }
        }
        self.sync_tray();
    }

    fn toggle_star(&mut self, folder_id: &str, uid: u32) {
        let Some(folder) = self
            .folders
            .iter()
            .find(|folder| folder.id == folder_id)
            .cloned()
        else {
            return;
        };
        let mut starred = false;
        if let Some(message) = self
            .cache
            .get_mut(folder_id)
            .and_then(|messages| messages.iter_mut().find(|message| message.uid == uid))
        {
            message.starred = !message.starred;
            starred = message.starred;
        }
        if folder.local {
            let _ = self.local.set_star(&folder.id, uid, starred);
            self.refresh_local_state();
            return;
        }
        if !self.demo
            && let Some(worker) = &self.worker
        {
            worker.set_star(&folder, uid, starred);
        }
    }

    fn move_message(&mut self, kind: FolderKind, target_label: &str) {
        let Some((folder_id, uid)) = self.selected.clone() else {
            return;
        };
        let Some(source) = self
            .folders
            .iter()
            .find(|folder| folder.id == folder_id)
            .cloned()
        else {
            return;
        };
        let target = if source.local {
            self.folders
                .iter()
                .find(|folder| folder.kind == kind && folder.local)
                .cloned()
        } else {
            self.folders
                .iter()
                .find(|folder| folder.kind == kind && !folder.local)
                .cloned()
        };
        let Some(target) = target else {
            self.toast(
                format!("No {target_label} folder is available."),
                ToastKind::Error,
            );
            return;
        };
        if source.kind == kind {
            self.toast(
                format!("This message is already in {target_label}."),
                ToastKind::Info,
            );
            return;
        }
        if source.local {
            if let Err(error) = self.local.move_message(&source.id, uid, &target.id) {
                self.toast(error, ToastKind::Error);
                return;
            }
            self.refresh_local_state();
            self.selected = None;
            self.compact_reader = false;
            self.toast(
                format!("Message moved to {}.", target.name),
                ToastKind::Success,
            );
        } else if self.demo {
            self.move_message_local(&source, &target, uid);
        } else if let Some(worker) = &self.worker {
            worker.move_to(&source, uid, &target);
        }
    }

    fn move_message_local(&mut self, source: &Folder, target: &Folder, uid: u32) {
        let Some(index) = self
            .cache
            .get(&source.id)
            .and_then(|messages| messages.iter().position(|message| message.uid == uid))
        else {
            return;
        };
        let Some(mut message) = self
            .cache
            .get_mut(&source.id)
            .map(|messages| messages.remove(index))
        else {
            return;
        };
        let was_unread = message.unread;
        message.folder_id = target.id.clone();
        message.uid = self.next_uid(&target.id);
        self.cache
            .entry(target.id.clone())
            .or_default()
            .insert(0, message);
        if was_unread {
            if let Some(folder) = self
                .folders
                .iter_mut()
                .find(|folder| folder.id == source.id)
            {
                folder.unread = folder.unread.saturating_sub(1);
            }
            if source.kind == FolderKind::Inbox {
                self.unread_inbox = self.unread_inbox.saturating_sub(1);
            }
        }
        self.selected = None;
        self.compact_reader = false;
        self.toast(
            format!("Message moved to {}.", target.name),
            ToastKind::Success,
        );
        self.sync_tray();
    }

    fn next_uid(&self, folder_id: &str) -> u32 {
        self.cache
            .get(folder_id)
            .into_iter()
            .flatten()
            .map(|message| message.uid)
            .max()
            .unwrap_or(0)
            + 1
    }

    fn remove_message(&mut self, folder_id: &str, uid: u32) -> bool {
        let Some(messages) = self.cache.get_mut(folder_id) else {
            return false;
        };
        let Some(index) = messages.iter().position(|message| message.uid == uid) else {
            return false;
        };
        messages.remove(index).unread
    }

    fn submit_new_folder(&mut self) {
        let name = self.new_folder_name.trim().to_owned();
        if name.is_empty() {
            self.new_folder_error = Some("Enter a folder name.".to_owned());
            return;
        }
        if name.chars().any(char::is_control) {
            self.new_folder_error = Some("The folder name contains invalid characters.".to_owned());
            return;
        }
        if self.new_folder_local {
            match self.local.create_folder(&name) {
                Ok(folder) => {
                    self.folders.push(folder.clone());
                    self.cache.insert(folder.id.clone(), Vec::new());
                    self.current_folder = folder.id;
                    self.selected = None;
                    self.compact_reader = false;
                    self.filter = MailFilter::All;
                    self.new_folder_open = false;
                    self.refresh_local_state();
                    self.toast("Local folder created.", ToastKind::Success);
                }
                Err(error) => self.new_folder_error = Some(error),
            }
            return;
        }
        if self.demo {
            if self
                .folders
                .iter()
                .any(|folder| folder.name.eq_ignore_ascii_case(&name))
            {
                self.new_folder_error = Some("A folder with that name already exists.".to_owned());
                return;
            }
            let folder = Folder::new(name.clone(), name.clone(), FolderKind::Custom);
            self.folders.push(folder);
            self.cache.entry(name.clone()).or_default();
            self.current_folder = name.clone();
            self.selected = None;
            self.compact_reader = false;
            self.filter = MailFilter::All;
            self.new_folder_open = false;
            self.toast("Folder created.", ToastKind::Success);
            return;
        }
        let Some(worker) = &self.worker else {
            self.new_folder_error = Some("Connect to a mail server first.".to_owned());
            return;
        };
        self.pending_new_folder = Some(name.clone());
        worker.create_folder(name);
        self.new_folder_open = false;
        self.new_folder_error = None;
    }

    fn save_attachment_action(
        &mut self,
        folder_id: String,
        uid: u32,
        index: usize,
        open_after_save: bool,
    ) {
        if self.demo {
            self.toast(
                "Demo attachments are placeholders and cannot be saved.",
                ToastKind::Info,
            );
            return;
        }
        let Some(folder) = self
            .folders
            .iter()
            .find(|folder| folder.id == folder_id)
            .cloned()
        else {
            return;
        };
        let (raw_path, attachment) = self
            .cache
            .get(&folder.id)
            .and_then(|messages| messages.iter().find(|message| message.uid == uid))
            .map(|message| {
                (
                    message.raw_path.clone(),
                    message.attachments.get(index).cloned(),
                )
            })
            .unwrap_or((None, None));
        if folder.local {
            if let (Some(raw_path), Some(attachment)) = (raw_path, attachment.as_ref()) {
                match local_store::save_attachment(&raw_path, attachment) {
                    Ok(path) => {
                        self.saved_attachments
                            .insert((folder_id, uid, index), path.clone());
                        if open_after_save {
                            self.open_external(path);
                        } else {
                            self.toast(
                                format!("Attachment saved to {}", path.display()),
                                ToastKind::Success,
                            );
                        }
                    }
                    Err(error) => self.toast(error, ToastKind::Error),
                }
            } else {
                self.toast(
                    "This local message has no stored attachment data.",
                    ToastKind::Error,
                );
            }
        } else if let Some(worker) = &self.worker {
            if let Some(attachment) = attachment {
                worker.save_attachment(&folder, uid, index, &attachment, open_after_save);
            } else {
                self.toast(
                    "Could not identify the selected attachment.",
                    ToastKind::Error,
                );
            }
        } else {
            self.toast("The remote account is not connected.", ToastKind::Error);
        }
    }

    fn open_attachment_action(&mut self, folder_id: String, uid: u32, index: usize) {
        let key = (folder_id.clone(), uid, index);
        if let Some(path) = self.saved_attachments.get(&key).cloned() {
            if path.exists() {
                self.open_external(path);
                return;
            }
            self.saved_attachments.remove(&key);
        }
        let name_and_size = self
            .cache
            .get(&folder_id)
            .and_then(|messages| messages.iter().find(|message| message.uid == uid))
            .and_then(|message| message.attachments.get(index))
            .map(|attachment| (attachment.name.clone(), attachment.size));
        if let Some((name, size)) = name_and_size
            && let Some(path) = find_saved_attachment(&name, size)
        {
            self.saved_attachments.insert(key, path.clone());
            self.open_external(path);
            return;
        }
        self.save_attachment_action(folder_id, uid, index, true);
    }

    fn signature_textures(
        &mut self,
        ctx: &egui::Context,
        images: &[InlineImage],
    ) -> HashMap<u64, egui::TextureHandle> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};

        let mut textures = HashMap::new();
        for image in images.iter().take(16) {
            let key = inline_image_key(image);
            if let Some(texture) = self.signature_textures.get(&key) {
                textures.insert(key, texture.clone());
                continue;
            }
            let Ok(bytes) = STANDARD.decode(image.data.as_bytes()) else {
                log::warn!("Could not decode an inline signature image");
                continue;
            };
            if bytes.len() > 2_000_000 {
                continue;
            }
            let Ok(decoded) = image::load_from_memory(&bytes) else {
                log::warn!("Could not decode an inline image ({})", image.mime);
                continue;
            };
            let width = decoded.width() as usize;
            let height = decoded.height() as usize;
            if width == 0 || height == 0 || width > 4096 || height > 4096 {
                continue;
            }
            let rgba = decoded.to_rgba8();
            let color = egui::ColorImage::from_rgba_unmultiplied([width, height], rgba.as_raw());
            let texture =
                ctx.load_texture(format!("inline-{key}"), color, egui::TextureOptions::LINEAR);
            self.signature_textures.insert(key, texture.clone());
            textures.insert(key, texture);
        }
        textures
    }

    fn open_external(&mut self, path: PathBuf) {
        if !path.exists() {
            self.toast("The saved attachment no longer exists.", ToastKind::Error);
            return;
        }
        if let Err(error) = Command::new("xdg-open").arg(&path).spawn() {
            self.toast(
                format!("Could not open the attachment: {error}"),
                ToastKind::Error,
            );
        }
    }

    fn submit_compose(&mut self) {
        let Some(compose) = self.compose.as_mut() else {
            return;
        };
        if compose.sending {
            return;
        }
        let to = compose.draft.to.trim().to_owned();
        if !to.contains('@') {
            compose.error = Some("Enter a valid recipient address.".to_owned());
            return;
        }
        if self.demo {
            let draft = compose.draft.clone();
            let mut uid = self.next_uid("Sent");
            if uid == 1 {
                uid = 300;
            }
            let message = Message {
                uid,
                folder_id: "Sent".to_owned(),
                raw_path: None,
                raw_size: 0,
                subject: if draft.subject.trim().is_empty() {
                    "(no subject)".to_owned()
                } else {
                    draft.subject.clone()
                },
                from_name: "Letters Studio".to_owned(),
                from_email: DEMO_EMAIL.to_owned(),
                to: draft.to.clone(),
                timestamp: crate::model::now_timestamp(),
                preview: draft.body.lines().next().unwrap_or_default().to_owned(),
                body: draft.body.clone(),
                unread: false,
                starred: false,
                attachments: Vec::new(),
                body_blocks: Vec::new(),
                message_id: Some(format!("<{uid}@demo.letters.app>")),
            };
            self.cache
                .entry("Sent".to_owned())
                .or_default()
                .insert(0, message);
            self.compose = None;
            self.toast("Message sent (demo).", ToastKind::Success);
            return;
        }
        let Some(worker) = &self.worker else {
            compose.error = Some("You are not connected to a mail server.".to_owned());
            return;
        };
        compose.sending = true;
        compose.error = None;
        worker.send(Outgoing {
            to,
            subject: compose.draft.subject.clone(),
            body: compose.draft.body.clone(),
            in_reply_to: compose.draft.in_reply_to.clone(),
        });
    }

    fn apply_settings(&mut self) {
        self.config.sync_secs = self.config.sync_secs.clamp(MIN_SYNC_SECS, MAX_SYNC_SECS);
        if let Some(worker) = &self.worker {
            worker.configure(
                self.config.notifications,
                self.config.sync_secs,
                self.config.save_sent_copy,
            );
        }
        if let Err(error) = self.config.save() {
            log::warn!("Could not save settings: {error}");
        }
        if self.config.default_store == DefaultStore::Local
            && !self.welcome
            && let Some(inbox) = self
                .folders
                .iter()
                .find(|folder| !folder.local && folder.kind == FolderKind::Inbox)
                .cloned()
            && let Some(worker) = &self.worker
        {
            worker.sync(&inbox, false);
        }
        self.sync_tray();
    }

    fn apply_actions(&mut self, ctx: &egui::Context, actions: Vec<Action>) {
        for action in actions {
            match action {
                Action::OpenFolder(folder_id) => {
                    if self.current_folder == folder_id {
                        continue;
                    }
                    self.current_folder = folder_id.clone();
                    self.selected = None;
                    self.compact_reader = false;
                    if let Some(folder) = self
                        .folders
                        .iter()
                        .find(|folder| folder.id == folder_id)
                        .cloned()
                        && folder.kind == FolderKind::Inbox
                    {
                        self.unread_inbox = folder.unread;
                        self.sync_tray();
                    }
                    if !self.demo
                        && !self.cache.contains_key(&folder_id)
                        && let Some(folder) = self
                            .folders
                            .iter()
                            .find(|folder| folder.id == folder_id)
                            .cloned()
                        && !folder.local
                        && let Some(worker) = &self.worker
                    {
                        self.loading.insert(folder.id.clone());
                        worker.sync(&folder, false);
                    }
                }
                Action::SelectMessage(folder_id, uid) => {
                    self.selected = Some((folder_id.clone(), uid));
                    self.compact_reader = true;
                    self.mark_read(&folder_id, uid);
                }
                Action::ToggleStar(folder_id, uid) => self.toggle_star(&folder_id, uid),
                Action::Archive(folder_id, uid) => {
                    self.selected = Some((folder_id, uid));
                    self.move_message(FolderKind::Archive, "Archive");
                }
                Action::Trash(folder_id, uid) => {
                    self.selected = Some((folder_id, uid));
                    self.move_message(FolderKind::Trash, "Trash");
                }
                Action::Back => {
                    self.selected = None;
                    self.compact_reader = false;
                }
                Action::Compose => self.open_compose(ComposeDraft::empty()),
                Action::Reply => self.open_reply(),
                Action::Sync => self.sync_now(ctx),
                Action::Settings => self.settings_open = true,
                Action::Starred => {
                    self.filter = MailFilter::Starred;
                    let inbox = self
                        .folders
                        .iter()
                        .find(|folder| folder.kind == FolderKind::Inbox)
                        .map(|folder| folder.id.clone());
                    if let Some(inbox) = inbox
                        && self.current_folder != inbox
                    {
                        self.current_folder = inbox;
                        self.selected = None;
                    }
                }
                Action::Filter(filter) => self.filter = filter,
                Action::NewFolder => {
                    self.new_folder_local = false;
                    self.new_folder_name.clear();
                    self.new_folder_error = None;
                    self.new_folder_open = true;
                }
                Action::NewLocalFolder => {
                    self.new_folder_local = true;
                    self.new_folder_name.clear();
                    self.new_folder_error = None;
                    self.new_folder_open = true;
                }
                Action::ConnectAccount => self.open_login(),
                Action::StoreLocally(folder_id, uid) => {
                    let target = self
                        .folders
                        .iter()
                        .find(|folder| folder.local && folder.kind == FolderKind::Inbox)
                        .or_else(|| self.folders.iter().find(|folder| folder.local))
                        .map(|folder| folder.id.clone());
                    if let Some(target) = target {
                        self.local_transfer = Some(LocalTransfer {
                            source_folder_id: folder_id,
                            uid,
                            target,
                        });
                    } else {
                        self.toast("No local folders are available.", ToastKind::Error);
                    }
                }
                Action::SaveAttachment(folder_id, uid, index) => {
                    self.save_attachment_action(folder_id, uid, index, false);
                }
                Action::OpenAttachment(folder_id, uid, index) => {
                    self.open_attachment_action(folder_id, uid, index);
                }
            }
        }
    }
}

pub(super) fn find_saved_attachment(name: &str, size: usize) -> Option<PathBuf> {
    let directory = directories::UserDirs::new()
        .and_then(|dirs| dirs.download_dir().map(|path| path.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Letters");
    let safe_name = crate::mail::sanitize_file_name(name);
    let file_name = std::path::Path::new(&safe_name);
    let stem = file_name
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("");
    let extension = file_name
        .extension()
        .and_then(|extension| extension.to_str());
    let entries = std::fs::read_dir(directory).ok()?;
    let mut numbered = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(candidate_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let matches_exact = candidate_name.eq_ignore_ascii_case(&safe_name);
        let matches_numbered = extension.is_some_and(|extension| {
            path.extension().and_then(|candidate| candidate.to_str()) == Some(extension)
                && candidate_name.strip_prefix(stem).is_some_and(|rest| {
                    rest.starts_with(" (") && rest.ends_with(&format!(".{extension}"))
                })
        });
        if !matches_exact && !matches_numbered {
            continue;
        }
        if size > 0
            && entry
                .metadata()
                .map(|metadata| metadata.len() != size as u64)
                .unwrap_or(true)
        {
            continue;
        }
        if matches_exact {
            return Some(path);
        }
        if numbered.is_none() {
            numbered = Some(path);
        }
    }
    numbered
}

fn message_size(message: &Message) -> usize {
    if message.raw_size > 0 {
        return message.raw_size;
    }
    let attachments = message
        .attachments
        .iter()
        .map(|attachment| attachment.size)
        .sum::<usize>();
    message.body.len()
        + message.preview.len()
        + message.subject.len()
        + message.from_name.len()
        + message.from_email.len()
        + attachments
}

impl eframe::App for LettersApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_cache();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_events(ctx);
        self.process_tray(ctx);
        self.apply_pending_hide(ctx);
        self.update_active_flag(ctx);
        self.handle_shortcuts(ctx);

        if self
            .toast
            .as_ref()
            .is_some_and(|toast| toast.created.elapsed() > Duration::from_secs(5))
        {
            self.toast = None;
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::BG))
            .show(ctx, |ui| {
                theme::paint_ambient(ui);
                if self.welcome {
                    self.ui_welcome(ui, ctx);
                } else {
                    self.ui_mail(ui, ctx);
                }
            });

        self.ui_settings(ctx);
        self.ui_compose(ctx);
        self.ui_new_folder(ctx);
        self.ui_local_transfer(ctx);
        self.ui_toast(ctx);
        self.handle_close(ctx);

        if self.syncing || self.connecting {
            ctx.request_repaint_after(Duration::from_millis(200));
        }
        if self.toast.is_some() {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }
}
