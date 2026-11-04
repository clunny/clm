//! Fixtures shared by the orchestrator tests.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use clm_check::Marker;
use clm_model::{Files, Graph};
use tempfile::TempDir;

#[path = "../../../clm-git/tests/exec/mod.rs"]
mod exec;
pub use exec::write_exec;

/// `command.output()` with no stdin, stdout and stderr captured.
pub fn output(command: &mut Command) -> Output {
    command.stdin(Stdio::null()).output().unwrap()
}

pub fn git_env(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> String {
    let out = output(
        Command::new("git")
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
            .current_dir(dir),
    );
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

pub fn git(dir: &Path, args: &[&str]) -> String {
    git_env(dir, args, &[])
}

pub fn write(dir: &Path, rel: &str, content: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// A `specd` node depending on `deps` (comma-separated) with requirements `reqs`, each
/// `It SHALL.`.
pub fn node(deps: &str, reqs: &[&str]) -> String {
    node_with(
        deps,
        "",
        &reqs.iter().map(|r| (*r, "It SHALL.")).collect::<Vec<_>>(),
    )
}

/// A `specd` node depending on `deps`, with `extra` frontmatter lines (such as `paths:`)
/// and requirements `(slug, text)`.
pub fn node_with(deps: &str, extra: &str, reqs: &[(&str, &str)]) -> String {
    let mut s = format!(
        "---\ntitle: T\nstatus: specd\nowners: []\ndepends_on: [{deps}]\n{extra}---\n\n\
         ## Contract\nc\n\n## Design\nd\n\n## Requirements\n"
    );
    for (r, text) in reqs {
        s.push_str(&format!(
            "\n### Requirement: {r}\n{text}\n\n#### Scenario: s\n- ok\n"
        ));
    }
    s
}

pub fn delta(node: &str, reqs: &[&str]) -> String {
    let mut s = format!("---\ncomponent: {node}\n---\n## ADDED Requirements\n");
    for r in reqs {
        s.push_str(&format!(
            "### Requirement: {r}\nIt SHALL.\n\n#### Scenario: s\n- ok\n\n"
        ));
    }
    s
}

pub fn change(status: &str) -> String {
    change_md(status, "")
}

/// `change.md` with `status`, and `extra` frontmatter lines.
pub fn change_md(status: &str, extra: &str) -> String {
    format!("---\nstatus: {status}\n{extra}---\n")
}

/// A graph loaded from `(path, text)` files, which must be valid.
pub fn graph(files: &[(&str, &str)]) -> Graph {
    let files: Files = files
        .iter()
        .map(|(p, t)| (PathBuf::from(p), t.as_bytes().to_vec()))
        .collect();
    let (graph, violations) = clm_model::load_files(&files);
    assert!(violations.is_empty(), "{violations:?}");
    graph
}

/// A marker citing `id` at `version` (`None` for a bare one) at `t.rs:<line>`.
pub fn marker(id: &str, version: Option<&str>, line: usize) -> Marker {
    Marker {
        id: id.into(),
        version: version.map(String::from),
        path: PathBuf::from("t.rs"),
        line,
    }
}

/// A marker citing `id` at its current version in `graph`.
pub fn current(graph: &Graph, id: &str) -> Marker {
    marker(id, Some(graph.requirement(id).unwrap().version()), 1)
}

/// A clone of a bare `origin` whose `main` has `a` (depends on `b`), `b` and `c`, and an
/// applied change `x` adding `a/p`, `a/q`, `b/r` and `c/s`.
pub struct Repo {
    pub _dirs: (TempDir, TempDir),
    pub origin: PathBuf,
    pub work: PathBuf,
}

pub fn repo() -> Repo {
    let (o, w) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let (origin, work) = (o.path().to_path_buf(), w.path().join("work"));
    git(&origin, &["init", "-q", "--bare", "-b", "main"]);
    git(
        o.path(),
        &[
            "clone",
            "-q",
            origin.to_str().unwrap(),
            work.to_str().unwrap(),
        ],
    );
    identify(&work);
    write(&work, "ARCHITECTURE.md", "# Arch\n");
    write(&work, "specs/SPEC.md", &node("", &[]));
    write(&work, "specs/a/SPEC.md", &node("b", &["p", "q"]));
    write(&work, "specs/b/SPEC.md", &node("", &["r"]));
    write(&work, "specs/c/SPEC.md", &node("", &["s"]));
    write(&work, "changes/x/change.md", &change("applied"));
    write(&work, "changes/x/deltas/a.md", &delta("a", &["p", "q"]));
    write(&work, "changes/x/deltas/b.md", &delta("b", &["r"]));
    write(&work, "changes/x/deltas/c.md", &delta("c", &["s"]));
    push_main(&work, "base");
    Repo {
        _dirs: (o, w),
        origin,
        work,
    }
}

/// `claim` commits as the user, so clones need an identity (CI runners have none).
pub fn identify(work: &Path) {
    git(work, &["config", "user.name", "t"]);
    git(work, &["config", "user.email", "t@t"]);
}

pub fn push_main(work: &Path, msg: &str) {
    git(work, &["add", "-A"]);
    git(work, &["commit", "-qm", msg]);
    git(work, &["push", "-q", "origin", "HEAD:main"]);
}

pub fn load(root: &Path) -> Graph {
    let (graph, violations) = clm_model::load(root);
    assert!(violations.is_empty(), "{violations:?}");
    graph
}

/// Writes `src/<file>` citing each of `ids` at its current version in the checkout. The
/// marker is built so this source never cites them itself.
pub fn cite(root: &Path, file: &str, ids: &[&str]) {
    let graph = load(root);
    let text: String = ids
        .iter()
        .map(|id| {
            format!(
                "// {}:{id}@{}\n",
                "req",
                graph.requirement(id).unwrap().version()
            )
        })
        .collect();
    write(root, &format!("src/{file}"), &text);
}

/// The wall clock, in Unix seconds.
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn hours_ago(h: u64) -> String {
    format!("@{} +0000", now() - h * 3600)
}

/// Pushes a commit on `main` to `impl/<node>` on origin, committed `age` hours ago.
pub fn push_claim(work: &Path, node: &str, age: u64) -> String {
    push_branch(work, &format!("impl/{node}"), age)
}

/// Pushes a commit on `main` to `propose/<slug>` on origin, committed `age` hours ago.
pub fn push_propose(work: &Path, slug: &str, age: u64) -> String {
    push_branch(work, &format!("propose/{slug}"), age)
}

/// Pushes an empty commit on `origin/main` to `branch` on origin, committed `age` hours
/// ago, and returns its SHA.
pub fn push_branch(work: &Path, branch: &str, age: u64) -> String {
    let date = hours_ago(age);
    let tree = git(work, &["rev-parse", "origin/main^{tree}"]);
    let main = git(work, &["rev-parse", "origin/main"]);
    let sha = git_env(
        work,
        &["commit-tree", tree.trim(), "-p", main.trim(), "-m", "work"],
        &[("GIT_COMMITTER_DATE", &date), ("GIT_AUTHOR_DATE", &date)],
    );
    git(
        work,
        &[
            "push",
            "-q",
            "origin",
            &format!("{}:refs/heads/{branch}", sha.trim()),
        ],
    );
    sha.trim().to_string()
}

/// Pushes to `branch` on origin a commit on `origin/main` adding `files`, through a
/// throwaway worktree so the checkout stays where it is, then fetches.
pub fn push_files(work: &Path, branch: &str, files: &[(&str, &str)]) {
    let tmp = TempDir::new().unwrap();
    let wt = tmp.path().join("wt");
    git(
        work,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            wt.to_str().unwrap(),
            "origin/main",
        ],
    );
    for (path, text) in files {
        write(&wt, path, text);
    }
    git(&wt, &["add", "-A"]);
    git(&wt, &["commit", "-qm", branch]);
    git(
        &wt,
        &["push", "-q", "origin", &format!("HEAD:refs/heads/{branch}")],
    );
    git(
        work,
        &["worktree", "remove", "--force", wt.to_str().unwrap()],
    );
    git(work, &["fetch", "-q", "origin"]);
}

/// A backlog item of kind `idea` with `nodes` (comma-separated).
pub fn item(nodes: &str) -> String {
    format!("---\ntitle: I\nkind: idea\ncomponents: [{nodes}]\n---\n")
}

/// Merges `origin/impl/<node>` into `origin/main` with a merge commit, as CI does.
pub fn merge_claim(work: &Path, node: &str) {
    git(work, &["fetch", "-q", "origin"]);
    let tree = git(work, &["rev-parse", "origin/main^{tree}"]);
    let main = git(work, &["rev-parse", "origin/main"]);
    let tip = git(work, &["rev-parse", &format!("origin/impl/{node}")]);
    let sha = git(
        work,
        &[
            "commit-tree",
            tree.trim(),
            "-p",
            main.trim(),
            "-p",
            tip.trim(),
            "-m",
            &format!("Merge impl/{node}"),
        ],
    );
    git(
        work,
        &[
            "push",
            "-q",
            "origin",
            &format!("{}:refs/heads/main", sha.trim()),
        ],
    );
    git(work, &["fetch", "-q", "origin"]);
}

pub fn remote_tip(r: &Repo, branch: &str) -> Option<String> {
    let out = output(
        Command::new("git")
            .args([
                "rev-parse",
                "--verify",
                "-q",
                &format!("refs/heads/{branch}"),
            ])
            .current_dir(&r.origin),
    );
    out.status
        .success()
        .then(|| String::from_utf8(out.stdout).unwrap().trim().to_string())
}

/// Installs `body` as `origin`'s `pre-receive` hook, logging each run to `hooks/runs`.
pub fn pre_receive(r: &Repo, body: &str) {
    write_exec(
        &r.origin.join("hooks/pre-receive"),
        &format!("#!/bin/sh\necho run >> \"$GIT_DIR/hooks/runs\"\n{body}"),
    );
}

pub fn hook_runs(r: &Repo) -> usize {
    fs::read_to_string(r.origin.join("hooks/runs"))
        .unwrap_or_default()
        .lines()
        .count()
}
