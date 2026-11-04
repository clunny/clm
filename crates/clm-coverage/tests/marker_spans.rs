use std::path::Path;

use clm_coverage::{marker_spans, markers_in};

// Fixture markers are built so the literal never appears whole (`dangling-markers`).
const REQ: &str = "req";
const REMOVE_BY: &str = "clm:remove";

#[test]
fn scenario_two_markers_on_a_line() {
    // req:check.coverage/marker-spans@3ee7977d
    let text = format!("// {REQ}:a/x@11111111 and {REQ}:b/y");
    let spans = marker_spans(&text);
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].id, "a/x");
    assert_eq!(spans[0].version.as_deref(), Some("11111111"));
    assert_eq!(spans[0].range, 3..19);
    assert_eq!(spans[1].id, "b/y");
    assert_eq!(spans[1].version, None);
    assert_eq!(spans[1].range, 24..31);
}

#[test]
fn scenario_the_same_markers_as_markers_in() {
    // req:check.coverage/marker-spans@3ee7977d
    let text = [
        format!("// {REQ}:a/x@11111111 and x{REQ}:a/no"),
        format!("fn f() {{}} // {REMOVE_BY}-by 20260101-gone"),
        format!("\tlet s = \"é {REQ}:n.m/y-z@abc\"; {REQ}:b/y@12345678\r"),
        format!("{REQ}:c/z"),
    ]
    .join("\n");
    let spans = marker_spans(&text);
    let from_spans: Vec<_> = spans
        .iter()
        .map(|s| (s.id.clone(), s.version.clone()))
        .collect();
    let from_markers: Vec<_> = markers_in(Path::new("t.rs"), &text)
        .into_iter()
        .map(|m| (m.id, m.version))
        .collect();
    assert_eq!(from_spans, from_markers);
    assert_eq!(from_spans.len(), 4);
    for s in &spans {
        assert!(text[s.range.clone()].starts_with("req:"));
    }
    assert_eq!(text[spans[2].range.clone()], format!("{REQ}:b/y@12345678"));
    assert_eq!(text[spans[3].range.clone()], format!("{REQ}:c/z"));
}
