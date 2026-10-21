#[derive(Clone, Debug)]
pub struct Node {
    pub tag: String,
    pub attrs: String,
    pub children: Vec<Node>,
    pub text: Option<String>,
}

impl Node {
    pub fn element(tag: impl Into<String>, attrs: impl Into<String>) -> Self {
        Self {
            tag: tag.into().to_ascii_lowercase(),
            attrs: attrs.into(),
            children: Vec::new(),
            text: None,
        }
    }

    pub fn text(value: impl Into<String>) -> Self {
        Self {
            tag: String::new(),
            attrs: String::new(),
            children: Vec::new(),
            text: Some(value.into()),
        }
    }

    pub fn tag_is(&self, tag: &str) -> bool {
        self.tag.eq_ignore_ascii_case(tag)
    }

    pub fn text_content(&self) -> String {
        let mut output = String::new();
        self.write_text(&mut output);
        normalize_text(&decode_entities(&output))
    }

    fn write_text(&self, output: &mut String) {
        if let Some(text) = &self.text {
            output.push_str(text);
            return;
        }
        if self.tag_is("br") {
            output.push('\n');
            return;
        }
        if self.tag_is("img") {
            return;
        }
        for child in &self.children {
            child.write_text(output);
        }
        if is_block_tag(&self.tag) {
            output.push('\n');
        }
    }
}

pub fn parse(input: &str) -> Node {
    let root = Node::element("#root", "");
    let mut stack = vec![root];
    let mut position = 0;
    while position < input.len() {
        if input[position..].starts_with("<!--") {
            if let Some(end) = input[position + 4..].find("-->") {
                position += 4 + end + 3;
                continue;
            }
            break;
        }
        if input.as_bytes()[position] != b'<' {
            let next = input[position..]
                .find('<')
                .map(|offset| position + offset)
                .unwrap_or(input.len());
            push_text(&mut stack, &input[position..next]);
            position = next;
            continue;
        }
        let Some(end) = input[position..].find('>') else {
            push_text(&mut stack, &input[position..]);
            break;
        };
        let end = position + end;
        let inner = input[position + 1..end].trim();
        if inner.starts_with('!') || inner.starts_with('?') {
            position = end + 1;
            continue;
        }
        if let Some(closing) = inner.strip_prefix('/') {
            let name = tag_name(closing);
            close_tag(&mut stack, &name);
            position = end + 1;
            continue;
        }
        let name = tag_name(inner);
        let attrs = inner[name.len()..].trim();
        let self_closing = inner.ends_with('/') || is_void_tag(&name);
        let node = Node::element(name.clone(), attrs);
        if name == "script" || name == "style" {
            let closing = format!("</{}", name);
            let lower = input[end + 1..].to_ascii_lowercase();
            if let Some(close) = lower.find(&closing) {
                let close_end = input[end + 1 + close..]
                    .find('>')
                    .map(|offset| end + 1 + close + offset)
                    .unwrap_or(input.len());
                position = close_end + 1;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                }
                continue;
            }
        }
        if self_closing {
            if let Some(parent) = stack.last_mut() {
                parent.children.push(node);
            }
        } else {
            stack.push(node);
        }
        position = end + 1;
    }
    while stack.len() > 1
        && let Some(node) = stack.pop()
        && let Some(parent) = stack.last_mut()
    {
        parent.children.push(node);
    }
    stack.pop().unwrap_or_else(|| Node::element("#root", ""))
}

fn push_text(stack: &mut [Node], value: &str) {
    if value.is_empty() {
        return;
    }
    if let Some(parent) = stack.last_mut() {
        parent.children.push(Node::text(value));
    }
}

fn close_tag(stack: &mut Vec<Node>, name: &str) {
    if stack.len() <= 1 {
        return;
    }
    let Some(index) = stack
        .iter()
        .rposition(|node| node.tag.eq_ignore_ascii_case(name))
    else {
        return;
    };
    while stack.len() > index + 1
        && let Some(node) = stack.pop()
        && let Some(parent) = stack.last_mut()
    {
        parent.children.push(node);
    }
    if stack.len() > 1
        && let Some(node) = stack.pop()
        && let Some(parent) = stack.last_mut()
    {
        parent.children.push(node);
    }
}

fn tag_name(tag: &str) -> String {
    tag.trim_start()
        .split(|ch: char| ch.is_whitespace() || ch == '/' || ch == '>')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn is_void_tag(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn is_block_tag(tag: &str) -> bool {
    matches!(
        tag,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "div"
            | "footer"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hr"
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

pub fn normalize_text(value: &str) -> String {
    let mut output = String::new();
    let mut blank = false;
    for line in value.replace("\r\n", "\n").replace('\r', "\n").lines() {
        let line = line.trim();
        if line.is_empty() {
            if !blank && !output.is_empty() {
                output.push('\n');
                blank = true;
            }
        } else {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            output.push_str(line);
            blank = false;
        }
    }
    output.trim().to_owned()
}

pub fn decode_entities(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        if ch != '&' {
            output.push(ch);
            continue;
        }
        let rest = &value[index..];
        let Some(semicolon) = rest.find(';') else {
            output.push('&');
            continue;
        };
        if semicolon > 10 {
            output.push('&');
            continue;
        }
        let entity = &rest[1..semicolon];
        let replacement = match entity {
            "nbsp" => Some(' '),
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                u32::from_str_radix(&entity[2..], 16)
                    .ok()
                    .and_then(char::from_u32)
            }
            _ if entity.starts_with('#') => {
                entity[1..].parse::<u32>().ok().and_then(char::from_u32)
            }
            _ => None,
        };
        if let Some(replacement) = replacement {
            output.push(replacement);
            for _ in 0..semicolon {
                chars.next();
            }
        } else {
            output.push('&');
        }
    }
    output
}
