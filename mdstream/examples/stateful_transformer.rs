//! Demonstrate a stateful pending transformer.
//!
//! Run:
//!   cargo run --example stateful_transformer

use mdstream::{FnPendingTransformer, MdStream, Options};

fn main() {
    let mut s = MdStream::new(Options::default());

    let mut seen = 0usize;
    s.push_pending_transformer(FnPendingTransformer(
        move |input: mdstream::PendingTransformInput<'_>| {
            seen += 1;
            Some(format!("[seen={seen}] {}", input.display))
        },
    ));

    for chunk in ["Hello", " ", "**wor", "ld"] {
        let u = s.append(chunk);
        if let Some(p) = u.pending {
            println!("pending: {}", p.display_or_raw());
        }
    }
}
