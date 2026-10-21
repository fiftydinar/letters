use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::{Duration, Instant};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use imap::types::{Flag, QuotaResourceName};
use imap::{ClientBuilder, Connection, ConnectionMode, Session};
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{Message as SmtpMessage, SmtpTransport, Transport};
use mail_parser::{MessageParser, MessagePart, MimeHeaders};

use crate::model::{
    Account, Attachment, Folder, FolderKind, InlineImage, Message, Security, classify_folder,
};
use crate::notify;
use directories::UserDirs;

const FETCH_LIMIT: u32 = 50;

#[derive(Clone, Debug)]
pub struct Outgoing {
    pub to: String,
    pub subject: String,
    pub body: String,
    pub in_reply_to: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Connect,
    Sync,
    Send,
    Action,
}

#[derive(Debug)]
pub enum MailEvent {
    Connected {
        folders: Vec<Folder>,
    },
    SyncStarted {
        folder_id: String,
    },
    Synced {
        folder_id: String,
        messages: Vec<Message>,
        unread: usize,
    },
    Sent {
        to: String,
    },
    Moved {
        folder_id: String,
        uid: u32,
        target: String,
    },
    AttachmentSaved {
        folder_id: String,
        uid: u32,
        attachment_index: usize,
        path: PathBuf,
        open_after_save: bool,
    },
    FoldersUpdated {
        folders: Vec<Folder>,
    },
    LocalCopyReady {
        source_folder_id: String,
        target_local: String,
        uid: u32,
        raw: Vec<u8>,
        unread: bool,
        starred: bool,
        remove_remote: bool,
        automatic: bool,
    },
    Quota {
        used: Option<u64>,
        total: Option<u64>,
    },
    Error {
        kind: ErrorKind,
        message: String,
    },
}

enum Command {
    Configure {
        notifications: bool,
        sync_secs: u64,
        save_sent_copy: bool,
    },
    Sync {
        folder: Folder,
        initial: bool,
    },
    Send(Outgoing),
    SetSeen {
        remote: String,
        uid: u32,
        seen: bool,
    },
    SetStar {
        remote: String,
        uid: u32,
        starred: bool,
    },
    Move {
        folder_id: String,
        remote: String,
        uid: u32,
        target: String,
    },
    SaveAttachment {
        folder_id: String,
        remote: String,
        uid: u32,
        attachment_index: usize,
        attachment: Attachment,
        open_after_save: bool,
    },
    CreateFolder {
        name: String,
    },
    FetchLocalCopy {
        source_folder_id: String,
        remote: String,
        uid: u32,
        target_local: String,
        remove_remote: bool,
        automatic: bool,
    },
    Shutdown,
}

pub struct WorkerOptions {
    pub notifications: bool,
    pub sync_secs: u64,
    pub save_sent_copy: bool,
    pub app_active: Arc<AtomicBool>,
    pub ui_live: Arc<AtomicBool>,
    pub tray_update: Option<Arc<dyn Fn(usize, bool) + Send + Sync>>,
}

pub struct MailWorker {
    tx: Sender<Command>,
    rx: Receiver<MailEvent>,
    handle: Option<thread::JoinHandle<()>>,
}

impl MailWorker {
    pub fn start(account: Account, password: String, options: WorkerOptions) -> Self {
        let (tx, command_rx) = channel();
        let (event_tx, rx) = channel();
        let handle = thread::Builder::new()
            .name("letters-mail".to_owned())
            .spawn(move || {
                let worker = Worker::new(account, password, event_tx, options);
                worker.run(command_rx);
            })
            .ok();
        Self { tx, rx, handle }
    }

    pub fn configure(&self, notifications: bool, sync_secs: u64, save_sent_copy: bool) {
        let _ = self.tx.send(Command::Configure {
            notifications,
            sync_secs,
            save_sent_copy,
        });
    }

    pub fn sync(&self, folder: &Folder, initial: bool) {
        let _ = self.tx.send(Command::Sync {
            folder: folder.clone(),
            initial,
        });
    }

    pub fn send(&self, outgoing: Outgoing) {
        let _ = self.tx.send(Command::Send(outgoing));
    }

    pub fn mark_seen(&self, folder: &Folder, uid: u32, seen: bool) {
        let _ = self.tx.send(Command::SetSeen {
            remote: folder.remote.clone(),
            uid,
            seen,
        });
    }

    pub fn set_star(&self, folder: &Folder, uid: u32, starred: bool) {
        let _ = self.tx.send(Command::SetStar {
            remote: folder.remote.clone(),
            uid,
            starred,
        });
    }

    pub fn move_to(&self, folder: &Folder, uid: u32, target: &Folder) {
        let _ = self.tx.send(Command::Move {
            folder_id: folder.id.clone(),
            remote: folder.remote.clone(),
            uid,
            target: target.remote.clone(),
        });
    }

    pub fn save_attachment(
        &self,
        folder: &Folder,
        uid: u32,
        attachment_index: usize,
        attachment: &Attachment,
        open_after_save: bool,
    ) {
        let _ = self.tx.send(Command::SaveAttachment {
            folder_id: folder.id.clone(),
            remote: folder.remote.clone(),
            uid,
            attachment_index,
            attachment: attachment.clone(),
            open_after_save,
        });
    }

    pub fn create_folder(&self, name: String) {
        let _ = self.tx.send(Command::CreateFolder { name });
    }

    pub fn fetch_local_copy(
        &self,
        source: &Folder,
        uid: u32,
        target_local: String,
        remove_remote: bool,
        automatic: bool,
    ) {
        let _ = self.tx.send(Command::FetchLocalCopy {
            source_folder_id: source.id.clone(),
            remote: source.remote.clone(),
            uid,
            target_local,
            remove_remote,
            automatic,
        });
    }

    pub fn try_recv(&self) -> Option<MailEvent> {
        self.rx.try_recv().ok()
    }

    pub fn shutdown(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        self.handle.take();
    }
}

impl Drop for MailWorker {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
    }
}

struct Worker {
    account: Account,
    password: String,
    events: Sender<MailEvent>,
    session: Option<Session<Connection>>,
    selected: Option<String>,
    notifications: bool,
    sync_secs: u64,
    save_sent_copy: bool,
    app_active: Arc<AtomicBool>,
    ui_live: Arc<AtomicBool>,
    tray_update: Option<Arc<dyn Fn(usize, bool) + Send + Sync>>,
    known_inbox: Option<HashSet<u32>>,
    unread_inbox: usize,
    inbox: Option<Folder>,
    folders: Vec<Folder>,
    last_quota_check: Option<Instant>,
}

impl Worker {
    fn new(
        account: Account,
        password: String,
        events: Sender<MailEvent>,
        options: WorkerOptions,
    ) -> Self {
        Self {
            account,
            password,
            events,
            session: None,
            selected: None,
            notifications: options.notifications,
            sync_secs: options.sync_secs.max(15),
            save_sent_copy: options.save_sent_copy,
            app_active: options.app_active,
            ui_live: options.ui_live,
            tray_update: options.tray_update,
            known_inbox: None,
            unread_inbox: 0,
            inbox: None,
            folders: Vec::new(),
            last_quota_check: None,
        }
    }

    fn run(mut self, commands: Receiver<Command>) {
        let folders = match self.connect_and_list() {
            Ok(folders) => {
                log::info!(
                    "Connected to {} ({} folders)",
                    self.account.email,
                    folders.len()
                );
                let _ = self.events.send(MailEvent::Connected {
                    folders: folders.clone(),
                });
                folders
            }
            Err(message) => {
                log::warn!("IMAP connection failed: {message}");
                let _ = self.events.send(MailEvent::Error {
                    kind: ErrorKind::Connect,
                    message,
                });
                Vec::new()
            }
        };

        self.folders = folders.clone();
        self.inbox = folders
            .iter()
            .find(|folder| folder.kind == FolderKind::Inbox)
            .cloned();
        if let Some(inbox) = self.inbox.clone() {
            self.sync_folder(inbox, true, false);
        }

        loop {
            match commands.recv_timeout(self.poll_interval()) {
                Ok(Command::Shutdown) => {
                    if let Some(mut session) = self.session.take() {
                        let _ = session.logout();
                    }
                    break;
                }
                Ok(Command::Configure {
                    notifications,
                    sync_secs,
                    save_sent_copy,
                }) => {
                    self.notifications = notifications;
                    self.sync_secs = sync_secs.max(15);
                    self.save_sent_copy = save_sent_copy;
                }
                Ok(Command::Sync { folder, initial }) => self.sync_folder(folder, initial, false),
                Ok(Command::Send(outgoing)) => self.send(outgoing),
                Ok(Command::SetSeen {
                    remote, uid, seen, ..
                }) => {
                    if let Err(message) = self.set_seen(&remote, uid, seen) {
                        let _ = self.events.send(MailEvent::Error {
                            kind: ErrorKind::Action,
                            message,
                        });
                    }
                }
                Ok(Command::SetStar {
                    remote,
                    uid,
                    starred,
                    ..
                }) => {
                    if let Err(message) = self.set_star(&remote, uid, starred) {
                        let _ = self.events.send(MailEvent::Error {
                            kind: ErrorKind::Action,
                            message,
                        });
                    }
                }
                Ok(Command::Move {
                    folder_id,
                    remote,
                    uid,
                    target,
                }) => match self.move_message(&remote, uid, &target) {
                    Ok(()) => {
                        let _ = self.events.send(MailEvent::Moved {
                            folder_id,
                            uid,
                            target,
                        });
                    }
                    Err(message) => {
                        let _ = self.events.send(MailEvent::Error {
                            kind: ErrorKind::Action,
                            message,
                        });
                    }
                },
                Ok(Command::SaveAttachment {
                    folder_id,
                    remote,
                    uid,
                    attachment_index,
                    attachment,
                    open_after_save,
                }) => match self.save_attachment(&remote, uid, &attachment) {
                    Ok(path) => {
                        let _ = self.events.send(MailEvent::AttachmentSaved {
                            folder_id,
                            uid,
                            attachment_index,
                            path,
                            open_after_save,
                        });
                    }
                    Err(message) => {
                        let _ = self.events.send(MailEvent::Error {
                            kind: ErrorKind::Action,
                            message,
                        });
                    }
                },
                Ok(Command::CreateFolder { name }) => match self.create_folder(&name) {
                    Ok(folders) => {
                        log::info!("Created folder {name}");
                        let _ = self.events.send(MailEvent::FoldersUpdated { folders });
                    }
                    Err(message) => {
                        let _ = self.events.send(MailEvent::Error {
                            kind: ErrorKind::Action,
                            message,
                        });
                    }
                },
                Ok(Command::FetchLocalCopy {
                    source_folder_id,
                    remote,
                    uid,
                    target_local,
                    remove_remote,
                    automatic,
                }) => match self.fetch_local_copy(&remote, uid) {
                    Ok((raw, unread, starred)) => {
                        let _ = self.events.send(MailEvent::LocalCopyReady {
                            source_folder_id,
                            target_local,
                            uid,
                            raw,
                            unread,
                            starred,
                            remove_remote,
                            automatic,
                        });
                    }
                    Err(message) => {
                        let _ = self.events.send(MailEvent::Error {
                            kind: ErrorKind::Action,
                            message,
                        });
                    }
                },
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if let Some(inbox) = self.inbox.clone() {
                        self.sync_folder(inbox, false, true);
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn poll_interval(&self) -> Duration {
        Duration::from_secs(self.sync_secs.clamp(15, 900))
    }

    fn refresh_quota_if_due(&mut self) {
        let now = Instant::now();
        if self
            .last_quota_check
            .is_some_and(|last| now.duration_since(last) < Duration::from_secs(300))
        {
            return;
        }
        self.last_quota_check = Some(now);
        let mut roots = Vec::new();
        let mut storage = None;
        {
            let Ok(session) = self.session() else {
                return;
            };
            match session.get_quota_root("INBOX") {
                Ok(response) => {
                    roots.extend(response.quota_root_names().map(str::to_owned));
                    for quota in response.quotas() {
                        for resource in &quota.resources {
                            if matches!(resource.name, QuotaResourceName::Storage) {
                                storage = Some((
                                    resource.usage.saturating_mul(1024),
                                    resource.limit.saturating_mul(1024),
                                ));
                            }
                        }
                    }
                }
                Err(error) => {
                    log::debug!("IMAP quota is not available: {error}");
                }
            }
        }
        if storage.is_none() && !roots.is_empty() {
            let Ok(session) = self.session() else {
                return;
            };
            for root in roots {
                if let Ok(response) = session.get_quota(&root) {
                    if let Some(quota) = response.parsed() {
                        for resource in &quota.resources {
                            if matches!(resource.name, QuotaResourceName::Storage) {
                                storage = Some((
                                    resource.usage.saturating_mul(1024),
                                    resource.limit.saturating_mul(1024),
                                ));
                                break;
                            }
                        }
                    }
                    if storage.is_some() {
                        break;
                    }
                }
            }
        }
        let _ = self.events.send(MailEvent::Quota {
            used: storage.map(|(used, _)| used),
            total: storage.map(|(_, total)| total),
        });
    }

    fn refresh_folders_if_needed(&mut self) {
        if !self.folders.is_empty() {
            return;
        }
        match self.connect_and_list() {
            Ok(folders) => {
                self.folders = folders.clone();
                self.inbox = folders
                    .iter()
                    .find(|folder| folder.kind == FolderKind::Inbox)
                    .cloned();
                let _ = self.events.send(MailEvent::Connected { folders });
            }
            Err(error) => log::warn!("Could not refresh the folder list: {error}"),
        }
    }

    fn update_tray(&self, syncing: bool) {
        if let Some(update) = &self.tray_update {
            update(self.unread_inbox, syncing);
        }
    }

    fn connect_and_list(&mut self) -> Result<Vec<Folder>, String> {
        let session = self.session()?;
        list_folders(session)
    }

    fn session(&mut self) -> Result<&mut Session<Connection>, String> {
        if self.session.is_none() {
            let mode = match self.account.imap.security {
                Security::Implicit => ConnectionMode::Tls,
                Security::Starttls => ConnectionMode::StartTls,
            };
            let client = ClientBuilder::new(&self.account.imap.host, self.account.imap.port)
                .mode(mode)
                .tls_kind(imap::TlsKind::Rust)
                .connect()
                .map_err(|err| format!("IMAP connection failed: {err}"))?;
            let session = client
                .login(&self.account.username, &self.password)
                .map_err(|err| format!("IMAP login failed: {}", err.0))?;
            self.session = Some(session);
            self.selected = None;
        }
        Ok(self.session.as_mut().expect("session just created"))
    }

    fn reset_connection(&mut self) {
        self.session = None;
        self.selected = None;
    }

    fn sync_folder(&mut self, folder: Folder, initial: bool, background: bool) {
        let quiet = background && !self.ui_live.load(Ordering::Relaxed);
        self.update_tray(true);
        if !quiet {
            let _ = self.events.send(MailEvent::SyncStarted {
                folder_id: folder.id.clone(),
            });
        }
        match self.sync_inner(&folder) {
            Ok((messages, unread)) => {
                if folder.kind == FolderKind::Inbox {
                    self.unread_inbox = unread;
                    self.handle_new_inbox_mail(&messages, initial);
                }
                log::debug!(
                    "Synced {} ({} messages, {} unread)",
                    folder.name,
                    messages.len(),
                    unread
                );
                if self.folders.is_empty() {
                    self.refresh_folders_if_needed();
                }
                if folder.kind == FolderKind::Inbox {
                    self.refresh_quota_if_due();
                }
                if !quiet {
                    let _ = self.events.send(MailEvent::Synced {
                        folder_id: folder.id,
                        messages,
                        unread,
                    });
                }
            }
            Err(message) => {
                log::warn!("Mail sync failed: {message}");
                self.reset_connection();
                if !quiet {
                    let _ = self.events.send(MailEvent::Error {
                        kind: ErrorKind::Sync,
                        message,
                    });
                }
            }
        }
        self.update_tray(false);
    }

    fn handle_new_inbox_mail(&mut self, messages: &[Message], initial: bool) {
        let current = messages
            .iter()
            .map(|message| message.uid)
            .collect::<HashSet<_>>();
        if let Some(known) = self.known_inbox.as_ref()
            && !initial
        {
            let new_messages = messages
                .iter()
                .filter(|message| {
                    !known.contains(&message.uid)
                        && message.unread
                        && !message.from_email.eq_ignore_ascii_case(&self.account.email)
                })
                .cloned()
                .collect::<Vec<_>>();
            if !new_messages.is_empty()
                && self.notifications
                && !self.app_active.load(Ordering::Relaxed)
            {
                notify::show_new_mail(&new_messages);
            }
        }
        self.known_inbox = Some(current);
    }

    fn sync_inner(&mut self, folder: &Folder) -> Result<(Vec<Message>, usize), String> {
        let fetch_limit = FETCH_LIMIT;
        let mut result = Vec::new();
        let mut unread = 0;
        {
            let session = self.session()?;
            let mailbox = session
                .select(&folder.remote)
                .map_err(|err| format!("Could not open folder {}: {err}", folder.name))?;
            if mailbox.exists > 0 {
                let start = mailbox.exists.saturating_sub(fetch_limit).saturating_add(1);
                let range = format!("{start}:*");
                let fetches = session
                    .fetch(&range, "(UID FLAGS BODY.PEEK[])")
                    .map_err(|err| format!("Could not fetch messages: {err}"))?;
                let unseen = session
                    .uid_search("UNSEEN")
                    .map_err(|err| format!("Could not check unread messages: {err}"))?;
                unread = unseen.len();
                let mut seen_message_ids: HashSet<String> = HashSet::new();
                for fetch in fetches.iter() {
                    let uid = fetch.uid.unwrap_or(fetch.message);
                    let Some(raw) = fetch.body() else {
                        continue;
                    };
                    let starred = fetch
                        .flags()
                        .iter()
                        .any(|flag| matches!(flag, Flag::Flagged));
                    let is_unread = unseen.contains(&uid)
                        || !fetch.flags().iter().any(|flag| matches!(flag, Flag::Seen));
                    let message = parse_message(&folder.id, uid, raw, is_unread, starred);
                    if let Some(message_id) = &message.message_id
                        && !seen_message_ids.insert(message_id.clone())
                    {
                        continue;
                    }
                    result.push(message);
                }
            }
        }
        self.selected = Some(folder.remote.clone());
        result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp).then(b.uid.cmp(&a.uid)));
        result.truncate(fetch_limit as usize);
        Ok((result, unread))
    }

    fn ensure_selected(&mut self, remote: &str) -> Result<(), String> {
        if self.selected.as_deref() != Some(remote) {
            let session = self.session()?;
            session
                .select(remote)
                .map_err(|err| format!("Could not open folder: {err}"))?;
            self.selected = Some(remote.to_owned());
        }
        Ok(())
    }

    fn set_seen(&mut self, remote: &str, uid: u32, seen: bool) -> Result<(), String> {
        self.ensure_selected(remote)?;
        let flag = if seen {
            "+FLAGS.SILENT (\\Seen)"
        } else {
            "-FLAGS.SILENT (\\Seen)"
        };
        let session = self.session()?;
        session
            .uid_store(uid.to_string(), flag)
            .map_err(|err| format!("Could not update message status: {err}"))?;
        Ok(())
    }

    fn set_star(&mut self, remote: &str, uid: u32, starred: bool) -> Result<(), String> {
        self.ensure_selected(remote)?;
        let flag = if starred {
            "+FLAGS.SILENT (\\Flagged)"
        } else {
            "-FLAGS.SILENT (\\Flagged)"
        };
        let session = self.session()?;
        session
            .uid_store(uid.to_string(), flag)
            .map_err(|err| format!("Could not update star: {err}"))?;
        Ok(())
    }

    fn move_message(&mut self, remote: &str, uid: u32, target: &str) -> Result<(), String> {
        self.ensure_selected(remote)?;
        let session = self.session()?;
        session
            .uid_mv(uid.to_string(), target)
            .map_err(|err| format!("Could not move message: {err}"))?;
        Ok(())
    }

    fn send(&mut self, outgoing: Outgoing) {
        if let Err(message) = self.send_inner(outgoing.clone()) {
            let _ = self.events.send(MailEvent::Error {
                kind: ErrorKind::Send,
                message,
            });
        } else {
            let _ = self.events.send(MailEvent::Sent { to: outgoing.to });
        }
    }

    fn send_inner(&mut self, outgoing: Outgoing) -> Result<(), String> {
        let from_address = self
            .account
            .email
            .parse()
            .map_err(|_| "The sender address is invalid.".to_owned())?;
        let from = Mailbox::new(Some(self.account.name.clone()), from_address);
        let to = parse_mailbox(&outgoing.to)?;
        let message_id = format!(
            "<{}.{}@letters>",
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default(),
            self.account.email.replace('@', ".")
        );
        let mut builder = SmtpMessage::builder()
            .from(from)
            .to(to)
            .subject(outgoing.subject.clone())
            .date_now()
            .message_id(Some(message_id));
        if let Some(message_id) = outgoing.in_reply_to.clone() {
            builder = builder.in_reply_to(message_id);
        }
        let email = builder
            .body(outgoing.body)
            .map_err(|err| format!("The message is invalid: {err}"))?;
        let raw = email.formatted();

        let host = self.account.smtp.host.clone();
        let tls_parameters = TlsParameters::new(host.clone())
            .map_err(|err| format!("SMTP TLS setup failed: {err}"))?;
        let tls = match self.account.smtp.security {
            Security::Implicit => Tls::Wrapper(tls_parameters),
            Security::Starttls => Tls::Required(tls_parameters),
        };
        let mailer = SmtpTransport::builder_dangerous(host)
            .port(self.account.smtp.port)
            .tls(tls)
            .credentials(Credentials::new(
                self.account.username.clone(),
                self.password.clone(),
            ))
            .build();
        mailer
            .send(&email)
            .map_err(|err| format!("Sending failed: {err}"))?;

        if self.save_sent_copy
            && let Some(sent) = self
                .folders
                .iter()
                .find(|folder| folder.kind == FolderKind::Sent)
                .cloned()
            && let Err(error) = self.append_to_sent(&sent.remote, &raw)
        {
            log::warn!("Message was sent but could not be saved to Sent: {error}");
        }
        Ok(())
    }

    fn append_to_sent(&mut self, remote: &str, raw: &[u8]) -> Result<(), String> {
        let remote = remote.to_owned();
        let session = self.session()?;
        let mut append = session.append(&remote, raw);
        append.flag(Flag::Seen);
        append
            .finish()
            .map(|_| ())
            .map_err(|err| format!("Could not append message to Sent: {err}"))
    }

    fn fetch_local_copy(
        &mut self,
        remote: &str,
        uid: u32,
    ) -> Result<(Vec<u8>, bool, bool), String> {
        self.ensure_selected(remote)?;
        let session = self.session()?;
        let fetches = session
            .uid_fetch(uid.to_string(), "(FLAGS BODY.PEEK[])")
            .map_err(|err| format!("Could not fetch the message: {err}"))?;
        let fetch = fetches
            .iter()
            .find(|fetch| fetch.uid == Some(uid))
            .or_else(|| fetches.iter().next())
            .ok_or_else(|| "The server did not return the message.".to_owned())?;
        let raw = fetch
            .body()
            .ok_or_else(|| "The server did not return the message body.".to_owned())?
            .to_vec();
        let unread = !fetch.flags().iter().any(|flag| matches!(flag, Flag::Seen));
        let starred = fetch
            .flags()
            .iter()
            .any(|flag| matches!(flag, Flag::Flagged));
        Ok((raw, unread, starred))
    }

    fn create_folder(&mut self, name: &str) -> Result<Vec<Folder>, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a folder name.".to_owned());
        }
        if name.chars().any(char::is_control) {
            return Err("The folder name contains invalid characters.".to_owned());
        }
        let folders = {
            let session = self.session()?;
            session
                .create(name)
                .map_err(|err| format!("Could not create the folder: {err}"))?;
            list_folders(session)?
        };
        self.folders = folders.clone();
        self.inbox = folders
            .iter()
            .find(|folder| folder.kind == FolderKind::Inbox)
            .cloned();
        Ok(folders)
    }

    fn save_attachment(
        &mut self,
        remote: &str,
        uid: u32,
        attachment: &Attachment,
    ) -> Result<PathBuf, String> {
        self.ensure_selected(remote)?;
        let session = self.session()?;
        let fetches = session
            .uid_fetch(uid.to_string(), "(BODY.PEEK[])")
            .map_err(|err| format!("Could not fetch the message: {err}"))?;
        let raw = fetches
            .iter()
            .find_map(|fetch| fetch.body())
            .ok_or_else(|| "The server did not return the message body.".to_owned())?;
        let parsed = MessageParser::default()
            .parse(raw)
            .ok_or_else(|| "The message could not be parsed.".to_owned())?;
        let part = resolve_attachment_part(&parsed.parts, attachment)
            .ok_or_else(|| "The attachment no longer exists.".to_owned())?;
        let name = part.attachment_name().unwrap_or("attachment");
        let file_name = sanitize_file_name(name);
        let directory = UserDirs::new()
            .and_then(|dirs| dirs.download_dir().map(|path| path.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Letters");
        std::fs::create_dir_all(&directory)
            .map_err(|err| format!("Could not create the download folder: {err}"))?;
        let path = unique_path(&directory, &file_name);
        std::fs::write(&path, part.contents())
            .map_err(|err| format!("Could not save the attachment: {err}"))?;
        Ok(path)
    }
}

pub(crate) fn sanitize_file_name(name: &str) -> String {
    let cleaned = name
        .chars()
        .map(|ch| {
            if ch == '/' || ch == '\\' || ch.is_control() {
                '_'
            } else {
                ch
            }
        })
        .collect::<String>();
    let cleaned = cleaned.trim().trim_matches('.').to_owned();
    if cleaned.is_empty() {
        "attachment".to_owned()
    } else {
        cleaned
    }
}

pub(crate) fn unique_path(directory: &std::path::Path, file_name: &str) -> PathBuf {
    let candidate = directory.join(file_name);
    if !candidate.exists() {
        return candidate;
    }
    let path = std::path::Path::new(file_name);
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("attachment");
    let extension = path.extension().and_then(|ext| ext.to_str());
    for index in 1..1000 {
        let name = match extension {
            Some(extension) => format!("{stem} ({index}).{extension}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = directory.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    directory.join(format!("{stem}-new"))
}

fn parse_mailbox(value: &str) -> Result<Mailbox, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("Enter a recipient.".to_owned());
    }
    if let Ok(mailbox) = value.parse::<Mailbox>() {
        return Ok(mailbox);
    }
    let address = value
        .parse()
        .map_err(|_| format!("Recipient address is invalid: {value}"))?;
    Ok(Mailbox::new(None, address))
}

fn list_folders(session: &mut Session<Connection>) -> Result<Vec<Folder>, String> {
    let names = session
        .list(None, Some("*"))
        .map_err(|err| format!("Folder list is unavailable: {err}"))?;
    let mut folders = Vec::new();
    let mut seen = HashSet::new();
    for name in names.iter() {
        let remote = name.name().to_owned();
        if remote.is_empty() || !seen.insert(remote.clone()) {
            continue;
        }
        let no_select = name
            .attributes()
            .iter()
            .any(|attribute| format!("{attribute:?}").contains("NoSelect"));
        if no_select {
            continue;
        }
        let kind = classify_folder(&remote);
        let display = match kind {
            FolderKind::Inbox => "Inbox",
            FolderKind::Sent => "Sent",
            FolderKind::Drafts => "Drafts",
            FolderKind::Archive => "Archive",
            FolderKind::Trash => "Trash",
            FolderKind::Custom => return_extract_display(&remote),
        };
        folders.push(Folder {
            id: remote.clone(),
            name: display.to_owned(),
            remote,
            kind,
            unread: 0,
            local: false,
        });
    }

    if !folders
        .iter()
        .any(|folder| folder.kind == FolderKind::Inbox)
    {
        folders.push(Folder::new("INBOX", "Inbox", FolderKind::Inbox));
    }
    folders.sort_by_key(folder_order);
    let mut counts = [0usize; 5];
    for folder in &folders {
        if let Some(index) = special_kind_index(folder.kind) {
            counts[index] += 1;
        }
    }
    for folder in &mut folders {
        if let Some(index) = special_kind_index(folder.kind)
            && counts[index] > 1
        {
            folder.name = return_extract_display(&folder.remote).to_owned();
        }
    }
    Ok(folders)
}

fn special_kind_index(kind: FolderKind) -> Option<usize> {
    match kind {
        FolderKind::Inbox => Some(0),
        FolderKind::Sent => Some(1),
        FolderKind::Drafts => Some(2),
        FolderKind::Archive => Some(3),
        FolderKind::Trash => Some(4),
        FolderKind::Custom => None,
    }
}

fn return_extract_display(remote: &str) -> &str {
    remote.rsplit(['/', '.']).next().unwrap_or(remote)
}

fn folder_order(folder: &Folder) -> (u8, String) {
    let rank = match folder.kind {
        FolderKind::Inbox => 0,
        FolderKind::Sent => 1,
        FolderKind::Drafts => 2,
        FolderKind::Archive => 3,
        FolderKind::Trash => 4,
        FolderKind::Custom => 5,
    };
    (rank, folder.name.to_lowercase())
}

pub fn resolve_attachment_part<'a>(
    parts: &'a [MessagePart<'a>],
    attachment: &Attachment,
) -> Option<&'a MessagePart<'a>> {
    fn is_file(part: &MessagePart<'_>) -> bool {
        !part.is_multipart() && (part.is_binary() || part.is_text() || part.is_message())
    }
    if let Some(part) = parts.get(attachment.part_index)
        && is_file(part)
    {
        return Some(part);
    }
    parts.iter().find(|part| {
        is_file(part)
            && part
                .attachment_name()
                .is_some_and(|name| name.eq_ignore_ascii_case(&attachment.name))
            && (attachment.size == 0 || part.len() == attachment.size)
    })
}

pub(crate) fn parse_message(
    folder_id: &str,
    uid: u32,
    raw: &[u8],
    unread: bool,
    starred: bool,
) -> Message {
    let parser = MessageParser::default();
    let Some(parsed) = parser.parse(raw) else {
        return Message {
            uid,
            folder_id: folder_id.to_owned(),
            raw_path: None,
            raw_size: raw.len(),
            subject: "Could not read message".to_owned(),
            from_name: "Unknown sender".to_owned(),
            from_email: String::new(),
            to: String::new(),
            timestamp: 0,
            preview: String::new(),
            body: String::new(),
            unread,
            starred,
            attachments: Vec::new(),
            body_blocks: Vec::new(),
            message_id: None,
        };
    };

    let from = parsed
        .from()
        .and_then(|address| address.first())
        .or_else(|| parsed.sender().and_then(|address| address.first()));
    let from_name = from
        .and_then(|address| address.name())
        .unwrap_or_default()
        .to_owned();
    let from_email = from
        .and_then(|address| address.address())
        .unwrap_or_default()
        .to_owned();
    let to = parsed
        .to()
        .map(|address| {
            address
                .iter()
                .filter_map(|address| address.address())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let timestamp = parsed
        .date()
        .and_then(|date| chrono::DateTime::parse_from_rfc3339(&date.to_rfc3339()).ok())
        .map(|date| date.timestamp())
        .unwrap_or_else(crate::model::now_timestamp);
    let preview = parsed
        .body_preview(180)
        .map(|preview| normalize_whitespace(&preview))
        .unwrap_or_default();
    let body = parsed
        .body_text(0)
        .map(|body| body.trim().to_owned())
        .unwrap_or_default();
    let html = parsed.body_html(0).map(|html| html.to_string());
    let mut inline_by_cid: HashMap<String, InlineImage> = HashMap::new();
    let mut attachments = Vec::new();
    for (part_index, part) in parsed.parts.iter().enumerate() {
        if part.is_multipart() {
            continue;
        }
        let mime = part
            .content_type()
            .map(|content_type| {
                format!(
                    "{}/{}",
                    content_type.c_type,
                    content_type.subtype().unwrap_or("octet-stream")
                )
            })
            .unwrap_or_else(|| "application/octet-stream".to_owned());
        let content_id = part.content_id().map(normalize_cid);
        let disposition = part.content_disposition();
        let is_image = part
            .content_type()
            .is_some_and(|content_type| content_type.c_type.eq_ignore_ascii_case("image"));
        let is_attachment = disposition
            .is_some_and(|disposition| disposition.ctype().eq_ignore_ascii_case("attachment"))
            || part.attachment_name().is_some();
        let is_inline = content_id.is_some()
            || disposition
                .is_some_and(|disposition| disposition.ctype().eq_ignore_ascii_case("inline"));
        if is_image && is_inline {
            let data = part.contents();
            if !data.is_empty() && data.len() <= 2_000_000 {
                let image = InlineImage {
                    content_id: content_id.clone(),
                    mime,
                    data: STANDARD.encode(data),
                };
                if let Some(content_id) = &content_id {
                    inline_by_cid.insert(content_id.clone(), image);
                }
            }
            continue;
        }
        if (part.is_text() || part.is_text_html()) && !is_attachment {
            continue;
        }
        attachments.push(Attachment {
            name: part.attachment_name().unwrap_or("attachment").to_owned(),
            size: part.len(),
            mime,
            part_index,
        });
    }

    let has_html_body = parsed.parts.iter().any(|part| {
        part.is_text_html()
            && !part
                .content_disposition()
                .is_some_and(|disposition| disposition.ctype().eq_ignore_ascii_case("attachment"))
            && part.attachment_name().is_none()
    });
    let body_blocks = if has_html_body {
        let blocks = html
            .as_deref()
            .map(|html| crate::body::parse_html_body(html, &inline_by_cid))
            .unwrap_or_default();
        if blocks.is_empty() {
            crate::body::parse_plain_body(&body)
        } else {
            blocks
        }
    } else {
        crate::body::parse_plain_body(&body)
    };

    Message {
        uid,
        folder_id: folder_id.to_owned(),
        raw_path: None,
        raw_size: raw.len(),
        subject: parsed
            .subject()
            .filter(|subject| !subject.trim().is_empty())
            .unwrap_or("(no subject)")
            .to_owned(),
        from_name,
        from_email,
        to,
        timestamp,
        preview,
        body,
        unread,
        starred,
        attachments,
        body_blocks,
        message_id: parsed.message_id().map(str::to_owned),
    }
}

fn normalize_cid(value: &str) -> String {
    value
        .trim()
        .trim_matches(['<', '>', '"', '\''])
        .trim()
        .to_owned()
}

fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_message() {
        let raw = b"From: Maja <maja@example.com>\r\nTo: me@example.com\r\nSubject: Test\r\nDate: Wed, 21 Oct 2026 10:00:00 +0200\r\nMessage-ID: <abc@example.com>\r\n\r\nHello there.\r\n";
        let message = parse_message("INBOX", 7, raw, true, false);
        assert_eq!(message.uid, 7);
        assert_eq!(message.subject, "Test");
        assert_eq!(message.from_name, "Maja");
        assert_eq!(message.from_email, "maja@example.com");
        assert!(message.body.contains("Hello there"));
        assert!(message.unread);
    }

    #[test]
    fn normalizes_preview_whitespace() {
        assert_eq!(normalize_whitespace("  hello\n\n world "), "hello world");
    }

    #[test]
    fn parses_multiple_nested_signatures() {
        let raw = br#"From: Maja <maja@example.com>
To: me@example.com
Subject: Thread
Date: Wed, 21 Oct 2026 10:00:00 +0200
Message-ID: <thread@example.com>
MIME-Version: 1.0
Content-Type: multipart/related; boundary="b1"

--b1
Content-Type: text/html; charset=utf-8

<html><body><div>Main text</div><div class="gmail_quote"><div class="gmail_attr">On Wed, Jovan wrote:</div><blockquote><div>First quoted message</div><div class="gmail_signature">Jovan signature</div><blockquote><div class="gmail_attr">On Tue, Marko wrote:</div><div>Second quoted message</div><div class="gmail_signature">Marko signature</div></blockquote></blockquote></div></body></html>
--b1
Content-Type: image/png; name="logo.png"
Content-ID: <logo@test>
Content-Disposition: inline; filename="logo.png"
Content-Transfer-Encoding: base64

iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=
--b1--
"#;
        let message = parse_message("INBOX", 11, raw, false, false);
        fn count_signatures(blocks: &[crate::model::BodyBlock]) -> usize {
            blocks
                .iter()
                .map(|block| match block {
                    crate::model::BodyBlock::Signature { .. } => 1,
                    crate::model::BodyBlock::Quote { blocks, .. } => count_signatures(blocks),
                    crate::model::BodyBlock::Text(_) => 0,
                })
                .sum()
        }
        fn count_quotes(blocks: &[crate::model::BodyBlock]) -> usize {
            blocks
                .iter()
                .map(|block| match block {
                    crate::model::BodyBlock::Quote { blocks, .. } => 1 + count_quotes(blocks),
                    _ => 0,
                })
                .sum()
        }
        assert_eq!(count_signatures(&message.body_blocks), 2);
        assert_eq!(count_quotes(&message.body_blocks), 2);
    }

    #[test]
    fn wraps_outlook_quote_and_nested_messages() {
        let raw = br#"From: Marko <marko@example.com>
To: me@example.com
Subject: Outlook thread
Date: Wed, 21 Oct 2026 10:00:00 +0200
Message-ID: <outlook-thread@example.com>
Content-Type: text/html; charset=utf-8

<html><body><div>Main text</div><div class="gmail_signature">--<br>Marko signature</div><div id="appendonsend"></div><div id="divRplyFwdMsg"><div><b>From:</b> Rada</div><div><b>Sent:</b> Wednesday</div><div><b>To:</b> Marko</div><div><b>Subject:</b> Re: Fw:</div></div><div>Quoted body</div><div class="gmail_signature">Rada signature</div><div class="gmail_attr">On 21.10.26 08:26, Rosa wrote:</div><blockquote><div>Nested text</div><div class="gmail_signature">Rosa signature</div></blockquote></body></html>
"#;
        let message = parse_message("INBOX", 13, raw, false, false);
        fn count(blocks: &[crate::model::BodyBlock], signatures: &mut usize, quotes: &mut usize) {
            for block in blocks {
                match block {
                    crate::model::BodyBlock::Signature { text, .. } => {
                        *signatures += 1;
                        let text = text.as_deref().unwrap_or_default();
                        assert!(!text.starts_with("--"));
                    }
                    crate::model::BodyBlock::Quote { header, blocks } => {
                        *quotes += 1;
                        if let Some(header) = header {
                            assert!(!header.contains("Main text"));
                        }
                        count(blocks, signatures, quotes);
                    }
                    crate::model::BodyBlock::Text(_) => {}
                }
            }
        }
        let mut signatures = 0;
        let mut quotes = 0;
        count(&message.body_blocks, &mut signatures, &mut quotes);
        assert_eq!(signatures, 3);
        assert_eq!(quotes, 2);
        let first_quote = message
            .body_blocks
            .iter()
            .find(|block| matches!(block, crate::model::BodyBlock::Quote { .. }))
            .expect("outer quote");
        let crate::model::BodyBlock::Quote { blocks, .. } = first_quote else {
            unreachable!()
        };
        assert!(blocks.iter().any(|block| {
            matches!(block, crate::model::BodyBlock::Text(text) if text.contains("Quoted body"))
        }));
    }

    #[test]
    fn splits_signature_without_delimiter() {
        let raw = r#"From: Marja <marja@example.com>
To: me@example.com
Subject: Closing
Date: Wed, 21 Oct 2026 10:00:00 +0200
Message-ID: <closing@example.com>
Content-Type: text/html; charset=utf-8

<html><body><div>Postovani, hvala na informacijama.</div><div>Srdačan pozdrav,<br>Marja<br>KIM DOOEL<br>www.kim.com.mk</div></body></html>
"#;
        let message = parse_message("INBOX", 14, raw.as_bytes(), false, false);
        let signature = message
            .body_blocks
            .iter()
            .find_map(|block| {
                if let crate::model::BodyBlock::Signature { text, .. } = block {
                    text.as_deref()
                } else {
                    None
                }
            })
            .expect("signature");
        assert!(signature.starts_with("Srdačan pozdrav,"));
        assert!(signature.contains("www.kim.com.mk"));
        assert!(message.body.contains("Postovani"));
        assert!(message.body_blocks.iter().all(|block| {
            !matches!(block, crate::model::BodyBlock::Text(text) if text.contains("www.kim.com.mk"))
        }));
    }

    #[test]
    fn wraps_forwarded_message_in_html() {
        let raw = br#"From: Marko <marko@example.com>
To: me@example.com
Subject: Fwd: Invoice
Date: Wed, 21 Oct 2026 10:00:00 +0200
Message-ID: <forwarded@example.com>
Content-Type: text/html; charset=utf-8

<html><body><div>See below.</div><div>-------- Forwarded Message --------</div><div><b>Subject:</b> Invoice</div><div><b>From:</b> Bank</div><div>Forwarded body</div></body></html>
"#;
        let message = parse_message("INBOX", 16, raw, false, false);
        let quote = message
            .body_blocks
            .iter()
            .find_map(|block| {
                if let crate::model::BodyBlock::Quote { blocks, .. } = block {
                    Some(blocks)
                } else {
                    None
                }
            })
            .expect("forwarded quote");
        assert!(quote.iter().any(|block| {
            matches!(block, crate::model::BodyBlock::Text(text) if text.contains("Forwarded body"))
        }));
        assert!(message.body_blocks.iter().all(|block| {
            !matches!(block, crate::model::BodyBlock::Text(text) if text.contains("Forwarded body"))
        }));
    }

    #[test]
    fn wraps_forwarded_message_in_plain_text() {
        let raw = b"From: Marko <marko@example.com>
To: me@example.com
Subject: Fwd: Invoice
Date: Wed, 21 Oct 2026 10:00:00 +0200
Message-ID: <forwarded-plain@example.com>

See below.

-------- Forwarded Message --------
Subject: Invoice
From: Bank

Forwarded body.
";
        let message = parse_message("INBOX", 17, raw, false, false);
        let quote = message
            .body_blocks
            .iter()
            .find_map(|block| {
                if let crate::model::BodyBlock::Quote { header, blocks } = block {
                    Some((header.as_deref(), blocks))
                } else {
                    None
                }
            })
            .expect("forwarded quote");
        assert!(quote.0.expect("header").contains("Forwarded Message"));
        assert!(quote.1.iter().any(|block| {
            matches!(block, crate::model::BodyBlock::Text(text) if text.contains("Forwarded body"))
        }));
    }

    #[test]
    fn keeps_multiline_attribution_with_quote() {
        let raw = br#"From: Rosa <rosa@example.com>
To: me@example.com
Subject: Attribution
Date: Wed, 21 Oct 2026 10:00:00 +0200
Message-ID: <attribution@example.com>
Content-Type: text/html; charset=utf-8

<html><body><div>Body text</div><div>On 21.10.26 08:26, Rosa &lt;rosa@example.com&gt;<br>wrote:</div><blockquote><div>Quoted text</div></blockquote></body></html>
"#;
        let message = parse_message("INBOX", 15, raw, false, false);
        let quote = message
            .body_blocks
            .iter()
            .find_map(|block| {
                if let crate::model::BodyBlock::Quote { header, blocks } = block {
                    Some((header.as_deref(), blocks))
                } else {
                    None
                }
            })
            .expect("quote");
        let header = quote.0.expect("quote header");
        assert!(header.contains("On 21.10.26"), "header: {header:?}");
        assert!(header.contains("wrote:"));
        assert!(!header.contains("Body text"));
        assert!(message.body.contains("Body text"));
        assert!(quote.1.iter().any(|block| {
            matches!(block, crate::model::BodyBlock::Text(text) if text.contains("Quoted text"))
        }));
    }

    #[test]
    fn parses_plain_nested_quotes_and_signatures() {
        let raw = b"From: Alice <alice@example.com>\r\nTo: me@example.com\r\nSubject: Plain thread\r\nDate: Wed, 21 Oct 2026 10:00:00 +0200\r\nMessage-ID: <plain-thread@example.com>\r\n\r\nMain text.\r\n\r\n-- \r\nAlice signature\r\n\r\nOn Wed, Jovan wrote:\r\n> First quoted message\r\n> \r\n> -- \r\n> Jovan signature\r\n> \r\n> On Tue, Marko wrote:\r\n> > Second quoted message\r\n> > -- \r\n> > Marko signature\r\n";
        let message = parse_message("INBOX", 12, raw, false, false);
        fn count(blocks: &[crate::model::BodyBlock], signatures: &mut usize, quotes: &mut usize) {
            for block in blocks {
                match block {
                    crate::model::BodyBlock::Signature { .. } => *signatures += 1,
                    crate::model::BodyBlock::Quote { blocks, .. } => {
                        *quotes += 1;
                        count(blocks, signatures, quotes);
                    }
                    crate::model::BodyBlock::Text(_) => {}
                }
            }
        }
        let mut signatures = 0;
        let mut quotes = 0;
        count(&message.body_blocks, &mut signatures, &mut quotes);
        assert_eq!(signatures, 3);
        assert_eq!(quotes, 2);
    }

    #[test]
    fn parses_html_signature_with_inline_image() {
        let raw = br#"From: Maja <maja@example.com>
To: me@example.com
Subject: Signature test
Date: Wed, 21 Oct 2026 10:00:00 +0200
Message-ID: <signature@example.com>
MIME-Version: 1.0
Content-Type: multipart/related; boundary="b1"

--b1
Content-Type: text/html; charset=utf-8

<html><body><p>Hello team,</p><p>This is the message body.</p><div class="gmail_signature"><div>Maja Petrovic</div><div>Studio Atlas</div><div><img src="cid:logo@test" width="32"></div></div></body></html>
--b1
Content-Type: image/png; name="logo.png"
Content-ID: <logo@test>
Content-Disposition: inline; filename="logo.png"
Content-Transfer-Encoding: base64

iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=
--b1--
"#;
        let message = parse_message("INBOX", 9, raw, false, false);
        assert!(message.attachments.is_empty());
        let signature = message
            .body_blocks
            .iter()
            .find_map(|block| {
                if let crate::model::BodyBlock::Signature { text, images } = block {
                    Some((text.as_deref(), images))
                } else {
                    None
                }
            })
            .expect("signature block");
        assert!(
            signature
                .0
                .expect("signature text")
                .contains("Maja Petrovic")
        );
        assert_eq!(signature.1.len(), 1);
        assert!(message.body.contains("Hello team"));
    }

    #[test]
    fn splits_plain_text_signature() {
        let raw = b"From: Alice <alice@example.com>\r\nTo: me@example.com\r\nSubject: Plain signature\r\nDate: Wed, 21 Oct 2026 10:00:00 +0200\r\nMessage-ID: <plain-signature@example.com>\r\n\r\nBody text.\r\n\r\n-- \r\nAlice\r\nExample Ltd\r\n";
        let message = parse_message("INBOX", 10, raw, false, false);
        assert!(message.body.contains("Body text."));
        let signature = message
            .body_blocks
            .iter()
            .find_map(|block| {
                if let crate::model::BodyBlock::Signature { text, .. } = block {
                    text.as_deref()
                } else {
                    None
                }
            })
            .expect("signature");
        assert!(signature.contains("Alice"));
        assert!(signature.contains("Example Ltd"));
    }

    #[test]
    fn sanitizes_attachment_names() {
        let name = sanitize_file_name("../../secret/report.pdf");
        assert!(!name.contains('/'));
        assert!(name.ends_with("report.pdf"));
        assert_eq!(sanitize_file_name(".."), "attachment");
        assert_eq!(sanitize_file_name(" "), "attachment");
        assert_eq!(sanitize_file_name("note.pdf"), "note.pdf");
    }

    #[test]
    fn parses_html_and_attachment() {
        let raw = br#"From: Bob <bob@example.com>
To: me@example.com
Subject: Multipart test
Date: Wed, 21 Oct 2026 11:15:00 +0200
Message-ID: <two@example.com>
MIME-Version: 1.0
Content-Type: multipart/mixed; boundary="b1"

--b1
Content-Type: text/html; charset=utf-8

<p>Hello <b>there</b>.</p>
--b1
Content-Type: application/pdf; name="note.pdf"
Content-Disposition: attachment; filename="note.pdf"
Content-Transfer-Encoding: base64

JVBERi0xLjQK
--b1--
"#;
        let message = parse_message("INBOX", 2, raw, false, true);
        assert_eq!(message.subject, "Multipart test");
        assert!(message.starred);
        assert!(message.body.contains("Hello"));
        assert_eq!(message.attachments.len(), 1);
        assert_eq!(message.attachments[0].name, "note.pdf");
        assert_eq!(message.attachments[0].mime, "application/pdf");
        assert!(message.attachments[0].size > 0);
        assert!(message.attachments[0].part_index > 0);
        let parsed = MessageParser::default().parse(raw).expect("valid message");
        let part = &parsed.parts[message.attachments[0].part_index];
        assert!(part.contents().starts_with(b"%PDF"));
        let mut stale = message.attachments[0].clone();
        stale.part_index = 0;
        let resolved = resolve_attachment_part(&parsed.parts, &stale).expect("fallback part");
        assert!(resolved.contents().starts_with(b"%PDF"));
    }
}
