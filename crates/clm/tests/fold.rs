//! `clm draft --fold <id>...` with a fake `claude` on PATH: it writes the prompt for one
//! session that folds drafts needing review into one spec branch, and changes nothing itself.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use common::*;
use tempfile::TempDir;

struct Ran {
    code: Option<i32>,
    stdout: String,
    stderr: String,
    /// Whether the fake `claude` ran.
    claude_ran: bool,
}

/// `clm draft --fold <ids>...` in the clone, with a fake `claude` first on PATH.
fn fold(r: &Repo, ids: &[&str]) -> Ran {
    let dir = TempDir::new().unwrap();
    let bin = dir.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let ran = dir.path().join("ran");
    write_exec(
        &bin.join("claude"),
        &format!("#!/bin/sh\ntouch '{}'\n", ran.display()),
    );
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = output(
        Command::new(env!("CARGO_BIN_EXE_clm"))
            .args(["draft", "--fold"])
            .args(ids)
            .arg("--root")
            .arg(&r.work)
            .current_dir(&r.work)
            .env("PATH", path)
            .env("XDG_CONFIG_HOME", r.work.join(".no-config")),
    );
    Ran {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        claude_ran: ran.exists(),
    }
}

/// Pushes `origin/spec/<id>` holding `changes/<id>/change.md` with `status` and `extra`
/// frontmatter lines.
fn spec_branch(r: &Repo, id: &str, status: &str, extra: &str) {
    push_files(
        &r.work,
        &format!("spec/{id}"),
        &[(
            &format!("changes/{id}/change.md"),
            &format!(
                "{}\n## Intent\n\nIntent of {id}.\n",
                change_md(status, extra)
            ),
        )],
    );
}

fn prompt_file(r: &Repo, id: &str) -> PathBuf {
    r.work.join(".clm/prompts/fold").join(format!("{id}.md"))
}

fn prompt(r: &Repo, id: &str) -> String {
    let file = prompt_file(r, id);
    fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()))
}

fn remote_branches(r: &Repo) -> String {
    git(&r.origin, &["branch", "--format=%(refname)"])
}

fn local_branches(r: &Repo) -> String {
    git(&r.work, &["branch", "--format=%(refname)"])
}

#[test]
fn scenario_several_changes_folded() {
    // req:orchestrator.triage/fold@8b73785c
    let r = repo();
    spec_branch(&r, "20261027-a", "applied", "resolves: [x]\n");
    spec_branch(&r, "20261027-b", "applied", "");
    let (remote, local) = (remote_branches(&r), local_branches(&r));
    let ran = fold(&r, &["20261027-a", "20261027-b"]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let file = prompt_file(&r, "20261027-a").canonicalize().unwrap();
    assert!(ran.stdout.lines().count() <= 5, "{}", ran.stdout);
    assert!(
        ran.stdout.contains(&format!("`{}`", file.display())),
        "{}",
        ran.stdout
    );
    assert!(
        ran.stdout.contains("git branch --show-current"),
        "{}",
        ran.stdout
    );
    assert!(ran.stdout.contains("all of it"), "{}", ran.stdout);
    assert_eq!(local_branches(&r), local, "no local branch");
    assert_eq!(remote_branches(&r), remote, "no new branch on origin");
    assert!(
        !r.work.join(".git/clm/builds").exists() || {
            fs::read_dir(r.work.join(".git/clm/builds"))
                .unwrap()
                .count()
                == 0
        }
    );
    assert!(!ran.claude_ran, "claude never ran");
    let p = prompt(&r, "20261027-a");
    for needle in [
        "20261027-a",
        "20261027-b",
        "`x`",
        "git branch --show-current",
        "git switch -c spec/20261027-a-fold origin/main",
        "clm branch push",
        "git diff origin/main...origin/spec/20261027-a",
        "git diff origin/main...origin/spec/20261027-b",
        "git apply --3way",
        "rebase ok",
        "clm hashes",
        "clm check",
        "clm gate --base origin/main",
        "`spec review`",
        "`gate/approval`",
        "a burden is never removed",
        "declare one for it in the `human_burdens` of the change it concerns",
        "that the branch needs review",
        "builder check",
        "left <id>",
        "clm defect",
        ".clm/pr-body.md",
        "## Intent",
        "## Decisions",
        "## Open questions",
        "## CLM defects",
        "clm pr open spec/20261027-a-fold --title <title> --body-file .clm/pr-body.md",
        "proposed 20261027-a-fold review <url>",
        "stopped:",
        "You are running unattended",
        "Never push to, rebase or delete a folded change's branch",
        "keeps both changes' edits",
        "into the later change's delta",
        "back to `status: applied` before committing",
        "git restore --source origin/main --staged --worktree specs/",
        "`dependency-work-is-a-requirement`",
        "mkdir -p .clm",
    ] {
        assert!(p.contains(needle), "{needle}:\n{p}");
    }
    for absent in [
        "git merge",
        "superpowers",
        "gh pr create",
        "--body ",
        "gh pr edit",
        "clm watch",
        "clm approve",
        "clm land",
        "clm significant",
        "## Significant",
        "gate: ",
        "HARD",
        "SOFT",
        "AUTO",
        "CARGO_TARGET_DIR",
    ] {
        assert!(!p.contains(absent), "{absent}:\n{p}");
    }
}

#[test]
fn scenario_fold_branch_name_taken() {
    // req:orchestrator.triage/fold@8b73785c
    let r = repo();
    spec_branch(&r, "20261027-a", "applied", "");
    spec_branch(&r, "20261027-b", "applied", "");
    push_branch(&r.work, "spec/20261027-a-fold", 0);
    push_branch(&r.work, "autosync/spec/20261027-a-fold-2", 0);
    git(
        &r.work,
        &["branch", "spec/20261027-a-fold-3", "origin/main"],
    );
    let ran = fold(&r, &["20261027-a", "20261027-b"]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let p = prompt(&r, "20261027-a");
    assert!(
        p.contains("git switch -c spec/20261027-a-fold-4 origin/main"),
        "{p}"
    );
}

#[test]
fn scenario_one_change() {
    // req:orchestrator.triage/fold@8b73785c
    let r = repo();
    spec_branch(&r, "20261027-a", "applied", "resolves: [x]\n");
    let ran = fold(&r, &["20261027-a"]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let p = prompt(&r, "20261027-a");
    for needle in [
        "git branch --show-current",
        "`x`",
        "git show origin/spec/20261027-a:changes/20261027-a/change.md",
        "clm defect",
        ".clm/pr-body.md",
        "clm pr open spec/20261027-a --title <title> --body-file .clm/pr-body.md",
        "proposed 20261027-a review <url>",
    ] {
        assert!(p.contains(needle), "{needle}:\n{p}");
    }
    for absent in [
        "git merge",
        "git switch -c",
        "clm land",
        "gate: ",
        "HARD",
        "SOFT",
        "AUTO",
        "CARGO_TARGET_DIR",
    ] {
        assert!(!p.contains(absent), "{absent}:\n{p}");
    }
}

#[test]
fn scenario_missing_or_unapplied_change_left() {
    // req:orchestrator.triage/fold@8b73785c
    let r = repo();
    spec_branch(&r, "20261027-a", "applied", "");
    spec_branch(&r, "20261027-p", "proposed", "");
    // A branch with no change, and one whose frontmatter doesn't parse.
    push_branch(&r.work, "spec/20261027-n", 0);
    push_files(
        &r.work,
        "spec/20261027-q",
        &[("changes/20261027-q/change.md", "---\nstatus: [\n---\n")],
    );
    let ran = fold(
        &r,
        &[
            "20261027-a",
            "20261027-p",
            "20261027-z",
            "20261027-n",
            "20261027-q",
        ],
    );
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    for id in ["20261027-p", "20261027-z", "20261027-n", "20261027-q"] {
        assert!(
            ran.stderr.contains(&format!("left {id}: ")),
            "{id}: {}",
            ran.stderr
        );
    }
    let p = prompt(&r, "20261027-a");
    assert!(p.contains("20261027-a"), "{p}");
    for id in ["20261027-p", "20261027-z", "20261027-n", "20261027-q"] {
        assert!(!p.contains(id), "{id}:\n{p}");
    }
    // One change left, so nothing to merge.
    assert!(!p.contains("git switch -c"), "{p}");
}

#[test]
fn scenario_nothing_to_fold() {
    // req:orchestrator.triage/fold@8b73785c
    let r = repo();
    let ran = fold(&r, &["20261027-z"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert_eq!(ran.stdout, "");
    assert!(ran.stderr.contains("left 20261027-z: "), "{}", ran.stderr);
}

#[test]
fn changes_fold_in_the_order_given() {
    // req:orchestrator.triage/fold@8b73785c
    let r = repo();
    spec_branch(&r, "20261027-a", "applied", "");
    spec_branch(&r, "20261027-b", "applied", "");
    let ran = fold(&r, &["20261027-b", "20261027-a", "20261027-b"]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert_eq!(ran.stderr, "", "a repeated ID is folded once");
    let p = prompt(&r, "20261027-b");
    assert!(
        p.contains("git switch -c spec/20261027-b-fold origin/main"),
        "{p}"
    );
    let at = |id: &str| {
        p.find(&format!("git diff origin/main...origin/spec/{id}"))
            .unwrap()
    };
    assert!(at("20261027-b") < at("20261027-a"), "{p}");
    assert_eq!(p.matches("`Fold 20261027-b`").count(), 1, "{p}");
}

#[test]
fn the_launcher_names_the_file_by_its_absolute_path() {
    // req:orchestrator.triage/fold@8b73785c
    let r = repo();
    spec_branch(&r, "20261027-a", "applied", "");
    let out = output(
        Command::new(env!("CARGO_BIN_EXE_clm"))
            .args(["draft", "--fold", "20261027-a", "--root", "."])
            .current_dir(&r.work)
            .env("XDG_CONFIG_HOME", r.work.join(".no-config")),
    );
    assert_eq!(out.status.code(), Some(0));
    let file = prompt_file(&r, "20261027-a").canonicalize().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(&format!("`{}`", file.display())),
        "{stdout}"
    );
}

#[test]
fn an_earlier_fold_prompt_is_replaced_and_clm_is_excluded() {
    // req:orchestrator.triage/fold@8b73785c
    let r = repo();
    spec_branch(&r, "20261027-a", "applied", "");
    write(
        &r.work,
        ".clm/prompts/fold/20261027-a.md",
        "stale-earlier-prompt\n",
    );
    let ran = fold(&r, &["20261027-a"]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(!prompt(&r, "20261027-a").contains("stale-earlier-prompt"));
    let exclude = fs::read_to_string(r.work.join(".git/info/exclude")).unwrap();
    assert!(exclude.lines().any(|l| l == ".clm/"), "{exclude}");
}
