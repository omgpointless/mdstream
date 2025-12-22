#![cfg(feature = "pulldown")]

use mdstream::adapters::pulldown::{PulldownAdapter, PulldownAdapterOptions};
use mdstream::{MdStream, Options};
use pulldown_cmark::{Event, Options as PulldownOptions, Tag};

#[test]
fn parses_committed_blocks_and_pending_display() {
    let mut stream = MdStream::new(Options::default());
    let mut adapter = PulldownAdapter::new(PulldownAdapterOptions {
        pulldown: PulldownOptions::ENABLE_TABLES | PulldownOptions::ENABLE_STRIKETHROUGH,
        prefer_display_for_pending: true,
    });

    let u1 = stream.append("Hello\n\n**bold");
    adapter.apply_update(&u1);
    assert_eq!(u1.committed.len(), 1);
    let p = u1.pending.unwrap();
    let pending_events = adapter.parse_pending(&p);
    // Pending display should be terminated: "**bold**"
    assert!(pending_events.iter().any(|e| matches!(e, Event::Start(Tag::Strong))));
}

