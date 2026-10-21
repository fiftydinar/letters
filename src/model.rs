use std::fmt;

use chrono::{Datelike, Local, TimeZone};
use serde::{Deserialize, Serialize};

pub const DEMO_EMAIL: &str = "studio@letters.app";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Security {
    Implicit,
    Starttls,
}

impl Security {
    pub fn label(self) -> &'static str {
        match self {
            Self::Implicit => "TLS",
            Self::Starttls => "STARTTLS",
        }
    }

    pub fn options() -> [Self; 2] {
        [Self::Implicit, Self::Starttls]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub security: Security,
}

impl ServerConfig {
    pub fn new(host: impl Into<String>, port: u16, security: Security) -> Self {
        Self {
            host: host.into(),
            port,
            security,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub name: String,
    pub email: String,
    pub username: String,
    pub imap: ServerConfig,
    pub smtp: ServerConfig,
}

impl Account {
    pub fn display_name(&self) -> &str {
        if self.name.trim().is_empty() {
            &self.email
        } else {
            &self.name
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FolderKind {
    Inbox,
    Sent,
    Drafts,
    Archive,
    Trash,
    Custom,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub remote: String,
    pub kind: FolderKind,
    pub unread: usize,
    #[serde(default)]
    pub local: bool,
}

impl Folder {
    pub fn new(remote: impl Into<String>, name: impl Into<String>, kind: FolderKind) -> Self {
        let remote = remote.into();
        Self {
            id: remote.clone(),
            name: name.into(),
            remote,
            kind,
            unread: 0,
            local: false,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Attachment {
    pub name: String,
    pub size: usize,
    pub mime: String,
    #[serde(default)]
    pub part_index: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InlineImage {
    pub content_id: Option<String>,
    pub mime: String,
    pub data: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BodyBlock {
    Text(String),
    Quote {
        header: Option<String>,
        blocks: Vec<BodyBlock>,
    },
    Signature {
        text: Option<String>,
        images: Vec<InlineImage>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub uid: u32,
    pub folder_id: String,
    #[serde(default)]
    pub raw_path: Option<String>,
    #[serde(default)]
    pub raw_size: usize,
    pub subject: String,
    pub from_name: String,
    pub from_email: String,
    pub to: String,
    pub timestamp: i64,
    pub preview: String,
    pub body: String,
    pub unread: bool,
    pub starred: bool,
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub body_blocks: Vec<BodyBlock>,
    pub message_id: Option<String>,
}

impl Message {
    pub fn sender_label(&self) -> &str {
        if self.from_name.trim().is_empty() {
            &self.from_email
        } else {
            &self.from_name
        }
    }

    pub fn has_attachments(&self) -> bool {
        !self.attachments.is_empty()
    }

    pub fn short_time(&self) -> String {
        short_time(self.timestamp)
    }

    pub fn long_date(&self) -> String {
        long_date(self.timestamp)
    }
}

#[derive(Clone, Debug)]
pub struct ComposeDraft {
    pub to: String,
    pub subject: String,
    pub body: String,
    pub in_reply_to: Option<String>,
}

impl ComposeDraft {
    pub fn empty() -> Self {
        Self {
            to: String::new(),
            subject: String::new(),
            body: String::new(),
            in_reply_to: None,
        }
    }

    pub fn reply_to(message: &Message) -> Self {
        let subject = if message.subject.to_lowercase().starts_with("re:") {
            message.subject.clone()
        } else {
            format!("Re: {}", message.subject)
        };
        let quoted = message
            .body
            .lines()
            .map(|line| format!("> {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        Self {
            to: message.from_email.clone(),
            subject,
            body: format!("\n\n---\n{} wrote:\n{}", message.sender_label(), quoted),
            in_reply_to: message.message_id.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MailFilter {
    #[default]
    All,
    Unread,
    Starred,
}

impl MailFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Unread => "Unread",
            Self::Starred => "Starred",
        }
    }
}

impl fmt::Display for MailFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

pub fn initials(name: &str) -> String {
    let mut chars = name
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>();
    if chars.is_empty() {
        chars = name.trim().chars().take(1).collect();
    }
    chars.to_uppercase()
}

pub fn classify_folder(remote: &str) -> FolderKind {
    let lower = remote.to_lowercase();
    let leaf = lower.rsplit(['/', '.']).next().unwrap_or(&lower);

    if lower == "inbox" {
        FolderKind::Inbox
    } else if leaf == "sent"
        || leaf == "sent items"
        || leaf == "sent mail"
        || leaf == "sent messages"
    {
        FolderKind::Sent
    } else if leaf == "drafts" || leaf == "draft" {
        FolderKind::Drafts
    } else if leaf == "trash"
        || leaf == "deleted"
        || leaf == "deleted items"
        || leaf == "deleted messages"
        || leaf == "bin"
    {
        FolderKind::Trash
    } else if leaf == "archive" || leaf == "archives" || leaf == "all mail" || leaf == "arhiva" {
        FolderKind::Archive
    } else {
        FolderKind::Custom
    }
}

pub fn short_time(timestamp: i64) -> String {
    let Some(dt) = Local.timestamp_opt(timestamp, 0).single() else {
        return String::new();
    };
    let now = Local::now();
    let days = (now.date_naive() - dt.date_naive()).num_days();
    if days <= 0 {
        dt.format("%H:%M").to_string()
    } else if days == 1 {
        "Yesterday".to_owned()
    } else if dt.year() == now.year() {
        format!("{} {}", month_short(dt.month()), dt.format("%-d"))
    } else {
        dt.format("%b %-d, %Y").to_string()
    }
}

pub fn long_date(timestamp: i64) -> String {
    let Some(dt) = Local.timestamp_opt(timestamp, 0).single() else {
        return String::new();
    };
    format!(
        "{} {}, {} at {}",
        month_long(dt.month()),
        dt.format("%-d"),
        dt.year(),
        dt.format("%H:%M")
    )
}

pub fn month_short(month: u32) -> &'static str {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    MONTHS
        .get(month.saturating_sub(1) as usize)
        .copied()
        .unwrap_or("")
}

pub fn month_long(month: u32) -> &'static str {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    MONTHS
        .get(month.saturating_sub(1) as usize)
        .copied()
        .unwrap_or("")
}

pub fn format_size(bytes: usize) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let bytes = bytes as f64;
    if bytes >= MB {
        format!("{:.1} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes / KB)
    } else {
        format!("{bytes:.0} B")
    }
}

pub fn now_timestamp() -> i64 {
    Local::now().timestamp()
}

pub fn inline_image_key(image: &InlineImage) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    image.mime.hash(&mut hasher);
    image.data.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_special_folders() {
        assert_eq!(classify_folder("INBOX"), FolderKind::Inbox);
        assert_eq!(classify_folder("[Gmail]/Sent Mail"), FolderKind::Sent);
        assert_eq!(classify_folder("Sent Items"), FolderKind::Sent);
        assert_eq!(classify_folder("[Gmail]/Drafts"), FolderKind::Drafts);
        assert_eq!(classify_folder("Deleted Messages"), FolderKind::Trash);
        assert_eq!(classify_folder("[Gmail]/All Mail"), FolderKind::Archive);
        assert_eq!(classify_folder("Projects"), FolderKind::Custom);
    }

    #[test]
    fn builds_initials() {
        assert_eq!(initials("Maja Petrović"), "MP");
        assert_eq!(initials("studio"), "S");
        assert_eq!(initials("  "), "");
    }

    #[test]
    fn creates_reply_draft() {
        let message = Message {
            uid: 1,
            folder_id: "INBOX".to_owned(),
            raw_path: None,
            raw_size: 0,
            subject: "Hello".to_owned(),
            from_name: "Ana".to_owned(),
            from_email: "ana@example.com".to_owned(),
            to: "me@example.com".to_owned(),
            timestamp: 0,
            preview: String::new(),
            body: "What's up?".to_owned(),
            unread: false,
            starred: false,
            attachments: Vec::new(),
            body_blocks: Vec::new(),
            message_id: Some("<1@example.com>".to_owned()),
        };
        let draft = ComposeDraft::reply_to(&message);
        assert_eq!(draft.to, "ana@example.com");
        assert_eq!(draft.subject, "Re: Hello");
        assert!(draft.body.contains("> What's up?"));
    }
}
