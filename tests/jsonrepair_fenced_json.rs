#![cfg(feature = "jsonrepair")]

use mdstream::{MdStream, Options};

#[test]
fn repairs_pending_fenced_json_display() {
    let mut opts = Options::default();
    opts.json_repair_in_fences = true;

    let mut s = MdStream::new(opts);
    let u = s.append("```json\n{name: 'John', age: 30,}\n");
    let pending = u.pending.expect("pending");
    let display = pending.display.expect("display");

    assert!(display.starts_with("```json\n"));
    assert!(
        display.contains(r#"{"name":"John","age":30}"#),
        "display={display:?}"
    );
}

