//! The model reads code fences as CommonMark does.

use std::path::PathBuf;

use clm_model::{Files, load_files, outline::fence_mask};

fn marked(lines: &[&str]) -> Vec<bool> {
    fence_mask(lines)
}

// req:model/code-fences@59b932df
#[test]
fn a_backtick_run_with_a_backtick_after_it_opens_no_fence() {
    assert_eq!(
        marked(&["```x```", "#### Scenario: s", "text"]),
        [false, false, false]
    );
}

// req:model/code-fences@59b932df
#[test]
fn a_fence_closes_only_on_a_bare_run_at_least_as_long() {
    assert_eq!(
        marked(&[
            "````",
            "```",
            "```rust",
            "#### Scenario: fake",
            "````",
            "after"
        ]),
        [true, true, true, true, true, false]
    );
}

// req:model/code-fences@59b932df
#[test]
fn a_tilde_fence_closes_only_on_tildes() {
    assert_eq!(
        marked(&["~~~", "```", "~~~ not a close", "~~~~", "after"]),
        [true, true, true, true, false]
    );
}

// req:model/code-fences@59b932df
#[test]
fn a_tilde_run_opens_whatever_follows_it() {
    assert_eq!(
        marked(&["~~~ a `b` ~", "x", "~~~", "after"]),
        [true, true, true, false]
    );
}

// req:model/code-fences@59b932df
#[test]
fn a_close_may_carry_leading_and_trailing_white_space() {
    assert_eq!(
        marked(&["  ```", "x", "  ````  ", "after"]),
        [true, true, true, false]
    );
}

// req:model/code-fences@59b932df
#[test]
fn a_fence_never_closed() {
    assert_eq!(marked(&["```", "## Design"]), [true, true]);
}

// req:model/code-fences@59b932df
#[test]
fn a_heading_in_a_longer_fence() {
    let spec = "---\ntitle: T\nstatus: active\nowners: []\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n\n### Requirement: a\nIt SHALL.\n\n````\n```\n### Requirement: fake\n````\n\n#### Scenario: s\n- ok\n";
    let mut files = Files::new();
    files.insert(
        PathBuf::from("specs/SPEC.md"),
        "---\ntitle: T\nstatus: active\nowners: []\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n"
            .to_string()
            .into(),
    );
    files.insert(PathBuf::from("specs/a/SPEC.md"), spec.to_string().into());
    let (graph, violations) = load_files(&files);
    assert!(violations.is_empty(), "{violations:?}");
    let reqs = &graph.nodes["a"].requirements;
    assert_eq!(
        reqs.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["a/a"]
    );
    assert!(reqs[0].text.contains("### Requirement: fake"));
}
