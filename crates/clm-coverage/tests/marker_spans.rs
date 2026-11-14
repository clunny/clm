use std::path::Path;

use clm_coverage::{marker_spans, markers_in, markers_outside_tests};

// Fixture markers are built so the literal never appears whole (`dangling-markers`).
const REQ: &str = "req";
const REMOVE_BY: &str = "clm:remove";

#[test]
fn scenario_two_markers_on_a_line() {
    // req:check.coverage/marker-spans@11d19af5
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
fn scenario_every_marker_of_the_file() {
    // req:check.coverage/marker-spans@11d19af5
    let text = [
        "#[test]".to_string(),
        "fn proves() {".to_string(),
        format!("    // {REQ}:a/x@11111111"),
        "}".to_string(),
        "fn helper() {".to_string(),
        format!("    // {REQ}:b/y@22222222 {REMOVE_BY}-by 20260101-gone and x{REQ}:c/no"),
        "}".to_string(),
    ]
    .join("\n");
    let path = Path::new("t.rs");
    let spans = marker_spans(&text);
    let ids: Vec<_> = spans
        .iter()
        .map(|s| (s.id.as_str(), s.version.as_deref()))
        .collect();
    assert_eq!(ids, [("a/x", Some("11111111")), ("b/y", Some("22222222"))]);
    let inside = markers_in(path, &text);
    let outside = markers_outside_tests(path, &text);
    let key = |id: &str, version: &Option<String>| (id.to_string(), version.clone());
    assert_eq!(
        key(&spans[0].id, &spans[0].version),
        key(&inside[0].id, &inside[0].version)
    );
    assert_eq!(outside.len(), 1);
    assert_eq!(
        key(&spans[1].id, &spans[1].version),
        key(&outside[0].id, &outside[0].version)
    );
    for s in &spans {
        assert!(text[s.range.clone()].starts_with("req:"));
    }
}
