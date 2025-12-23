mod support;

use mdstream::{BlockKind, MdStream, Options};

#[test]
fn very_long_line_is_chunking_invariant() {
    let text = "a".repeat(100_000);
    let opts = Options::default();

    let blocks_whole = support::collect_final_blocks(support::chunk_whole(&text), opts.clone());
    let blocks_rand = support::collect_final_blocks(
        support::chunk_pseudo_random(&text, "very_long_line", 0, 64),
        opts,
    );

    assert_eq!(blocks_rand, blocks_whole);
    assert_eq!(blocks_whole.len(), 1);
    assert_eq!(blocks_whole[0].0, BlockKind::Paragraph);
    assert_eq!(blocks_whole[0].1.len(), 100_000);
}

#[test]
fn many_short_paragraphs_are_stable() {
    let mut text = String::new();
    for i in 0..500usize {
        text.push_str(&format!("line {i}\n\n"));
    }

    let opts = Options::default();
    let blocks = support::collect_final_blocks(support::chunk_whole(&text), opts);

    assert_eq!(blocks.len(), 500);
    assert_eq!(blocks[0].0, BlockKind::Paragraph);
    assert_eq!(blocks[0].1, "line 0\n\n");
    assert_eq!(blocks[499].1, "line 499\n\n");
}

#[test]
fn deep_blockquote_nesting_does_not_panic() {
    let mut text = String::new();
    for i in 0..50usize {
        text.push_str(&">".repeat(i + 1));
        text.push_str(" layer\n");
    }

    let mut s = MdStream::new(Options::default());
    let _ = s.append(&text);
    let u = s.finalize();
    assert_eq!(u.committed.len(), 1);
    assert_eq!(u.committed[0].kind, BlockKind::BlockQuote);
    assert!(u.committed[0].raw.contains("layer\n"));
}

#[test]
fn deep_list_nesting_does_not_panic() {
    let mut text = String::new();
    for i in 0..30usize {
        text.push_str(&"  ".repeat(i));
        text.push_str("- item\n");
    }

    let mut s = MdStream::new(Options::default());
    let _ = s.append(&text);
    let u = s.finalize();
    assert_eq!(u.committed.len(), 1);
    assert_eq!(u.committed[0].kind, BlockKind::List);
    assert!(u.committed[0].raw.contains("- item\n"));
}
