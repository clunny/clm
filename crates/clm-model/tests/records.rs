use std::fs;
use std::path::Path;

use clm_model::{BacklogKind, Priority, RequirementAt, Violation, load};
use tempfile::TempDir;

const NODE: &str = "---\ntitle: T\nstatus: active\nowners: []\n---\n";

fn repo(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    let mut all = vec![("specs/SPEC.md", NODE), ("specs/sync/SPEC.md", NODE)];
    all.extend_from_slice(files);
    for (rel, content) in all {
        let path = dir.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    dir
}

fn rules(v: &[Violation]) -> Vec<&str> {
    v.iter().map(|v| v.rule).collect()
}

const DONE: &str = "---\nstatus: done\nfrom: backlog/idea\n---\n";

#[test]
fn archived_changes_load_and_are_flagged() {
    // req:model/parse-changes@d4cfec66
    let dir = repo(&[
        ("changes/archive/x/change.md", DONE),
        ("changes/y/change.md", "---\nstatus: proposed\n---\n"),
    ]);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert!(graph.changes["x"].archived);
    assert_eq!(
        graph.changes["x"].frontmatter.from.as_deref(),
        Some("backlog/idea")
    );
    assert!(!graph.changes["y"].archived);
    assert!(!graph.changes.contains_key("archive"));
}

#[test]
fn backlog_items_are_parsed() {
    // req:model/parse-backlog@fa4b09c8
    let dir = repo(&[
        ("changes/archive/x/change.md", DONE),
        (
            "backlog/thing.md",
            "---\ntitle: A thing\nkind: deferred\ncomponents: [sync]\norigin: x\n---\n\nBody.\n",
        ),
    ]);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    let item = &graph.backlog["thing"];
    assert_eq!(item.frontmatter.kind, BacklogKind::Deferred);
    assert_eq!(item.path, Path::new("backlog/thing.md"));
}

#[test]
fn human_step() {
    // req:model/parse-backlog@fa4b09c8
    let dir = repo(&[(
        "backlog/x.md",
        "---\ntitle: Set a secret\nkind: human-task\n---\n",
    )]);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    let item = &graph.backlog["x"];
    assert_eq!(item.frontmatter.kind, BacklogKind::Human);
    assert_eq!(item.to_string(), "x human-task P5 [] Set a secret");
}

#[test]
fn item_priority() {
    // req:model/parse-backlog@fa4b09c8
    let dir = repo(&[
        (
            "backlog/x.md",
            "---\ntitle: X\nkind: idea\npriority: P1\n---\n",
        ),
        ("backlog/y.md", "---\ntitle: Y\nkind: idea\n---\n"),
        (
            "backlog/z.md",
            "---\ntitle: Z\nkind: idea\npriority: high\n---\n",
        ),
        (
            "backlog/n.md",
            "---\ntitle: N\nkind: idea\npriority: 1\n---\n",
        ),
    ]);
    let (graph, violations) = load(dir.path());
    assert_eq!(graph.backlog["x"].frontmatter.priority, Priority::P1);
    assert_eq!(graph.backlog["y"].frontmatter.priority, Priority::P5);
    assert!(!graph.backlog.contains_key("z") && !graph.backlog.contains_key("n"));
    assert_eq!(rules(&violations), ["model/parse-backlog"; 2]);
    for (path, value) in [("backlog/z.md", "high"), ("backlog/n.md", "1")] {
        let found = violations
            .iter()
            .find(|v| v.path == Path::new(path))
            .unwrap_or_else(|| panic!("{path} not reported: {violations:?}"));
        assert!(found.message.contains(value), "{violations:?}");
    }
}

const ITEM_N: &str = "---\ntitle: X\nkind: idea\ncomponents: [n]\n---\n";
const CREATE_N: &str = "---\ncomponent: n\ncreate: true\n---\n## Frontmatter\n```yaml\ntitle: N\nstatus: specd\nowners: []\n```\n\n## Contract\nN.\n";
const PROPOSED: &str = "---\nstatus: proposed\n---\n";

/// The `model/parse-backlog` violations on `backlog/x.md`.
fn item_violations(v: &[Violation]) -> Vec<&Violation> {
    v.iter()
        .filter(|v| v.rule == "model/parse-backlog" && v.path == Path::new("backlog/x.md"))
        .collect()
}

#[test]
fn backlog_may_name_node_a_live_change_creates() {
    // req:model/parse-backlog@fa4b09c8
    let dir = repo(&[
        ("backlog/x.md", ITEM_N),
        ("changes/c/change.md", PROPOSED),
        ("changes/c/deltas/n.md", CREATE_N),
    ]);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.backlog["x"].frontmatter.nodes, ["n"]);
}

#[test]
fn backlog_may_not_name_node_an_archived_change_created() {
    // req:model/parse-backlog@fa4b09c8
    let dir = repo(&[
        ("backlog/x.md", ITEM_N),
        ("changes/archive/c/change.md", DONE),
        ("changes/archive/c/deltas/n.md", CREATE_N),
    ]);
    let (_, violations) = load(dir.path());
    let named = item_violations(&violations);
    assert_eq!(named.len(), 1, "{violations:?}");
    assert!(named[0].message.contains("`n`"), "{violations:?}");
}

#[test]
fn backlog_may_not_name_node_a_live_change_only_modifies() {
    // req:model/parse-backlog@fa4b09c8
    // Only a `create: true` delta creates a node; this guards that filter.
    let dir = repo(&[
        ("backlog/x.md", ITEM_N),
        ("changes/c/change.md", PROPOSED),
        (
            "changes/c/deltas/n.md",
            "---\ncomponent: n\n---\n## MODIFIED Contract\nN.\n",
        ),
    ]);
    let (_, violations) = load(dir.path());
    let named = item_violations(&violations);
    assert_eq!(named.len(), 1, "{violations:?}");
    assert!(named[0].message.contains("`n`"), "{violations:?}");
}

#[test]
fn backlog_node_known_answers() {
    // req:model/backlog-components@5d7d1e8f
    let create = |n: &str| CREATE_N.replace("component: n", &format!("component: {n}"));
    let dir = repo(&[
        ("specs/a/SPEC.md", NODE),
        ("changes/c/change.md", PROPOSED),
        ("changes/c/deltas/b.md", &create("b")),
        ("changes/archive/d/change.md", DONE),
        ("changes/archive/d/deltas/c.md", &create("c")),
    ]);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    for (id, known) in [("a", true), ("b", true), ("c", false), ("d", false)] {
        assert_eq!(graph.backlog_node_known(id), known, "{id}");
    }
    // Loading judges an item's `nodes` by the same rule.
    let item = "---\ntitle: X\nkind: idea\ncomponents: [a, b, c, d]\n---\n";
    let dir = repo(&[
        ("specs/a/SPEC.md", NODE),
        ("changes/c/change.md", PROPOSED),
        ("changes/c/deltas/b.md", &create("b")),
        ("changes/archive/d/change.md", DONE),
        ("changes/archive/d/deltas/c.md", &create("c")),
        ("backlog/x.md", item),
    ]);
    let (_, violations) = load(dir.path());
    let named = item_violations(&violations);
    assert_eq!(named.len(), 2, "{violations:?}");
    assert!(named[0].message.contains("`c`"), "{violations:?}");
    assert!(named[1].message.contains("`d`"), "{violations:?}");
}

const WITH_REQS: &str = "---\ntitle: A\nstatus: active\nowners: []\n---\n## Requirements\n\n### Requirement: x\nX.\n#### Scenario: s\n- y\n\n### Requirement: y\nY.\n#### Scenario: s\n- y\n";

#[test]
fn blocks_name_existing_requirements() {
    // req:model/parse-backlog@fa4b09c8
    let dir = repo(&[
        ("specs/a/SPEC.md", WITH_REQS),
        (
            "backlog/x.md",
            "---\ntitle: Q\nkind: question\nblocks: [a/x, a/y]\n---\n",
        ),
        (
            "backlog/z.md",
            "---\ntitle: Q\nkind: question\nblocks: [a/ghost, ghost/x, x]\n---\n",
        ),
    ]);
    let (graph, violations) = load(dir.path());
    assert_eq!(graph.backlog["x"].frontmatter.blocks, ["a/x", "a/y"]);
    assert_eq!(rules(&violations), ["model/parse-backlog"; 3]);
    for (v, id) in violations.iter().zip(["`a/ghost`", "`ghost/x`", "`x`"]) {
        assert_eq!(v.path, Path::new("backlog/z.md"));
        assert!(v.message.contains(id), "{violations:?}");
    }
}

const ADDS_NEW: &str = "---\ncomponent: a\n---\n## ADDED Requirements\n\n### Requirement: new\nN.\n#### Scenario: s\n- y\n";

fn blocking(id: &str) -> String {
    format!("---\ntitle: Q\nkind: question\nblocks: [{id}]\n---\n")
}

#[test]
fn blocks_a_requirement_a_live_change_adds() {
    // req:model/parse-backlog@fa4b09c8
    let dir = repo(&[
        ("specs/a/SPEC.md", WITH_REQS),
        ("backlog/x.md", &blocking("a/new")),
        ("changes/c/change.md", PROPOSED),
        ("changes/c/deltas/a.md", ADDS_NEW),
    ]);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.backlog["x"].frontmatter.blocks, ["a/new"]);
}

#[test]
fn blocks_a_requirement_of_a_node_a_live_change_creates() {
    // req:model/parse-backlog@fa4b09c8
    let create = format!(
        "{CREATE_N}\n## ADDED Requirements\n\n### Requirement: y\nY.\n#### Scenario: s\n- y\n"
    );
    let dir = repo(&[
        ("backlog/x.md", &blocking("n/y")),
        ("changes/c/change.md", PROPOSED),
        ("changes/c/deltas/n.md", &create),
    ]);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.backlog["x"].frontmatter.blocks, ["n/y"]);
}

#[test]
fn blocks_a_requirement_a_change_adds_whatever_its_status() {
    // req:model/parse-backlog@fa4b09c8
    let abandoned = "---\nstatus: abandoned\n---\n";
    let dir = repo(&[
        ("specs/a/SPEC.md", WITH_REQS),
        ("backlog/x.md", &blocking("a/new")),
        ("changes/c/change.md", abandoned),
        ("changes/c/deltas/a.md", ADDS_NEW),
    ]);
    let (_, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
}

/// Asserts the only `model/parse-backlog` violation is `backlog/x.md`'s, naming `id`.
fn reports_block(violations: &[Violation], id: &str) {
    let named = item_violations(violations);
    assert_eq!(named.len(), 1, "{violations:?}");
    assert!(
        named[0].message.contains(&format!("`{id}`")),
        "{violations:?}"
    );
}

#[test]
fn blocks_a_requirement_only_an_archived_change_added() {
    // req:model/parse-backlog@fa4b09c8
    let dir = repo(&[
        ("specs/a/SPEC.md", WITH_REQS),
        ("backlog/x.md", &blocking("a/old")),
        ("changes/archive/c/change.md", DONE),
        (
            "changes/archive/c/deltas/a.md",
            &ADDS_NEW.replace(": new", ": old"),
        ),
    ]);
    let (_, violations) = load(dir.path());
    reports_block(&violations, "a/old");
}

#[test]
fn blocks_a_requirement_a_change_only_modifies() {
    // req:model/parse-backlog@fa4b09c8
    let modifies = ADDS_NEW
        .replace("ADDED", "MODIFIED")
        .replace(": new", ": ghost <!-- base: 0123456789abcdef -->");
    let dir = repo(&[
        ("specs/a/SPEC.md", WITH_REQS),
        ("backlog/x.md", &blocking("a/ghost")),
        ("changes/c/change.md", PROPOSED),
        ("changes/c/deltas/a.md", &modifies),
    ]);
    let (graph, violations) = load(dir.path());
    assert_eq!(graph.changes["c"].deltas.len(), 1, "{violations:?}");
    reports_block(&violations, "a/ghost");
}

#[test]
fn blocks_a_requirement_another_node_s_delta_adds() {
    // req:model/parse-backlog@fa4b09c8
    let dir = repo(&[
        ("specs/a/SPEC.md", WITH_REQS),
        ("specs/b/SPEC.md", NODE),
        ("backlog/x.md", &blocking("a/new")),
        ("changes/c/change.md", PROPOSED),
        (
            "changes/c/deltas/b.md",
            &ADDS_NEW.replace("component: a", "component: b"),
        ),
    ]);
    let (graph, violations) = load(dir.path());
    assert_eq!(graph.changes["c"].deltas.len(), 1, "{violations:?}");
    reports_block(&violations, "a/new");
}

#[test]
fn blocks_a_requirement_a_rejected_delta_adds() {
    // req:model/parse-backlog@fa4b09c8
    // A delta that fails to load adds nothing.
    let rejected = format!("{ADDS_NEW}\n## MODIFIED Design\n### Requirement: z\n");
    let dir = repo(&[
        ("specs/a/SPEC.md", WITH_REQS),
        ("backlog/x.md", &blocking("a/new")),
        ("changes/c/change.md", PROPOSED),
        ("changes/c/deltas/a.md", &rejected),
    ]);
    let (graph, violations) = load(dir.path());
    assert!(graph.changes["c"].deltas.is_empty());
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "model/misplaced-delta-text"),
        "{violations:?}"
    );
    reports_block(&violations, "a/new");
}

/// A workaround flag on `requirement`.
fn flag(requirement: &str) -> String {
    format!("---\ntitle: Hard to build\nkind: workaround\nrequirement: {requirement}\n---\n")
}

/// `a/x`'s version in `WITH_REQS`.
fn version_of_a_x() -> String {
    let dir = repo(&[("specs/a/SPEC.md", WITH_REQS)]);
    let (graph, _) = load(dir.path());
    graph.requirement("a/x").unwrap().version().to_string()
}

#[test]
fn workaround_flag() {
    // req:model/parse-backlog@fa4b09c8
    let version = version_of_a_x();
    let dir = repo(&[
        ("specs/a/SPEC.md", WITH_REQS),
        (
            "backlog/workaround-a-x-1.md",
            &flag(&format!("a/x@{version}")),
        ),
    ]);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    let fm = &graph.backlog["workaround-a-x-1"].frontmatter;
    assert_eq!(fm.kind, BacklogKind::Friction);
    assert_eq!(fm.kind.to_string(), "workaround");
    let at = RequirementAt {
        id: "a/x".into(),
        version,
    };
    assert_eq!(fm.requirement, Some(at));
}

/// Asserts the only violation is `model/parse-backlog`'s on `path`, naming each of `named`.
fn reports_item(violations: &[Violation], path: &str, named: &[&str]) {
    assert_eq!(rules(violations), ["model/parse-backlog"], "{violations:?}");
    assert_eq!(violations[0].path, Path::new(path));
    for name in named {
        assert!(
            violations[0].message.contains(name),
            "{name}: {violations:?}"
        );
    }
}

#[test]
fn flag_at_an_old_version() {
    // req:model/parse-backlog@fa4b09c8
    let version = version_of_a_x();
    assert_ne!(version, "00000000");
    let dir = repo(&[
        ("specs/a/SPEC.md", WITH_REQS),
        ("backlog/workaround-a-x-1.md", &flag("a/x@00000000")),
    ]);
    let (_, violations) = load(dir.path());
    reports_item(
        &violations,
        "backlog/workaround-a-x-1.md",
        &["`a/x`", &version],
    );
}

#[test]
fn flag_on_a_removed_requirement() {
    // req:model/parse-backlog@fa4b09c8
    let dir = repo(&[
        ("specs/a/SPEC.md", NODE),
        ("backlog/workaround-a-x-1.md", &flag("a/x@3fa2c9d1")),
    ]);
    let (_, violations) = load(dir.path());
    reports_item(&violations, "backlog/workaround-a-x-1.md", &["`a/x`"]);
}

#[test]
fn flag_without_a_requirement() {
    // req:model/parse-backlog@fa4b09c8
    let dir = repo(&[("backlog/f.md", "---\ntitle: F\nkind: workaround\n---\n")]);
    let (graph, violations) = load(dir.path());
    reports_item(&violations, "backlog/f.md", &["requirement"]);
    assert!(
        graph.backlog.contains_key("f"),
        "kept loaded, as for a bad reference"
    );
}

#[test]
fn requirement_on_another_kind() {
    // req:model/parse-backlog@fa4b09c8
    let dir = repo(&[
        ("specs/a/SPEC.md", WITH_REQS),
        (
            "backlog/i.md",
            "---\ntitle: I\nkind: idea\nrequirement: a/x@3fa2c9d1\n---\n",
        ),
    ]);
    let (graph, violations) = load(dir.path());
    reports_item(&violations, "backlog/i.md", &["requirement"]);
    assert!(
        graph.backlog.contains_key("i"),
        "kept loaded, so its `blocks` hold"
    );
}

#[test]
fn malformed_flag_requirement() {
    // req:model/parse-backlog@fa4b09c8
    for bad in [
        "a/x",
        "a/x@",
        "ax@3fa2c9d1",
        "a/x@3fa2",
        "a/x@3FA2C9D1",
        "a/x/y@3fa2c9d1",
        "a/ x@3fa2c9d1",
    ] {
        let dir = repo(&[("specs/a/SPEC.md", WITH_REQS), ("backlog/f.md", &flag(bad))]);
        let (_, violations) = load(dir.path());
        reports_item(&violations, "backlog/f.md", &["requirement"]);
    }
}

#[test]
fn backlog_rejects_unknown_node_origin_kind_and_keys() {
    // req:model/parse-backlog@fa4b09c8
    for (fm, named) in [
        ("title: X\nkind: idea\ncomponents: [ghost]", "ghost"),
        ("title: X\nkind: idea\ncomponents: [ghost]", "ghost"),
        ("title: X\nkind: idea\norigin: nowhere", "nowhere"),
        ("title: X\nkind: wish", "wish"),
        ("title: X\nkind: idea\npriority: high", "priority"),
    ] {
        let text = format!("---\n{fm}\n---\n");
        let dir = repo(&[("backlog/x.md", &text)]);
        let (_, violations) = load(dir.path());
        assert_eq!(rules(&violations), ["model/parse-backlog"], "{fm}");
        assert_eq!(violations[0].path, Path::new("backlog/x.md"));
        assert!(violations[0].message.contains(named), "{violations:?}");
    }
}

#[test]
fn decisions_are_parsed_and_supersession_checked() {
    // req:model/parse-decisions@36d68fa6
    let ok = repo(&[
        (
            "docs/decisions/old.md",
            "---\ntitle: Old\ndate: 2026-10-21\nstatus: superseded\nsuperseded_by: new\n---\n",
        ),
        (
            "docs/decisions/new.md",
            "---\ntitle: New\ndate: 2026-10-22\nstatus: accepted\n---\n",
        ),
    ]);
    let (graph, violations) = load(ok.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.decisions["new"].frontmatter.date, "2026-10-22");

    for fm in [
        "status: superseded\nsuperseded_by: ghost",
        "status: superseded",
        "status: accepted\nsuperseded_by: new",
    ] {
        let text = format!("---\ntitle: D\ndate: 2026-10-21\n{fm}\n---\n");
        let dir = repo(&[
            ("docs/decisions/d.md", &text),
            (
                "docs/decisions/new.md",
                "---\ntitle: New\ndate: 2026-10-22\nstatus: accepted\n---\n",
            ),
        ]);
        let (_, violations) = load(dir.path());
        assert_eq!(rules(&violations), ["model/parse-decisions"], "{fm}");
        assert_eq!(violations[0].path, Path::new("docs/decisions/d.md"));
    }
}

#[test]
fn taken_in_from_issues() {
    // req:model/parse-backlog@fa4b09c8
    let dir = repo(&[(
        "backlog/x.md",
        "---\ntitle: T\nkind: bug\nissues: [73, 91]\n---\n",
    )]);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.backlog["x"].frontmatter.issues, vec![73, 91]);
}

#[test]
fn an_issue_that_isnt_a_number() {
    // req:model/parse-backlog@fa4b09c8
    for bad in ["[\"#73\"]", "[0]", "[-3]", "[1.5]"] {
        let text = format!("---\ntitle: T\nkind: bug\nissues: {bad}\n---\n");
        let dir = repo(&[("backlog/x.md", &text)]);
        let (_, violations) = load(dir.path());
        assert!(
            violations
                .iter()
                .any(|v| v.path == Path::new("backlog/x.md") && v.message.contains("issues")),
            "{bad}: {violations:?}"
        );
    }
}

#[test]
fn taken_in_from_alerts() {
    // req:model/parse-backlog@fa4b09c8
    let dir = repo(&[(
        "backlog/x.md",
        "---\ntitle: T\nkind: bug\nalerts: [4]\n---\n",
    )]);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.backlog["x"].frontmatter.alerts, vec![4]);
    assert!(graph.backlog["x"].frontmatter.issues.is_empty());

    let dir = repo(&[(
        "backlog/y.md",
        "---\ntitle: T\nkind: bug\nissues: [7]\nalerts: [4]\n---\n",
    )]);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.backlog["y"].frontmatter.issues, vec![7]);
    assert_eq!(graph.backlog["y"].frontmatter.alerts, vec![4]);
}

#[test]
fn taken_in_from_a_red_run() {
    // req:model/parse-backlog@fa4b09c8
    let commit = "0123456789abcdef0123456789abcdef01234567";
    let text = format!("---\ntitle: T\nkind: bug\nverify_failures: [\"{commit}\"]\n---\n");
    let dir = repo(&[("backlog/x.md", &text)]);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.backlog["x"].frontmatter.verify_failures, vec![commit]);

    let dir = repo(&[("backlog/y.md", "---\ntitle: T\nkind: bug\n---\n")]);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert!(graph.backlog["y"].frontmatter.verify_failures.is_empty());
}

#[test]
fn a_short_or_upper_case_commit_is_refused() {
    // req:model/parse-backlog@fa4b09c8
    for bad in [
        "\"abc123\"",
        "\"\"",
        "\"0123456789ABCDEF0123456789ABCDEF01234567\"",
        "\"0123456789abcdef0123456789abcdef0123456g\"",
        "\"0123456789abcdef0123456789abcdef012345678\"",
        "123",
    ] {
        let text = format!("---\ntitle: T\nkind: bug\nverify_failures: [{bad}]\n---\n");
        let dir = repo(&[("backlog/x.md", &text)]);
        let (_, violations) = load(dir.path());
        assert!(
            violations
                .iter()
                .any(|v| v.path == Path::new("backlog/x.md")
                    && v.message.contains("verify_failures")),
            "{bad}: {violations:?}"
        );
    }
}

#[test]
fn an_alert_that_isnt_a_number() {
    // req:model/parse-backlog@fa4b09c8
    for bad in ["[0]", "[\"#4\"]", "[-1]", "[1.5]"] {
        let text = format!("---\ntitle: T\nkind: bug\nalerts: {bad}\n---\n");
        let dir = repo(&[("backlog/x.md", &text)]);
        let (_, violations) = load(dir.path());
        assert!(
            violations
                .iter()
                .any(|v| v.path == Path::new("backlog/x.md") && v.message.contains("alerts")),
            "{bad}: {violations:?}"
        );
    }
}

#[test]
fn a_deleted_items_components_are_gone() {
    // req:model/parse-backlog-item@ac29a1d0
    let item = clm_model::parse_backlog_item(
        "x",
        "---\ntitle: T\nkind: bug\ncomponents: [ghost]\nissues: [67]\n---\n\nBody\n",
    )
    .unwrap();
    assert_eq!(item.slug, "x");
    assert_eq!(item.frontmatter.issues, vec![67]);
    assert_eq!(item.frontmatter.nodes, vec!["ghost".to_string()]);
}

#[test]
fn parse_item_reads_the_old_key_and_defaults_issues() {
    // req:model/parse-backlog-item@ac29a1d0
    let item = clm_model::parse_backlog_item("x", "---\ntitle: T\nkind: bug\nnodes: [old]\n---\n")
        .unwrap();
    assert_eq!(item.frontmatter.nodes, vec!["old".to_string()]);
    assert!(item.frontmatter.issues.is_empty());
}

#[test]
fn parse_item_unknown_key() {
    // req:model/parse-backlog-item@ac29a1d0
    let violations =
        clm_model::parse_backlog_item("x", "---\ntitle: T\nkind: bug\ncolour: red\n---\n")
            .unwrap_err();
    assert!(violations.iter().any(|v| v.message.contains("colour")));
}

#[test]
fn parse_item_reports_a_workaround_without_requirement() {
    // req:model/parse-backlog-item@ac29a1d0
    let violations =
        clm_model::parse_backlog_item("x", "---\ntitle: T\nkind: workaround\n---\n").unwrap_err();
    assert!(violations.iter().any(|v| v.message.contains("requirement")));
}
