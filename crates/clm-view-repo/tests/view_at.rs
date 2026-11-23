//! `clm view <name> --at <rev>`: a commit's views, read from the local objects (`view-at-rev`).

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use std::time::SystemTime;

use common::{Output, clm};
use tempfile::TempDir;

const REQ: &str = concat!("req", ":");

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
    write(d, "tests/t.rs", &in_test(&format!("// {REQ}a/x@{x}\n")));
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
    // req:view.repo/view-at-rev@a1f0b81d
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
    // req:view.repo/view-at-rev@a1f0b81d
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
    // req:view.repo/view-at-rev@a1f0b81d
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
    // The other way round: the main checkout sees the newer fetch of a second linked
    // worktree whose directory was deleted without `git worktree prune`.
    let gone = linked.path().join("gone");
    git(
        d,
        &["worktree", "add", "-q", "--detach", gone.to_str().unwrap()],
    );
    let theirs = git(&gone, &["rev-parse", "--path-format=absolute", "--git-dir"]);
    let theirs = Path::new(theirs.trim()).join("FETCH_HEAD");
    fetch_head(&theirs, std::time::Duration::from_secs(120));
    fs::remove_dir_all(&gone).unwrap();
    assert!(theirs.exists());
    fetch_head(&main, std::time::Duration::from_secs(30 * 3600));
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
    // req:view.repo/view-at-rev@a1f0b81d
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
    // req:view.repo/view-at-rev@a1f0b81d
    let dir = ahead_of_the_checkout();
    let out = clm(dir.path(), &["view", "spec", "--at", "nowhere"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty(), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("nowhere"), "{err}");
    // A revision that begins with `-` is read as one, never as an option.
    for args in [
        &["view", "spec", "--at=--all"][..],
        &["pending", "--at=--all"][..],
        &["view", "spec", "--at", "--all"][..],
        &["pending", "--at", "--all"][..],
    ] {
        let out = clm(dir.path(), args);
        assert_eq!(out.status.code(), Some(1), "{args:?}: {out:?}");
        assert!(out.stdout.is_empty(), "{args:?}: {out:?}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("`--all`"), "{args:?}: {err}");
    }
}

#[test]
fn a_commit_it_cant_read() {
    // req:view.repo/view-at-rev@a1f0b81d
    // No repository: `HEAD` names nothing that can be read.
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &node("", &["x"]));
    for args in [
        &["view", "spec", "--at", "HEAD"][..],
        &["pending", "--at", "HEAD"][..],
    ] {
        let out = clm(d, args);
        assert_eq!(out.status.code(), Some(1), "{args:?}: {out:?}");
        assert!(out.stdout.is_empty(), "{args:?}: {out:?}");
        let err = stderr(&out);
        assert!(err.contains("`HEAD`"), "{args:?}: {err}");
        assert!(err.contains("not a git repository"), "{args:?}: {err}");
    }
    // A partial clone whose `origin/main` blobs were never fetched: refused, fetching none.
    let origin = ahead_of_the_checkout();
    let o = origin.path();
    git(o, &["switch", "-q", "main"]);
    git(o, &["merge", "-q", "--ff-only", "origin/main"]);
    git(o, &["config", "uploadpack.allowFilter", "true"]);
    git(o, &["config", "uploadpack.allowAnySHA1InWant", "true"]);
    let place = TempDir::new().unwrap();
    let clone = place.path().join("c");
    git(
        place.path(),
        &[
            "-c",
            "protocol.file.allow=always",
            "clone",
            "-q",
            "--filter=blob:none",
            "--no-checkout",
            &format!("file://{}", o.display()),
            clone.to_str().unwrap(),
        ],
    );
    let objects = || git(&clone, &["count-objects", "-v"]);
    let before = objects();
    let out = clm(&clone, &["view", "spec", "--at", "origin/main"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
    // Named, and why: the blob git couldn't read without fetching.
    let err = stderr(&out);
    assert!(err.starts_with("`origin/main`: "), "{err}");
    assert!(err.contains("cat-file"), "{err}");
    assert_eq!(objects(), before);
}

#[test]
fn at_a_commit_sizes_by_its_own_settings() {
    // req:view.repo/view-at-rev@a1f0b81d
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
    // A commit whose `clm.toml` doesn't parse: sized by nothing, said on stderr.
    git(d, &["switch", "-q", "--detach", "origin/main"]);
    write(d, "clm.toml", "[component_size\n");
    // `a`'s spec over the default budget: nothing marks it, not even the defaults.
    let big = format!("{}\n{}\n", node("", &["x"]), "word ".repeat(20000));
    write(d, "specs/a/SPEC.md", &big);
    git(d, &["add", "-A"]);
    git(d, &["commit", "-qm", "broken budgets"]);
    git(d, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(d, &["switch", "-q", "impl/a"]);
    let out = clm(
        d,
        &["view", "spec", "--at", "origin/main", "--format", "json"],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        stderr(&out).starts_with("size not checked: "),
        "{}",
        stderr(&out)
    );
    let there = json(&out);
    for n in there["nodes"].as_array().unwrap() {
        assert_ne!(n["oversized"], true, "{there:#}");
    }
}

#[test]
fn at_a_commit_below_the_repositorys_top() {
    // req:view.repo/view-at-rev@a1f0b81d
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
    write(&d, "src/vendor.rs", &"y".repeat(400));
    write(
        top,
        ".gitattributes",
        "clm/src/gen.rs linguist-generated\nclm/src/vendor.rs linguist-vendored\n",
    );
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
    // The working tree moves on; `--at` reads the commit, its `.gitattributes` included.
    fs::remove_file(d.join("backlog/q.md")).unwrap();
    fs::remove_file(top.join(".gitattributes")).unwrap();
    let at = || {
        json(&clm(
            &d,
            &["view", "spec", "--at", "origin/main", "--format", "json"],
        ))
    };
    let v = at();
    let a = node_json(&v, "a");
    assert_eq!(a["oversized"], false, "{v:#}");
    assert_eq!(a["current"], 0, "{v:#}");
    assert_eq!(a["backlog"][0]["slug"], "q", "{v:#}");
    // A 400-byte file that isn't UTF-8 and holds no NUL is text to git: it counts.
    let mut latin = vec![b'a'; 399];
    latin.push(0xe9);
    fs::write(d.join("src/latin.rs"), latin).unwrap();
    git(top, &["add", "clm/src/latin.rs"]);
    git(top, &["commit", "-qm", "latin"]);
    git(top, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    let v = at();
    assert_eq!(node_json(&v, "a")["oversized"], true, "{v:#}");
    let here = json(&clm(&d, &["view", "spec", "--format", "json"]));
    assert_eq!(node_json(&here, "a")["oversized"], true, "{here:#}");
}

fn stderr(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).unwrap()
}

#[test]
fn pending_behind_origin_main() {
    // req:view.repo/pending-names-its-commit@3e612c59
    let dir = ahead_of_the_checkout();
    let d = dir.path();
    let before = tree_state(d);
    let head = git(d, &["rev-parse", "HEAD"])[..7].to_string();
    let main = git(d, &["rev-parse", "origin/main"])[..7].to_string();
    let here = clm(d, &["pending"]);
    assert_eq!(here.status.code(), Some(0), "{here:?}");
    // Exactly the list `check.coverage` gives: `a/x <version> unbuilt`.
    let (graph, _) = clm_model::load(d);
    let x = graph.requirement("a/x").unwrap().version().to_string();
    assert_eq!(stdout(&here), format!("a/x {x} unbuilt\n"));
    assert_eq!(stdout(&here), pending_lines(d, None));
    assert_eq!(
        stderr(&here),
        format!("commit {head}  ⚠ 1 behind origin/main\n")
    );
    let there = clm(d, &["pending", "--at", "origin/main"]);
    assert_eq!(there.status.code(), Some(0), "{there:?}");
    assert_eq!(stdout(&there), "");
    let err = stderr(&there);
    assert!(err.starts_with(&format!("origin/main {main}")), "{err}");
    assert!(!err.contains('⚠'), "{err}");
    // The same first line the chart of that commit begins with.
    let chart = stdout(&clm(d, &["view", "spec", "--at", "origin/main"]));
    assert_eq!(err, format!("{}\n", chart.lines().next().unwrap()));
    assert!(err.trim_end().ends_with("· checkout impl/a"), "{err}");
    assert_eq!(tree_state(d), before);
}

/// What `check.coverage` lists as pending for the working tree of `d`, one line each.
fn pending_lines(d: &Path, component: Option<&str>) -> String {
    clm_check::pending_in(d, component)
        .unwrap()
        .iter()
        .map(|p| format!("{p}\n"))
        .collect()
}

#[test]
fn pending_names_no_commit_without_head() {
    // req:view.repo/pending-names-its-commit@3e612c59
    // No repository, and then a repository with no commit yet: no `HEAD` to name.
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &node("", &["x"]));
    for _ in 0..2 {
        let out = clm(d, &["pending"]);
        assert_eq!(out.status.code(), Some(0), "{out:?}");
        assert_eq!(stdout(&out), pending_lines(d, None));
        assert!(!stdout(&out).is_empty());
        assert!(!stderr(&out).contains("commit"), "{}", stderr(&out));
        git(d, &["init", "-q", "-b", "main"]);
    }
}

#[test]
fn pending_at_narrows_to_a_component() {
    // req:view.repo/pending-names-its-commit@3e612c59
    // Two pending components at the commit: `--at` narrows to the one named, as without it,
    // and `g`, which an applied change at that commit removed, is refused as one it doesn't hold.
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &node("", &["x"]));
    write(d, "specs/b/SPEC.md", &node("", &["y"]));
    init(d);
    let here = clm(d, &["pending", "a"]);
    let there = clm(d, &["pending", "a", "--at", "HEAD"]);
    assert_eq!(there.status.code(), Some(0), "{there:?}");
    assert_eq!(stdout(&there), pending_lines(d, Some("a")));
    assert!(stdout(&there).starts_with("a/x "), "{}", stdout(&there));
    assert!(!stdout(&there).contains("b/y"), "{}", stdout(&there));
    assert_eq!(stdout(&here), stdout(&there));
    write(
        d,
        "changes/r/change.md",
        "---\nstatus: applied\n---\n\nIntent.\n",
    );
    write(
        d,
        "changes/r/deltas/g.md",
        "---\ncomponent: g\nremove: true\n---\n## REMOVED Requirements\n\n### Requirement: y <!-- base: 0123456789abcdef -->\n",
    );
    git(d, &["add", "-A"]);
    git(d, &["commit", "-qm", "remove g"]);
    let here = clm(d, &["pending", "g"]);
    let there = clm(d, &["pending", "g", "--at", "HEAD"]);
    assert_eq!(there.status.code(), Some(1), "{there:?}");
    assert!(
        stdout(&there).contains("unknown component `g`"),
        "{}",
        stdout(&there)
    );
    assert_eq!(stdout(&here), stdout(&there));
}

#[test]
fn pending_up_to_date() {
    // req:view.repo/pending-names-its-commit@3e612c59
    let dir = ahead_of_the_checkout();
    let d = dir.path();
    git(d, &["merge", "-q", "--ff-only", "origin/main"]);
    let head = git(d, &["rev-parse", "HEAD"])[..7].to_string();
    let out = clm(d, &["pending"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "");
    assert_eq!(stderr(&out), format!("commit {head}\n"));
}

#[test]
fn pending_no_such_commit() {
    // req:view.repo/pending-names-its-commit@3e612c59
    let dir = ahead_of_the_checkout();
    let out = clm(dir.path(), &["pending", "--at", "nowhere"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty(), "{out:?}");
    assert!(stderr(&out).contains("nowhere"), "{}", stderr(&out));
}

#[test]
fn pending_names_its_commit_beside_violations_and_narrows_alike() {
    // req:view.repo/pending-names-its-commit@3e612c59
    // req:view.repo/arguments-and-exit@c4b91292
    let dir = ahead_of_the_checkout();
    let d = dir.path();
    let head = git(d, &["rev-parse", "HEAD"])[..7].to_string();
    // An unknown component is the same violation with or without `--at`.
    let here = clm(d, &["pending", "ghost"]);
    let there = clm(d, &["pending", "ghost", "--at", "origin/main"]);
    assert_eq!(here.status.code(), Some(1), "{here:?}");
    assert_eq!(stdout(&here), stdout(&there));
    assert!(stdout(&here).contains("ghost"), "{}", stdout(&here));
    assert!(stderr(&here).starts_with(&format!("commit {head}")));
    assert!(stderr(&there).starts_with("origin/main "));
    // A graph that doesn't load: violations on stdout, the commit line still on stderr.
    write(d, "specs/a/SPEC.md", "not a spec");
    let bad = clm(d, &["pending"]);
    assert_eq!(bad.status.code(), Some(1), "{bad:?}");
    assert!(!stdout(&bad).is_empty());
    assert!(stderr(&bad).starts_with(&format!("commit {head}")));
    git(d, &["add", "-A"]);
    git(d, &["commit", "-qm", "bad"]);
    git(d, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    let bad = clm(d, &["pending", "--at", "origin/main"]);
    assert_eq!(bad.status.code(), Some(1), "{bad:?}");
    assert!(!stdout(&bad).is_empty());
    assert!(stderr(&bad).starts_with("origin/main "), "{}", stderr(&bad));
    // `clm view --at` on that commit prints the violations too, and no chart.
    let view = clm(d, &["view", "spec", "--at", "origin/main"]);
    assert_eq!(view.status.code(), Some(1), "{view:?}");
    assert_eq!(stdout(&view), stdout(&bad));
}

/// `s` as a Rust file whose markers count as a test's: those on its third line on are inside
/// a test, the earlier ones directly above it. Every line of `s` keeps its line number.
fn in_test(s: &str) -> String {
    let (head, body) = match s.strip_prefix('\n') {
        Some(rest) => ("", rest),
        None => s.split_once("\n\n").map_or((s, ""), |(a, b)| (a, b)),
    };
    let nl = if body.ends_with('\n') || body.is_empty() {
        ""
    } else {
        "\n"
    };
    if s.starts_with('\n') {
        format!("#[test]\nfn t() {{{body}{nl}}}\n")
    } else if body.is_empty() {
        format!("{s}#[test]\nfn t() {{}}\n")
    } else {
        format!("{head}\n#[test]\nfn t() {{{body}{nl}}}\n")
    }
}

#[test]
fn at_a_commit_changes_from_its_tree_and_claims_from_the_refs() {
    // req:view.repo/view-at-rev@a1f0b81d
    // `HEAD` holds a live change the working tree has deleted, and `origin/impl/a` claims
    // `a`: `--at HEAD` lists the commit's change, and both list the claim.
    let dir = ahead_of_the_checkout();
    let d = dir.path();
    write(
        d,
        "changes/c/change.md",
        "---\nstatus: applied\n---\n\nIntent.\n",
    );
    write(
        d,
        "changes/c/deltas/a.md",
        "---\ncomponent: a\n---\n## ADDED Requirements\n\n### Requirement: y\nIt SHALL.\n\n#### Scenario: s\n- ok\n",
    );
    git(d, &["add", "-A"]);
    git(d, &["commit", "-qm", "change"]);
    git(d, &["update-ref", "refs/remotes/origin/impl/a", "HEAD"]);
    fs::remove_dir_all(d.join("changes")).unwrap();
    let hash = git(d, &["rev-parse", "HEAD"])[..7].to_string();
    let here = stdout(&clm(d, &["view", "flight"]));
    let there = stdout(&clm(d, &["view", "flight", "--at", "HEAD"]));
    // Without `--at`, the checkout line `view/view-command` defines.
    assert!(here.starts_with(&format!("commit {hash} ")), "{here}");
    assert!(here.contains("\n  a  P5  claimed "), "{here}");
    assert!(here.contains("changes\n  none\n"), "{here}");
    assert!(there.starts_with(&format!("HEAD {hash} ")), "{there}");
    assert!(there.contains("\n  a  P5  claimed "), "{there}");
    assert!(there.contains("\n    c  P5 "), "{there}");
}
