//! `clm view <name> --at <rev>`: a commit's views, read from the local objects (`view-at-rev`).

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::time::SystemTime;

use tempfile::TempDir;

const REQ: &str = concat!("req", ":");

/// `clm <args> --root <root>`, with no user settings.
fn clm(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .env(
            "XDG_CONFIG_HOME",
            Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-user-config"),
        )
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
        .unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";

/// A `specd` node depending on `deps` with requirements `reqs`.
fn node(deps: &str, reqs: &[&str]) -> String {
    let mut s = format!(
        "---\ntitle: T\nstatus: specd\nowners: []\ndepends_on: [{deps}]\n---\n\n## Requirements\n"
    );
    for r in reqs {
        s.push_str(&format!(
            "\n### Requirement: {r}\nIt SHALL.\n\n#### Scenario: s\n- ok\n"
        ));
    }
    s
}

fn git(dir: &Path, args: &[&str]) -> String {
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
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

/// Makes `root` a repository with one commit on `main`.
fn init(root: &Path) {
    git(root, &["init", "-q", "-b", "main"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "base"]);
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_str(&stdout(out)).unwrap_or_else(|e| panic!("{e}: {}", stdout(out)))
}

/// Node `a` with `a/x` uncited at `HEAD`, on branch `impl/a`, and `origin/main` one commit
/// ahead adding a test that cites it at its current version, as a fetch would leave it.
fn ahead_of_the_checkout() -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &node("", &["x"]));
    init(d);
    let (graph, _) = clm_model::load(d);
    let x = graph.requirement("a/x").unwrap().version().to_string();
    write(d, "tests/t.rs", &format!("// {REQ}a/x@{x}\n"));
    git(d, &["add", "-A"]);
    git(d, &["commit", "-qm", "cite"]);
    git(d, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(d, &["reset", "-q", "--hard", "HEAD~1"]);
    git(d, &["switch", "-qc", "impl/a"]);
    dir
}

/// `git status --porcelain` and the index's tree: what `--at` must leave as they were.
fn tree_state(d: &Path) -> (String, String) {
    (
        git(d, &["status", "--porcelain", "--untracked-files=all"]),
        git(d, &["write-tree"]),
    )
}

#[test]
fn scenario_ahead_of_the_checkout() {
    // req:view.repo/view-at-rev@55922294
    let dir = ahead_of_the_checkout();
    let d = dir.path();
    let before = tree_state(d);
    let here = stdout(&clm(d, &["view", "flight"]));
    assert!(here.contains("\n  a "), "{here}");
    let out = clm(d, &["view", "flight", "--at", "origin/main"]);
    let there = stdout(&out);
    assert_eq!(out.status.code(), Some(0), "{there}");
    assert!(there.contains("components\n  none\n"), "{there}");
    assert_eq!(tree_state(d), before);
    assert!(!d.join("tests/t.rs").exists());
}

#[test]
fn scenario_header() {
    // req:view.repo/view-at-rev@55922294
    let dir = ahead_of_the_checkout();
    let d = dir.path();
    fetch_head(
        &d.join(".git/FETCH_HEAD"),
        std::time::Duration::from_secs(200),
    );
    let hash = git(d, &["rev-parse", "origin/main"])[..7].to_string();
    let text = stdout(&clm(d, &["view", "spec", "--at", "origin/main"]));
    assert_eq!(
        text.lines().next(),
        Some(format!("origin/main {hash} · fetched 3m ago · checkout impl/a").as_str()),
        "{text}"
    );
    let v = json(&clm(
        d,
        &["view", "spec", "--at", "origin/main", "--format", "json"],
    ));
    assert_eq!(v.get("checkout"), None, "{v:#}");
    assert_eq!(v["at"]["rev"], "origin/main", "{v:#}");
    assert_eq!(v["at"]["commit"], hash, "{v:#}");
    assert_eq!(v["at"]["branch"], "impl/a", "{v:#}");
    // The model is that commit's: `a/x` is current there.
    assert_eq!(node_json(&v, "a")["current"], 1, "{v:#}");
}

/// Writes an empty `FETCH_HEAD` at `path` last modified `ago` before now.
fn fetch_head(path: &Path, ago: std::time::Duration) {
    fs::write(path, "").unwrap();
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(SystemTime::now() - ago)
        .unwrap();
}

#[test]
fn scenario_fetched_from_another_worktree() {
    // req:view.repo/view-at-rev@55922294
    let dir = ahead_of_the_checkout();
    let d = dir.path();
    git(d, &["switch", "-q", "--detach"]);
    let linked = TempDir::new().unwrap();
    let wt = linked.path().join("wt");
    git(
        d,
        &["worktree", "add", "-q", wt.to_str().unwrap(), "impl/a"],
    );
    let own = git(&wt, &["rev-parse", "--path-format=absolute", "--git-dir"]);
    let own = Path::new(own.trim()).join("FETCH_HEAD");
    let main = d.join(".git/FETCH_HEAD");
    fetch_head(&own, std::time::Duration::from_secs(30 * 3600));
    fetch_head(&main, std::time::Duration::from_secs(60));
    let text = stdout(&clm(&wt, &["view", "spec", "--at", "origin/main"]));
    let first = text.lines().next().unwrap();
    assert!(first.contains(" · fetched 1m ago"), "{text}");
    assert!(first.ends_with("· checkout impl/a"), "{text}");
    // The other way round: the main checkout sees the linked worktree's newer fetch.
    fetch_head(&main, std::time::Duration::from_secs(30 * 3600));
    fetch_head(&own, std::time::Duration::from_secs(120));
    let text = stdout(&clm(d, &["view", "spec", "--at", "origin/main"]));
    let first = text.lines().next().unwrap();
    assert!(first.contains(" · fetched 2m ago"), "{text}");
    assert!(first.ends_with("· checkout detached"), "{text}");
}

fn node_json<'v>(v: &'v serde_json::Value, id: &str) -> &'v serde_json::Value {
    v["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["node"] == id)
        .unwrap()
}

#[test]
fn scenario_detached_never_fetched() {
    // req:view.repo/view-at-rev@55922294
    let dir = ahead_of_the_checkout();
    let d = dir.path();
    git(d, &["switch", "-q", "--detach"]);
    // A linked worktree that never fetched either.
    let linked = TempDir::new().unwrap();
    let wt = linked.path().join("wt");
    git(
        d,
        &["worktree", "add", "-q", wt.to_str().unwrap(), "impl/a"],
    );
    let text = stdout(&clm(d, &["view", "spec", "--at", "origin/main"]));
    let first = text.lines().next().unwrap();
    assert!(first.ends_with("· checkout detached"), "{text}");
    assert!(!first.contains("fetched"), "{text}");
}

#[test]
fn scenario_no_such_commit() {
    // req:view.repo/view-at-rev@55922294
    let dir = ahead_of_the_checkout();
    let out = clm(dir.path(), &["view", "spec", "--at", "nowhere"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty(), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("nowhere"), "{err}");
}

#[test]
fn at_a_commit_sizes_by_its_own_settings() {
    // req:view.repo/view-at-rev@55922294
    let dir = ahead_of_the_checkout();
    let d = dir.path();
    // The working tree's settings mark `a`; the commit holds no `clm.toml`, so its
    // defaults apply and mark nothing.
    write(d, "clm.toml", "[component_size]\nspec_tokens = 1\n");
    let here = json(&clm(d, &["view", "spec", "--format", "json"]));
    let there = json(&clm(
        d,
        &["view", "spec", "--at", "origin/main", "--format", "json"],
    ));
    assert_eq!(node_json(&here, "a")["oversized"], true, "{here:#}");
    assert_eq!(node_json(&there, "a")["oversized"], false, "{there:#}");
    // A `clm.toml` the commit holds is read: it marks `a` there, not here.
    fs::remove_file(d.join("clm.toml")).unwrap();
    git(d, &["switch", "-q", "--detach", "origin/main"]);
    write(d, "clm.toml", "[component_size]\nspec_tokens = 1\n");
    git(d, &["add", "-A"]);
    git(d, &["commit", "-qm", "budgets"]);
    git(d, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(d, &["switch", "-q", "impl/a"]);
    let here = json(&clm(d, &["view", "spec", "--format", "json"]));
    let there = json(&clm(
        d,
        &["view", "spec", "--at", "origin/main", "--format", "json"],
    ));
    assert_eq!(node_json(&here, "a")["oversized"], false, "{here:#}");
    assert_eq!(node_json(&there, "a")["oversized"], true, "{there:#}");
}

#[test]
fn at_a_commit_below_the_repositorys_top() {
    // req:view.repo/view-at-rev@55922294
    // The CLM root is `clm/` in a larger repository whose top `.gitattributes` marks `a`'s
    // generated code, and which holds a binary blob citing `a/x`, and a backlog item.
    let dir = TempDir::new().unwrap();
    let top = dir.path();
    let d = top.join("clm");
    write(&d, "specs/SPEC.md", ROOT);
    let mut a = node("", &["x"]);
    a = a.replace("owners: []", "owners: []\npaths: [\"src/**\"]");
    write(&d, "specs/a/SPEC.md", &a);
    write(&d, "clm.toml", "[component_size]\ncode_tokens = 10\n");
    write(&d, "src/gen.rs", &"x".repeat(400));
    write(top, ".gitattributes", "clm/src/gen.rs linguist-generated\n");
    write(
        &d,
        "backlog/q.md",
        "---\ntitle: Q\nkind: idea\ncomponents: [a]\n---\nQ.\n",
    );
    let (graph, _) = clm_model::load(&d);
    let x = graph.requirement("a/x").unwrap().version().to_string();
    fs::write(d.join("blob.bin"), format!("\0// {REQ}a/x@{x}\n")).unwrap();
    init(top);
    git(top, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    // The working tree moves on; `--at` reads the commit.
    fs::remove_file(d.join("backlog/q.md")).unwrap();
    let v = json(&clm(
        &d,
        &["view", "spec", "--at", "origin/main", "--format", "json"],
    ));
    let a = node_json(&v, "a");
    assert_eq!(a["oversized"], false, "{v:#}");
    assert_eq!(a["current"], 0, "{v:#}");
    assert_eq!(a["backlog"][0]["slug"], "q", "{v:#}");
    let here = json(&clm(&d, &["view", "spec", "--format", "json"]));
    assert_eq!(node_json(&here, "a")["oversized"], false, "{here:#}");
}
