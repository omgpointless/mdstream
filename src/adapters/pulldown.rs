use std::collections::HashMap;

use crate::types::{Block, BlockId, Update};

use pulldown_cmark::{Event, Options as PulldownOptions, Parser};

#[derive(Debug, Clone)]
pub struct PulldownAdapterOptions {
    pub pulldown: PulldownOptions,
    /// If true, pending blocks are parsed from `display` (terminated) when available.
    pub prefer_display_for_pending: bool,
}

impl Default for PulldownAdapterOptions {
    fn default() -> Self {
        Self {
            pulldown: PulldownOptions::empty(),
            prefer_display_for_pending: true,
        }
    }
}

#[derive(Debug, Default)]
pub struct PulldownAdapter {
    opts: PulldownAdapterOptions,
    committed_raw: HashMap<BlockId, String>,
    committed_cache: HashMap<BlockId, Vec<Event<'static>>>,
    reference_definitions: String,
}

impl PulldownAdapter {
    pub fn new(opts: PulldownAdapterOptions) -> Self {
        Self {
            opts,
            committed_raw: HashMap::new(),
            committed_cache: HashMap::new(),
            reference_definitions: String::new(),
        }
    }

    pub fn clear(&mut self) {
        self.committed_raw.clear();
        self.committed_cache.clear();
        self.reference_definitions.clear();
    }

    pub fn apply_update(&mut self, update: &Update) {
        if update.reset {
            self.clear();
        }
        for block in &update.committed {
            self.committed_raw.insert(block.id, block.raw.clone());
            self.collect_reference_definitions(&block.raw);
            let events = self.parse_committed_with_definitions(&block.raw);
            self.committed_cache.insert(block.id, events);
        }

        // If definitions arrived late, selectively re-parse invalidated blocks.
        for id in &update.invalidated {
            let Some(raw) = self.committed_raw.get(id) else {
                continue;
            };
            let events = self.parse_committed_with_definitions(raw);
            self.committed_cache.insert(*id, events);
        }
    }

    pub fn committed_events(&self, id: BlockId) -> Option<&[Event<'static>]> {
        self.committed_cache.get(&id).map(|v| v.as_slice())
    }

    pub fn parse_pending(&self, pending: &Block) -> Vec<Event<'static>> {
        let input = if self.opts.prefer_display_for_pending {
            pending.display.as_deref().unwrap_or(&pending.raw)
        } else {
            &pending.raw
        };
        // Pending should reflect the best-known definitions so far too.
        self.parse_with_definitions(input)
    }

    fn parse_committed_with_definitions(&self, raw: &str) -> Vec<Event<'static>> {
        self.parse_with_definitions(raw)
    }

    fn parse_with_definitions(&self, raw: &str) -> Vec<Event<'static>> {
        if self.reference_definitions.is_empty() {
            return parse_events_static(raw, self.opts.pulldown);
        }
        let mut input = String::with_capacity(self.reference_definitions.len() + 2 + raw.len());
        input.push_str(&self.reference_definitions);
        input.push_str("\n\n");
        input.push_str(raw);
        parse_events_static(&input, self.opts.pulldown)
    }

    fn collect_reference_definitions(&mut self, raw: &str) {
        // Best-effort: extract single-line reference definitions and keep the latest per label.
        for line in raw.split('\n') {
            if let Some((label, def_line)) = extract_reference_definition(line) {
                self.upsert_reference_definition(&label, &def_line);
            }
        }
    }

    fn upsert_reference_definition(&mut self, label: &str, def_line: &str) {
        // Keep a simple stable format: one definition per line.
        // Rebuild on update to avoid complicated string patching.
        let mut map: HashMap<String, String> = HashMap::new();
        for line in self.reference_definitions.split('\n') {
            if let Some((k, v)) = extract_reference_definition(line) {
                map.insert(k, v);
            }
        }
        map.insert(label.to_string(), def_line.to_string());
        let mut defs: Vec<_> = map.into_iter().collect();
        defs.sort_by(|a, b| a.0.cmp(&b.0));
        self.reference_definitions = defs
            .into_iter()
            .map(|(_, v)| v)
            .collect::<Vec<_>>()
            .join("\n");
    }
}

fn parse_events_static(input: &str, options: PulldownOptions) -> Vec<Event<'static>> {
    Parser::new_ext(input, options)
        .map(|e| e.into_static())
        .collect()
}

fn extract_reference_definition(line: &str) -> Option<(String, String)> {
    // Match up to 3 leading spaces, then "[label]:"
    let mut s = line;
    let mut spaces = 0usize;
    while spaces < 3 && s.starts_with(' ') {
        s = &s[1..];
        spaces += 1;
    }
    let bytes = s.as_bytes();
    if bytes.len() < 4 || bytes[0] != b'[' {
        return None;
    }
    let close = s.find(']')?;
    if close == 1 {
        return None;
    }
    if s.as_bytes().get(close + 1) != Some(&b':') {
        return None;
    }
    let label = &s[1..close];
    if label.starts_with('^') {
        return None;
    }
    let label = normalize_label(label)?;
    Some((label, line.trim_end().to_string()))
}

fn normalize_label(label: &str) -> Option<String> {
    let trimmed = label.trim();
    if trimmed.is_empty() || trimmed.len() > 200 {
        return None;
    }
    let mut out = String::with_capacity(trimmed.len());
    let mut last_ws = false;
    for ch in trimmed.chars() {
        if ch.is_whitespace() {
            last_ws = true;
            continue;
        }
        if last_ws && !out.is_empty() {
            out.push(' ');
        }
        last_ws = false;
        for lc in ch.to_lowercase() {
            out.push(lc);
        }
    }
    if out.is_empty() { None } else { Some(out) }
}
