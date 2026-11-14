//! `removed-requirement-names`, through `clm_check::run`.

use std::fs;
use std::path::Path;

use clm_check::run;
use tempfile::TempDir;

const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";
const RULE: &str = "check/removed-requirement-names";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// A node spec whose body (after its 5-line frontmatter and a blank line) is `body`, so
/// the body's first line is line 7.
fn spec(title: &str, body: &str) -> String {
    format!("---\ntitle: {title}\nstatus: active\nowners: []\n---\n\n{body}")
}

/// Node `a` holding requirement `keep`, whose text names `` `gone` `` on line 30.
fn spec_a() -> String {
    let mut body = String::from("## Requirements\n\n### Requirement: keep\n");
    // `### Requirement: keep` is line 9; lines 10 to 29 are filler.
    for i in 10..30 {
        body.push_str(&format!("Filler {i}.\n"));
    }
    body.push_str("Keep replaces `gone`.\n\n#### Scenario: s\n- ok\n");
    spec("A", &body)
}

/// A change in `dir` with `status` whose delta for `a` removes `slug`.
fn removing(root: &Path, dir: &str, status: &str, slug: &str) {
    write(
        root,
        &format!("{dir}/change.md"),
        &format!("---\nstatus: {status}\n---\n"),
    );
    write(
        root,
        &format!("{dir}/deltas/a.md"),
        &format!(
            "---\ncomponent: a\n---\n## REMOVED Requirements\n\n### Requirement: {slug} <!-- base: 0123456789abcdef -->\n"
        ),
    );
}

/// The first scenario's files: node `a` and an applied change `20260101-x` removing `gone`.
fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &spec_a());
    removing(d, "changes/20260101-x", "applied", "gone");
    dir
}

/// Gives node `a` `serves: [<id>]`, as `check/untraced` wants once a layer holds an entry.
fn serve(root: &Path, id: &str) {
    let a = spec_a().replacen("---\n", &format!("---\nserves: [{id}]\n"), 1);
    write(root, "specs/a/SPEC.md", &a);
}

/// Every violation `run` reports, formatted, after checking that only this rule fired,
/// so a broken fixture can't pass a test that expects none.
fn found(root: &Path) -> Vec<String> {
    let all: Vec<String> = run(root).iter().map(clm_check::format).collect();
    assert!(
        all.iter().all(|v| v.contains(&format!(": {RULE}: "))),
        "{all:?}"
    );
    all
}

// req:check/removed-requirement-names@2a727e8a
#[test]
fn requirement_naming_a_removed_sibling_is_reported() {
    let dir = repo();
    assert_eq!(
        found(dir.path()),
        [
            "specs/a/SPEC.md:30: check/removed-requirement-names: names `a/gone`, which 20260101-x removes"
        ]
    );
}

// req:check/removed-requirement-names@2a727e8a
#[test]
fn the_first_removing_change_by_id_is_named() {
    let dir = repo();
    removing(dir.path(), "changes/20250101-w", "applied", "gone");
    assert_eq!(
        found(dir.path()),
        [
            "specs/a/SPEC.md:30: check/removed-requirement-names: names `a/gone`, which 20250101-w removes"
        ]
    );
}

// req:check/removed-requirement-names@2a727e8a
#[test]
fn another_node_naming_the_full_id_is_reported() {
    let dir = repo();
    write(
        dir.path(),
        "specs/b/SPEC.md",
        &spec(
            "B",
            "## Design\n\nIt once used `a/gone`, and `gone` alone.\n",
        ),
    );
    assert_eq!(
        found(dir.path()),
        [
            "specs/a/SPEC.md:30: check/removed-requirement-names: names `a/gone`, which 20260101-x removes",
            "specs/b/SPEC.md:9: check/removed-requirement-names: names `a/gone`, which 20260101-x removes",
        ]
    );
}

// req:check/removed-requirement-names@2a727e8a
#[test]
fn slug_with_a_decision_is_shared() {
    let dir = repo();
    write(
        dir.path(),
        "docs/decisions/gone.md",
        "---\ntitle: Gone\ndate: 2026-01-01\nstatus: accepted\n---\n# Gone\n",
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

// req:check/removed-requirement-names@2a727e8a
#[test]
fn slug_of_architecture_requirement_is_shared() {
    let dir = repo();
    write(
        dir.path(),
        "ARCHITECTURE.md",
        "# A\n\n## Constraints\n\n### Constraint: gone\nIt SHALL hold.\n\n#### Scenario: s\n- ok\n\n#### Untested\nA judgement.\n",
    );
    serve(dir.path(), "architecture/gone");
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

// req:check/removed-requirement-names@2a727e8a
#[test]
fn slug_of_capability_is_shared() {
    let dir = repo();
    write(
        dir.path(),
        "CAPABILITIES.md",
        "# C\n\n## Capabilities\n\n### Capability: gone\nA person can.\n\n#### Why\nBecause.\n\n#### Scenario: s\n- ok\n",
    );
    serve(dir.path(), "capability/gone");
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

// req:check/removed-requirement-names@2a727e8a
#[test]
fn slug_mentioned_in_architecture_but_not_as_an_entry_is_not_shared() {
    let dir = repo();
    write(
        dir.path(),
        "ARCHITECTURE.md",
        "# A\n\n- **`gone`**: an old-style invariant. See `gone` and **gone**.\n",
    );
    assert_eq!(found(dir.path()).len(), 1);
}

// req:check/removed-requirement-names@2a727e8a
#[test]
fn slug_another_node_holds_is_shared() {
    let dir = repo();
    write(
        dir.path(),
        "specs/b/SPEC.md",
        &spec(
            "B",
            "## Requirements\n\n### Requirement: gone\nGone.\n\n#### Scenario: s\n- ok\n",
        ),
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

// req:check/removed-requirement-names@2a727e8a
#[test]
fn not_applied_yet_stale_delta_is_skipped() {
    let dir = pending_repo();
    let d = dir.path();
    // The fake base makes the pending change stale.
    removing(d, "changes/20260101-y", "proposed", "keep");
    let all: Vec<String> = run(d).iter().map(clm_check::format).collect();
    assert!(
        all.iter().any(|v| v.contains("apply/stale-base")),
        "{all:?}"
    );
    assert!(all.iter().all(|v| !v.contains(RULE)), "{all:?}");
}

/// Node `a` holding `keep` and `other`, which names nothing, and node `b` whose Design
/// names `` `a/keep` `` on line 12; no change yet.
fn pending_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(
        d,
        "specs/a/SPEC.md",
        &spec(
            "A",
            "## Requirements\n\n### Requirement: keep\nKeep.\n\n#### Scenario: s\n- ok\n\n### Requirement: other\nOther.\n\n#### Scenario: s\n- ok\n",
        ),
    );
    write(
        d,
        "specs/b/SPEC.md",
        &spec(
            "B",
            "## Design\n\nOne.\nTwo.\nThree.\nIt uses `a/keep`, and `keep` alone.\n",
        ),
    );
    dir
}

/// The current hash of requirement `id`, a delta's base for it.
fn base(root: &Path, id: &str) -> String {
    let (graph, _) = clm_model::load(root);
    graph.requirement(id).unwrap().hash.clone()
}

/// A pending change `id` in `changes/<id>` whose delta for `a` removes `keep` with its base,
/// and holds `more` after that, such as a `## MODIFIED Requirements` section.
fn removing_keep(root: &Path, id: &str, status: &str, more: &str) {
    let keep = base(root, "a/keep");
    write(
        root,
        &format!("changes/{id}/change.md"),
        &format!("---\nstatus: {status}\n---\n"),
    );
    write(
        root,
        &format!("changes/{id}/deltas/a.md"),
        &format!(
            "---\ncomponent: a\n---\n## REMOVED Requirements\n\n### Requirement: keep <!-- base: {keep} -->\n{more}"
        ),
    );
}

// req:check/removed-requirement-names@2a727e8a
#[test]
fn not_applied_yet_is_reported() {
    let dir = pending_repo();
    let d = dir.path();
    removing_keep(d, "20260101-y", "proposed", "");
    assert_eq!(
        found(d),
        [
            "specs/b/SPEC.md:12: check/removed-requirement-names: names `a/keep`, which 20260101-y would remove"
        ]
    );
}

// req:check/removed-requirement-names@2a727e8a
#[test]
fn not_applied_yet_name_rewritten_is_not_reported() {
    let dir = pending_repo();
    let d = dir.path();
    // `a`'s `other` names `keep`, and `b` no longer names it, so only `a`'s text is at stake.
    write(
        d,
        "specs/a/SPEC.md",
        &spec(
            "A",
            "## Requirements\n\n### Requirement: keep\nKeep.\n\n#### Scenario: s\n- ok\n\n### Requirement: other\nOther builds on `keep`.\n\n#### Scenario: s\n- ok\n",
        ),
    );
    write(d, "specs/b/SPEC.md", &spec("B", "## Design\n\nNothing.\n"));
    // Without the rewrite, the bare slug in `a` is reported.
    removing_keep(d, "20260101-y", "approved", "");
    assert_eq!(found(d).len(), 1);
    let other = base(d, "a/other");
    removing_keep(
        d,
        "20260101-y",
        "approved",
        &format!(
            "\n## MODIFIED Requirements\n\n### Requirement: other <!-- base: {other} -->\nOther stands alone.\n\n#### Scenario: s\n- ok\n"
        ),
    );
    assert_eq!(found(d), Vec::<String>::new());
}

// req:check/removed-requirement-names@2a727e8a
#[test]
fn line_is_that_of_the_text_the_change_leaves() {
    let dir = pending_repo();
    let d = dir.path();
    let other = base(d, "a/other");
    // The rewrite makes `other` name `keep`, at a line the current text doesn't have.
    removing_keep(
        d,
        "20260101-y",
        "proposed",
        &format!(
            "\n## MODIFIED Requirements\n\n### Requirement: other <!-- base: {other} -->\nOther.\nStill.\nIt builds on `keep`.\n\n#### Scenario: s\n- ok\n"
        ),
    );
    // With `keep` gone, `other`'s heading is line 9 and its third line, naming `keep`, is 12;
    // the current text has no such line.
    let all = found(d);
    let a: Vec<&String> = all.iter().filter(|v| v.starts_with("specs/a/")).collect();
    assert_eq!(
        a,
        [
            "specs/a/SPEC.md:12: check/removed-requirement-names: names `a/keep`, which 20260101-y would remove"
        ]
    );
}

// req:check/removed-requirement-names@2a727e8a
#[test]
fn each_pending_change_names_itself() {
    let dir = pending_repo();
    let d = dir.path();
    removing_keep(d, "20260101-y", "proposed", "");
    removing_keep(d, "20260102-z", "proposed", "");
    assert_eq!(
        found(d),
        [
            "specs/b/SPEC.md:12: check/removed-requirement-names: names `a/keep`, which 20260101-y would remove",
            "specs/b/SPEC.md:12: check/removed-requirement-names: names `a/keep`, which 20260102-z would remove",
        ]
    );
}

// req:check/removed-requirement-names@2a727e8a
#[test]
fn shared_slug_is_judged_on_the_graph_the_change_leaves() {
    let dir = pending_repo();
    let d = dir.path();
    // `b` holds a requirement `keep` too, so the bare slug is shared, before and after.
    write(
        d,
        "specs/b/SPEC.md",
        &spec(
            "B",
            "## Requirements\n\n### Requirement: keep\nB keeps.\n\n#### Scenario: s\n- ok\n",
        ),
    );
    removing_keep(d, "20260101-y", "proposed", "");
    assert_eq!(found(d), Vec::<String>::new());
}

// req:check/removed-requirement-names@2a727e8a
#[test]
fn archived_change_removes_nothing_whatever_its_status() {
    for status in ["done", "applied"] {
        let dir = TempDir::new().unwrap();
        let d = dir.path();
        write(d, "specs/SPEC.md", ROOT);
        write(d, "specs/a/SPEC.md", &spec_a());
        removing(d, "changes/archive/20260101-x", status, "gone");
        // An archived `applied` change is `archive-done`'s to report, not this rule's.
        let all: Vec<String> = run(d)
            .iter()
            .filter(|v| v.rule != "check/archive-done")
            .map(clm_check::format)
            .collect();
        assert_eq!(all, Vec::<String>::new(), "{status}");
    }
}

// req:check/removed-requirement-names@2a727e8a
#[test]
fn backticks_after_a_backtick_run_open_no_fence() {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(
        d,
        "specs/a/SPEC.md",
        &spec(
            "A",
            "## Design\n\n```x``` is no fence.\nIt names `gone`.\n\n## Requirements\n\n### Requirement: keep\nKeep.\n\n#### Scenario: s\n- ok\n",
        ),
    );
    removing(d, "changes/20260101-x", "applied", "gone");
    assert_eq!(
        found(d),
        [
            "specs/a/SPEC.md:10: check/removed-requirement-names: names `a/gone`, which 20260101-x removes"
        ]
    );
}

// req:check/removed-requirement-names@2a727e8a
#[test]
fn fenced_example_and_frontmatter_are_not_read() {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(
        d,
        "specs/a/SPEC.md",
        "---\ntitle: \"`gone`\"\nstatus: active\nowners: []\n---\n\n## Design\n\n```text\nIt names `gone` and `a/gone`.\n```\n\n~~~~\n`gone`\n~~~~\n\n## Requirements\n\n### Requirement: keep\nKeep.\n\n#### Scenario: s\n- ok\n",
    );
    removing(d, "changes/20260101-x", "applied", "gone");
    assert_eq!(found(d), Vec::<String>::new());
}

// req:check/removed-requirement-names@2a727e8a
#[test]
fn span_must_be_the_whole_id() {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(
        d,
        "specs/a/SPEC.md",
        &spec(
            "A",
            "## Design\n\nSee `gone-too`, `not gone`, `` gone ``.\n\n## Requirements\n\n### Requirement: keep\nKeep.\n\n#### Scenario: s\n- ok\n",
        ),
    );
    removing(d, "changes/20260101-x", "applied", "gone");
    // Only `` gone `` is a span whose whole text is the slug: CommonMark strips one space
    // on each side.
    assert_eq!(
        found(d),
        [
            "specs/a/SPEC.md:9: check/removed-requirement-names: names `a/gone`, which 20260101-x removes"
        ]
    );
}

// req:check/removed-requirement-names@2a727e8a
#[test]
fn removed_capability_is_named() {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "CAPABILITIES.md", "## Capabilities\n");
    write(
        d,
        "specs/b/SPEC.md",
        &spec("B", "## Design\n\n\n\n\nIt serves `capability/gone`.\n"),
    );
    write(
        d,
        "specs/capability/SPEC.md",
        &spec("Capability", "## Design\n\nIt tests `gone`.\n"),
    );
    write(
        d,
        "changes/20260101-x/change.md",
        "---\nstatus: proposed\nremoves_capabilities: [gone]\n---\n",
    );
    assert_eq!(
        found(d),
        [
            "specs/b/SPEC.md:12: check/removed-requirement-names: names `capability/gone`, which 20260101-x removes",
            "specs/capability/SPEC.md:9: check/removed-requirement-names: names `capability/gone`, which 20260101-x removes",
        ]
    );
}
