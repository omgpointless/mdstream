use std::collections::HashSet;

use crate::reference::normalize_reference_label;

pub(super) fn extract_reference_usages(text: &str) -> HashSet<String> {
    // Best-effort extractor for reference-style link labels:
    // - [text][label]
    // - [label][]
    // - [label] (shortcut)
    //
    // We intentionally over-approximate: false positives only cause extra invalidations.
    let bytes = text.as_bytes();
    let mut out = HashSet::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'[' {
            i += 1;
            continue;
        }
        let mut close1 = i + 1;
        while close1 < bytes.len() && bytes[close1] != b']' {
            close1 += 1;
        }
        if close1 >= bytes.len() {
            break;
        }
        let label1 = &text[i + 1..close1];
        // Skip footnote-ish labels.
        if label1.as_bytes().first() == Some(&b'^') {
            i = close1 + 1;
            continue;
        }

        // Inline links/images: [text](...) / ![alt](...)
        if bytes.get(close1 + 1) == Some(&b'(') {
            i = close1 + 1;
            continue;
        }
        // Definition: [label]: ...
        if bytes.get(close1 + 1) == Some(&b':') {
            i = close1 + 1;
            continue;
        }

        // Reference form: [text][label] or [label][]
        if bytes.get(close1 + 1) == Some(&b'[') {
            let start2 = close1 + 2;
            if start2 >= bytes.len() {
                break;
            }
            let mut close2 = start2;
            while close2 < bytes.len() && bytes[close2] != b']' {
                close2 += 1;
            }
            if close2 >= bytes.len() {
                break;
            }
            let label2 = &text[start2..close2];
            let chosen = if label2.trim().is_empty() {
                label1
            } else {
                label2
            };
            if let Some(norm) = normalize_reference_label(chosen) {
                out.insert(norm);
            }
            i = close2 + 1;
            continue;
        }

        // Shortcut reference: [label]
        if let Some(norm) = normalize_reference_label(label1) {
            out.insert(norm);
        }
        i = close1 + 1;
    }
    out
}
