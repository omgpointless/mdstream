use mdstream::{MdStream, Options};

fn collect_final_blocks(chunks: impl IntoIterator<Item = String>, opts: Options) -> Vec<String> {
    let mut s = MdStream::new(opts);
    let mut out = Vec::new();

    for chunk in chunks {
        let u = s.append(&chunk);
        out.extend(u.committed.into_iter().map(|b| b.raw));
    }
    let u = s.finalize();
    out.extend(u.committed.into_iter().map(|b| b.raw));
    out
}

fn chunk_whole(text: &str) -> Vec<String> {
    vec![text.to_string()]
}

fn chunk_lines(text: &str) -> Vec<String> {
    text.split_inclusive('\n').map(|s| s.to_string()).collect()
}

fn chunk_pseudo_random(text: &str, mut seed: u32) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0usize;
    while start < text.len() {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let want = (seed % 40 + 1) as usize; // 1..=40 bytes
        let mut end = (start + want).min(text.len());
        while end < text.len() && !text.is_char_boundary(end) {
            end += 1;
        }
        out.push(text[start..end].to_string());
        start = end;
    }
    out
}

#[test]
fn streamdown_benchmark_single_block() {
    let markdown = "# Heading\n\nThis is a paragraph.";

    let opts = Options::default();
    let blocks_whole = collect_final_blocks(chunk_whole(markdown), opts.clone());
    let blocks_lines = collect_final_blocks(chunk_lines(markdown), opts.clone());
    let blocks_rand = collect_final_blocks(chunk_pseudo_random(markdown, 1), opts.clone());

    assert_eq!(blocks_lines, blocks_whole);
    assert_eq!(blocks_rand, blocks_whole);

    assert_eq!(
        blocks_whole,
        vec![
            "# Heading\n".to_string(),
            "This is a paragraph.".to_string()
        ]
    );
}

#[test]
fn streamdown_benchmark_multiple_blocks_10() {
    // From Streamdown's parse-blocks benchmark ("multiple blocks (10)").
    let markdown = r#"
# Heading 1

This is paragraph 1.

## Heading 2

This is paragraph 2.

- List item 1
- List item 2

> Blockquote text
"#;

    let opts = Options::default();
    let blocks_whole = collect_final_blocks(chunk_whole(markdown), opts.clone());
    let blocks_lines = collect_final_blocks(chunk_lines(markdown), opts.clone());
    let blocks_rand = collect_final_blocks(chunk_pseudo_random(markdown, 1), opts.clone());

    assert_eq!(blocks_lines, blocks_whole);
    assert_eq!(blocks_rand, blocks_whole);

    // Sanity checks: key blocks exist and order is stable.
    assert!(blocks_whole.iter().any(|b| b == "# Heading 1\n"));
    assert!(
        blocks_whole
            .iter()
            .any(|b| b.contains("This is paragraph 1."))
    );
    assert!(blocks_whole.iter().any(|b| b == "## Heading 2\n"));
    assert!(blocks_whole.iter().any(|b| b.contains("- List item 1")));
    assert!(blocks_whole.iter().any(|b| b.contains("> Blockquote text")));
}

#[test]
fn streamdown_benchmark_many_blocks_100() {
    // From Streamdown's parse-blocks benchmark ("many blocks (100)").
    let mut parts = Vec::new();
    for i in 0..100 {
        parts.push(format!("## Section {i}\n\nParagraph {i}"));
    }
    let markdown = parts.join("\n\n");

    let opts = Options::default();
    let blocks_whole = collect_final_blocks(chunk_whole(&markdown), opts.clone());
    let blocks_lines = collect_final_blocks(chunk_lines(&markdown), opts.clone());
    let blocks_rand = collect_final_blocks(chunk_pseudo_random(&markdown, 1), opts.clone());

    assert_eq!(blocks_lines, blocks_whole);
    assert_eq!(blocks_rand, blocks_whole);

    // Each section yields one heading block and one paragraph block.
    assert_eq!(blocks_whole.len(), 200);
    assert_eq!(blocks_whole[0], "## Section 0\n");
    assert!(blocks_whole[1].starts_with("Paragraph 0"));
    assert!(blocks_whole[blocks_whole.len() - 2].starts_with("## Section 99"));
    assert!(blocks_whole[blocks_whole.len() - 1].starts_with("Paragraph 99"));
}
