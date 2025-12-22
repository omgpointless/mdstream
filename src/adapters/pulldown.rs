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
    committed_cache: HashMap<BlockId, Vec<Event<'static>>>,
}

impl PulldownAdapter {
    pub fn new(opts: PulldownAdapterOptions) -> Self {
        Self {
            opts,
            committed_cache: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.committed_cache.clear();
    }

    pub fn apply_update(&mut self, update: &Update) {
        for block in &update.committed {
            let events = parse_events_static(&block.raw, self.opts.pulldown);
            self.committed_cache.insert(block.id, events);
        }

        // Post-MVP: update.invalidated handling can selectively re-parse blocks.
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
        parse_events_static(input, self.opts.pulldown)
    }
}

fn parse_events_static(input: &str, options: PulldownOptions) -> Vec<Event<'static>> {
    Parser::new_ext(input, options)
        .map(|e| e.into_static())
        .collect()
}

