pub(super) fn html_block_start_state(line: &str) -> Option<(Vec<String>, bool)> {
    // Best-effort HTML block start (block-level):
    // - up to 3 leading spaces
    // - starts with an HTML tag open/close or comment opener
    let mut s = line;
    let mut spaces = 0usize;
    while spaces < 3 && s.starts_with(' ') {
        s = &s[1..];
        spaces += 1;
    }
    let s = s.trim_end();
    if !s.starts_with('<') || s.len() < 3 {
        return None;
    }
    // Recognize a tag-like start. This rejects autolinks like "<https://...>" (':' after name).
    let _ = parse_tag_at(s, 0)?;
    // We intentionally start with an empty stack; the per-line state update will process tags
    // (including the opening tag on the first line) in a single place.
    Some((Vec::new(), false))
}

#[derive(Debug, Clone)]
enum HtmlTag {
    Opening { name: String, self_closing: bool },
    Closing { name: String },
    CommentOpen,
}

fn is_ascii_tag_name_char(b: u8) -> bool {
    // Streamdown uses `<(\\w+)[\\s>]` for tag names (`\\w` includes `_`).
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_void_html_tag(name: &str) -> bool {
    // Common void elements that never have closing tags.
    // This list is intentionally small and can be expanded post-MVP.
    matches!(
        name,
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

fn parse_tag_at(s: &str, lt_index: usize) -> Option<(HtmlTag, &str)> {
    // Parse a tag starting at '<' (at byte offset lt_index within s).
    let bytes = s.as_bytes();
    if lt_index >= bytes.len() || bytes[lt_index] != b'<' {
        return None;
    }
    if s[lt_index..].starts_with("<!--") {
        return Some((HtmlTag::CommentOpen, &s[lt_index + 4..]));
    }
    let mut i = lt_index + 1;
    if i >= bytes.len() {
        return None;
    }
    let is_closing = bytes[i] == b'/';
    if is_closing {
        i += 1;
    }
    if i >= bytes.len() || !bytes[i].is_ascii_alphabetic() {
        return None;
    }
    let name_start = i;
    i += 1;
    while i < bytes.len() && is_ascii_tag_name_char(bytes[i]) {
        i += 1;
    }
    let name = &s[name_start..i];
    // Must be followed by whitespace, '>', or '/' to be tag-like.
    let next = bytes.get(i).copied().unwrap_or(b'\0');
    if !(next == b' ' || next == b'\t' || next == b'>' || next == b'/') {
        return None;
    }
    // Find end of tag '>' on this line segment.
    let close_rel = s[i..].find('>')?;
    let close = i + close_rel;

    if is_closing {
        return Some((
            HtmlTag::Closing {
                name: name.to_ascii_lowercase(),
            },
            &s[close + 1..],
        ));
    }

    // Determine self-closing by checking '/' before '>' (ignoring trailing whitespace).
    let mut j = close;
    while j > i && matches!(bytes[j - 1], b' ' | b'\t') {
        j -= 1;
    }
    let self_closing =
        (j > i && bytes[j - 1] == b'/') || is_void_html_tag(&name.to_ascii_lowercase());
    Some((
        HtmlTag::Opening {
            name: name.to_ascii_lowercase(),
            self_closing,
        },
        &s[close + 1..],
    ))
}

fn apply_tag_to_stack(tag: &HtmlTag, rest: &str, stack: &mut Vec<String>, in_comment: &mut bool) {
    match tag {
        HtmlTag::CommentOpen => {
            // If the comment closes on the same line, do not enter comment mode.
            if !rest.contains("-->") {
                *in_comment = true;
            }
        }
        HtmlTag::Opening { name, self_closing } => {
            if !*self_closing {
                stack.push(name.clone());
            }
        }
        HtmlTag::Closing { name } => {
            if stack.last().is_some_and(|t| t == name) {
                stack.pop();
            } else {
                // Best-effort: do not attempt arbitrary stack rewrites.
            }
        }
    }
}

pub(super) fn update_html_block_state(line: &str, stack: &mut Vec<String>, in_comment: &mut bool) {
    let mut s = line;
    loop {
        if *in_comment {
            let Some(pos) = s.find("-->") else {
                return;
            };
            *in_comment = false;
            s = &s[pos + 3..];
            continue;
        }

        let Some(lt_rel) = s.find('<') else {
            return;
        };
        let lt = lt_rel;
        let after = &s[lt..];
        let Some((tag, rest)) = parse_tag_at(after, 0) else {
            s = &s[lt + 1..];
            continue;
        };
        apply_tag_to_stack(&tag, rest, stack, in_comment);

        // Continue scanning after the parsed tag opener/closer.
        s = rest;
    }
}
