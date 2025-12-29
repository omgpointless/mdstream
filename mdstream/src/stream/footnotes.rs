pub(super) fn is_footnote_definition_start(line: &str) -> bool {
    let s = line.trim_start();
    s.starts_with("[^") && s.contains("]:")
}

pub(super) fn is_footnote_continuation(line: &str) -> bool {
    line.starts_with("    ") || line.starts_with('\t')
}

pub(super) fn detect_footnotes(text: &str) -> bool {
    // Very small, streaming-friendly detector:
    // - references: [^id] (not followed by :)
    // - definitions: [^id]:
    //
    // Compatibility notes:
    // - Align with Streamdown/Incremark: identifiers must not contain whitespace, and must be non-empty.
    // - Keep a conservative identifier length cap to avoid pathological scans.
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i + 2 < bytes.len() {
        if bytes[i] == b'[' && bytes[i + 1] == b'^' {
            const MAX_ID_LEN: usize = 200;
            // Find closing `]` while validating identifier rules.
            let mut j = i + 2;
            let mut id_len = 0usize;
            while j < bytes.len() {
                let b = bytes[j];
                if b == b']' {
                    break;
                }
                if b == b'\n' || b == b'\r' || b == b' ' || b == b'\t' {
                    // Invalid footnote identifier; do not treat as footnote.
                    id_len = 0;
                    break;
                }
                id_len += 1;
                if id_len > MAX_ID_LEN {
                    id_len = 0;
                    break;
                }
                j += 1;
            }
            if id_len > 0 && j < bytes.len() && bytes[j] == b']' {
                // Either a reference (`[^id]`) or a definition (`[^id]:`) should trigger single-block mode.
                return true;
            }
        }
        i += 1;
    }
    false
}
