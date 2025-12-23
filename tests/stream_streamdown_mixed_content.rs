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
fn streamdown_benchmark_realistic_ai_response_chunking_invariance() {
    // From Streamdown's parse-blocks benchmark ("Mixed Content").
    let markdown = r#"# AI Response Example

Here's a comprehensive example of markdown content:

## Code Example

```typescript
interface User {
  id: string;
  name: string;
  email: string;
}

function getUser(id: string): User {
  return { id, name: "John", email: "john@example.com" };
}
```

## Math Formula

The quadratic formula is:

$$
x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}
$$

## Lists and Tables

| Feature | Status |
|---------|--------|
| Bold    | ✓      |
| Italic  | ✓      |
| Code    | ✓      |

### Checklist

- [x] Implement parser
- [ ] Add tests
- [ ] Write docs

> **Note**: This is a blockquote with **bold** text.

For more info, see [documentation](https://example.com).
"#;

    let opts = Options::default();
    let blocks_whole = collect_final_blocks(chunk_whole(markdown), opts.clone());
    let blocks_lines = collect_final_blocks(chunk_lines(markdown), opts.clone());
    let blocks_rand = collect_final_blocks(chunk_pseudo_random(markdown, 1), opts.clone());

    assert_eq!(blocks_lines, blocks_whole);
    assert_eq!(blocks_rand, blocks_whole);
}

