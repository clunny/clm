//! `needs_review_of` (gate/needs-review-of): whether a spec branch's diff needs review.

use clm_gate::needs_review_of;
use clm_model::{ChangeFrontmatter, parse_change_frontmatter};

fn change(frontmatter: &str) -> Option<ChangeFrontmatter> {
    let fm = parse_change_frontmatter(&format!("---\n{frontmatter}---\n\n## Intent\n\nx\n"));
    assert!(fm.is_some(), "fixture doesn't parse: {frontmatter}");
    fm
}

#[test]
fn scenario_a_human_layer() {
    // req:gate/needs-review-of@a4b7f8da
    assert!(needs_review_of(&["ARCHITECTURE.md"], false, &[]));
    assert!(needs_review_of(&["CAPABILITIES.md"], false, &[]));
    assert!(needs_review_of(&["docs/allowed_signers"], false, &[]));
    assert!(needs_review_of(&[], true, &[]));
    // A path under one of them counts; a sibling that only shares a prefix doesn't.
    assert!(needs_review_of(&["docs/allowed_signers/alice"], false, &[]));
    assert!(!needs_review_of(&["docs/allowed_signers.md"], false, &[]));
    assert!(!needs_review_of(&["specs/ARCHITECTURE.md"], false, &[]));
}

#[test]
fn scenario_burdens_and_an_unreadable_change() {
    // req:gate/needs-review-of@a4b7f8da
    let paths = ["changes/x/change.md"];
    let burdened = change("status: proposed\nhuman_burdens: [b]\n");
    assert!(needs_review_of(&paths, false, &[burdened]));
    assert!(needs_review_of(&paths, false, &[None]));
    // One such change among others is enough.
    let plain = change("status: proposed\n");
    assert!(needs_review_of(&paths, false, &[plain, None]));
}

#[test]
fn scenario_nothing_else() {
    // req:gate/needs-review-of@a4b7f8da
    let paths = [
        "specs/a/SPEC.md",
        "docs/decisions/d.md",
        "CLAUDE.md",
        "changes/x/change.md",
    ];
    let plain = change("status: applied\n");
    assert!(!needs_review_of(&paths, false, &[plain]));
    assert!(!needs_review_of(
        &paths,
        false,
        &[change("status: applied\nhuman_burdens: []\n")]
    ));
    assert!(!needs_review_of(&[], false, &[]));
}

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n";
const REQUIREMENTS: &str = "\n### Requirement: x\nX SHALL hold.\n\n#### Scenario: holds\n- ok\n\n### Requirement: y\nY SHALL hold.\n\n#### Scenario: holds\n- ok\n";

/// Component `a` with `exemptions`, `contract` and `requirements`.
fn spec_a(exemptions: &str, contract: &str, requirements: &str) -> String {
    format!(
        "---\ntitle: A\nstatus: unbuilt\nowners: []\npaths: [\"src/a/**\"]\n{exemptions}---\n\n## Contract\n{contract}\n\n## Design\nd\n\n## Requirements\n{requirements}"
    )
}

fn exemption(files: &str) -> String {
    format!("exemptions:\n  - rule: skipped-tests\n    files: [{files}]\n    until: backlog/x\n")
}

fn git(d: &std::path::Path, args: &[&str]) -> String {
    let id = [
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "-c",
        "commit.gpgsign=false",
    ];
    clm_git::git(d, &[&id[..], args].concat()).unwrap()
}

fn write(d: &std::path::Path, rel: &str, text: &str) {
    let path = d.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// The specs of the working tree, loaded with no violation.
fn graph(d: &std::path::Path) -> clm_model::Graph {
    let (graph, violations) = clm_model::load(d);
    assert!(violations.is_empty(), "{violations:?}");
    graph
}

#[test]
fn scenario_the_gate_agrees() {
    // req:gate/needs-review-of@a4b7f8da
    let base = spec_a(&exemption("\"src/a/**\""), "c", REQUIREMENTS);
    let only_x = &REQUIREMENTS[..REQUIREMENTS.find("\n### Requirement: y").unwrap()];
    let widened = spec_a(&exemption("\"src/a/**\", \"src/b/**\""), "c", REQUIREMENTS);
    let removed = spec_a(&exemption("\"src/a/**\""), "c", only_x);
    let contract = spec_a(&exemption("\"src/a/**\""), "changed", REQUIREMENTS);
    let branches: [(&str, &str, bool); 4] = [
        ("ARCHITECTURE.md", "# Architecture\n\nchanged\n", true),
        ("specs/a/SPEC.md", &widened, true),
        ("specs/a/SPEC.md", &removed, true),
        ("specs/a/SPEC.md", &contract, false),
    ];
    for (file, text, needs) in branches {
        let dir = tempfile::TempDir::new().unwrap();
        let d = dir.path();
        git(d, &["init", "-q", "-b", "main"]);
        write(d, "specs/SPEC.md", ROOT);
        write(d, "specs/a/SPEC.md", &base);
        write(d, "ARCHITECTURE.md", "# Architecture\n");
        write(d, "backlog/x.md", "---\ntitle: X\nkind: idea\n---\n");
        git(d, &["add", "-A"]);
        git(d, &["commit", "-qm", "base"]);
        let before = graph(d);
        git(d, &["checkout", "-q", "-b", "pr"]);
        write(d, file, text);
        git(d, &["add", "-A"]);
        git(d, &["commit", "-qm", "pr"]);
        let after = graph(d);

        let paths = git(d, &["diff", "--name-only", "main...HEAD"]);
        let paths: Vec<&str> = paths.lines().collect();
        let by_three = needs_review_of(&paths, false, &[])
            || !clm_gate::widened_exemptions(&before, &after).is_empty()
            || !clm_gate::cuts(&before, &after, &[]).is_empty();
        let out = clm_gate::evaluate_at(d, "main", "HEAD", false).unwrap();
        assert_eq!(out.kind, clm_gate::Kind::Spec, "{file}");
        assert_eq!((out.review, by_three), (needs, needs), "{file}: {text}");
    }
}
