use mdstream::{MdStream, Options, ReferenceDefinitionsMode};

#[test]
fn emits_invalidated_when_reference_definition_is_committed() {
    let mut opts = Options::default();
    opts.reference_definitions = ReferenceDefinitionsMode::Invalidate;

    let mut s = MdStream::new(opts);

    // First paragraph with a shortcut reference. This won't be committed until a later non-empty line arrives.
    let u1 = s.append("See [ref].\n\n");
    assert!(u1.committed.is_empty());

    // Starting the definition line will commit the previous paragraph block.
    let u2 = s.append("[ref]: https://example.com\n");
    assert_eq!(u2.committed.len(), 1);
    assert!(u2.invalidated.is_empty(), "definition not committed yet");

    // Commit the definition block by introducing a blank line then a non-empty line.
    s.append("\n");
    let u3 = s.append("Next\n");
    assert!(!u3.committed.is_empty());
    assert_eq!(u3.invalidated, vec![mdstream::BlockId(1)]);
}

#[test]
fn stability_first_mode_does_not_emit_invalidations() {
    let mut opts = Options::default();
    opts.reference_definitions = ReferenceDefinitionsMode::StabilityFirst;

    let mut s = MdStream::new(opts);
    s.append("See [ref].\n\n");
    let u2 = s.append("[ref]: https://example.com\n");
    assert_eq!(u2.committed.len(), 1);
    s.append("\n");
    let u3 = s.append("Next\n");
    assert!(u3.invalidated.is_empty());
}

