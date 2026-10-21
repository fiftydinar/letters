use std::collections::HashMap;

use base64::{Engine as _, engine::general_purpose::STANDARD};

use crate::html::{self, Node};
use crate::model::{BodyBlock, InlineImage};

pub fn parse_html_body(
    html_text: &str,
    inline_by_cid: &HashMap<String, InlineImage>,
) -> Vec<BodyBlock> {
    let root = html::parse(html_text);
    let mut blocks = Vec::new();
    let mut text = String::new();
    collect_blocks(&root, &mut blocks, &mut text, inline_by_cid);
    flush_text(&mut blocks, &mut text);
    normalize_blocks(&mut blocks);
    blocks
}

pub fn parse_plain_body(text: &str) -> Vec<BodyBlock> {
    let mut blocks = Vec::new();
    parse_plain_into(text, &mut blocks);
    normalize_blocks(&mut blocks);
    blocks
}

fn normalize_blocks(blocks: &mut Vec<BodyBlock>) {
    split_text_signatures(blocks);
    split_text_at_markers(blocks);
    wrap_embedded_quotes(blocks);
    split_text_at_markers(blocks);
    wrap_embedded_quotes(blocks);
    split_text_signatures(blocks);
    merge_signature_prefixes(blocks);
    merge_signature_fragments(blocks);
    merge_header_prefixes(blocks);
    merge_attribution_prefixes(blocks);
    flatten_forward_quotes(blocks);
}

fn collect_blocks(
    node: &Node,
    blocks: &mut Vec<BodyBlock>,
    text: &mut String,
    inline_by_cid: &HashMap<String, InlineImage>,
) {
    collect_blocks_inner(node, blocks, text, inline_by_cid, true);
}

fn collect_blocks_inner(
    node: &Node,
    blocks: &mut Vec<BodyBlock>,
    text: &mut String,
    inline_by_cid: &HashMap<String, InlineImage>,
    detect_outlook_quote: bool,
) {
    let children = &node.children;
    let mut index = 0;
    while index < children.len() {
        let child = &children[index];
        if let Some(value) = &child.text {
            text.push_str(&html::decode_entities(value));
            index += 1;
            continue;
        }
        if child.tag.is_empty() {
            index += 1;
            continue;
        }
        if is_signature(child) {
            let _ = take_attribution(text);
            flush_text(blocks, text);
            push_signature(child, blocks, inline_by_cid);
            index += 1;
            continue;
        }
        if detect_outlook_quote && is_outlook_quote_start(child) {
            let header = take_attribution(text);
            flush_text(blocks, text);
            let mut wrapper = Node::element("#quote", "");
            wrapper.children.extend_from_slice(&children[index..]);
            let mut quote_blocks = Vec::new();
            let mut quote_text = String::new();
            collect_blocks_inner(
                &wrapper,
                &mut quote_blocks,
                &mut quote_text,
                inline_by_cid,
                false,
            );
            flush_text(&mut quote_blocks, &mut quote_text);
            if !quote_blocks.is_empty() {
                blocks.push(BodyBlock::Quote {
                    header,
                    blocks: quote_blocks,
                });
            }
            break;
        }
        if child.tag_is("blockquote") {
            let header = take_attribution(text);
            flush_text(blocks, text);
            let mut quote_blocks = Vec::new();
            let mut quote_text = String::new();
            collect_blocks_inner(
                child,
                &mut quote_blocks,
                &mut quote_text,
                inline_by_cid,
                true,
            );
            flush_text(&mut quote_blocks, &mut quote_text);
            if !quote_blocks.is_empty() {
                blocks.push(BodyBlock::Quote {
                    header,
                    blocks: quote_blocks,
                });
            }
            index += 1;
            continue;
        }
        if child.tag_is("br") || child.tag_is("hr") {
            text.push('\n');
            index += 1;
            continue;
        }
        if child.tag_is("img") {
            index += 1;
            continue;
        }
        collect_blocks_inner(child, blocks, text, inline_by_cid, detect_outlook_quote);
        if is_block_tag(&child.tag) {
            text.push('\n');
        }
        index += 1;
    }
}

fn push_signature(
    node: &Node,
    blocks: &mut Vec<BodyBlock>,
    inline_by_cid: &HashMap<String, InlineImage>,
) {
    let text = clean_signature_text(&node.text_content());
    let mut images = Vec::new();
    let mut sources = Vec::new();
    collect_img_sources(node, &mut sources);
    let mut seen = std::collections::HashSet::new();
    for source in sources {
        let image = if let Some(content_id) = source.strip_prefix("cid:") {
            inline_by_cid.get(&normalize_cid(content_id)).cloned()
        } else if let Some((mime, data)) = decode_data_url(&source) {
            Some(InlineImage {
                content_id: None,
                mime,
                data,
            })
        } else {
            None
        };
        if let Some(image) = image
            && seen.insert(image.data.clone())
        {
            images.push(image);
        }
    }
    if text.is_none() && images.is_empty() {
        return;
    }
    blocks.push(BodyBlock::Signature { text, images });
}

fn collect_img_sources(node: &Node, sources: &mut Vec<String>) {
    if node.tag_is("img")
        && let Some(src) = attribute_value(&node.attrs, "src")
    {
        sources.push(src);
    }
    for child in &node.children {
        collect_img_sources(child, sources);
    }
}

fn attribute_value(attrs: &str, name: &str) -> Option<String> {
    let lower = attrs.to_ascii_lowercase();
    let index = lower.find(name)?;
    let after = attrs[index + name.len()..].trim_start();
    let after = after.strip_prefix('=')?.trim_start();
    let quote = after.chars().next()?;
    if quote == '"' || quote == '\'' {
        let end = after[1..].find(quote)?;
        Some(after[1..1 + end].to_owned())
    } else {
        after
            .split_whitespace()
            .next()
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    }
}

fn is_signature(node: &Node) -> bool {
    let attrs = node.attrs.to_ascii_lowercase();
    attrs.contains("signature") && !node.tag_is("blockquote")
}

fn is_block_tag(tag: &str) -> bool {
    matches!(
        tag,
        "address"
            | "article"
            | "aside"
            | "div"
            | "footer"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "li"
            | "main"
            | "ol"
            | "p"
            | "section"
            | "table"
            | "tr"
            | "ul"
    )
}

fn flush_text(blocks: &mut Vec<BodyBlock>, text: &mut String) {
    let normalized = html::normalize_text(text);
    if !normalized.is_empty() {
        blocks.push(BodyBlock::Text(normalized));
    }
    text.clear();
}

fn take_attribution(text: &mut String) -> Option<String> {
    let trimmed_end = text.trim_end();
    if trimmed_end.is_empty() {
        return None;
    }
    let line_start = trimmed_end.rfind('\n').map(|index| index + 1).unwrap_or(0);
    let line = trimmed_end[line_start..].trim();
    if !is_attribution(line) {
        return None;
    }
    let paragraph_start = trimmed_end
        .rfind("\n\n")
        .map(|index| index + 2)
        .unwrap_or(0);
    let paragraph = trimmed_end[paragraph_start..].trim();
    let lower = paragraph.to_ascii_lowercase();
    let marker_count = attribution_marker_count(paragraph);
    let header_like = lower.starts_with("on ")
        || lower.starts_with("dana ")
        || lower.starts_with("уто ")
        || lower.starts_with("-----original message-----")
        || lower.starts_with("sent from")
        || (lower.starts_with("from:") && marker_count >= 3);
    if header_like && paragraph.len() <= 500 && paragraph.lines().count() <= 8 {
        let header = paragraph.to_owned();
        text.truncate(paragraph_start);
        return finish_header(header);
    }

    let lower_line = line.to_ascii_lowercase();
    if lower_line == "wrote:"
        || lower_line == "wrote :"
        || lower_line.contains("написа")
        || lower_line.contains("написала")
    {
        let before_line = trimmed_end[..line_start].trim_end();
        let previous_start = before_line.rfind('\n').map(|index| index + 1).unwrap_or(0);
        let previous = before_line[previous_start..].trim();
        let previous_lower = previous.to_lowercase();
        if previous_lower.starts_with("on ")
            || previous_lower.starts_with("уто")
            || previous_lower.starts_with("дана")
            || previous_lower.contains("wrote")
            || previous_lower.contains("написа")
            || previous_lower.contains('@')
            || previous.chars().any(|ch| ch.is_ascii_digit())
        {
            let header = format!("{previous}\n{line}");
            text.truncate(previous_start);
            return finish_header(header);
        }
    }

    let header = line.to_owned();
    text.truncate(line_start);
    finish_header(header)
}

fn finish_header(header: String) -> Option<String> {
    let header = html::normalize_text(&header);
    (!header.is_empty()).then_some(header)
}

fn attribution_marker_count(value: &str) -> usize {
    let lower = value.to_ascii_lowercase();
    ["from:", "sent:", "to:", "cc:", "subject:"]
        .iter()
        .filter(|marker| lower.contains(**marker))
        .count()
}

fn is_outlook_quote_start(node: &Node) -> bool {
    let attrs = node.attrs.to_ascii_lowercase();
    if attrs.contains("appendonsend")
        || attrs.contains("divrplyfwdmsg")
        || attrs.contains("outlookmessageheader")
    {
        return true;
    }
    let text = node.text_content();
    if text.trim().is_empty() {
        return false;
    }
    let lower = text.to_ascii_lowercase();
    let trimmed = lower.trim_start();
    (trimmed.starts_with("-----") || trimmed.starts_with("----------"))
        && (lower.contains("forwarded message") || lower.contains("original message"))
        || trimmed.starts_with("-----original message-----")
        || (trimmed.starts_with("from:")
            && lower.contains("sent:")
            && (lower.contains("subject:") || marker_count_for(&lower) >= 2))
}

fn marker_count_for(lower: &str) -> usize {
    ["from:", "sent:", "to:", "subject:"]
        .iter()
        .filter(|marker| lower.contains(**marker))
        .count()
}

fn is_attribution(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("wrote:")
        || lower.contains("wrote :")
        || lower.contains("schrieb")
        || lower.contains("napisa")
        || lower.contains("написа")
        || lower.contains("original message")
        || lower.contains("from:")
        || lower.contains("sent:")
        || lower.contains("subject:")
}

fn parse_plain_into(text: &str, blocks: &mut Vec<BodyBlock>) {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut normal = String::new();
    let mut quote = String::new();
    let mut quote_header = None;
    let mut signature: Option<String> = None;
    let mut forwarded = false;

    for line in normalized.lines() {
        let trimmed = line.trim_start();
        if forwarded {
            if !quote.is_empty() {
                quote.push('\n');
            }
            quote.push_str(line);
            continue;
        }
        if let Some(quoted) = trimmed.strip_prefix('>') {
            if signature.is_some() {
                push_plain_signature(blocks, &mut signature);
            }
            if quote.is_empty() {
                quote_header = take_attribution(&mut normal);
                flush_text(blocks, &mut normal);
            } else {
                quote.push('\n');
            }
            quote.push_str(quoted.strip_prefix(' ').unwrap_or(quoted));
            continue;
        }

        if !quote.is_empty() {
            push_plain_quote(blocks, &mut quote, quote_header.take());
        }

        if is_forward_marker(line) {
            push_plain_signature(blocks, &mut signature);
            flush_text(blocks, &mut normal);
            quote_header = Some(line.trim().to_owned());
            forwarded = true;
            continue;
        }

        if line.trim() == "--" {
            flush_text(blocks, &mut normal);
            if signature.is_some() {
                push_plain_signature(blocks, &mut signature);
            }
            signature = Some(String::new());
            continue;
        }

        if let Some(current) = &mut signature {
            if is_attribution(line) {
                push_plain_signature(blocks, &mut signature);
                normal.push_str(line);
                continue;
            }
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
            continue;
        }

        if line.trim().is_empty() {
            flush_text(blocks, &mut normal);
        } else {
            if !normal.is_empty() {
                normal.push('\n');
            }
            normal.push_str(line);
        }
    }

    if !quote.is_empty() {
        push_plain_quote(blocks, &mut quote, quote_header.take());
    }
    flush_text(blocks, &mut normal);
    push_plain_signature(blocks, &mut signature);
}

fn is_forward_marker(line: &str) -> bool {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    (trimmed.starts_with("-----") || trimmed.starts_with("----------"))
        && (lower.contains("forwarded message") || lower.contains("original message"))
}

fn push_plain_quote(blocks: &mut Vec<BodyBlock>, quote: &mut String, header: Option<String>) {
    let mut quote_blocks = Vec::new();
    parse_plain_into(quote, &mut quote_blocks);
    if !quote_blocks.is_empty() {
        blocks.push(BodyBlock::Quote {
            header,
            blocks: quote_blocks,
        });
    }
    quote.clear();
}

fn push_plain_signature(blocks: &mut Vec<BodyBlock>, signature: &mut Option<String>) {
    let Some(text) = signature.take() else {
        return;
    };
    let text = text.trim();
    if !text.is_empty() {
        blocks.push(BodyBlock::Signature {
            text: Some(text.to_owned()),
            images: Vec::new(),
        });
    }
}

fn clean_signature_text(value: &str) -> Option<String> {
    let normalized = html::normalize_text(value);
    if normalized.is_empty() {
        return None;
    }
    let cleaned = normalized
        .strip_prefix("--")
        .map(str::trim_start)
        .unwrap_or(&normalized);
    let cleaned = cleaned.trim();
    (!cleaned.is_empty()).then(|| cleaned.to_owned())
}

fn split_text_signatures(blocks: &mut Vec<BodyBlock>) {
    let mut output = Vec::with_capacity(blocks.len());
    for block in blocks.drain(..) {
        match block {
            BodyBlock::Quote { header, blocks } => {
                let mut blocks = blocks;
                split_text_signatures(&mut blocks);
                let mut header = header;
                if let Some(existing) = header.as_mut() {
                    merge_forward_headers(existing, &mut blocks);
                }
                let header = header.or_else(|| take_quote_header(&mut blocks));
                output.push(BodyBlock::Quote { header, blocks });
            }
            BodyBlock::Text(text) => {
                if let Some((body, signature)) = split_signature_text(&text) {
                    if !body.is_empty() {
                        output.push(BodyBlock::Text(body));
                    }
                    output.push(BodyBlock::Signature {
                        text: Some(signature),
                        images: Vec::new(),
                    });
                } else {
                    output.push(BodyBlock::Text(text));
                }
            }
            other => output.push(other),
        }
    }
    *blocks = output;
}

fn merge_forward_headers(header: &mut String, blocks: &mut Vec<BodyBlock>) {
    if !is_forward_marker(header.lines().next().unwrap_or_default()) {
        return;
    }
    let BodyBlock::Text(text) = &blocks[0] else {
        return;
    };
    let lines: Vec<&str> = text.lines().collect();
    let Some((headers, body)) = extract_header_block(&lines) else {
        return;
    };
    if headers.is_empty() {
        return;
    }
    *header = format!("{}\n{headers}", header.trim_end());
    replace_quote_body(blocks, body);
}

fn take_quote_header(blocks: &mut Vec<BodyBlock>) -> Option<String> {
    let BodyBlock::Text(text) = blocks.first()? else {
        return None;
    };
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return None;
    }
    if let Some((marker, used)) = forward_marker(&lines) {
        if let Some((headers, body)) = extract_header_block(&lines[used..]) {
            replace_quote_body(blocks, body);
            return Some(format!("{marker}\n{headers}"));
        }
        let rest = lines[used..].join("\n").trim().to_owned();
        if rest.is_empty() {
            blocks.remove(0);
        } else if let Some(BodyBlock::Text(text)) = blocks.first_mut() {
            *text = rest;
        }
        return Some(marker);
    }
    let (headers, body) = extract_header_block(&lines)?;
    replace_quote_body(blocks, body);
    Some(headers)
}

fn replace_quote_body(blocks: &mut Vec<BodyBlock>, body: String) {
    if body.is_empty() {
        blocks.remove(0);
    } else if let Some(BodyBlock::Text(text)) = blocks.first_mut() {
        *text = body;
    }
}

fn extract_header_block(lines: &[&str]) -> Option<(String, String)> {
    const LABELS: [&str; 8] = [
        "from:",
        "sent:",
        "date:",
        "to:",
        "cc:",
        "reply-to:",
        "subject:",
        "šalje:",
    ];
    fn is_label(line: &str) -> bool {
        let lower = line.trim_start().to_lowercase();
        LABELS.iter().any(|label| lower.starts_with(label))
    }
    if lines.is_empty() || !is_label(lines[0]) {
        return None;
    }
    let mut labels = lines
        .iter()
        .take(12)
        .enumerate()
        .filter(|(_, line)| is_label(line))
        .map(|(index, _)| index);
    let mut count = 0;
    let mut last = None;
    for index in labels.by_ref() {
        count += 1;
        last = Some(index);
    }
    if count < 2 {
        return None;
    }
    let last = last?;
    let ends_with_value = lines[last]
        .split_once(':')
        .is_some_and(|(_, value)| !value.trim().is_empty());
    let mut end = if !ends_with_value && last + 1 < lines.len() && !is_label(lines[last + 1]) {
        last + 1
    } else {
        last
    };
    let label = lines[last]
        .split_once(':')
        .map(|(label, _)| label.trim().to_lowercase())
        .unwrap_or_default();
    let address_label = matches!(label.as_str(), "from" | "to" | "cc" | "reply-to");
    if address_label {
        while end + 1 < lines.len() {
            let next = lines[end + 1].trim();
            let continuation = next.starts_with('<')
                || next.starts_with('\'')
                || next.starts_with('"')
                || next.contains('@');
            if continuation {
                end += 1;
            } else {
                break;
            }
        }
    }
    while end + 1 < lines.len() {
        let next = lines[end + 1].trim();
        let last_line = lines[end].trim_end();
        let open_end = last_line.ends_with('-')
            || last_line.chars().last().is_some_and(|ch| ch.is_lowercase());
        let lower_start = next.chars().next().is_some_and(|ch| ch.is_lowercase());
        if open_end && lower_start && !next.is_empty() && next.len() <= 100 {
            end += 1;
        } else {
            break;
        }
    }
    if label == "subject" {
        let value = lines[last]
            .split_once(':')
            .map(|(_, value)| value.trim())
            .unwrap_or_default();
        if value.len() <= 3 && value.ends_with(':') {
            while end + 1 < lines.len() && end < last + 2 {
                let next = lines[end + 1].trim();
                if next.is_empty()
                    || is_label(next)
                    || looks_like_body_start(next)
                    || next.len() > 120
                {
                    break;
                }
                end += 1;
            }
        }
    }
    let header = lines[..=end].join("\n");
    if header.len() > 800 || end > 12 {
        return None;
    }
    let body = lines[end + 1..].join("\n").trim().to_owned();
    Some((header, body))
}

fn forward_marker(lines: &[&str]) -> Option<(String, usize)> {
    let first = lines.first()?.trim();
    let lower = first.to_lowercase();
    if !first.starts_with("-----")
        || (!lower.contains("forwarded message") && !lower.contains("original message"))
    {
        return None;
    }
    let mut used = 1;
    while used < lines.len() {
        let line = lines[used].trim();
        if line.len() >= 3 && line.chars().all(|ch| ch == '-') {
            used += 1;
        } else {
            break;
        }
    }
    let marker = lines[..used]
        .iter()
        .map(|line| line.trim())
        .collect::<Vec<_>>()
        .join(" ");
    Some((marker, used))
}

fn split_embedded_quote(text: &str) -> Option<(String, String)> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return None;
    }
    if let Some((marker, used)) = forward_marker(&lines) {
        let (headers, body) = extract_header_block(&lines[used..])
            .unwrap_or_else(|| (String::new(), lines[used..].join("\n").trim().to_owned()));
        let header = if headers.is_empty() {
            marker
        } else {
            format!("{marker}\n{headers}")
        };
        return Some((header, body));
    }
    extract_header_block(&lines)
}

fn wrap_embedded_quotes(blocks: &mut Vec<BodyBlock>) {
    let mut index = 0;
    while index < blocks.len() {
        if let BodyBlock::Quote { blocks, .. } = &mut blocks[index] {
            wrap_embedded_quotes(blocks);
            index += 1;
            continue;
        }
        let split = match &blocks[index] {
            BodyBlock::Text(text) => split_embedded_quote(text),
            _ => None,
        };
        let Some((header, body)) = split else {
            index += 1;
            continue;
        };
        let tail = blocks.split_off(index + 1);
        blocks.pop();
        let mut quote_blocks = Vec::new();
        if !body.is_empty() {
            quote_blocks.push(BodyBlock::Text(body));
        }
        quote_blocks.extend(tail);
        blocks.push(BodyBlock::Quote {
            header: Some(header),
            blocks: quote_blocks,
        });
        if let Some(BodyBlock::Quote { blocks, .. }) = blocks.last_mut() {
            wrap_embedded_quotes(blocks);
        }
        break;
    }
}

fn merge_signature_prefixes(blocks: &mut Vec<BodyBlock>) {
    let mut index = 0;
    while index < blocks.len() {
        if let BodyBlock::Quote { blocks, .. } = &mut blocks[index] {
            merge_signature_prefixes(blocks);
            index += 1;
            continue;
        }
        let rest = match &blocks[index] {
            BodyBlock::Text(text) => signature_delimiter_rest(text),
            _ => None,
        };
        let Some(rest) = rest else {
            index += 1;
            continue;
        };
        if let Some(BodyBlock::Signature { text, .. }) = blocks.get_mut(index + 1) {
            if !rest.is_empty() {
                let current = text.take().unwrap_or_default();
                *text = Some(merge_signature_text(&rest, &current));
            }
            blocks.remove(index);
            continue;
        }
        if rest.is_empty() {
            blocks.remove(index);
            continue;
        }
        if let Some(BodyBlock::Text(text)) = blocks.get_mut(index) {
            *text = rest;
        }
        index += 1;
    }
}

fn signature_delimiter_rest(text: &str) -> Option<String> {
    let mut lines = text.lines();
    let first = lines.next()?.trim();
    let rest_first = if first == "--" {
        ""
    } else {
        first.strip_prefix("-- ")?.trim()
    };
    let mut rest = Vec::new();
    if !rest_first.is_empty() {
        rest.push(rest_first);
    }
    rest.extend(lines);
    Some(rest.join("\n").trim().to_owned())
}

fn merge_signature_text(prefix: &str, signature: &str) -> String {
    let prefix = prefix.trim();
    let signature = signature.trim();
    if prefix.is_empty() {
        return signature.to_owned();
    }
    if signature.is_empty() {
        return prefix.to_owned();
    }
    if signature.to_lowercase().starts_with(&prefix.to_lowercase()) {
        return signature.to_owned();
    }
    let mut merged = prefix.to_owned();
    if signature.chars().next().is_some_and(|ch| ch.is_lowercase())
        && prefix.chars().last().is_some_and(|ch| ch.is_alphanumeric())
    {
        merged.push(' ');
    } else {
        merged.push('\n');
    }
    merged.push_str(signature);
    merged
}

fn split_text_at_markers(blocks: &mut Vec<BodyBlock>) {
    let mut index = 0;
    while index < blocks.len() {
        if let BodyBlock::Quote { blocks, .. } = &mut blocks[index] {
            split_text_at_markers(blocks);
            index += 1;
            continue;
        }
        let marker = match &blocks[index] {
            BodyBlock::Text(text) => find_quote_marker_line(text),
            _ => None,
        };
        let Some(marker) = marker else {
            index += 1;
            continue;
        };
        let BodyBlock::Text(text) = &blocks[index] else {
            index += 1;
            continue;
        };
        let lines: Vec<&str> = text.lines().collect();
        if marker == 0 || marker >= lines.len() {
            index += 1;
            continue;
        }
        let prefix = lines[..marker].join("\n").trim().to_owned();
        let suffix = lines[marker..].join("\n").trim_start().to_owned();
        if prefix.is_empty() {
            if let BodyBlock::Text(text) = &mut blocks[index] {
                *text = suffix;
            }
        } else {
            if let BodyBlock::Text(text) = &mut blocks[index] {
                *text = prefix;
            }
            blocks.insert(index + 1, BodyBlock::Text(suffix));
            index += 1;
        }
        index += 1;
    }
}

fn is_primary_header(line: &str) -> bool {
    let lower = line.trim_start().to_lowercase();
    lower.starts_with("from:")
        || lower.starts_with("subject:")
        || lower.starts_with("šalje:")
        || lower.starts_with("-----original message-----")
}

fn find_quote_marker_line(text: &str) -> Option<usize> {
    let lines: Vec<&str> = text.lines().collect();
    for index in 1..lines.len() {
        if forward_marker(&lines[index..]).is_some()
            || (is_primary_header(lines[index]) && extract_header_block(&lines[index..]).is_some())
        {
            return Some(index);
        }
    }
    None
}

fn merge_attribution_prefixes(blocks: &mut Vec<BodyBlock>) {
    let mut index = 0;
    while index < blocks.len() {
        if index > 0 {
            let header = match &blocks[index] {
                BodyBlock::Quote {
                    header: Some(header),
                    ..
                } => Some(header.clone()),
                _ => None,
            };
            if let Some(header) = header {
                let lower = header.to_lowercase();
                if !lower.starts_with("on ")
                    && !lower.starts_with("уто")
                    && !lower.starts_with("дана")
                    && (lower.ends_with("wrote:") || lower.ends_with("написао/ла:"))
                {
                    if let BodyBlock::Text(text) = &blocks[index - 1]
                        && let Some((body, line)) = attribution_tail(text)
                    {
                        let separator = if header
                            .trim_start()
                            .chars()
                            .next()
                            .is_some_and(|ch| ch.is_ascii_digit())
                        {
                            " "
                        } else {
                            "\n"
                        };
                        if let Some(BodyBlock::Quote {
                            header: Some(header),
                            ..
                        }) = blocks.get_mut(index)
                        {
                            *header = format!("{line}{separator}{header}");
                        }
                        let remove = body.is_empty();
                        if remove {
                            blocks.remove(index - 1);
                            index -= 1;
                        } else if let Some(BodyBlock::Text(text)) = blocks.get_mut(index - 1) {
                            *text = body;
                        }
                        if let Some(BodyBlock::Quote { blocks, .. }) = blocks.get_mut(index) {
                            merge_attribution_prefixes(blocks);
                        }
                        if !remove {
                            index += 1;
                        }
                        continue;
                    }
                    if let BodyBlock::Signature {
                        text: Some(signature),
                        ..
                    } = &blocks[index - 1]
                        && let Some((body, line)) = attribution_tail(signature)
                    {
                        let separator = if header
                            .trim_start()
                            .chars()
                            .next()
                            .is_some_and(|ch| ch.is_ascii_digit())
                        {
                            " "
                        } else {
                            "\n"
                        };
                        if let Some(BodyBlock::Quote {
                            header: Some(header),
                            ..
                        }) = blocks.get_mut(index)
                        {
                            *header = format!("{line}{separator}{header}");
                        }
                        if let Some(BodyBlock::Signature { text, .. }) = blocks.get_mut(index - 1) {
                            *text = (!body.is_empty()).then_some(body);
                        }
                        if let Some(BodyBlock::Quote { blocks, .. }) = blocks.get_mut(index) {
                            merge_attribution_prefixes(blocks);
                        }
                        index += 1;
                        continue;
                    }
                }
            }
        }
        if let BodyBlock::Quote { blocks, .. } = &mut blocks[index] {
            merge_attribution_prefixes(blocks);
        }
        index += 1;
    }
}

fn attribution_tail(text: &str) -> Option<(String, String)> {
    let trimmed = text.trim_end();
    let start = trimmed.rfind('\n').map(|pos| pos + 1).unwrap_or(0);
    let line = trimmed[start..].trim();
    if !looks_like_attribution_prefix(line) {
        return None;
    }
    Some((trimmed[..start].trim_end().to_owned(), line.to_owned()))
}

fn merge_header_prefixes(blocks: &mut Vec<BodyBlock>) {
    let mut index = 1;
    while index < blocks.len() {
        let merged = match (&blocks[index - 1], &blocks[index]) {
            (
                BodyBlock::Text(text),
                BodyBlock::Quote {
                    header: Some(header),
                    ..
                },
            ) => {
                let lines: Vec<&str> = text.lines().collect();
                if lines.len() != 1 {
                    None
                } else {
                    let line = lines[0].trim();
                    let lower = line.to_lowercase();
                    let header_lower = header.trim_start().to_lowercase();
                    if lower.starts_with("from:")
                        && (header_lower.starts_with("sent:")
                            || header_lower.starts_with("date:")
                            || header_lower.starts_with("to:")
                            || header_lower.starts_with("subject:"))
                    {
                        Some((format!("{line}\n{header}"), line.to_owned()))
                    } else {
                        None
                    }
                }
            }
            _ => None,
        };
        let Some((new_header, _line)) = merged else {
            if let BodyBlock::Quote { blocks, .. } = &mut blocks[index] {
                merge_header_prefixes(blocks);
            }
            index += 1;
            continue;
        };
        if let Some(BodyBlock::Quote { header, .. }) = blocks.get_mut(index) {
            *header = Some(new_header);
        }
        blocks.remove(index - 1);
        index -= 1;
        if let Some(BodyBlock::Quote { blocks, .. }) = blocks.get_mut(index) {
            merge_header_prefixes(blocks);
        }
        index += 1;
    }
}

fn looks_like_attribution_prefix(line: &str) -> bool {
    let lower = line.trim_end_matches([',', ' ', ':']).to_lowercase();
    if matches!(lower.as_str(), "on" | "уто" | "дана" | "dana") {
        return true;
    }
    (lower.starts_with("on ")
        || lower.starts_with("уто")
        || lower.starts_with("дана")
        || lower.starts_with("dana"))
        && line.chars().any(|ch| ch.is_ascii_digit())
}

fn flatten_forward_quotes(blocks: &mut [BodyBlock]) {
    let mut index = 0;
    while index < blocks.len() {
        let merge = match &blocks[index] {
            BodyBlock::Quote {
                header: Some(header),
                blocks: inner,
            } if is_forward_marker(header.lines().next().unwrap_or_default())
                && inner.len() == 1 =>
            {
                if let BodyBlock::Quote {
                    header: Some(inner_header),
                    blocks: inner,
                } = &inner[0]
                {
                    Some((
                        format!("{}\n{inner_header}", header.trim_end()),
                        inner.clone(),
                    ))
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some((header, inner)) = merge {
            blocks[index] = BodyBlock::Quote {
                header: Some(header),
                blocks: inner,
            };
            continue;
        }
        if let BodyBlock::Quote { blocks: inner, .. } = &mut blocks[index] {
            flatten_forward_quotes(inner);
        }
        index += 1;
    }
}

fn merge_signature_fragments(blocks: &mut Vec<BodyBlock>) {
    let mut index = 0;
    while index < blocks.len() {
        if let BodyBlock::Quote { blocks, .. } = &mut blocks[index] {
            merge_signature_fragments(blocks);
            index += 1;
            continue;
        }
        if matches!(blocks[index], BodyBlock::Signature { .. })
            && matches!(blocks.get(index + 1), Some(BodyBlock::Signature { .. }))
        {
            let next = blocks.remove(index + 1);
            if let (
                BodyBlock::Signature { text, images },
                BodyBlock::Signature {
                    text: next_text,
                    images: next_images,
                },
            ) = (&mut blocks[index], next)
            {
                let merged = merge_signature_text(
                    text.as_deref().unwrap_or_default(),
                    next_text.as_deref().unwrap_or_default(),
                );
                *text = (!merged.is_empty()).then_some(merged);
                for image in next_images {
                    if !images.iter().any(|existing| existing.data == image.data) {
                        images.push(image);
                    }
                }
            }
            continue;
        }
        if let BodyBlock::Text(text) = &blocks[index]
            && matches!(blocks.get(index + 1), Some(BodyBlock::Signature { .. }))
            && let Some((body, closing)) = split_trailing_closing(text)
        {
            if let Some(BodyBlock::Signature {
                text: signature, ..
            }) = blocks.get_mut(index + 1)
            {
                let merged =
                    merge_signature_text(&closing, signature.as_deref().unwrap_or_default());
                *signature = Some(merged);
            }
            if body.is_empty() {
                blocks.remove(index);
                continue;
            }
            if let BodyBlock::Text(text) = &mut blocks[index] {
                *text = body;
            }
        }
        index += 1;
    }
}

fn split_trailing_closing(text: &str) -> Option<(String, String)> {
    let trimmed = text.trim_end();
    let lines: Vec<&str> = trimmed.lines().collect();
    let start = lines
        .iter()
        .rposition(|line| is_closing_line(line))
        .or_else(|| {
            lines
                .last()
                .filter(|line| is_closing_fragment(line))
                .map(|_| lines.len() - 1)
        })?;
    if start == 0 || lines.len() - start > 4 {
        return None;
    }
    if !signature_tail_plausible(&lines[start + 1..]) {
        return None;
    }
    let closing = lines[start..].join("\n").trim().to_owned();
    if closing.len() > 200 {
        return None;
    }
    let body = lines[..start].join("\n").trim_end().to_owned();
    if body.is_empty() {
        return None;
    }
    Some((body, closing))
}

fn split_signature_text(text: &str) -> Option<(String, String)> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() < 3 {
        return None;
    }
    let marker = lines
        .iter()
        .enumerate()
        .rev()
        .find(|(_, line)| is_closing_line(line))
        .map(|(index, _)| index)?;
    if marker == 0 || marker + 1 >= lines.len() {
        return None;
    }
    if !signature_tail_plausible(&lines[marker + 1..]) {
        return None;
    }
    let signature = lines[marker..].join("\n");
    if signature.len() > 1500 || lines.len() - marker > 18 {
        return None;
    }
    let body = lines[..marker].join("\n").trim().to_owned();
    if body.is_empty() {
        return None;
    }
    Some((body, signature.trim().to_owned()))
}

fn looks_like_body_start(line: &str) -> bool {
    let lower = line.trim().to_lowercase();
    [
        "poštovani",
        "postovani",
        "poštovana",
        "postovana",
        "dear ",
        "hello",
        "hi ",
        "dobar dan",
        "dobro jutro",
        "u redu",
        "hvala",
        "molim",
        "cajo",
        "marko,",
        "rada,",
    ]
    .iter()
    .any(|greeting| lower.starts_with(greeting))
}

fn signature_tail_plausible(lines: &[&str]) -> bool {
    lines.iter().all(|line| {
        let line = line.trim();
        if line.is_empty() || line.len() > 100 {
            return false;
        }
        let words = line.split_whitespace().count();
        words <= 8 && !(line.ends_with('.') && words > 2)
    })
}

fn is_closing_fragment(line: &str) -> bool {
    let normalized = line
        .trim()
        .trim_end_matches([',', '.', '!', ':'])
        .to_lowercase();
    matches!(
        normalized.as_str(),
        "srdačan" | "best" | "kind" | "warm" | "many" | "thank" | "thanks"
    )
}

fn is_closing_line(line: &str) -> bool {
    let normalized = line
        .trim()
        .trim_end_matches([',', '.', '!', ':'])
        .to_lowercase();
    matches!(
        normalized.as_str(),
        "best regards"
            | "kind regards"
            | "warm regards"
            | "regards"
            | "sincerely"
            | "many thanks"
            | "thanks"
            | "thank you"
            | "srdačan pozdrav"
            | "s poštovanjem"
            | "pozdrav"
            | "lp"
            | "hvala"
    )
}

fn normalize_cid(value: &str) -> String {
    value
        .trim()
        .trim_matches(['<', '>', '"', '\''])
        .trim()
        .to_owned()
}

fn decode_data_url(src: &str) -> Option<(String, String)> {
    let value = src.strip_prefix("data:")?;
    let (metadata, data) = value.split_once(',')?;
    if !metadata.contains(";base64") {
        return None;
    }
    let mime = metadata.split(';').next().unwrap_or("image/png");
    let cleaned = data.split_whitespace().collect::<String>();
    let bytes = STANDARD.decode(cleaned.as_bytes()).ok()?;
    if bytes.is_empty() || bytes.len() > 2_000_000 {
        return None;
    }
    Some((mime.to_owned(), STANDARD.encode(bytes)))
}
