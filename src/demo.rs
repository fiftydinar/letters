use std::collections::HashMap;

use crate::model::{Attachment, DEMO_EMAIL, Folder, FolderKind, Message};

pub struct DemoData {
    pub folders: Vec<Folder>,
    pub cache: HashMap<String, Vec<Message>>,
}

pub fn demo_data() -> DemoData {
    let now = crate::model::now_timestamp();
    let mut cache: HashMap<String, Vec<Message>> = HashMap::new();

    let inbox = vec![
        message(
            "INBOX",
            108,
            "Welcome to Letters",
            "The Letters Team",
            "hello@letters.app",
            now - 9 * 60,
            "A small client with a lot of care for detail. Here is everything you can try right now.",
            "Hi,\n\nWelcome to Letters — an email client designed to disappear into the background and let you work.\n\nThis is a demo mailbox. Click around, star messages, open conversations, and try composing a reply. When you are ready, connect your own account over IMAP and SMTP.\n\nWarmly,\nThe Letters Team",
            true,
            false,
            vec![],
        ),
        message(
            "INBOX",
            107,
            "New visuals for the autumn campaign",
            "Maja Petrović",
            "maja@studioatlas.rs",
            now - 32 * 60,
            "I sent three directions. The third feels closest to what we talked about.",
            "Hi,\n\nI sent three directions for the autumn campaign. The first is safe, the second is a little more experimental, and the third feels closest to what we talked about — lots of air, a warm accent, and typography carrying the whole composition.\n\nTake a look at the attachment and let me know what you think. If we agree, I can prepare the exports by Thursday.\n\nMaja",
            true,
            true,
            vec![(
                "autumn-visuals.pdf".to_owned(),
                2_480_000,
                "application/pdf".to_owned(),
            )],
        ),
        message(
            "INBOX",
            106,
            "Re: Sync API",
            "Nikola Jovanović",
            "nikola@example.dev",
            now - 2 * 3600,
            "IDLE works, but I have a question about reconnecting when the server closes the connection silently.",
            "Hi,\n\nIDLE works the way we agreed. The only thing I still want to verify is reconnecting when the server closes the connection silently after a longer pause.\n\nI suggest keeping polling as a 60-second fallback and letting IDLE be the primary channel. That gives us fast notifications without losing refreshes if the connection drops.\n\nCould you take a look at the timeout part?\n\nNikola",
            true,
            false,
            vec![],
        ),
        message(
            "INBOX",
            105,
            "Figma: 4 new changes in Letters UI",
            "Figma",
            "notifications@figma.com",
            now - 5 * 3600,
            "Maja Petrović left a comment on the frame Reader / Desktop.",
            "Maja Petrović left a comment on the frame Reader / Desktop:\n\n“I think the title can be a little larger, but the preview should stay calm. The contrast is good now.”\n\nOpen the file in Figma to see all changes.",
            true,
            false,
            vec![],
        ),
        message(
            "INBOX",
            104,
            "Weekend in the mountains?",
            "Ana & Luka",
            "ana.luka@example.com",
            now - 26 * 3600,
            "We found a small cabin with a view. If you are in, we can book it today.",
            "Hey!\n\nWe found a small cabin with a big window and a view of the forest. There is room for all of us and a yard where we can have morning coffee.\n\nIf you are in, we can book it today — there are only two dates left.\n\nAna & Luka",
            false,
            false,
            vec![],
        ),
        message(
            "INBOX",
            103,
            "Weekly digest: Rust & UI",
            "GitHub",
            "noreply@github.com",
            now - 2 * 86400,
            "5 new items in repositories you follow.",
            "This week in repositories you follow:\n\n• egui — new font and accessibility improvements\n• eframe — better Wayland support\n• mail-parser — faster MIME header decoding\n• imap — more stable IDLE reconnect\n\nView all items on GitHub.",
            false,
            false,
            vec![],
        ),
        message(
            "INBOX",
            102,
            "Your week in Linear",
            "Linear",
            "updates@linear.app",
            now - 3 * 86400,
            "12 issues closed, 3 in review, 1 blocked.",
            "Great week!\n\nYou closed 12 issues, 3 are in review, and one is waiting for a reply. Most of your time went into “Reader polish” and “Compose flow”.\n\nOpen Linear for the full report.",
            false,
            false,
            vec![],
        ),
        message(
            "INBOX",
            101,
            "Payment confirmation",
            "Cloud Studio",
            "billing@cloudstudio.example",
            now - 8 * 86400,
            "We received your payment. The invoice is attached.",
            "Hello,\n\nwe received your payment successfully. You can find the invoice and specification in the attachment.\n\nThank you for using Cloud Studio.\n\nThe Cloud Studio Team",
            false,
            false,
            vec![(
                "invoice-2026-09.pdf".to_owned(),
                184_000,
                "application/pdf".to_owned(),
            )],
        ),
    ];

    let sent = vec![
        message(
            "Sent",
            209,
            "Re: New visuals for the autumn campaign",
            "Letters Studio",
            DEMO_EMAIL,
            now - 28 * 60,
            "The third direction is my favorite too. Let's go with it.",
            "Maja,\n\nthe third direction is my favorite too. Let's go with it. I would only soften the shadow under the title a little and leave more space around the logo.\n\nThanks!\n\nLetters Studio",
            false,
            false,
            vec![],
        ),
        message(
            "Sent",
            208,
            "Re: Sync API",
            "Letters Studio",
            DEMO_EMAIL,
            now - 90 * 60,
            "Agreed. Poll stays as fallback, IDLE becomes primary.",
            "Nikola,\n\nagreed. Poll stays as a 60-second fallback and IDLE becomes the primary channel. I will look at the timeout and reconnect this afternoon.\n\nI will let you know when it is ready to test.\n\nLetters Studio",
            false,
            false,
            vec![],
        ),
        message(
            "Sent",
            207,
            "Coffee on Wednesday?",
            "Letters Studio",
            DEMO_EMAIL,
            now - 3 * 86400,
            "If 10 works for you, you know the place.",
            "Hey,\n\nif Wednesday at 10 works for you, you know the place. I will come a little early to grab the table by the window.\n\nSee you!\n\nLetters Studio",
            false,
            false,
            vec![],
        ),
    ];

    let drafts = vec![message(
        "Drafts",
        301,
        "Radio pitch — autumn season",
        "Letters Studio",
        DEMO_EMAIL,
        now - 19 * 3600,
        "A short idea: three episodes about making things that last.",
        "Pitch for the series:\n\n1. Why we build small tools\n2. Speed is not the same as ease\n3. What a good interface looks like when there is nothing to decorate\n\nThis is only a skeleton. I will refine the questions and guests.",
        false,
        false,
        vec![],
    )];

    let archive = vec![
        message(
            "Archive",
            401,
            "Invoice #2026-041",
            "Paper & Pixel",
            "finance@paperpixel.example",
            now - 34 * 86400,
            "Paid. Archived for accounting.",
            "Invoice #2026-041 has been paid. Details are attached.\n\nThanks for working with us.",
            false,
            false,
            vec![],
        ),
        message(
            "Archive",
            402,
            "Notes from the typography workshop",
            "Maja Petrović",
            "maja@studioatlas.rs",
            now - 52 * 86400,
            "Three things we took away: rhythm, contrast, and silence.",
            "Workshop notes:\n\n• Rhythm matters more than the number of fonts.\n• Contrast creates hierarchy, not size alone.\n• Empty space is not empty — it is the silence that holds the whole together.\n\nMaja",
            false,
            true,
            vec![],
        ),
    ];

    let trash = vec![message(
        "Trash",
        501,
        "Newsletter #148",
        "Design Weekly",
        "hello@designweekly.example",
        now - 12 * 86400,
        "This week's selection of links and inspiration.",
        "This week's selection: new prototyping tools, essays on craft, and a few beautiful portfolios.\n\nUnsubscribe if you no longer want this newsletter.",
        false,
        false,
        vec![],
    )];

    cache.insert("INBOX".to_owned(), inbox);
    cache.insert("Sent".to_owned(), sent);
    cache.insert("Drafts".to_owned(), drafts);
    cache.insert("Archive".to_owned(), archive);
    cache.insert("Trash".to_owned(), trash);

    let folders = vec![
        Folder {
            id: "INBOX".to_owned(),
            name: "Inbox".to_owned(),
            remote: "INBOX".to_owned(),
            kind: FolderKind::Inbox,
            unread: 4,
            local: false,
        },
        Folder {
            id: "Sent".to_owned(),
            name: "Sent".to_owned(),
            remote: "Sent".to_owned(),
            kind: FolderKind::Sent,
            unread: 0,
            local: false,
        },
        Folder {
            id: "Drafts".to_owned(),
            name: "Drafts".to_owned(),
            remote: "Drafts".to_owned(),
            kind: FolderKind::Drafts,
            unread: 0,
            local: false,
        },
        Folder {
            id: "Archive".to_owned(),
            name: "Archive".to_owned(),
            remote: "Archive".to_owned(),
            kind: FolderKind::Archive,
            unread: 0,
            local: false,
        },
        Folder {
            id: "Trash".to_owned(),
            name: "Trash".to_owned(),
            remote: "Trash".to_owned(),
            kind: FolderKind::Trash,
            unread: 0,
            local: false,
        },
    ];

    DemoData { folders, cache }
}

#[allow(clippy::too_many_arguments)]
fn message(
    folder: &str,
    uid: u32,
    subject: &str,
    from_name: &str,
    from_email: &str,
    timestamp: i64,
    preview: &str,
    body: &str,
    unread: bool,
    starred: bool,
    attachments: Vec<(String, usize, String)>,
) -> Message {
    Message {
        uid,
        folder_id: folder.to_owned(),
        raw_path: None,
        raw_size: 0,
        subject: subject.to_owned(),
        from_name: from_name.to_owned(),
        from_email: from_email.to_owned(),
        to: DEMO_EMAIL.to_owned(),
        timestamp,
        preview: preview.to_owned(),
        body: body.to_owned(),
        unread,
        starred,
        attachments: attachments
            .into_iter()
            .map(|(name, size, mime)| Attachment {
                name,
                size,
                mime,
                part_index: 0,
            })
            .collect(),
        body_blocks: Vec::new(),
        message_id: Some(format!("<{uid}@demo.letters.app>")),
    }
}
