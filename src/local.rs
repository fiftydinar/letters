use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use directories::{BaseDirs, UserDirs};
use serde::{Deserialize, Serialize};

use mail_parser::{MessageParser, MimeHeaders};

use crate::mail::{parse_message, sanitize_file_name, unique_path};
use crate::model::{Attachment, Folder, FolderKind, Message};

const LOCAL_VERSION: u32 = 1;
const LOCAL_PREFIX: &str = "local:";

#[derive(Serialize, Deserialize)]
struct LocalIndex {
    version: u32,
    next_uid: u32,
    folders: Vec<Folder>,
    messages: HashMap<String, Vec<Message>>,
}

pub struct LocalStore {
    base: PathBuf,
    index: LocalIndex,
}

impl LocalStore {
    pub fn load() -> Self {
        let Some(base) = base_dir() else {
            return Self::in_memory();
        };
        let path = base.join("index.json");
        let index = fs::read(&path)
            .ok()
            .and_then(|data| serde_json::from_slice::<LocalIndex>(&data).ok())
            .filter(|index| index.version == LOCAL_VERSION)
            .unwrap_or_else(Self::default_index);
        let mut store = Self { base, index };
        store.ensure_default_folders();
        store
    }

    fn in_memory() -> Self {
        Self {
            base: std::env::temp_dir().join("letters-local"),
            index: Self::default_index(),
        }
    }

    #[cfg(test)]
    fn at(base: PathBuf) -> Self {
        let path = base.join("index.json");
        let index = fs::read(&path)
            .ok()
            .and_then(|data| serde_json::from_slice::<LocalIndex>(&data).ok())
            .filter(|index| index.version == LOCAL_VERSION)
            .unwrap_or_else(Self::default_index);
        let mut store = Self { base, index };
        store.ensure_default_folders();
        store
    }

    fn default_index() -> LocalIndex {
        let folders = [
            ("local:inbox", "Inbox", FolderKind::Inbox),
            ("local:sent", "Sent", FolderKind::Sent),
            ("local:drafts", "Drafts", FolderKind::Drafts),
            ("local:archive", "Archive", FolderKind::Archive),
            ("local:trash", "Trash", FolderKind::Trash),
        ]
        .into_iter()
        .map(|(id, name, kind)| Folder {
            id: id.to_owned(),
            name: name.to_owned(),
            remote: id.to_owned(),
            kind,
            unread: 0,
            local: true,
        })
        .collect();
        LocalIndex {
            version: LOCAL_VERSION,
            next_uid: 1,
            folders,
            messages: HashMap::new(),
        }
    }

    fn ensure_default_folders(&mut self) {
        let defaults = Self::default_index().folders;
        for folder in defaults {
            if !self
                .index
                .folders
                .iter()
                .any(|existing| existing.kind == folder.kind || existing.id == folder.id)
            {
                self.index.messages.entry(folder.id.clone()).or_default();
                self.index.folders.push(folder);
            }
        }
        self.index.folders.sort_by_key(local_folder_order);
    }

    pub fn folders(&self) -> Vec<Folder> {
        let mut folders = self.index.folders.clone();
        for folder in &mut folders {
            folder.unread = self
                .index
                .messages
                .get(&folder.id)
                .map(|messages| messages.iter().filter(|message| message.unread).count())
                .unwrap_or(0);
        }
        folders
    }

    pub fn messages(&self, folder_id: &str) -> Vec<Message> {
        let mut messages = self
            .index
            .messages
            .get(folder_id)
            .cloned()
            .unwrap_or_default();
        messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp).then(b.uid.cmp(&a.uid)));
        messages
    }

    pub fn create_folder(&mut self, name: &str) -> Result<Folder, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a folder name.".to_owned());
        }
        if name.chars().any(char::is_control) {
            return Err("The folder name contains invalid characters.".to_owned());
        }
        if self
            .index
            .folders
            .iter()
            .any(|folder| folder.name.eq_ignore_ascii_case(name))
        {
            return Err("A local folder with that name already exists.".to_owned());
        }
        let mut id = format!("{LOCAL_PREFIX}{}", slug(name));
        let mut suffix = 2;
        while self.index.folders.iter().any(|folder| folder.id == id) {
            id = format!("{LOCAL_PREFIX}{}-{suffix}", slug(name));
            suffix += 1;
        }
        let folder = Folder {
            id: id.clone(),
            name: name.to_owned(),
            remote: id.clone(),
            kind: FolderKind::Custom,
            unread: 0,
            local: true,
        };
        self.index.folders.push(folder.clone());
        self.index.messages.entry(id).or_default();
        self.save()?;
        Ok(folder)
    }

    pub fn store_raw(
        &mut self,
        folder_id: &str,
        raw: &[u8],
        unread: bool,
        starred: bool,
    ) -> Result<Message, String> {
        if !self
            .index
            .folders
            .iter()
            .any(|folder| folder.id == folder_id)
        {
            return Err("The local folder does not exist.".to_owned());
        }
        let mut message = parse_message(folder_id, 0, raw, unread, starred);
        if let Some(message_id) = &message.message_id
            && self.index.messages.get(folder_id).is_some_and(|messages| {
                messages
                    .iter()
                    .any(|existing| existing.message_id.as_ref() == Some(message_id))
            })
        {
            return Err("This message is already in that local folder.".to_owned());
        }
        let uid = self.index.next_uid;
        self.index.next_uid += 1;
        message.uid = uid;
        message.folder_id = folder_id.to_owned();
        message.raw_size = raw.len();
        let raw_path = self.raw_path(folder_id, uid)?;
        if let Some(parent) = raw_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Could not create the local mail folder: {error}"))?;
        }
        fs::write(&raw_path, raw)
            .map_err(|error| format!("Could not write the local message: {error}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&raw_path, fs::Permissions::from_mode(0o600));
        }
        message.raw_path = Some(raw_path.display().to_string());
        let messages = self.index.messages.entry(folder_id.to_owned()).or_default();
        messages.push(message.clone());
        messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp).then(b.uid.cmp(&a.uid)));
        self.save()?;
        Ok(message)
    }

    pub fn has_message_id_any(&self, message_id: &str) -> bool {
        self.index.messages.values().any(|messages| {
            messages
                .iter()
                .any(|message| message.message_id.as_deref() == Some(message_id))
        })
    }

    pub fn set_read(&mut self, folder_id: &str, uid: u32, unread: bool) -> Result<(), String> {
        if let Some(message) = self
            .index
            .messages
            .get_mut(folder_id)
            .and_then(|messages| messages.iter_mut().find(|message| message.uid == uid))
        {
            message.unread = unread;
            self.save()?;
        }
        Ok(())
    }

    pub fn set_star(&mut self, folder_id: &str, uid: u32, starred: bool) -> Result<(), String> {
        if let Some(message) = self
            .index
            .messages
            .get_mut(folder_id)
            .and_then(|messages| messages.iter_mut().find(|message| message.uid == uid))
        {
            message.starred = starred;
            self.save()?;
        }
        Ok(())
    }

    pub fn move_message(
        &mut self,
        source_id: &str,
        uid: u32,
        target_id: &str,
    ) -> Result<(), String> {
        if !self
            .index
            .folders
            .iter()
            .any(|folder| folder.id == target_id)
        {
            return Err("The target local folder does not exist.".to_owned());
        }
        let Some(index) = self
            .index
            .messages
            .get(source_id)
            .and_then(|messages| messages.iter().position(|message| message.uid == uid))
        else {
            return Err("The local message does not exist.".to_owned());
        };
        let mut message = self
            .index
            .messages
            .get_mut(source_id)
            .expect("source folder removed")
            .remove(index);
        message.folder_id = target_id.to_owned();
        let messages = self.index.messages.entry(target_id.to_owned()).or_default();
        messages.push(message);
        messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp).then(b.uid.cmp(&a.uid)));
        self.save()
    }

    pub fn raw_path(&self, folder_id: &str, uid: u32) -> Result<PathBuf, String> {
        let folder = folder_id.trim_start_matches(LOCAL_PREFIX);
        let safe_folder = slug(folder);
        Ok(self
            .base
            .join("messages")
            .join(safe_folder)
            .join(format!("{uid}.eml")))
    }

    fn save(&self) -> Result<(), String> {
        fs::create_dir_all(&self.base)
            .map_err(|error| format!("Could not create the local mail directory: {error}"))?;
        let path = self.base.join("index.json");
        let data = serde_json::to_vec(&self.index)
            .map_err(|error| format!("Could not serialize local mail: {error}"))?;
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, data)
            .map_err(|error| format!("Could not write local mail: {error}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600));
        }
        fs::rename(temporary, path)
            .map_err(|error| format!("Could not finalize local mail: {error}"))?;
        Ok(())
    }
}

pub fn is_local_folder(folder_id: &str) -> bool {
    folder_id.starts_with(LOCAL_PREFIX)
}

fn local_folder_order(folder: &Folder) -> (u8, String) {
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

fn base_dir() -> Option<PathBuf> {
    BaseDirs::new().map(|dirs| dirs.data_dir().join("letters").join("local"))
}

fn slug(value: &str) -> String {
    let mut slug = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    let slug = slug.trim_matches('-').to_owned();
    if slug.is_empty() {
        "folder".to_owned()
    } else {
        slug
    }
}

pub fn save_attachment(raw_path: &str, attachment: &Attachment) -> Result<PathBuf, String> {
    let raw =
        fs::read(raw_path).map_err(|error| format!("Could not read the local message: {error}"))?;
    let parsed = MessageParser::default()
        .parse(&raw)
        .ok_or_else(|| "The local message could not be parsed.".to_owned())?;
    let part = crate::mail::resolve_attachment_part(&parsed.parts, attachment)
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
    fs::write(&path, part.contents())
        .map_err(|err| format!("Could not save the attachment: {err}"))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let path = std::env::temp_dir().join(format!(
            "letters-local-test-{}-{}-{}",
            std::process::id(),
            crate::model::now_timestamp(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn stores_reads_and_moves_messages() {
        let base = test_dir();
        let mut store = LocalStore::at(base.clone());
        assert_eq!(store.folders().len(), 5);
        let raw = b"From: Alice <alice@example.com>\r\nTo: me@example.com\r\nSubject: Local test\r\nMessage-ID: <local@example.com>\r\nDate: Wed, 21 Oct 2026 10:00:00 +0200\r\n\r\nHello local.\r\n";
        let message = store
            .store_raw("local:inbox", raw, true, false)
            .expect("store raw");
        assert_eq!(message.subject, "Local test");
        assert!(message.raw_path.is_some());
        assert_eq!(store.folders()[0].unread, 1);
        let message_id = message.message_id.clone().expect("message id");
        assert!(store.has_message_id_any(&message_id));
        store.set_read("local:inbox", message.uid, false).unwrap();
        store.set_star("local:inbox", message.uid, true).unwrap();
        let stored = store.messages("local:inbox").remove(0);
        assert!(!stored.unread);
        assert!(stored.starred);
        store
            .move_message("local:inbox", stored.uid, "local:archive")
            .unwrap();
        assert!(store.messages("local:inbox").is_empty());
        assert_eq!(store.messages("local:archive").len(), 1);

        let reloaded = LocalStore::at(base.clone());
        assert_eq!(reloaded.messages("local:archive").len(), 1);
        assert_eq!(reloaded.messages("local:archive")[0].subject, "Local test");
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn creates_custom_local_folders() {
        let base = test_dir();
        let mut store = LocalStore::at(base.clone());
        let folder = store.create_folder("Projects").expect("create folder");
        assert!(folder.local);
        assert_eq!(folder.name, "Projects");
        assert!(store.create_folder("projects").is_err());
        let _ = fs::remove_dir_all(base);
    }
}
