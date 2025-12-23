use mdstream::{FenceBoundaryPlugin, MdStream, Options};

fn collect_final_blocks(chunks: impl IntoIterator<Item = String>, opts: Options) -> Vec<String> {
    let mut s = MdStream::new(opts).with_boundary_plugin(FenceBoundaryPlugin::triple_colon());
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

fn chunk_chars(text: &str) -> Vec<String> {
    text.chars().map(|c| c.to_string()).collect()
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
fn triple_colon_container_is_single_block() {
    let markdown = "Intro\n\n:::warning\nA\n\nB\n:::\n\nAfter\n";

    let mut s =
        MdStream::new(Options::default()).with_boundary_plugin(FenceBoundaryPlugin::triple_colon());
    let u1 = s.append(markdown);
    let u2 = s.finalize();
    let blocks: Vec<String> = u1
        .committed
        .into_iter()
        .chain(u2.committed.into_iter())
        .map(|b| b.raw)
        .collect();

    assert_eq!(
        blocks,
        vec![
            "Intro\n\n".to_string(),
            ":::warning\nA\n\nB\n:::\n".to_string(),
            "After\n".to_string(),
        ]
    );
}

#[test]
fn triple_colon_container_chunking_invariance() {
    let markdown = "Intro\n\n:::note\nA\n\nB\n:::\n\nAfter\n";
    let opts = Options::default();

    let blocks_whole = collect_final_blocks(chunk_whole(markdown), opts.clone());
    let blocks_lines = collect_final_blocks(chunk_lines(markdown), opts.clone());
    let blocks_chars = collect_final_blocks(chunk_chars(markdown), opts.clone());
    let blocks_rand = collect_final_blocks(chunk_pseudo_random(markdown, 7), opts.clone());

    assert_eq!(blocks_lines, blocks_whole);
    assert_eq!(blocks_chars, blocks_whole);
    assert_eq!(blocks_rand, blocks_whole);
}

#[test]
fn reset_clears_boundary_plugin_state() {
    let mut s =
        MdStream::new(Options::default()).with_boundary_plugin(FenceBoundaryPlugin::triple_colon());
    s.append(":::\nA\n");
    s.reset();

    let u = s.append("A\n\nB\n");
    assert_eq!(u.committed.len(), 1);
    assert_eq!(u.committed[0].raw, "A\n\n");
    assert_eq!(u.pending.as_ref().unwrap().raw, "B\n");
}
