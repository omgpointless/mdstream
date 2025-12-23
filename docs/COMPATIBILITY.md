# Compatibility & Edge Cases

This document tracks compatibility goals with:

- Streamdown (block splitting + termination behavior)
- Incremark (stable boundary detection + streaming edge cases)
- pulldown-cmark ecosystem (optional adapter)

## Scope Rule

`mdstream` follows a strict scope boundary:

- If Streamdown's `parseMarkdownIntoBlocks` supports a streaming-stability behavior at the block
  boundary level, `mdstream` must support it (with stable `committed + pending` semantics).
- If Streamdown does not support it (e.g. provider-specific tool calls, message parts, citations as
  business semantics), `mdstream` does not implement it in the core library.

Any behavior outside Streamdown parity must be explicitly agreed as project scope before adding.

## Streamdown parity checklist (block splitting)

Baseline: `repo-ref/streamdown/packages/streamdown/lib/parse-blocks.tsx`.

- Basic parsing: headings + paragraphs (covered: `tests/stream_streamdown_basic_parsing.rs`)
- Code fences: ``` / ~~~ (covered: `tests/stream_streamdown_code_blocks.rs`)
- Math blocks: `$$ ... $$` (covered: `tests/stream_block_splitting.rs`)
- HTML blocks: conservative merge with tag stack (covered: `tests/stream_block_splitting.rs`)
- Footnotes: if any footnotes exist, return single block (covered: `tests/stream_block_splitting.rs`)
- Tables: GFM delimiter row (covered: `tests/stream_block_splitting.rs`)
- Streaming simulation (covered: `tests/stream_streamdown_simulation.rs`)
- Mixed content scenario (covered: `tests/stream_streamdown_mixed_content.rs`)

## Streaming edge cases (must handle)

### Incomplete inline constructs

- emphasis markers: `*`, `**`, `***`, `_`, `__`
- inline code: backticks
- strikethrough: `~~`
- links/images:
  - incomplete URL
  - incomplete link text with nested brackets

### Block constructs spanning chunks

- fenced code blocks
- blockquotes + lists (nested)
- HTML blocks
- tables
- math blocks with `$$`
- footnote definitions with continuation indentation

## Footnotes and reference definitions

These constructs are document-scoped and can force either:

- stability-first behavior (single block)
- invalidation behavior (selective re-parse in adapters)

The chosen default should prioritize streaming stability.

## Non-standard and ecosystem behaviors

### Incomplete link placeholder

Streamdown uses a special URL marker: `streamdown:incomplete-link`.

`mdstream` should:

- default to the same marker for compatibility
- allow configuring it

### Images

Streamdown removes incomplete images (because partial images cannot display meaningfully).

`mdstream` should support the same default behavior.
