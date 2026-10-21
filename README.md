# Letters

Letters is a fast, quiet email client built with Rust and [egui](https://github.com/emilk/egui). It is designed to open in a blink, stay out of your way, and use very little memory compared with a full desktop mail suite.

> Status: early local MVP. One remote account, IMAP/SMTP, Local Folders, tray notifications and a demo mailbox are working. See [Current limitations](#current-limitations) before using it with an important mailbox.

## Features

- Native egui interface with a responsive three-pane layout
- Nested quoted messages and multiple signatures are rendered as separate blocks
- Inline signature images from CID or base64 data parts are rendered in the message
- IMAP over TLS or STARTTLS
- SMTP over TLS or STARTTLS
- App-password sign-in for Gmail, Outlook, Fastmail, Yahoo and custom servers
- Background inbox sync with a configurable interval
- Desktop notifications for new mail
- System tray with unread count, background mode and quick actions
- Read, star, archive and trash actions synced to the mail server
- Compose, reply and optionally save sent mail to the server's Sent folder
- Save message attachments and open them with the system default application
- Local Folders mailbox stored on disk, with default and custom folders
- Copy or move messages from a remote account into Local Folders
- Local-only mode, available from the welcome screen without a mail account
- Fast-start and offline cache for remote accounts
- Create folders on the mail server or in Local Folders
- Default storage setting: remote inbox, or copy new Inbox mail into Local Folders
- Storage status from the mail server quota and local mailbox size
- Password storage in the system keyring through `secret-tool` (libsecret)
- Built-in demo mailbox, usable without an account

Everything runs locally. Letters has no telemetry and no third-party service in the middle.

## Scope

Letters is only an email client. It does not embed a web browser or a document viewer. Attachments are saved to `~/Downloads/Letters` and can be opened with the system default application; document previews and embedded browsing stay out of scope. HTML messages are converted to readable text.

## Installation

Requirements:

- Rust 1.89 or newer
- A C compiler and linker for native dependencies
- `secret-tool` (provided by libsecret) for password storage
- `notify-send` (provided by libnotify) for desktop notifications
- On GNOME, the AppIndicator/KStatusNotifierItem extension for the tray icon

On Fedora Workstation:

```bash
sudo dnf install gcc gcc-c++ libsecret
cargo run --release
```

On Fedora Silverblue or Kinoite, build inside a toolbox:

```bash
toolbox create --image fedora-toolbox:44
toolbox run -c fedora-toolbox-44 sudo dnf install -y gcc gcc-c++
toolbox run -c fedora-toolbox-44 bash -lc 'cd "$PWD" && cargo run --release'
```

On Debian or Ubuntu:

```bash
sudo apt install build-essential libsecret-tools
cargo run --release
```

## First run

You can choose **Try the demo** to explore the interface, or **Open local folders** to start with a local mailbox that does not require a mail account.

For a real account:

1. Pick a provider preset, or open **Server settings** for a custom IMAP/SMTP server.
2. Use an app password rather than your main account password. Gmail, Fastmail, Yahoo and several other providers require one.
3. Set the IMAP and SMTP security mode to `TLS` for port 993/465, or `STARTTLS` for port 143/587.
4. Connect. The inbox is synced immediately and letters can be composed from the list header or with `Ctrl+N`.

The account configuration is stored in `~/.config/letters/config.json` with owner-only permissions. Passwords are not written to that file; they are stored through the system keyring. If no keyring is available, the session password is kept only in memory.

Recent messages are cached in `~/.local/share/letters/cache.json` with owner-only permissions so the inbox can open quickly and remain readable when the server is unreachable. The cache is plaintext, like a local mail store; remove the file if you do not want messages stored on disk.

Local Folders are stored separately under `~/.local/share/letters/local/`. Each copied or moved message keeps its raw RFC 822 source (including attachments) as a `.eml` file, together with a local index. `Open local folders` on the welcome screen opens this mailbox without connecting an account. From a remote message, use the download icon in the reader to copy or move it into a local folder.

The **Default storage** setting can be set to **Remote inbox** (server-side, the factory default) or **Local folders**. With Local folders selected, new remote Inbox messages are also copied into the local Inbox. The storage status uses the mail server's IMAP quota when the server reports it; Local Folders show their actual size because they have no server quota.

Some providers already save a copy of messages sent over SMTP. Turn off **Save sent copy** in Settings if you see duplicates in Sent.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl+N` | New message |
| `Ctrl+R` | Refresh the current folder |
| `Ctrl+Enter` | Send the message being composed |
| `Ctrl+Q` | Quit Letters |
| `/` | Focus search |
| `Esc` | Close a dialog |

## Notifications, tray and UnifiedPush

On Linux, UnifiedPush is implemented over D-Bus by distributors such as KUnifiedPush. It also requires an application server that sends the push message. An IMAP mailbox does not provide such a server by itself, so Letters currently uses the reliable desktop fallback:

- IMAP sync runs in a background thread, independently of whether the window is visible.
- New unread mail produces a desktop notification when the app is not focused.
- Closing the window keeps Letters in the system tray and background sync continues. Use the tray menu to quit, or turn off **Close to tray** in Settings if the window close button should quit instead.
- On Wayland, winit cannot hide a native window. When **Close to tray** is enabled, Letters starts through XWayland so the close button can hide the window reliably; disable Close to tray to use the native Wayland backend.
- On desktops without a tray implementation, notifications still work while the app is running, but closing the window exits the app.

A KUnifiedPush connector can be added on top of this behavior later without changing the account model. It would need a companion app server or bridge that watches the mailbox and publishes a push event.

## Performance

The release build is a single stripped binary of roughly 11–12 MB. On Fedora 44 GNOME/Wayland with the Mesa driver, the demo mailbox used around 80 MB resident memory (RSS), of which roughly 28 MB was private dirty memory; most of the rest was file-backed graphics driver memory shared with other processes. New mail arrives through a background thread, so the UI thread never performs network I/O.

## Privacy and security

- Only encrypted IMAP and SMTP connections are supported.
- The app connects directly to the configured mail servers.
- Account passwords are delegated to the system keyring.
- Configuration, cache and local index files are written with `0600` permissions.
- Cached and local mail is not encrypted at rest.
- Logging never includes passwords.

## Current limitations

This is an early MVP, not a Thunderbird replacement yet:

- One account per profile
- UnifiedPush wiring is not included yet; the tray/background-sync fallback is used
- HTML mail is reduced to readable text with quote and signature structure; full HTML styling is not reproduced
- Remote images and remote signature images are never fetched for privacy
- Only the latest 50 messages of a folder are fetched
- Attachments can be saved and opened externally, but composing attachments is not supported yet
- Inline images used in signatures are shown inline and are not offered as downloadable attachments
- Plain-text fallback is used when a message has no real HTML body; Outlook-style quote containers may still be rendered as ordinary text
- Local Folders can copy or move messages and attachments, but cannot receive mail directly from a mail server
- Drafts are not synchronized to the server
- No OAuth2; use app passwords
- Inbox refresh is polling-based; IMAP IDLE is planned
- Local Folders and the remote cache are plaintext on disk
- The remote cache is limited to the most recent 50 messages per synced folder
- Local-only mode cannot send mail and does not create server-side drafts
- Moving a message to Local Folders puts the remote copy in the remote Trash rather than deleting it permanently
- Folder creation is supported, but renaming and deleting folders are not
- No message threading, rules or calendar integration
- Server-specific folder names need to be selectable from the folder list; standard Sent/Drafts/Archive/Trash folders are detected automatically

## Development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The UI source is split into `src/app.rs` (state and event flow) and `src/app/views*.rs` (rendering). Mail networking lives in `src/mail.rs`, configuration and keyring access in `src/config.rs`, and the demo mailbox in `src/demo.rs`. UI strings and code documentation are written in English so that additional locales can be added later.

## Roadmap

- UnifiedPush/KUnifiedPush connector and an optional bridge for mail notifications
- IMAP IDLE
- Multiple accounts
- Compose attachments
- Local drafts and local-only composing
- Server-side drafts
- OAuth2
- Message threading and richer search
- Translations

## License

MIT. The interface uses the Montserrat typeface, licensed under the SIL Open Font License; see `assets/fonts/OFL.txt`.
