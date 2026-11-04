//! The signals that a requirement is fighting its implementation, and its score.

mod common;

use std::path::Path;

use common::{repo, report, report_with, section, settings, spec, write};

/// `backlog/<slug>.md`, a `workaround` flag on `id` at its current version.
fn flag(root: &Path, slug: &str, id: &str, title: &str) {
    let (graph, _) = clm_model::load(root);
    let version = graph.requirement(id).unwrap().version().to_string();
    let node = id.split_once('/').unwrap().0;
    write(
        root,
        &format!("backlog/{slug}.md"),
        &format!(
            "---\ntitle: \"{title}\"\nkind: workaround\ncomponents: [{node}]\nrequirement: {id}@{version}\n---\n\nWhy.\n"
        ),
    );
}

/// An archived, done change whose delta for `a` modifies `slugs`.
fn archived(root: &std::path::Path, id: &str, slugs: &[&str]) {
    write(
        root,
        &format!("changes/archive/{id}/change.md"),
        "---\nstatus: done\n---\n",
    );
    let mut delta = String::from("---\ncomponent: a\n---\n## MODIFIED Requirements\n");
    for slug in slugs {
        delta.push_str(&format!(
            "\n### Requirement: {slug} <!-- base: 0123456789abcdef -->\n{slug}.\n\n#### Scenario: s\n- ok\n"
        ));
    }
    write(root, &format!("changes/archive/{id}/deltas/a.md"), &delta);
}

#[test]
fn scenario_workaround_and_rewrites() {
    // req:check.health/requirement-signals@55b8d0c4
    let dir = repo();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &spec("[]", &["x", "y"]));
    archived(d, "20260101-p", &["x"]);
    archived(d, "20260102-q", &["x"]);
    archived(d, "20260103-r", &["y"]);
    flag(d, "workaround-a-x-1", "a/x", "Faked the clock");
    let out = report(d);
    assert_eq!(
        section(&out, "workaround"),
        [
            "health/workaround: 1 flags",
            "backlog/workaround-a-x-1.md: health/workaround: a/x Faked the clock",
        ],
        "{out}"
    );
    assert_eq!(
        section(&out, "rewritten"),
        [
            "health/rewritten: 1 requirements",
            "specs/a/SPEC.md: health/rewritten: a/x modified by 2 archived changes",
        ],
        "{out}"
    );
}

#[test]
fn scenario_scenarios_and_exceptions() {
    // req:check.health/requirement-signals@55b8d0c4
    let dir = repo();
    let d = dir.path();
    let a = "---\ntitle: N\nstatus: active\nowners: []\npaths: []\n---\n\n## Requirements\n\n\
### Requirement: x\nx.\n\n#### Scenario: 1\n- ok\n\n#### Scenario: 2\n- ok\n\n#### Scenario: 3\n- ok\n\n\
### Requirement: y\nIt reads it, unless the file is missing, except on Windows; an exception.\n\n\
#### Scenario: s\n- unless this, unless that\n";
    write(d, "specs/a/SPEC.md", a);
    let tight = clm_health::Settings {
        scenarios: 2,
        exceptions: 1,
        ..settings(12000, 40000)
    };
    let out = report_with(d, &tight);
    assert_eq!(
        section(&out, "many-scenarios"),
        [
            "health/many-scenarios: 1 requirements",
            "specs/a/SPEC.md: health/many-scenarios: a/x 3 scenarios",
        ],
        "{out}"
    );
    assert_eq!(
        section(&out, "many-exceptions"),
        [
            "health/many-exceptions: 1 requirements",
            "specs/a/SPEC.md: health/many-exceptions: a/y 2 exception clauses",
        ],
        "{out}"
    );
}

/// `a/x` with two flags and one archived rewrite, `a/y` with one flag, and `a/z` with no
/// flag, five archived rewrites and eight scenarios.
fn scored() -> tempfile::TempDir {
    let dir = repo();
    let d = dir.path();
    let mut a = spec("[]", &["x", "y"]).replace("\ny.\n", "\ny, unless it isn't.\n");
    a.push_str("\n### Requirement: z\nz.\n");
    for n in 1..=8 {
        a.push_str(&format!("\n#### Scenario: {n}\n- ok\n"));
    }
    write(d, "specs/a/SPEC.md", &a);
    archived(d, "20260101-p", &["x", "z"]);
    for id in ["20260102-q", "20260103-r", "20260104-s", "20260105-t"] {
        archived(d, id, &["z"]);
    }
    flag(d, "workaround-a-x-1", "a/x", "Faked the clock");
    flag(d, "workaround-a-x-2", "a/x", "Workaround");
    flag(d, "workaround-a-y-1", "a/y", "Hard");
    dir
}

#[test]
fn scenario_at_the_threshold() {
    // req:check.health/reconsider-score@3d4ea3d7
    let dir = scored();
    let out = report(dir.path());
    assert_eq!(
        section(&out, "reconsider"),
        [
            "health/reconsider: 1 requirements",
            "specs/a/SPEC.md: health/reconsider: a/x score 3 (workarounds 2, rewritten 1, scenarios 0, exceptions 0); not filed",
        ],
        "{out}"
    );
}

#[test]
fn scenario_rewrites_without_a_flag() {
    // req:check.health/reconsider-score@3d4ea3d7
    let dir = scored();
    let out = report(dir.path());
    assert!(
        out.contains("specs/a/SPEC.md: health/rewritten: a/z modified by 5 archived changes\n"),
        "{out}"
    );
    assert!(
        out.contains("specs/a/SPEC.md: health/many-scenarios: a/z 8 scenarios\n"),
        "{out}"
    );
    assert!(
        !section(&out, "reconsider")
            .iter()
            .any(|l| l.contains("a/z")),
        "{out}"
    );
}

#[test]
fn scenario_already_filed() {
    // req:check.health/reconsider-score@3d4ea3d7
    let dir = scored();
    let d = dir.path();
    write(
        d,
        "backlog/reconsider-a-x.md",
        "---\ntitle: \"Reconsider a/x\"\nkind: question\ncomponents: [a]\n---\n",
    );
    let out = report(d);
    let lines = section(&out, "reconsider");
    assert!(
        lines[1].ends_with("; filed as backlog/reconsider-a-x.md"),
        "{out}"
    );
}

#[test]
fn a_score_counts_scenarios_and_exceptions_once_flagged() {
    // req:check.health/reconsider-score@3d4ea3d7
    let dir = scored();
    let d = dir.path();
    let (graph, _) = clm_model::load(d);
    let tight = clm_health::Settings {
        scenarios: 0,
        exceptions: 0,
        ..settings(12000, 40000)
    };
    let signals = clm_health::signals(&graph, &graph.backlog, &tight);
    let score = |id: &str| signals.iter().find(|s| s.id == id).unwrap().score;
    // `a/y`: one flag, one scenario over 0, one exception clause over 0.
    assert_eq!(score("a/y"), 3);
    // `a/z`: no flag, whatever else.
    assert_eq!(score("a/z"), 0);
    assert_eq!(score("a/x"), 4);
}

#[test]
fn scenario_library_returns_the_item() {
    // req:check.health/reconsider-score@3d4ea3d7
    // req:check.health/says-component@15cdb4fa
    let dir = scored();
    let d = dir.path();
    let (graph, _) = clm_model::load(d);
    let items = clm_health::reconsider(&graph, &graph.backlog, &settings(12000, 40000), "a");
    assert_eq!(items.len(), 1, "{items:?}");
    assert_eq!(items[0].id, "a/x");
    let text = &items[0].text;
    for want in [
        "title: \"Reconsider a/x\"",
        "kind: question",
        "components: [a]",
        "backlog/workaround-a-x-1.md",
        "backlog/workaround-a-x-2.md",
        "20260101-p",
        "scores 3",
        "workarounds 2, rewritten 1, scenarios 0, exceptions 0",
        "Scenarios: 1 (setting 6). Exception clauses: 0 (setting 4).",
        "drop the requirement, change it, or redesign around it",
    ] {
        assert!(text.contains(want), "{want} not in {text}");
    }
    assert!(
        text.contains(
            "Deleting this item without one brings it back on the component's next build.\n"
        ),
        "{text}"
    );
    assert!(!text.contains("split item"), "{text}");
    // The item `clm sync` files loads as a question with no violation.
    write(d, "backlog/reconsider-a-x.md", text);
    let (graph, violations) = clm_model::load(d);
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(
        graph.backlog["reconsider-a-x"].frontmatter.kind,
        clm_model::BacklogKind::Question
    );
    // Another node's requirements are never returned.
    assert!(
        clm_health::reconsider(&graph, &graph.backlog, &settings(12000, 40000), "b").is_empty()
    );
}

/// `CAPABILITIES.md` holding an entry per slug.
fn capabilities(root: &Path, slugs: &[&str]) {
    let mut text = String::from("# Capabilities\n\n## Capabilities\n");
    for slug in slugs {
        text.push_str(&format!(
            "\n### Capability: {slug}\nA person can {slug}.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n"
        ));
    }
    write(root, "CAPABILITIES.md", &text);
}

#[test]
fn unserved_entry_reported() {
    // req:check.health/unserved-entries@de8f199d
    let dir = repo();
    let d = dir.path();
    capabilities(d, &["c", "d"]);
    let a = spec("[]", &["x"]).replace("owners: []\n", "owners: []\nserves: [capability/c]\n");
    write(d, "specs/a/SPEC.md", &a);
    let out = report(d);
    assert_eq!(
        section(&out, "unserved"),
        [
            "health/unserved: 1 entries",
            "CAPABILITIES.md: health/unserved: capability/d served by no rule",
        ],
        "{out}"
    );
}

#[test]
fn served_by_rule_block_is_not_unserved() {
    // req:check.health/unserved-entries@de8f199d
    let dir = repo();
    let d = dir.path();
    capabilities(d, &["c"]);
    let a = spec("[]", &["x"]).replace(
        "#### Scenario: s\n- ok\n",
        "#### Scenario: s\n- ok\n\n#### Serves\n- capability/c\n",
    );
    write(d, "specs/a/SPEC.md", &a);
    let out = report(d);
    assert_eq!(
        section(&out, "unserved"),
        ["health/unserved: 0 entries"],
        "{out}"
    );
}
