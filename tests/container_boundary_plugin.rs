use mdstream::{ContainerBoundaryPlugin, MdStream, Options};

fn collect_final_blocks(
    chunks: impl IntoIterator<Item = String>,
    plugin: ContainerBoundaryPlugin,
) -> Vec<String> {
    let mut s = MdStream::new(Options::default()).with_boundary_plugin(plugin);
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
fn detects_container_start_and_end() {
    let markdown = "Intro\n\n::: warning\nA\n:::\n\nAfter\n";
    let blocks = collect_final_blocks(chunk_whole(markdown), ContainerBoundaryPlugin::default());
    assert_eq!(
        blocks,
        vec![
            "Intro\n\n".to_string(),
            "::: warning\nA\n:::\n".to_string(),
            "After\n".to_string(),
        ]
    );
}

#[test]
fn longer_markers_allow_nesting_depth() {
    let markdown = "::: outer\nA\n::::: inner\nB\n:::\nC\n:::\nAfter\n";
    let blocks = collect_final_blocks(chunk_whole(markdown), ContainerBoundaryPlugin::default());
    assert_eq!(
        blocks,
        vec![
            "::: outer\nA\n::::: inner\nB\n:::\nC\n:::\n".to_string(),
            "After\n".to_string(),
        ]
    );
}

#[test]
fn allowed_names_filters_opening() {
    let mut plugin = ContainerBoundaryPlugin::default();
    plugin.allowed_names = Some(vec!["warning".to_string(), "info".to_string()]);

    let markdown = "::: danger\nA\n:::\n\nAfter\n";
    let blocks = collect_final_blocks(chunk_whole(markdown), plugin);

    // Not detected as a container -> split by blank line only.
    assert_eq!(
        blocks,
        vec!["::: danger\nA\n:::\n\n".to_string(), "After\n".to_string()]
    );
}

#[test]
fn chunking_invariance_for_containers() {
    let markdown = "Intro\n\n::: note attr=1\nA\n\nB\n:::\n\nAfter\n";
    let plugin = ContainerBoundaryPlugin::default();

    let blocks_whole = collect_final_blocks(chunk_whole(markdown), plugin.clone());
    let blocks_lines = collect_final_blocks(chunk_lines(markdown), plugin.clone());
    let blocks_chars = collect_final_blocks(chunk_chars(markdown), plugin.clone());
    let blocks_rand = collect_final_blocks(chunk_pseudo_random(markdown, 9), plugin.clone());

    assert_eq!(blocks_lines, blocks_whole);
    assert_eq!(blocks_chars, blocks_whole);
    assert_eq!(blocks_rand, blocks_whole);
}
