mod common;

use std::path::Path;

use common::{commit_ago, git, repo, report, section, spec, write};
use tempfile::TempDir;

/// What `clm health --root <root>` with `extra` arguments after it prints to standard output.
fn clm_health(root: &Path, extra: &[&str]) -> String {
    let mut args = vec!["--root", root.to_str().unwrap()];
    args.extend(extra);
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let status = clm_health::health_command(
        args,
        Path::new("/nonexistent"),
        Ok(common::settings(12000, 40000)),
        &|_| None,
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(status, 0, "{}", String::from_utf8_lossy(&stderr));
    String::from_utf8(stdout).unwrap()
}

/// A repository whose `origin` holds `main`, committed 20 hours ago, and each of `claims`,
/// a branch off `main` committed the given seconds ago.
fn claims(claims: &[(&str, u64)]) -> (TempDir, TempDir) {
    let origin = TempDir::new().unwrap();
    git(origin.path(), &["init", "-q", "--bare"]);
    let dir = repo();
    let d = dir.path();
    commit_ago(d, "main", 20 * HOUR);
    let url = origin.path().to_str().unwrap();
    git(d, &["remote", "add", "origin", url]);
    git(d, &["push", "-q", "origin", "main"]);
    for &(branch, ago) in claims {
        git(d, &["switch", "-q", "-c", branch, "main"]);
        commit_ago(d, branch, ago);
        git(d, &["push", "-q", "origin", branch]);
    }
    git(d, &["switch", "-q", "main"]);
    (origin, dir)
}

const HOUR: u64 = 3600;
const DAY: u64 = 86_400;

#[test]
fn scenario_lapsed_claim() {
    // req:check.health/state-signals@baf22711
    let (_origin, dir) = claims(&[
        ("impl/a", 10 * HOUR),
        ("impl/b", HOUR),
        ("propose/q", 6 * HOUR),
    ]);
    let out = clm_health(dir.path(), &["--expiry", "4"]);
    assert_eq!(
        section(&out, "lapsed-claim"),
        [
            "health/lapsed-claim: 2 claims",
            "impl/a: health/lapsed-claim: last pushed 10h ago",
            "propose/q: health/lapsed-claim: last pushed 6h ago",
        ],
        "{out}"
    );
    // A longer expiry keeps them.
    let out = clm_health(dir.path(), &["--expiry", "11"]);
    assert_eq!(
        section(&out, "lapsed-claim"),
        ["health/lapsed-claim: 0 claims"],
        "{out}"
    );
}

#[test]
fn scenario_the_default_expiry() {
    // req:check.health/state-signals@baf22711
    let (_origin, dir) = claims(&[("propose/q", 5 * HOUR), ("impl/b", 3 * HOUR)]);
    let out = clm_health(dir.path(), &[]);
    assert_eq!(
        section(&out, "lapsed-claim"),
        [
            "health/lapsed-claim: 1 claims",
            "propose/q: health/lapsed-claim: last pushed 5h ago",
        ],
        "{out}"
    );
}

#[test]
fn a_merged_claim_never_lapses() {
    // req:check.health/state-signals@baf22711
    let origin = TempDir::new().unwrap();
    git(origin.path(), &["init", "-q", "--bare"]);
    let dir = repo();
    let d = dir.path();
    commit_ago(d, "main", 20 * HOUR);
    git(
        d,
        &["remote", "add", "origin", origin.path().to_str().unwrap()],
    );
    git(d, &["push", "-q", "origin", "main", "main:impl/a"]);
    git(d, &["fetch", "-q", "origin"]);
    let out = report(d);
    assert_eq!(
        section(&out, "lapsed-claim"),
        ["health/lapsed-claim: 0 claims"],
        "{out}"
    );
}

#[test]
fn without_an_origin_nothing_is_listed() {
    // req:check.health/state-signals@baf22711
    let dir = repo();
    let out = report(dir.path());
    assert_eq!(
        section(&out, "lapsed-claim"),
        ["health/lapsed-claim: no origin"],
        "{out}"
    );
}

/// A change `id` with `status` whose delta for node `a` adds `added`.
fn change(root: &std::path::Path, id: &str, status: &str, added: &[&str]) {
    write(
        root,
        &format!("changes/{id}/change.md"),
        &format!("---\nstatus: {status}\n---\n"),
    );
    let mut delta = String::from("---\ncomponent: a\n---\n## ADDED Requirements\n");
    for slug in added {
        delta.push_str(&format!(
            "\n### Requirement: {slug}\n{slug}.\n\n#### Scenario: s\n- ok\n"
        ));
    }
    write(root, &format!("changes/{id}/deltas/a.md"), &delta);
}

#[test]
fn scenario_stalled_change() {
    // req:check.health/state-signals@baf22711
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &spec("[\"src/**\"]", &["x", "y", "z", "w"]),
    );
    change(d, "20260101-c", "applied", &["x", "y"]);
    change(d, "20260102-d", "applied", &["z"]);
    change(d, "20260103-e", "implementing", &["w"]);
    change(d, "20260104-f", "proposed", &["v"]);
    let (graph, _) = clm_model::load(d);
    let version = graph.requirement("a/z").unwrap().version().to_string();
    write(
        d,
        "src/t.rs",
        &format!("// {}:a/z@{version}\n#[test]\nfn t() {{}}\n", "req"),
    );
    git(d, &["add", "-A"]);
    let out = report(d);
    assert_eq!(
        section(&out, "stalled-change"),
        [
            "health/stalled-change: 2 changes",
            "changes/20260101-c/change.md: health/stalled-change: 0/2 requirements current",
            "changes/20260103-e/change.md: health/stalled-change: 0/1 requirements current",
        ],
        "{out}"
    );
}

#[test]
fn scenario_a_removed_component_and_an_empty_change() {
    // req:check.health/state-signals@baf22711
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "changes/20260105-r/change.md",
        "---\nstatus: applied\n---\n",
    );
    write(
        d,
        "changes/20260105-r/deltas/b.c.md",
        "---\ncomponent: b.c\nremove: true\n---\n## REMOVED Requirements\n\n\
### Requirement: x <!-- base: 00000000 -->\n",
    );
    write(
        d,
        "tests/t.rs",
        &format!("// {}:b.c/x@00000000\n#[test]\nfn t() {{}}\n", "req"),
    );
    // A second citing file another component owns: the requirement still counts once.
    write(d, "specs/a/SPEC.md", &spec("[\"src/**\"]", &[]));
    write(
        d,
        "src/t.rs",
        &format!("// {}:b.c/x@00000000\n#[test]\nfn t() {{}}\n", "req"),
    );
    git(d, &["add", "-A"]);
    let out = report(d);
    assert_eq!(
        section(&out, "pending"),
        [
            "health/pending: 1 components",
            "specs/b/c/SPEC.md: health/pending: 1 pending (0 unbuilt, 0 behind, 1 obsolete)",
        ],
        "{out}"
    );
    assert_eq!(
        section(&out, "stalled-change"),
        ["health/stalled-change: 0 changes"],
        "{out}"
    );
}

#[test]
fn scenario_unanswered_question() {
    // req:check.health/state-signals@baf22711
    let dir = repo();
    let d = dir.path();
    let item = |kind: &str| format!("---\ntitle: T\nkind: {kind}\n---\nWhat?\n");
    commit_ago(d, "root", 50 * DAY);
    write(d, "backlog/q.md", &item("question"));
    write(d, "backlog/b.md", &item("bug"));
    commit_ago(d, "q", 45 * DAY);
    git(d, &["rm", "-q", "backlog/q.md"]);
    commit_ago(d, "q gone", 2 * DAY + HOUR);
    write(d, "backlog/q.md", &item("question"));
    commit_ago(d, "q again", 2 * DAY);
    write(d, "backlog/h.md", &item("human-task"));
    commit_ago(d, "h", 3 * DAY);
    write(d, "backlog/u.md", &item("question"));
    let out = report(d);
    assert_eq!(
        section(&out, "unanswered"),
        [
            "health/unanswered: 1 items",
            "backlog/q.md: health/unanswered: question open 45 days",
        ],
        "{out}"
    );
}

#[test]
fn a_stale_human_task_is_reported() {
    // req:check.health/state-signals@baf22711
    let dir = repo();
    let d = dir.path();
    commit_ago(d, "root", 50 * DAY);
    write(
        d,
        "backlog/h.md",
        "---\ntitle: T\nkind: human-task\n---\nDo it.\n",
    );
    commit_ago(d, "h", 40 * DAY);
    let out = report(d);
    assert_eq!(
        section(&out, "unanswered"),
        [
            "health/unanswered: 1 items",
            "backlog/h.md: health/unanswered: human-task open 40 days",
        ],
        "{out}"
    );
}

#[test]
fn an_item_in_a_repository_without_commits_is_untracked() {
    // req:check.health/state-signals@baf22711
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "backlog/u.md",
        "---\ntitle: T\nkind: question\n---\nWhat?\n",
    );
    let out = report(d);
    assert_eq!(
        section(&out, "unanswered"),
        ["health/unanswered: 0 items"],
        "{out}"
    );
}
