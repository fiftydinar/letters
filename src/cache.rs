use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use directories::BaseDirs;
use serde::{Deserialize, Serialize};

use crate::model::{Folder, Message};

const CACHE_VERSION: u32 = 4;

#[derive(Serialize, Deserialize)]
struct CacheFile {
    version: u32,
    email: String,
    folders: Vec<Folder>,
    messages: HashMap<String, Vec<Message>>,
    current_folder: String,
    saved_at: i64,
}

pub struct MailCache {
    pub folders: Vec<Folder>,
    pub messages: HashMap<String, Vec<Message>>,
    pub current_folder: String,
}

pub fn load(email: &str) -> Option<MailCache> {
    let path = cache_path()?;
    let data = fs::read(&path).ok()?;
    let cache: CacheFile = serde_json::from_slice(&data).ok()?;
    if cache.version != CACHE_VERSION || !cache.email.eq_ignore_ascii_case(email) {
        return None;
    }
    Some(MailCache {
        folders: cache.folders,
        messages: cache.messages,
        current_folder: cache.current_folder,
    })
}

pub fn save(
    email: &str,
    folders: &[Folder],
    messages: &HashMap<String, Vec<Message>>,
    current_folder: &str,
) -> std::io::Result<()> {
    let Some(path) = cache_path() else {
        return Err(std::io::Error::other("no cache path available"));
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let cache = CacheFile {
        version: CACHE_VERSION,
        email: email.to_owned(),
        folders: folders.to_vec(),
        messages: messages.clone(),
        current_folder: current_folder.to_owned(),
        saved_at: crate::model::now_timestamp(),
    };
    let data = serde_json::to_vec(&cache).map_err(std::io::Error::other)?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, data)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600));
    }
    fs::rename(temporary, path)
}

pub fn clear() {
    if let Some(path) = cache_path() {
        let _ = fs::remove_file(path);
    }
}

fn cache_path() -> Option<PathBuf> {
    BaseDirs::new().map(|dirs| dirs.data_dir().join("letters").join("cache.json"))
}
