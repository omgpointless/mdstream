use mdstream::{FnPendingTransformer, MdStream, Options};

#[test]
fn pending_transformer_can_override_display() {
    let mut s = MdStream::new(Options::default());
    s.push_pending_transformer(FnPendingTransformer(|input: mdstream::PendingTransformInput<'_>| {
        Some(format!("{}<<t>>", input.display))
    }));

    let u1 = s.append("hi");
    let p1 = u1.pending.expect("pending");
    assert_eq!(p1.raw, "hi");
    assert_eq!(p1.display.as_deref(), Some("hi<<t>>"));

    let u2 = s.append(" there");
    let p2 = u2.pending.expect("pending");
    assert_eq!(p2.raw, "hi there");
    assert_eq!(p2.display.as_deref(), Some("hi there<<t>>"));
}
