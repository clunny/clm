//! `clm status`: a summary, then the `components`, `changes` and `claims` sections.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use tempfile::TempDir;

const REQ: &str = concat!("req", ":");

/// `clm status --root <root>`, with no user settings.
fn clm_status(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .env(
            "XDG_CONFIG_HOME",
            Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-user-config"),
        )
        .args(["status", "--root"])
        .arg(root)
        .output()
        .unwrap()
}

fn status(root: &Path) -> String {
    let out = clm_status(root);
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(0), "{text}");
    text
}

/// The summary: the lines up to the first blank one.
fn summary(text: &str) -> String {
    text.split("\n\n").next().unwrap().to_string()
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";

fn node(status: &str, reqs: &[&str]) -> String {
    let mut s = format!("---\ntitle: A\nstatus: {status}\nowners: []\n---\n\n## Requirements\n");
    for r in reqs {
        s.push_str(&format!(
            "\n### Requirement: {r}\n{}\n\n#### Scenario: s\n- s\n",
            r.to_uppercase()
        ));
    }
    s
}

/// Node `a` with `a/x` (cited at its current version) and `a/y` (uncited).
fn base() -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/a/SPEC.md", &node("active", &["x", "y"]));
    cite(dir.path(), "t", &["a/x"]);
    dir
}

/// `tests/<file>.rs` citing `ids` at their current versions.
fn cite(root: &Path, file: &str, ids: &[&str]) {
    let (graph, _) = clm_model::load(root);
    let text: String = ids
        .iter()
        .map(|id| {
            format!(
                "// {REQ}{id}@{}\n",
                graph.requirement(id).unwrap().version()
            )
        })
        .collect();
    write(root, &format!("tests/{file}.rs"), &text);
}

fn change(root: &Path, id: &str, status: &str, adds: &[&str]) {
    write(
        root,
        &format!("changes/{id}/change.md"),
        &format!("---\nstatus: {status}\n---\n"),
    );
    let mut delta = String::from("---\ncomponent: a\n---\n## ADDED Requirements\n");
    for slug in adds {
        delta.push_str(&format!(
            "### Requirement: {slug}\n{}\n\n#### Scenario: s\n- s\n\n",
            slug.to_uppercase()
        ));
    }
    write(root, &format!("changes/{id}/deltas/a.md"), &delta);
}

fn item(root: &Path, slug: &str, kind: &str) {
    write(
        root,
        &format!("backlog/{slug}.md"),
        &format!("---\ntitle: I\nkind: {kind}\n---\n"),
    );
}

fn git_env(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> String {
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .envs(env.iter().copied())
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

/// Makes `root` a repository with a remote-tracking ref `origin/impl/<node>` per
/// `(node, hours ago)`, as a fetch would leave them, and `origin/main` at its commit.
fn claim_refs(root: &Path, claims: &[(&str, u64)]) {
    let git = |args: &[&str]| git_env(root, args, &[]);
    git(&["init", "-q", "-b", "main"]);
    git(&["add", "-A"]);
    git(&["commit", "-qm", "base"]);
    git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    for (node, hours) in claims {
        remote_ref(root, &format!("impl/{node}"), *hours);
    }
}

/// A remote-tracking ref `origin/<name>` on a commit on `HEAD` made `hours` hours ago.
fn remote_ref(root: &Path, name: &str, hours: u64) {
    let git = |args: &[&str]| git_env(root, args, &[]);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let date = format!("@{} +0000", now - hours * 3600);
    let tree = git(&["rev-parse", "HEAD^{tree}"]);
    let head = git(&["rev-parse", "HEAD"]);
    let sha = git_env(
        root,
        &[
            "commit-tree",
            tree.trim(),
            "-p",
            head.trim(),
            "-m",
            &format!("Claim {name}"),
        ],
        &[("GIT_COMMITTER_DATE", &date), ("GIT_AUTHOR_DATE", &date)],
    );
    git(&[
        "update-ref",
        &format!("refs/remotes/origin/{name}"),
        sha.trim(),
    ]);
}

#[test]
fn scenario_summary() {
    // req:view/status-report@538240fb
    let dir = base();
    let d = dir.path();
    change(d, "20260101-c", "applied", &["x", "y"]);
    item(d, "i1", "idea");
    item(d, "i2", "idea");
    let text = status(d);
    assert!(text.starts_with("CLM status\n"), "{text}");
    let summary = summary(&text);
    for line in [
        "  components 2: 2 active, 0 unbuilt; 1/2 requirements current",
        "  changes 1 live: 1 applied",
        "  pending 1 components: 1 ready",
        "  waiting on you 0",
        "  backlog 2: 2 idea",
        "  claims 0 live",
    ] {
        assert!(
            summary.contains(&format!("\n{line}\n")) || summary.ends_with(line),
            "{line:?} missing from {text}"
        );
    }
    // The sections follow, each after a blank line.
    assert!(
        text.ends_with("\n\ncomponents\n  a    active  1/2\n  clm  active  0/0\n\nchanges\n  applied\n    20260101-c  1/2 requirements\n\nclaims\n  none\n\n"),
        "{text}"
    );
}

#[test]
fn summary_counts_unbuilt_nodes() {
    // req:view/status-report@538240fb
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/b/SPEC.md", &node("specd", &["x"]));
    let text = status(d);
    assert!(
        text.contains("  components 2: 1 active, 1 unbuilt; 0/1 requirements current"),
        "{text}"
    );
}

#[test]
fn scenario_aligned_columns() {
    // req:view/status-report@538240fb
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    let ten: Vec<String> = (0..10).map(|i| format!("r{i}")).collect();
    let ten: Vec<&str> = ten.iter().map(String::as_str).collect();
    write(d, "specs/a/SPEC.md", &node("active", &ten));
    write(d, "specs/check/SPEC.md", &node("specd", &["x", "y"]));
    let text = status(d);
    assert!(
        text.contains(
            "\ncomponents\n  a      active  0/10\n  check  specd   0/2\n  clm    active  0/0\n\n"
        ),
        "{text}"
    );
}

#[test]
fn scenario_changes_grouped() {
    // req:view/status-report@538240fb
    // req:check.coverage/status-report@47ccc387
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &node("active", &["x", "y", "z"]));
    change(d, "20260101-c", "implementing", &["x", "y", "z"]);
    cite(d, "t", &["a/x"]);
    let text = status(d);
    assert!(
        text.contains("\nchanges\n  implementing\n    20260101-c  1/3 requirements\n\n"),
        "{text}"
    );
    assert!(
        text.contains("\n  changes 1 live: 1 implementing\n"),
        "{text}"
    );
}

#[test]
fn scenario_complete_change() {
    // req:check.coverage/status-report@47ccc387
    // req:view/status-report@538240fb
    let dir = base();
    let d = dir.path();
    change(d, "20260101-c", "applied", &["x", "y"]);
    cite(d, "u", &["a/y"]);
    let text = status(d);
    assert!(
        text.contains("\n  applied\n    20260101-c  2/2 requirements  complete\n"),
        "{text}"
    );
    assert!(
        text.contains("\n  changes 1 live: 1 applied, 1 complete\n"),
        "{text}"
    );
}

#[test]
fn scenario_groups_in_order() {
    // req:check.coverage/status-report@47ccc387
    let dir = base();
    let d = dir.path();
    change(d, "20260101-a", "applied", &["x"]);
    change(d, "20260101-z", "implementing", &["y"]);
    let text = status(d);
    assert!(
        text.contains(
            "changes\n  implementing\n    20260101-z  0/1 requirements\n  applied\n    20260101-a  1/1 requirements  complete\n"
        ),
        "{text}"
    );
    assert!(
        text.contains("changes 2 live: 1 implementing, 1 applied, 1 complete\n"),
        "{text}"
    );
}

#[test]
fn scenario_waiting_on_you() {
    // req:view/status-report@538240fb
    let dir = base();
    let d = dir.path();
    item(d, "q", "question");
    item(d, "h", "human-task");
    item(d, "i", "idea");
    let text = status(d);
    assert!(
        text.contains("\n  waiting on you 2: backlog/h, backlog/q\n"),
        "{text}"
    );
    assert!(
        text.contains("\n  backlog 3: 1 idea, 1 question, 1 human-task\n"),
        "{text}"
    );
}

#[test]
fn a_change_awaiting_approval_waits_on_you_and_a_node_writing_a_protected_path_is_ready() {
    // req:view/status-report@538240fb
    let dir = base();
    let d = dir.path();
    write(
        d,
        "specs/i/SPEC.md",
        &(node("specd", &[]).replace("owners: []", "owners: []\npaths: [\".claude/**\"]")
            + "\n### Requirement: w\nWrites `.claude/settings.json`.\n\n#### Scenario: s\n- s\n"),
    );
    change(d, "20260101-p", "proposed", &["z"]);
    change(d, "20260101-q", "approved", &["w"]);
    let text = status(d);
    assert!(
        text.contains("\n  waiting on you 2: changes/20260101-p, changes/20260101-q\n"),
        "{text}"
    );
    assert!(
        text.contains("\n  pending 2 components: 2 ready\n"),
        "{text}"
    );
}

#[test]
fn scenario_claims_counted() {
    // req:view/status-report@538240fb
    // req:orchestrator/status-claims@726cb7c5
    let dir = base();
    let d = dir.path();
    write(d, "specs/b/SPEC.md", &node("specd", &["x"]));
    write(d, "specs/d/SPEC.md", &node("specd", &["x"]));
    claim_refs(d, &[("b", 1), ("d", 5)]);
    let text = status(d);
    assert!(text.contains("\n  claims 1 live, 1 lapsed\n"), "{text}");
    assert!(
        text.ends_with("\nclaims\n  b  live\n  d  lapsed\n\n"),
        "{text}"
    );
    // The lapsed claim leaves `d` ready; the live one makes `b` claimed.
    assert!(
        text.contains("\n  pending 3 components: 1 claimed, 2 ready\n"),
        "{text}"
    );
}

#[test]
fn backlog_claims_are_counted_with_node_claims() {
    // req:view/status-report@538240fb
    // req:orchestrator/status-claims@726cb7c5
    let dir = base();
    let d = dir.path();
    write(d, "specs/b/SPEC.md", &node("specd", &["x"]));
    claim_refs(d, &[("b", 1)]);
    remote_ref(d, "propose/q", 1);
    remote_ref(d, "propose/r", 5);
    let text = status(d);
    assert!(text.contains("\n  claims 2 live, 1 lapsed\n"), "{text}");
    assert!(
        text.ends_with("\nclaims\n  b          live\n  backlog/q  live\n  backlog/r  lapsed\n\n"),
        "{text}"
    );
}

#[test]
fn merged_claims_are_counted_and_released() {
    // req:view/status-report@538240fb
    let dir = base();
    let d = dir.path();
    write(d, "specs/b/SPEC.md", &node("specd", &["x"]));
    write(d, "specs/e/SPEC.md", &node("specd", &["x"]));
    claim_refs(d, &[("b", 1), ("e", 1)]);
    // `origin/main` merges `impl/e`, as CI does.
    let git = |args: &[&str]| git_env(d, args, &[]);
    let tree = git(&["rev-parse", "HEAD^{tree}"]);
    let main = git(&["rev-parse", "refs/remotes/origin/main"]);
    let tip = git(&["rev-parse", "refs/remotes/origin/impl/e"]);
    let merge = git(&[
        "commit-tree",
        tree.trim(),
        "-p",
        main.trim(),
        "-p",
        tip.trim(),
        "-m",
        "Merge impl/e",
    ]);
    git(&["update-ref", "refs/remotes/origin/main", merge.trim()]);
    let text = status(d);
    assert!(text.contains("\n  claims 1 live, 1 merged\n"), "{text}");
    assert!(
        text.ends_with("\nclaims\n  b  live\n  e  merged\n\n"),
        "{text}"
    );
    assert!(
        text.contains("\n  pending 3 components: 1 claimed, 2 ready\n"),
        "{text}"
    );
}

#[test]
fn scenario_pending_states() {
    // req:view/status-report@538240fb
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &node("specd", &["x"]));
    write(d, "specs/b/SPEC.md", &node("specd", &["x"]));
    write(d, "specs/x/SPEC.md", &node("specd", &["x"]));
    write(
        d,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [x/x]\n---\n",
    );
    claim_refs(d, &[("b", 1)]);
    let text = status(d);
    assert!(
        text.contains("\n  pending 3 components: 1 claimed, 1 ready, 1 blocked\n"),
        "{text}"
    );
    assert!(!summary(&text).contains("waiting\n"), "{text}");
    assert!(!summary(&text).contains("waiting,"), "{text}");
}

#[test]
fn summary_orders_statuses_states_and_kinds() {
    // req:view/status-report@538240fb
    let dir = base();
    let d = dir.path();
    // Nodes in every pending state: `a` and `i` ready (`i` writes a protected path), `b`
    // claimed, `w` ready though it depends on `a` (`needs-alone-order`), `x` blocked by
    // `backlog/q`.
    write(d, "specs/b/SPEC.md", &node("specd", &["x"]));
    write(
        d,
        "specs/i/SPEC.md",
        &(node("specd", &[]).replace("owners: []", "owners: []\npaths: [\".claude/**\"]")
            + "\n### Requirement: w\nWrites `.claude/settings.json`.\n\n#### Scenario: s\n- s\n"),
    );
    write(
        d,
        "specs/w/SPEC.md",
        &node("specd", &["x"]).replace("owners: []", "owners: []\ndepends_on: [a]"),
    );
    write(d, "specs/x/SPEC.md", &node("specd", &["x"]));
    write(
        d,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [x/x]\n---\n",
    );
    for (slug, kind) in [
        ("h", "human-task"),
        ("i", "idea"),
        ("f", "deferred"),
        ("g", "bug"),
    ] {
        item(d, slug, kind);
    }
    for (id, status) in [
        ("20260101-p", "proposed"),
        ("20260101-a", "approved"),
        ("20260101-i", "implementing"),
        ("20260101-d", "applied"),
    ] {
        change(d, id, status, &["y"]);
    }
    claim_refs(d, &[("b", 1)]);
    let text = status(d);
    for line in [
        "  changes 4 live: 1 implementing, 1 applied, 1 approved, 1 proposed\n",
        "  pending 5 components: 1 claimed, 3 ready, 1 blocked\n",
        "  waiting on you 4: backlog/h, backlog/q, changes/20260101-a, changes/20260101-p\n",
        "  backlog 5: 1 bug, 1 deferred, 1 idea, 1 question, 1 human-task\n",
    ] {
        assert!(text.contains(line), "{line:?} missing from {text}");
    }
}

#[test]
fn scenario_nothing_in_flight() {
    // req:view/status-report@538240fb
    // req:check.coverage/status-report@47ccc387
    // req:orchestrator/status-claims@726cb7c5 (not a git repository)
    let dir = base();
    let d = dir.path();
    cite(d, "u", &["a/y"]);
    write(
        d,
        "changes/archive/20250101-old/change.md",
        "---\nstatus: done\n---\n",
    );
    let text = status(d);
    for line in [
        "  changes 0 live\n",
        "  pending 0 components\n",
        "  backlog 0\n",
        "  claims 0 live\n",
    ] {
        assert!(text.contains(line), "{line:?} missing from {text}");
    }
    assert!(
        text.ends_with("\n\nchanges\n  none\n\nclaims\n  none\n\n"),
        "{text}"
    );
}

#[test]
fn invalid_graph_prints_violations() {
    // req:view/status-report@538240fb
    // req:check.coverage/status-report@47ccc387
    let dir = base();
    write(dir.path(), "specs/b/SPEC.md", "---\ntitle: B\n---\n");
    let out = clm_status(dir.path());
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(text.contains("model/valid-frontmatter"), "{text}");
    assert!(!text.contains("CLM status"), "{text}");
}

#[test]
fn status_and_view_never_fetch() {
    // req:view/status-report@538240fb
    // req:view/view-command@8e470e55
    let dir = base();
    let d = dir.path();
    write(d, "specs/z/SPEC.md", &node("specd", &["x"]));
    claim_refs(d, &[("a", 1), ("z", 1)]);
    // `origin` holds a claim on `z` that the last fetch didn't bring: a fetch would show it.
    let origin = TempDir::new().unwrap();
    git_env(origin.path(), &["init", "-q", "--bare", "-b", "main"], &[]);
    let git = |args: &[&str]| git_env(d, args, &[]);
    git(&["remote", "add", "origin", origin.path().to_str().unwrap()]);
    git(&[
        "push",
        "-q",
        "origin",
        "refs/remotes/origin/impl/z:refs/heads/impl/z",
    ]);
    git(&["update-ref", "-d", "refs/remotes/origin/impl/z"]);
    let text = status(d);
    assert!(text.contains("\n  claims 1 live\n"), "{text}");
    assert!(text.ends_with("\nclaims\n  a  live\n\n"), "{text}");
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(["view", "flight", "--format", "json", "--root"])
        .arg(d)
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let z = v["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["node"] == "z")
        .unwrap();
    assert_eq!(z["state"], "ready", "{v:#}");
}

/// A repository whose `clm.toml` sets a code budget of 100 tokens, with node `a` owning
/// `src/a/**` (`frontmatter` added to its own), a tracked `src/a/x.rs` of 200 tokens, and
/// node `b` within its budgets.
fn sized(frontmatter: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(
        d,
        "specs/a/SPEC.md",
        &node("active", &[]).replace(
            "owners: []\n",
            &format!("owners: []\npaths: [\"src/a/**\"]\n{frontmatter}"),
        ),
    );
    write(d, "specs/b/SPEC.md", &node("active", &[]));
    write(d, "clm.toml", "[component_size]\ncode_tokens = 100\n");
    write(d, "src/a/x.rs", &"a".repeat(800));
    claim_refs(d, &[]);
    dir
}

/// The row of `node` in the `components` section.
fn row<'a>(text: &'a str, node: &str) -> &'a str {
    let section = text.split("\n\ncomponents\n").nth(1).unwrap();
    section
        .lines()
        .find(|l| l.starts_with(&format!("  {node} ")))
        .unwrap_or_else(|| panic!("no {node} row in {text}"))
}

/// The summary's `components` line.
fn nodes_line(text: &str) -> String {
    summary(text)
        .lines()
        .find(|l| l.starts_with("  components "))
        .unwrap()
        .to_string()
}

#[test]
fn scenario_oversized_node() {
    // req:view/status-report@538240fb
    let dir = sized("");
    let text = status(dir.path());
    let line = nodes_line(&text);
    assert!(line.ends_with(", 1 oversized"), "{text}");
    assert!(row(&text, "a").ends_with("  oversized"), "{text}");
    assert!(!row(&text, "b").contains("oversized"), "{text}");
}

#[test]
fn scenario_budget_accepted() {
    // req:view/status-report@538240fb
    let dir = sized("size_budget: { code_tokens: 1000 }\n");
    let text = status(dir.path());
    assert!(!nodes_line(&text).contains("oversized"), "{text}");
    assert!(!row(&text, "a").contains("oversized"), "{text}");
}

#[test]
fn sizes_that_cant_be_checked_mark_nothing() {
    // req:view/status-report@538240fb
    let dir = sized("");
    write(
        dir.path(),
        "clm.toml",
        "[component_size]\ncode_tokens = \"many\"\n",
    );
    let out = clm_status(dir.path());
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let stderr = String::from_utf8(out.stderr.clone()).unwrap();
    assert!(stderr.starts_with("size not checked: "), "{stderr}");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(!text.contains("oversized"), "{text}");
}

/// A `landed` record for change `y` as an older `clm` wrote it, `tier` `SOFT`, which then
/// awaited an after-merge review.
const SOFT_LAND: &str = "{\"id\":\"r1\",\"writer\":\"a/1\",\"at\":\"2026-10-01T10:00:00.000Z\",\"kind\":\"landed\",\"commit\":\"1111111111111111111111111111111111111111\",\"branch\":\"spec/y\",\"land_kind\":\"spec\",\"tier\":\"SOFT\",\"fingerprint\":\"f-r1\",\"changes\":[\"y\"]}\n";

/// The remote-tracking ledger ref `origin/clm/ledger/<writer>` holding `records`, as a fetch
/// would leave it.
fn ledger_ref(root: &Path, writer: &str, records: &str) {
    let git = |args: &[&str]| git_env(root, args, &[]);
    let file = root.join("records.jsonl.tmp");
    fs::write(&file, records).unwrap();
    let blob = git(&["hash-object", "-w", file.to_str().unwrap()]);
    fs::remove_file(&file).unwrap();
    let index = root.join(".git/ledger-index");
    let env = [("GIT_INDEX_FILE", index.to_str().unwrap())];
    let cacheinfo = format!("100644,{},records.jsonl", blob.trim());
    git_env(
        root,
        &["update-index", "--add", "--cacheinfo", &cacheinfo],
        &env,
    );
    let tree = git_env(root, &["write-tree"], &env);
    fs::remove_file(&index).unwrap();
    let commit = git(&["commit-tree", tree.trim(), "-m", "ledger"]);
    git(&[
        "update-ref",
        &format!("refs/remotes/origin/clm/ledger/{writer}"),
        commit.trim(),
    ]);
}

#[test]
fn status_has_no_review_section() {
    // req:view/status-report@538240fb
    let dir = base();
    let d = dir.path();
    claim_refs(d, &[]);
    ledger_ref(d, "a/1", &format!("{SOFT_LAND}not a record\n"));
    let out = clm_status(d);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8(out.stderr).unwrap(), "");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(!text.contains("review"), "{text}");
    assert!(text.contains("\n  waiting on you 0\n"), "{text}");
    assert!(text.ends_with("\nclaims\n  none\n\n"), "{text}");
}

/// `HEAD`'s hash cut to its first 7 hex digits.
fn head7(root: &Path) -> String {
    git_env(root, &["rev-parse", "HEAD"], &[])[..7].to_string()
}

/// `n` more commits on `HEAD`, then `HEAD` moved back to where it was: the commits stay
/// on `origin/main`, which `HEAD` is then `n` behind.
fn origin_ahead(root: &Path, n: usize) {
    let git = |args: &[&str]| git_env(root, args, &[]);
    let base = git(&["rev-parse", "HEAD"]);
    for i in 0..n {
        git(&["commit", "-q", "--allow-empty", "-m", &format!("later {i}")]);
    }
    git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(&["reset", "-q", "--hard", base.trim()]);
}

#[test]
fn the_commit_is_cut_to_seven_digits_whatever_git_abbreviates_to() {
    // req:view/status-report@538240fb
    let dir = base();
    let d = dir.path();
    claim_refs(d, &[]);
    // `--short` would follow this and print 12.
    git_env(d, &["config", "core.abbrev", "12"], &[]);
    let text = status(d);
    assert_eq!(
        text.lines().nth(1),
        Some(format!("  commit {}", head7(d)).as_str()),
        "{text}"
    );
}

#[test]
fn scenario_checkout_named() {
    // req:view/status-report@538240fb
    let dir = base();
    claim_refs(dir.path(), &[]);
    let text = status(dir.path());
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "CLM status", "{text}");
    assert_eq!(
        lines[1],
        format!("  commit {}", head7(dir.path())),
        "{text}"
    );
}

#[test]
fn scenario_checkout_behind() {
    // req:view/status-report@538240fb
    let dir = base();
    let d = dir.path();
    claim_refs(d, &[]);
    origin_ahead(d, 2);
    git_env(
        d,
        &[
            "remote",
            "add",
            "origin",
            "https://127.0.0.1:9/unreachable.git",
        ],
        &[],
    );
    let text = status(d);
    let line = text.lines().nth(1).unwrap();
    assert!(line.starts_with("  commit "), "{text}");
    assert!(line.ends_with("⚠ 2 behind origin/main"), "{text}");
    assert_eq!(
        line,
        format!("  commit {}  ⚠ 2 behind origin/main", head7(d))
    );
}

#[test]
fn scenario_no_origin() {
    // req:view/status-report@538240fb
    let dir = base();
    let d = dir.path();
    git_env(d, &["init", "-q", "-b", "main"], &[]);
    git_env(d, &["add", "-A"], &[]);
    git_env(d, &["commit", "-qm", "base"], &[]);
    let text = status(d);
    let line = text.lines().nth(1).unwrap();
    assert!(line.starts_with("  commit "), "{text}");
    assert!(!line.contains("behind"), "{text}");
}

#[test]
fn no_repository_names_no_commit() {
    // req:view/status-report@538240fb
    let dir = base();
    let text = status(dir.path());
    assert!(
        text.lines().nth(1).unwrap().starts_with("  components "),
        "{text}"
    );
    // A repository with no commit yet has no `HEAD` to name either.
    git_env(dir.path(), &["init", "-q", "-b", "main"], &[]);
    let text = status(dir.path());
    assert!(
        text.lines().nth(1).unwrap().starts_with("  components "),
        "{text}"
    );
}
