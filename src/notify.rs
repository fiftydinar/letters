use notify_rust::{Hint, Notification, Timeout};

use crate::model::Message;

fn send_notification(summary: &str, body: &str) {
    #[cfg(target_os = "linux")]
    {
        let status = std::process::Command::new("notify-send")
            .args([
                "--app-name=Letters",
                "--app-icon=letters",
                "--icon=mail-unread",
                "--category=email",
                "--expire-time=7000",
                "--hint=string:desktop-entry:letters",
                summary,
                body,
            ])
            .status();
        if status.is_ok_and(|status| status.success()) {
            return;
        }
    }

    let result = Notification::new()
        .appname("Letters")
        .summary(summary)
        .body(body)
        .icon("mail-unread")
        .hint(Hint::Category("email".to_owned()))
        .hint(Hint::DesktopEntry("letters".to_owned()))
        .timeout(Timeout::Milliseconds(7000))
        .show();
    if let Err(err) = result {
        log::warn!("Could not show notification: {err}");
    }
}

pub fn show_new_mail(messages: &[Message]) {
    if messages.is_empty() {
        return;
    }
    let count = messages.len();
    let first = &messages[0];
    let summary = if count == 1 {
        format!("{} · {}", first.sender_label(), first.subject)
    } else {
        format!("{count} new messages · {}", first.subject)
    };
    let body = if count == 1 {
        if first.preview.is_empty() {
            first.body.lines().take(3).collect::<Vec<_>>().join("\n")
        } else {
            first.preview.clone()
        }
    } else {
        messages
            .iter()
            .take(4)
            .map(|message| format!("{}: {}", message.sender_label(), message.subject))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let result = Notification::new()
        .appname("Letters")
        .summary(&summary)
        .body(&body)
        .icon("mail-unread")
        .hint(Hint::Category("email".to_owned()))
        .timeout(Timeout::Milliseconds(7000))
        .show();
    if let Err(err) = result {
        log::warn!("Could not show notification: {err}");
    }
}

pub fn show_test() {
    send_notification(
        "Notifications are working",
        "This is how new mail will appear on your desktop.",
    );
}

pub fn show_tray_notice() {
    send_notification(
        "Letters is still running",
        "Close to tray is enabled. Reopen or quit from the tray icon.",
    );
}
