mod common;

use common::{git, repo, report, section, spec, write};

fn decision(status: &str, body: &str) -> String {
    format!("---\ntitle: D\ndate: 2026-01-01\nstatus: {status}\n---\n{body}\n")
}

#[test]
fn scenario_cited_and_uncited() {
    // req:check.health/uncited-decisions@43e37f91
    let dir = repo();
    let d = dir.path();
    let mut a = spec("[]", &["x"]);
    a.push_str("\nWe keep `kept` here.\n");
    write(d, "specs/a/SPEC.md", &a);
    write(d, "docs/decisions/kept.md", &decision("accepted", "Kept."));
    write(d, "docs/decisions/lone.md", &decision("accepted", "Alone."));
    write(d, "docs/decisions/old.md", &decision("superseded", "Old."));
    let out = report(d);
    assert_eq!(
        section(&out, "uncited-decision"),
        [
            "health/uncited-decision: 1 decisions",
            "docs/decisions/lone.md: health/uncited-decision: nothing names it",
        ],
        "{out}"
    );
}

#[test]
fn scenario_named_only_in_a_fence() {
    // req:check.health/uncited-decisions@43e37f91
    let dir = repo();
    let d = dir.path();
    let mut a = spec("[]", &["x"]);
    a.push_str("\n```\nsee `fenced`\n```\n");
    write(d, "specs/a/SPEC.md", &a);
    write(d, "docs/decisions/fenced.md", &decision("accepted", "F."));
    let out = report(d);
    assert!(
        out.contains("docs/decisions/fenced.md: health/uncited-decision: nothing names it\n"),
        "{out}"
    );
}

#[test]
fn every_other_source_cites_a_decision() {
    // req:check.health/uncited-decisions@43e37f91
    let dir = repo();
    let d = dir.path();
    write(d, "ARCHITECTURE.md", "Invariant `by-arch`.\n");
    write(d, "CAPABILITIES.md", "## Capabilities\n\nSee `by-caps`.\n");
    write(d, "CLAUDE.md", "Rule `by-claude`.\n");
    write(d, "docs/guide/page.md", "See `by-guide`.\n");
    write(
        d,
        "changes/20260101-c/change.md",
        "---\nstatus: proposed\n---\nWhy: `by-change`.\n",
    );
    write(d, "changes/archive/20250101-z/change.md", "`by-archive`\n");
    for name in [
        "by-arch",
        "by-caps",
        "by-claude",
        "by-guide",
        "by-change",
        "by-archive",
    ] {
        write(
            d,
            &format!("docs/decisions/{name}.md"),
            &decision("accepted", "."),
        );
    }
    write(
        d,
        "docs/decisions/by-other.md",
        &decision("accepted", "Other."),
    );
    write(
        d,
        "docs/decisions/by-self.md",
        &decision("accepted", "Names `by-other` and `by-self`."),
    );
    let out = report(d);
    assert_eq!(
        section(&out, "uncited-decision"),
        [
            "health/uncited-decision: 2 decisions",
            "docs/decisions/by-archive.md: health/uncited-decision: nothing names it",
            "docs/decisions/by-self.md: health/uncited-decision: nothing names it",
        ],
        "{out}"
    );
}

/// A node spec: frontmatter on lines 1 to 6, then `## Contract` on line 8.
fn node(contract: &str, design: &str, requirements: &str) -> String {
    format!(
        "---\ntitle: N\nstatus: active\nowners: []\npaths: []\n---\n\n## Contract\n{contract}## Design\n{design}## Requirements\n{requirements}"
    )
}

const REQ_X: &str = "\n### Requirement: x\nx.\n\n#### Scenario: s\n- on 2026-10-01, ok\n";

#[test]
fn scenario_history_in_a_design() {
    // req:check.health/spec-bloat-signals@6a8806e4
    let dir = repo();
    let d = dir.path();
    // Line 11 is `## Design`, so its third line is 14.
    let design =
        "Design.\n\nThe old path, removed 2026-10-01, no longer runs\n```\nformerly\n```\n";
    write(d, "specs/a/SPEC.md", &node("Contract.\n\n", design, REQ_X));
    let out = report(d);
    assert_eq!(
        section(&out, "history-words"),
        [
            "health/history-words: 1 lines",
            "specs/a/SPEC.md:14: health/history-words: 2026-10-01",
        ],
        "{out}"
    );
}

#[test]
fn scenario_a_present_condition() {
    // req:check.health/spec-bloat-signals@6a8806e4
    let dir = repo();
    let d = dir.path();
    let design = "Design.\n\nA local branch that no longer contains its remote tip is kept\n";
    write(d, "specs/a/SPEC.md", &node("Contract.\n\n", design, REQ_X));
    let out = report(d);
    assert_eq!(
        section(&out, "history-words"),
        ["health/history-words: 0 lines"],
        "{out}"
    );
}

#[test]
fn scenario_a_passive_used_to() {
    // req:check.health/spec-bloat-signals@6a8806e4
    let dir = repo();
    let d = dir.path();
    let design = "Design.\n\nThe claim ref is used to hold the session's place\n";
    write(d, "specs/a/SPEC.md", &node("Contract.\n\n", design, REQ_X));
    let out = report(d);
    assert_eq!(
        section(&out, "history-words"),
        ["health/history-words: 0 lines"],
        "{out}"
    );
}

#[test]
fn history_phrases_match_whole_words_in_any_case() {
    // req:check.health/spec-bloat-signals@6a8806e4
    let dir = repo();
    let d = dir.path();
    let contract = "It Previously ran.\nThe refused run stays.\nToday's build.\n\n";
    write(d, "specs/a/SPEC.md", &node(contract, "Design.\n\n", REQ_X));
    let out = report(d);
    assert_eq!(
        section(&out, "history-words"),
        [
            "health/history-words: 2 lines",
            "specs/a/SPEC.md:9: health/history-words: Previously",
            "specs/a/SPEC.md:11: health/history-words: Today's",
        ],
        "{out}"
    );
}

/// `fill` repeated, then a newline, `bytes` bytes in all.
fn bytes(fill: char, bytes: usize) -> String {
    let mut s: String = std::iter::repeat_n(fill, bytes - 1).collect();
    s.push('\n');
    s
}

#[test]
fn scenario_design_heavier_than_the_rest() {
    // req:check.health/spec-bloat-signals@6a8806e4
    // req:check.health/says-component@15cdb4fa
    let dir = repo();
    let d = dir.path();
    let (head, tail) = ("\n### Requirement: x\n", "\n#### Scenario: s\n- ok\n");
    let requirement = format!("{head}{}{tail}", bytes('r', 300 - head.len() - tail.len()));
    let a = node(&bytes('c', 200), &bytes('d', 2000), &requirement);
    write(d, "specs/a/SPEC.md", &a);
    let out = report(d);
    assert_eq!(
        section(&out, "design-heavy"),
        [
            "health/design-heavy: 1 components",
            "specs/a/SPEC.md: health/design-heavy: design 500 tokens, contract and requirements 125",
        ],
        "{out}"
    );
}

#[test]
fn scenario_outlier() {
    // req:check.health/spec-bloat-signals@6a8806e4
    // req:check.health/says-component@15cdb4fa
    let dir = repo();
    let d = dir.path();
    for (n, size) in [("a", 1000), ("b", 1200), ("c", 1100), ("d", 8000)] {
        write(
            d,
            &format!("specs/{n}/SPEC.md"),
            &spec(&format!("[\"src/{n}/**\"]"), &["x"]),
        );
        write(d, &format!("src/{n}/code.txt"), &"x".repeat(size));
    }
    git(d, &["add", "-A"]);
    let out = report(d);
    assert_eq!(
        section(&out, "outlier-size"),
        [
            "health/outlier-size: 1 components",
            "specs/d/SPEC.md: health/outlier-size: code 2000 tokens per requirement, median 275",
        ],
        "{out}"
    );
}

#[test]
fn fewer_than_three_nodes_give_no_median() {
    // req:check.health/spec-bloat-signals@6a8806e4
    // req:check.health/says-component@15cdb4fa
    let dir = repo();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &spec("[]", &["x"]));
    let out = report(d);
    assert_eq!(
        section(&out, "outlier-size"),
        ["health/outlier-size: no median: fewer than 3 components with requirements"],
        "{out}"
    );
}
