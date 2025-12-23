# mdstream

`mdstream` is a **streaming-first Markdown middleware** for Rust.

It targets LLM token-by-token / chunk-by-chunk output and helps downstream UIs (egui, gpui/Zed, TUI, etc.) avoid the classic **O(n²)** re-parse + re-render pattern that causes latency and flicker.

## Goals

- **Fix the O(n²) problem**: only the *new* part should be processed on each chunk.
- **Render-agnostic**: no rendering; provide stable, incremental building blocks that any UI can consume.
- **Handle the dirty work**: deal with chunk boundaries and incomplete Markdown so downstream parsers/renderers behave predictably.
- **Match capabilities of Streamdown + Incremark** in streaming Markdown handling and edge-case coverage.

## Non-goals

- Not a Markdown renderer.
- Not a full CommonMark/GFM conformance test suite (we prioritize streaming stability and practical compatibility).
- Not a hard dependency on a specific parser (pulldown-cmark integration is optional).

## Core Model (high level)

- The input stream is represented as a sequence of **blocks**:
  - **Committed blocks**: stable, never change again (safe for UI to cache by `BlockId`).
  - A single **pending block**: may change while streaming (UI updates only this block).
- Some scope-driven transitions require a full reset (e.g. switching into single-block footnote mode):
  - `append()` may return `Update { reset: true, .. }` to tell consumers to drop cached blocks.
- A **pending pipeline** can optionally produce a `display` view for the pending block:
  - Markdown terminator (remend-like) for incomplete constructs near the tail.
  - Custom transforms via `PendingTransformer` (eg placeholders, sanitizers).

## Optional: Reference Definitions Invalidation (Best-effort)

Markdown reference-style links/images can be defined *after* they are used:

- usage: `See [docs][ref].` or `See [ref].`
- definition (often later): `[ref]: https://example.com`

In streaming UIs that parse/render **each committed block independently**, late-arriving reference
definitions can require re-parsing earlier blocks so they turn into real links.

`mdstream` provides an **opt-in** invalidation signal for this:

- Enable: `opts.reference_definitions = ReferenceDefinitionsMode::Invalidate`
- When a reference definition is **committed**, `Update.invalidated` contains the `BlockId`s of
  previously committed blocks that likely used the label.
- Consumers/adapters can re-parse only those blocks instead of re-parsing the entire document.

This is intentionally **best-effort** (optimized for LLM streaming), not a full CommonMark/GFM
reference definition implementation:

- Only single-line definitions are recognized (`^[ ]{0,3}[label]: ...`), footnotes (`[^x]:`) are excluded.
- Label matching is normalized (trim, collapse whitespace, case-insensitive).
- Usage extraction over-approximates: false positives may cause extra invalidations; the goal is to
  avoid missing invalidations.
- Definitions inside fenced code blocks do not trigger invalidations.

Example:

```rust
use mdstream::{MdStream, Options, ReferenceDefinitionsMode};

let mut opts = Options::default();
opts.reference_definitions = ReferenceDefinitionsMode::Invalidate;

let mut s = MdStream::new(opts);
let u1 = s.append("See [ref].\n\n");
assert!(u1.committed.is_empty());

let u2 = s.append("[ref]: https://example.com\n\nNext\n");
assert!(u2.invalidated.contains(&mdstream::BlockId(1)));
```

## Documentation

- Architecture: `docs/ARCHITECTURE.md`
- MVP definition & acceptance tests: `docs/MVP.md`
- Roadmap: `docs/ROADMAP.md`
- Usage (integration patterns): `docs/USAGE.md`
- Compatibility & edge cases: `docs/COMPATIBILITY.md`
- Adapters (pulldown-cmark, etc.): `docs/ADAPTERS.md`
- Extension points: `docs/EXTENSIONS.md`
  - Note: End-user integration guidance is kept in this README; the `docs/` folder is primarily for
    development notes and may be pruned for releases.

## Status

Initial MVP implementation is in progress:

- `MdStream` core state machine (blocks: committed + pending)
- Pending terminator (Streamdown/remend-inspired)
- Streaming boundary tests (Streamdown/Incremark-inspired)
- Reference-style link definitions invalidation (opt-in, for adapters)
- Optional `pulldown-cmark` adapter via the `pulldown` feature

Try the demo:

`cargo run --example tui_like`

Try the `pulldown-cmark` incremental demo:

`cargo run --features pulldown --example pulldown_incremental`
