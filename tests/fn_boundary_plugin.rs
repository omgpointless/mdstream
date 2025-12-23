use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use mdstream::{BoundaryUpdate, FnBoundaryPlugin, MdStream, Options};

fn collect_final_blocks(
    chunks: impl IntoIterator<Item = String>,
    plugin: FnBoundaryPlugin,
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
fn fn_boundary_plugin_can_model_a_custom_fence() {
    let started = Arc::new(AtomicUsize::new(0));
    let just_started = Arc::new(AtomicBool::new(false));

    let started_for_start = started.clone();
    let just_started_for_start = just_started.clone();
    let just_started_for_update = just_started.clone();

    let plugin = FnBoundaryPlugin::new(
        |line| line.trim() == "@@@",
        move |line| {
            if just_started_for_update.swap(false, Ordering::SeqCst) {
                return BoundaryUpdate::Continue;
            }
            if line.trim() == "@@@" {
                BoundaryUpdate::Close
            } else {
                BoundaryUpdate::Continue
            }
        },
    )
    .with_start(move |_| {
        started_for_start.fetch_add(1, Ordering::SeqCst);
        just_started_for_start.store(true, Ordering::SeqCst);
    })
    .with_reset({
        let just_started = just_started.clone();
        move || {
            just_started.store(false, Ordering::SeqCst);
        }
    });

    let markdown = "Intro\n\n@@@\nA\n\nB\n@@@\n\nAfter\n";
    let blocks = collect_final_blocks(chunk_whole(markdown), plugin);
    assert_eq!(
        blocks,
        vec![
            "Intro\n\n".to_string(),
            "@@@\nA\n\nB\n@@@\n".to_string(),
            "After\n".to_string(),
        ]
    );
    assert_eq!(started.load(Ordering::SeqCst), 1);
}

#[test]
fn fn_boundary_plugin_chunking_invariance() {
    let just_started = Arc::new(AtomicBool::new(false));
    let js_start = just_started.clone();
    let js_update = just_started.clone();

    let plugin = FnBoundaryPlugin::new(
        |line| line.trim() == "@@@",
        move |line| {
            if js_update.swap(false, Ordering::SeqCst) {
                return BoundaryUpdate::Continue;
            }
            if line.trim() == "@@@" {
                BoundaryUpdate::Close
            } else {
                BoundaryUpdate::Continue
            }
        },
    )
    .with_start(move |_| {
        js_start.store(true, Ordering::SeqCst);
    });

    let markdown = "Intro\n\n@@@\nA\n\nB\n@@@\n\nAfter\n";

    let blocks_whole = collect_final_blocks(chunk_whole(markdown), plugin);

    let just_started = Arc::new(AtomicBool::new(false));
    let js_start = just_started.clone();
    let js_update = just_started.clone();
    let plugin = FnBoundaryPlugin::new(
        |line| line.trim() == "@@@",
        move |line| {
            if js_update.swap(false, Ordering::SeqCst) {
                return BoundaryUpdate::Continue;
            }
            if line.trim() == "@@@" {
                BoundaryUpdate::Close
            } else {
                BoundaryUpdate::Continue
            }
        },
    )
    .with_start(move |_| {
        js_start.store(true, Ordering::SeqCst);
    });
    let blocks_lines = collect_final_blocks(chunk_lines(markdown), plugin);

    let just_started = Arc::new(AtomicBool::new(false));
    let js_start = just_started.clone();
    let js_update = just_started.clone();
    let plugin = FnBoundaryPlugin::new(
        |line| line.trim() == "@@@",
        move |line| {
            if js_update.swap(false, Ordering::SeqCst) {
                return BoundaryUpdate::Continue;
            }
            if line.trim() == "@@@" {
                BoundaryUpdate::Close
            } else {
                BoundaryUpdate::Continue
            }
        },
    )
    .with_start(move |_| {
        js_start.store(true, Ordering::SeqCst);
    });
    let blocks_chars = collect_final_blocks(chunk_chars(markdown), plugin);

    let just_started = Arc::new(AtomicBool::new(false));
    let js_start = just_started.clone();
    let js_update = just_started.clone();
    let plugin = FnBoundaryPlugin::new(
        |line| line.trim() == "@@@",
        move |line| {
            if js_update.swap(false, Ordering::SeqCst) {
                return BoundaryUpdate::Continue;
            }
            if line.trim() == "@@@" {
                BoundaryUpdate::Close
            } else {
                BoundaryUpdate::Continue
            }
        },
    )
    .with_start(move |_| {
        js_start.store(true, Ordering::SeqCst);
    });
    let blocks_rand = collect_final_blocks(chunk_pseudo_random(markdown, 2), plugin);

    assert_eq!(blocks_lines, blocks_whole);
    assert_eq!(blocks_chars, blocks_whole);
    assert_eq!(blocks_rand, blocks_whole);
}
