# Roadmap

This roadmap is intentionally practical: it prioritizes streaming stability and compatibility with Streamdown + Incremark behaviors.

## v0.1 (MVP)

- Block stream model: `committed + pending`
- Stable boundary detection (core block-level constructs)
- Pending termination (remend-like)
- Minimal configuration options
- Unit tests covering streaming edge cases
- Reference-style link definitions invalidation (opt-in mode)
- Optional fenced-JSON repair via `jsonrepair` (feature-gated, opt-in)
- Optional `pulldown-cmark` adapter (feature-gated)

## v0.2 (Ergonomics + Robustness)

- Add `snapshot_blocks()` and `snapshot_text()` convenience APIs
- Improve HTML block handling and table/list heuristics
- More remend parity tests (regression suite)

## v0.3 (Cross-block semantics)

- Footnote mode improvements:
  - default remains stability-first
  - optional invalidation-based strategy for advanced consumers

## v0.4+ (Extensions)

- Extension points for custom containers / directives
- More built-in analyzers for code fence info strings (mermaid, json, etc.)
- Performance benchmarks and regression suite
