use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use directories::BaseDirs;
use serde::{Deserialize, Serialize};

use crate::model::{Account, Security, ServerConfig};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefaultStore {
    #[default]
    Remote,
    Local,
}

impl DefaultStore {
    pub fn label(self) -> &'static str {
        match self {
            Self::Remote => "Remote inbox",
            Self::Local => "Local folders",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub account: Option<Account>,
    pub demo: bool,
    pub sync_secs: u64,
    pub notifications: bool,
    pub close_to_tray: bool,
    pub save_sent_copy: bool,
    pub local_mode: bool,
    pub default_store: DefaultStore,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            account: None,
            demo: false,
            sync_secs: 60,
            notifications: true,
            close_to_tray: true,
            save_sent_copy: true,
            local_mode: false,
            default_store: DefaultStore::Remote,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let Some(path) = config_path() else {
            return Self::default();
        };
        match fs::read_to_string(&path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_else(|err| {
                log::warn!("Could not read config: {err}");
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = config_path() else {
            return Err(std::io::Error::other("no config path available"));
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, data)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600));
        }
        fs::rename(tmp, path)
    }
}

pub fn config_path() -> Option<PathBuf> {
    BaseDirs::new().map(|dirs| dirs.config_dir().join("letters").join("config.json"))
}

pub fn load_password(email: &str) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("secret-tool")
            .args(["lookup", "application", "letters", "email", email])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let password = String::from_utf8(output.stdout).ok()?;
        let password = password.trim_end_matches(['\r', '\n']).to_owned();
        (!password.is_empty()).then_some(password)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = email;
        None
    }
}

pub fn save_password(email: &str, password: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        let label = format!("Letters: {email}");
        let Ok(mut child) = Command::new("secret-tool")
            .args([
                "store",
                "--label",
                &label,
                "application",
                "letters",
                "email",
                email,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            return false;
        };
        let Some(stdin) = child.stdin.as_mut() else {
            return false;
        };
        if stdin.write_all(password.as_bytes()).is_err() || stdin.write_all(b"\n").is_err() {
            return false;
        }
        child.wait().map(|status| status.success()).unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (email, password);
        false
    }
}

pub fn delete_password(email: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        Command::new("secret-tool")
            .args(["clear", "application", "letters", "email", email])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = email;
        false
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ProviderPreset {
    pub name: &'static str,
    pub imap_host: &'static str,
    pub imap_port: u16,
    pub imap_security: Security,
    pub smtp_host: &'static str,
    pub smtp_port: u16,
    pub smtp_security: Security,
    pub hint: &'static str,
}

pub const PRESETS: [ProviderPreset; 5] = [
    ProviderPreset {
        name: "Gmail",
        imap_host: "imap.gmail.com",
        imap_port: 993,
        imap_security: Security::Implicit,
        smtp_host: "smtp.gmail.com",
        smtp_port: 465,
        smtp_security: Security::Implicit,
        hint: "Gmail requires an app password (2FA must be enabled).",
    },
    ProviderPreset {
        name: "Outlook",
        imap_host: "outlook.office365.com",
        imap_port: 993,
        imap_security: Security::Implicit,
        smtp_host: "smtp-mail.outlook.com",
        smtp_port: 587,
        smtp_security: Security::Starttls,
        hint: "Outlook may require an app password or OAuth.",
    },
    ProviderPreset {
        name: "Fastmail",
        imap_host: "imap.fastmail.com",
        imap_port: 993,
        imap_security: Security::Implicit,
        smtp_host: "smtp.fastmail.com",
        smtp_port: 465,
        smtp_security: Security::Implicit,
        hint: "Create an app password for Letters in Fastmail.",
    },
    ProviderPreset {
        name: "Yahoo",
        imap_host: "imap.mail.yahoo.com",
        imap_port: 993,
        imap_security: Security::Implicit,
        smtp_host: "smtp.mail.yahoo.com",
        smtp_port: 465,
        smtp_security: Security::Implicit,
        hint: "Yahoo requires an app password in account settings.",
    },
    ProviderPreset {
        name: "Other",
        imap_host: "",
        imap_port: 993,
        imap_security: Security::Implicit,
        smtp_host: "",
        smtp_port: 465,
        smtp_security: Security::Implicit,
        hint: "Enter your provider IMAP and SMTP details.",
    },
];

#[derive(Clone, Debug)]
pub struct AccountForm {
    pub preset: usize,
    pub name: String,
    pub email: String,
    pub username: String,
    pub password: String,
    pub imap_host: String,
    pub imap_port: String,
    pub imap_security: Security,
    pub smtp_host: String,
    pub smtp_port: String,
    pub smtp_security: Security,
    pub remember: bool,
    pub advanced: bool,
}

impl Default for AccountForm {
    fn default() -> Self {
        let preset = &PRESETS[0];
        Self {
            preset: 0,
            name: String::new(),
            email: String::new(),
            username: String::new(),
            password: String::new(),
            imap_host: preset.imap_host.to_owned(),
            imap_port: preset.imap_port.to_string(),
            imap_security: preset.imap_security,
            smtp_host: preset.smtp_host.to_owned(),
            smtp_port: preset.smtp_port.to_string(),
            smtp_security: preset.smtp_security,
            remember: true,
            advanced: false,
        }
    }
}

impl AccountForm {
    pub fn from_account(account: &Account) -> Self {
        Self {
            name: account.name.clone(),
            email: account.email.clone(),
            username: account.username.clone(),
            imap_host: account.imap.host.clone(),
            imap_port: account.imap.port.to_string(),
            imap_security: account.imap.security,
            smtp_host: account.smtp.host.clone(),
            smtp_port: account.smtp.port.to_string(),
            smtp_security: account.smtp.security,
            preset: PRESETS.len() - 1,
            ..Self::default()
        }
    }

    pub fn apply_preset(&mut self, index: usize) {
        let Some(preset) = PRESETS.get(index) else {
            return;
        };
        self.preset = index;
        self.imap_host = preset.imap_host.to_owned();
        self.imap_port = preset.imap_port.to_string();
        self.imap_security = preset.imap_security;
        self.smtp_host = preset.smtp_host.to_owned();
        self.smtp_port = preset.smtp_port.to_string();
        self.smtp_security = preset.smtp_security;
    }

    pub fn preset_hint(&self) -> &'static str {
        PRESETS
            .get(self.preset)
            .map(|preset| preset.hint)
            .unwrap_or("")
    }

    pub fn to_account(&self) -> Result<Account, String> {
        let email = self.email.trim();
        if !email.contains('@') || email.starts_with('@') || email.ends_with('@') {
            return Err("Enter a valid email address.".to_owned());
        }
        let imap_host = self.imap_host.trim();
        let smtp_host = self.smtp_host.trim();
        if imap_host.is_empty() || smtp_host.is_empty() {
            return Err("IMAP and SMTP hosts cannot be empty.".to_owned());
        }
        let imap_port = self
            .imap_port
            .trim()
            .parse::<u16>()
            .map_err(|_| "IMAP port is invalid.".to_owned())?;
        let smtp_port = self
            .smtp_port
            .trim()
            .parse::<u16>()
            .map_err(|_| "SMTP port is invalid.".to_owned())?;
        let name = if self.name.trim().is_empty() {
            email.split('@').next().unwrap_or(email).to_owned()
        } else {
            self.name.trim().to_owned()
        };
        let username = if self.username.trim().is_empty() {
            email.to_owned()
        } else {
            self.username.trim().to_owned()
        };

        Ok(Account {
            name,
            email: email.to_owned(),
            username,
            imap: ServerConfig::new(imap_host, imap_port, self.imap_security),
            smtp: ServerConfig::new(smtp_host, smtp_port, self.smtp_security),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_fills_servers() {
        let mut form = AccountForm::default();
        form.apply_preset(2);
        assert_eq!(form.imap_host, "imap.fastmail.com");
        assert_eq!(form.smtp_host, "smtp.fastmail.com");
        assert_eq!(form.smtp_security, Security::Implicit);
    }

    #[test]
    fn validates_account_form() {
        let mut form = AccountForm {
            email: "maja@example.com".to_owned(),
            ..AccountForm::default()
        };
        let account = form.to_account().expect("valid account");
        assert_eq!(account.username, "maja@example.com");
        assert_eq!(account.name, "maja");
        assert!(form.to_account().is_ok());
        form.imap_host.clear();
        assert!(form.to_account().is_err());
    }

    #[test]
    fn config_uses_safe_defaults() {
        let config = Config::default();
        assert_eq!(config.sync_secs, 60);
        assert!(config.notifications);
        assert!(config.account.is_none());
        assert!(config.save_sent_copy);
        assert!(!config.local_mode);
        assert_eq!(config.default_store, DefaultStore::Remote);
    }
}
