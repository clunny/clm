//! `clm draft` with a fake `claude` on PATH: it claims backlog items and prints the prompt
//! for an unattended proposing session, and starts no session.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use common::*;
use tempfile::TempDir;

/// A fake `claude` that records that it ran.
struct Fake {
    _dir: TempDir,
    bin: PathBuf,
    out: PathBuf,
}

fn fake() -> Fake {
    let dir = TempDir::new().unwrap();
    let (bin, out) = (dir.path().join("bin"), dir.path().join("out"));
    for d in [&bin, &out] {
        fs::create_dir_all(d).unwrap();
    }
    write_exec(&bin.join("claude"), "#!/bin/sh\ntouch \"$FAKE_OUT/ran\"\n");
    Fake {
        _dir: dir,
        bin,
        out,
    }
}

struct Ran {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn clm_draft(r: &Repo, f: &Fake, slugs: &[&str]) -> Ran {
    clm_draft_env(r, f, slugs, &[])
}

/// `clm draft <args>...` with `env` set, the user's settings out of reach.
fn clm_draft_env(r: &Repo, f: &Fake, args: &[&str], env: &[(&str, &str)]) -> Ran {
    let path = format!(
        "{}:{}",
        f.bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = output(
        Command::new(env!("CARGO_BIN_EXE_clm"))
            .arg("draft")
            .args(args)
            .arg("--root")
            .arg(&r.work)
            .current_dir(&r.work)
            .env("PATH", path)
            .env("FAKE_OUT", &f.out)
            .env("XDG_CONFIG_HOME", r.work.join(".no-config"))
            .envs(env.iter().copied()),
    );
    Ran {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// The repo with component `check` (requirement `x`), bug `a` (`components: [check]`, `P3`,
/// `blocks: [check/x]`) and bug `b` in `origin/main`'s backlog.
fn with_items() -> Repo {
    let r = repo();
    write(&r.work, "specs/check/SPEC.md", &node("", &["x"]));
    write(
        &r.work,
        "backlog/a.md",
        "---\ntitle: A\nkind: bug\ncomponents: [check]\npriority: P3\nblocks: [check/x]\n---\n\
         The body of item a.\n",
    );
    write(
        &r.work,
        "backlog/b.md",
        "---\ntitle: B\nkind: bug\n---\nBody b.\n",
    );
    push_main(&r.work, "items");
    r
}

fn claim_subject(r: &Repo, branch: &str) -> String {
    let tip = remote_tip(r, branch).unwrap_or_else(|| panic!("origin/{branch} exists"));
    git(&r.origin, &["log", "-1", "--format=%s", &tip])
        .trim()
        .to_string()
}

/// Whether `.git/info/exclude` holds `.clm/`.
fn excluded(r: &Repo) -> bool {
    fs::read_to_string(r.work.join(".git/info/exclude"))
        .is_ok_and(|t| t.lines().any(|l| l == ".clm/"))
}

/// Today's local date as `YYYYMMDD`.
fn today() -> String {
    let out = output(Command::new("date").arg("+%Y%m%d"));
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// The ID on the `draft <id>` line that ends stderr.
fn drafted_id(ran: &Ran) -> String {
    let last = ran.stderr.lines().last().unwrap_or_default();
    last.strip_prefix("draft ")
        .unwrap_or_else(|| panic!("last line: {}", ran.stderr))
        .to_string()
}

fn local_branches(r: &Repo) -> String {
    git(&r.work, &["branch", "--format=%(refname)"])
}

#[test]
fn scenario_items_claimed_and_prompt_written() {
    // req:orchestrator.triage/draft@908b8dd5
    let (r, f) = (with_items(), fake());
    let before = local_branches(&r);
    let worktrees = git(&r.work, &["worktree", "list", "--porcelain"]);
    let dates = [today()];
    let ran = clm_draft(&r, &f, &["a", "b"]);
    assert_eq!(
        git(&r.work, &["worktree", "list", "--porcelain"]),
        worktrees,
        "no worktree was made"
    );
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let id = drafted_id(&ran);
    let dates = [dates[0].clone(), today()];
    assert!(
        dates.iter().any(|d| id == format!("{d}-a")),
        "{id} on {dates:?}"
    );
    assert_eq!(claim_subject(&r, "propose/a"), "Claim backlog/a");
    assert_eq!(claim_subject(&r, "propose/b"), "Claim backlog/b");
    assert_eq!(local_branches(&r), before, "no local branch was created");
    assert!(excluded(&r), ".git/info/exclude holds .clm/");
    assert!(!f.out.join("ran").exists(), "claude never ran");
    let p = &prompt_of(&r, &ran);
    let item_a = p
        .lines()
        .find(|l| l.contains("`a`"))
        .unwrap_or_else(|| panic!("{p}"));
    for needle in ["bug", "P3", "check", "check/x"] {
        assert!(item_a.contains(needle), "{needle}: {item_a}");
    }
    for needle in [
        "The body of item a.",
        "Body b.",
        "git branch --show-current",
        &format!("git switch -c spec/{id} origin/main"),
        "resolves",
        "left <slug>",
        "git log -1 --diff-filter=A --format=%as origin/main -- backlog/<slug>.md",
        "changes/archive/",
        "already fixes",
        "a week before that",
        "or no delta at all",
        "delete its `backlog/<slug>.md` on `spec/<id>`",
        "run no `clm context`",
        "skip the design and the builder check",
        "git log --since=<week-before> --grep='Resolves: backlog/'",
        "`## Intent`",
        "docs/decisions/",
        "clm claim <component> --fix backlog/<slug>",
        "Resolves: backlog/<slug>",
        "clm hashes",
        "clm check",
        "clm apply",
        "rebase ok",
        "clm gate --base origin/main",
        "`human_burdens`",
        "gate/undeclared-burden",
        "`serves`",
        "#### Serves",
        "check/untraced",
        "holds an entry",
        "`spec review`",
        "`gate/approval`",
        "a burden is never removed",
        "should the gate's first line be `spec review`",
        ".clm/pr-body.md",
        "clm pr open spec/<id> --title <title> --body-file .clm/pr-body.md",
        "clm land spec/<id>",
        "autosync/spec/<id>",
        "re-applies the edited deltas",
        "clm defect",
        "landed <sha>",
        "proposed <id> review",
        "stopped:",
        "`priority: P3`",
        "`status: proposed`",
        "builder check",
        "`#### Needs`",
        "builds nothing",
        "name it in the relying requirement's `#### Needs`",
        "`dependency-work-is-a-requirement`",
        "When it is `spec review`, open no pull request and stop after your last push",
        "When it is `spec`, write the pull request body",
    ] {
        assert!(p.contains(needle), "{needle}:\n{p}");
    }
    for absent in [
        "superpowers",
        "gh pr create",
        "--body ",
        "gh pr edit",
        "clm watch",
        "clm approve",
        "CARGO_TARGET_DIR",
        "exits 0",
        "clm significant",
        "## Significant",
        "gate: ",
        "HARD",
        "SOFT",
        "AUTO",
    ] {
        assert!(!p.contains(absent), "{absent}:\n{p}");
    }
    let at = |s: &str| p.find(s).unwrap_or_else(|| panic!("{s}"));
    assert!(
        at("already fixes") < at("Read each item"),
        "the check comes before the job"
    );
}

#[test]
fn a_p5_draft_sets_no_priority() {
    // req:orchestrator.triage/draft@908b8dd5
    let (r, f) = (with_items(), fake());
    let ran = clm_draft(&r, &f, &["b"]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    // Only `b` was claimed, and it is P5, so the change sets no priority.
    let p = prompt_of(&r, &ran);
    assert!(!p.contains("`priority: P"), "{p}");
}

#[test]
fn scenario_one_item_refused() {
    // req:orchestrator.triage/draft@908b8dd5
    let (r, f) = (with_items(), fake());
    push_propose(&r.work, "b", 1);
    let ran = clm_draft(&r, &f, &["a", "b"]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let left = ran
        .stderr
        .lines()
        .find(|l| l.starts_with("left b: "))
        .unwrap_or_else(|| panic!("{}", ran.stderr));
    assert!(left.contains("propose/b"), "{left}");
    assert!(drafted_id(&ran).ends_with("-a"), "{}", ran.stderr);
    let p = prompt_of(&r, &ran);
    assert!(p.contains("`a`"), "{p}");
    assert!(!p.contains("`b`"), "{p}");
    assert!(!p.contains("Body b."), "{p}");
}

#[test]
fn scenario_nothing_claimed() {
    // req:orchestrator.triage/draft@908b8dd5
    let (r, f) = (with_items(), fake());
    push_propose(&r.work, "b", 1);
    let ran = clm_draft(&r, &f, &["b"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert_eq!(ran.stdout, "");
    assert!(ran.stderr.contains("propose/b"), "{}", ran.stderr);
}

#[test]
fn an_unknown_item_is_left() {
    // req:orchestrator.triage/draft@908b8dd5
    let (r, f) = (with_items(), fake());
    let ran = clm_draft(&r, &f, &["nope", "a"]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(ran.stderr.contains("left nope: "), "{}", ran.stderr);
}

#[test]
fn a_slug_given_twice_is_claimed_once() {
    // req:orchestrator.triage/draft@908b8dd5
    let (r, f) = (with_items(), fake());
    let ran = clm_draft(&r, &f, &["a", "a"]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert_eq!(ran.stderr.lines().count(), 1, "{}", ran.stderr);
    assert!(drafted_id(&ran).ends_with("-a"), "{}", ran.stderr);
    let p = prompt_of(&r, &ran);
    assert_eq!(p.matches("### `backlog/a.md`").count(), 1);
}

#[test]
fn scenario_cap_reached() {
    // req:orchestrator.triage/draft-takes-place@2865c8cd
    let (r, f) = (with_items(), fake());
    let placed =
        clm_orchestrator::take_place(&r.work, "x", clm_orchestrator::PlaceKind::Build, &[], 3, 4)
            .unwrap();
    assert_eq!(placed, clm_orchestrator::Placed::New);
    let ran = clm_draft_env(&r, &f, &["a"], &[("CLM_MAX_BUILDS", "1")]);
    assert_eq!(ran.code, Some(3), "{}", ran.stderr);
    assert_eq!(ran.stdout, "");
    assert!(
        ran.stderr.contains("1 of max_builds = 1 in use"),
        "{}",
        ran.stderr
    );
    assert!(remote_tip(&r, "propose/a").is_none(), "no claim");
}

#[test]
fn scenario_nothing_claimed_frees_the_place() {
    // req:orchestrator.triage/draft-takes-place@2865c8cd
    let (r, f) = (with_items(), fake());
    push_propose(&r.work, "b", 1);
    let ran = clm_draft(&r, &f, &["b"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert!(place_names(&r).is_empty(), "{:?}", place_names(&r));
}

fn place_names(r: &Repo) -> Vec<String> {
    clm_orchestrator::places(&r.work, 4)
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect()
}

/// `clm draft --finish <change>` run in the main worktree.
fn clm_finish(r: &Repo, change: &str) -> Ran {
    clm_draft_env(r, &fake(), &["--finish", change], &[])
}

/// A draft place `id`, and a worktree under `.claude/worktrees/<name>` on a local
/// `spec/<id>` cut from `origin/main`. Returns the worktree's canonical path.
fn draft_worktree(r: &Repo, id: &str, name: &str) -> PathBuf {
    let placed = clm_orchestrator::take_place(
        &r.work,
        id,
        clm_orchestrator::PlaceKind::Draft,
        &[format!("spec/{id}")],
        3,
        4,
    )
    .unwrap();
    assert_eq!(placed, clm_orchestrator::Placed::New);
    let wt = r.work.join(".claude/worktrees").join(name);
    git(
        &r.work,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            &format!("spec/{id}"),
            wt.to_str().unwrap(),
            "origin/main",
        ],
    );
    wt.canonicalize().unwrap()
}

#[test]
fn scenario_landed_draft_tidied() {
    // req:orchestrator.triage/draft-finish@dbfa74f2
    let r = with_items();
    let id = "20261027-x";
    let wt = draft_worktree(&r, id, "agent-2");
    git(&r.work, &["worktree", "lock", wt.to_str().unwrap()]);
    write(
        &r.work,
        &format!("changes/{id}/change.md"),
        &change("applied"),
    );
    push_main(&r.work, "landed");
    let ran = clm_finish(&r, id);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(place_names(&r).is_empty(), "{:?}", place_names(&r));
    assert!(!wt.exists(), "the landed worktree is gone");
    assert!(
        ran.stdout.contains(&format!("freed {id}")),
        "{}",
        ran.stdout
    );
    assert!(
        ran.stdout.contains(&format!("removed {}", wt.display())),
        "{}",
        ran.stdout
    );
}

#[test]
fn an_archived_draft_has_landed_and_a_dirty_worktree_stays() {
    // req:orchestrator.triage/draft-finish@dbfa74f2
    let r = with_items();
    let id = "20261027-z";
    let wt = draft_worktree(&r, id, "agent-3");
    fs::write(wt.join("notes.txt"), "mine\n").unwrap();
    write(
        &r.work,
        &format!("changes/archive/{id}/change.md"),
        &change("applied"),
    );
    push_main(&r.work, "archived");
    let ran = clm_finish(&r, id);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(wt.exists());
    assert!(
        ran.stdout
            .contains(&format!("kept {}: uncommitted changes", wt.display())),
        "{}",
        ran.stdout
    );
}

#[test]
fn scenario_folded_drafts_branches_deleted() {
    // req:orchestrator.triage/draft-finish@dbfa74f2
    let r = with_items();
    let id = "20261027-f";
    let wt = draft_worktree(&r, id, "agent-5");
    let file = format!("changes/{id}/notes.md");
    for branch in [format!("spec/{id}"), format!("autosync/spec/{id}")] {
        push_files(&r.work, &branch, &[(&file, &branch)]);
    }
    write(
        &r.work,
        &format!("changes/{id}/change.md"),
        &change("applied"),
    );
    push_main(&r.work, "folded");
    let ran = clm_finish(&r, id);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(!wt.exists(), "the worktree is gone");
    assert_eq!(
        git(&r.work, &["branch", "--list", &format!("spec/{id}")]).trim(),
        "",
        "the local branch is gone"
    );
    for branch in [format!("spec/{id}"), format!("autosync/spec/{id}")] {
        assert_eq!(remote_tip(&r, &branch), None, "{branch} on origin");
        assert!(
            ran.stdout.contains(&format!("removed origin/{branch}")),
            "{}",
            ran.stdout
        );
    }
    assert!(
        ran.stdout
            .lines()
            .any(|l| l == format!("removed spec/{id}")),
        "{}",
        ran.stdout
    );
}

#[test]
fn a_kept_worktree_keeps_its_branch() {
    // req:orchestrator.triage/draft-finish@dbfa74f2
    let r = with_items();
    let id = "20261027-k";
    let wt = draft_worktree(&r, id, "agent-6");
    fs::write(wt.join("notes.txt"), "mine\n").unwrap();
    push_files(&r.work, &format!("spec/{id}"), &[("x.md", "x")]);
    write(
        &r.work,
        &format!("changes/{id}/change.md"),
        &change("applied"),
    );
    push_main(&r.work, "folded");
    let ran = clm_finish(&r, id);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(wt.exists());
    assert_ne!(
        git(&r.work, &["branch", "--list", &format!("spec/{id}")]).trim(),
        "",
        "the checked-out branch stays"
    );
    // The remote branch has no worktree to lose, so it goes.
    assert_eq!(remote_tip(&r, &format!("spec/{id}")), None);
}

#[test]
fn a_local_branch_with_unpushed_commits_is_kept() {
    // req:orchestrator.triage/draft-finish@dbfa74f2
    let r = with_items();
    let id = "20261027-u";
    let wt = draft_worktree(&r, id, "agent-7");
    write(&wt, "mine.md", "mine\n");
    git(&wt, &["add", "-A"]);
    git(&wt, &["commit", "-qm", "unpushed"]);
    write(
        &r.work,
        &format!("changes/{id}/change.md"),
        &change("applied"),
    );
    push_main(&r.work, "landed");
    let ran = clm_finish(&r, id);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(!wt.exists(), "the clean worktree is gone");
    assert_ne!(
        git(&r.work, &["branch", "--list", &format!("spec/{id}")]).trim(),
        "",
        "the branch holding unpushed commits stays"
    );
    assert!(
        ran.stdout.contains(&format!("kept spec/{id}: ")),
        "{}",
        ran.stdout
    );
}

/// A clean worktree under `.claude/worktrees/<name>` on a new local `branch` cut from
/// `origin/main`, with no place. Returns its canonical path.
fn fold_worktree(r: &Repo, branch: &str, name: &str) -> PathBuf {
    let wt = r.work.join(".claude/worktrees").join(name);
    git(
        &r.work,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            branch,
            wt.to_str().unwrap(),
            "origin/main",
        ],
    );
    wt.canonicalize().unwrap()
}

/// Commits a file in `wt`, so its branch holds a commit `origin/main` lacks.
fn commit_in(wt: &std::path::Path, file: &str) {
    write(wt, file, "fold\n");
    git(wt, &["add", "-A"]);
    git(wt, &["commit", "-qm", file]);
}

/// Merges the local `branch` into `main` with a merge commit, as `clm land` does, and
/// pushes it, so its tip is the second parent of `origin/main`'s last merge commit.
fn land_merge(r: &Repo, branch: &str) {
    git(&r.work, &["merge", "--no-ff", "-q", "-m", "land", branch]);
    git(&r.work, &["push", "-q", "origin", "HEAD:main"]);
}

/// The local `branch`'s tip, or empty when there is none.
fn local_branch(r: &Repo, branch: &str) -> String {
    git(
        &r.work,
        &["branch", "--list", "--format=%(objectname)", branch],
    )
    .trim()
    .to_string()
}

#[test]
fn scenario_landed_fold_tidied() {
    // req:orchestrator.triage/draft-finish@dbfa74f2
    let r = with_items();
    let id = "20261027-a-fold";
    let branch = format!("spec/{id}");
    let wt = fold_worktree(&r, &branch, "agent-8");
    commit_in(&wt, "fold.md");
    land_merge(&r, &branch);
    push_files(&r.work, &format!("autosync/{branch}"), &[("s.md", "s")]);
    git(&r.work, &["worktree", "lock", wt.to_str().unwrap()]);
    let ran = clm_finish(&r, id);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(!wt.exists(), "the landed fold's worktree is gone");
    assert_eq!(local_branch(&r, &branch), "", "the local branch is gone");
    assert_eq!(remote_tip(&r, &format!("autosync/{branch}")), None);
    assert!(
        ran.stdout.contains(&format!("removed {}", wt.display())),
        "{}",
        ran.stdout
    );
}

#[test]
fn scenario_fold_not_landed() {
    // req:orchestrator.triage/draft-finish@dbfa74f2
    let r = with_items();
    let (b, c) = ("spec/20261027-b-fold", "spec/20261027-c-fold");
    let wt_b = fold_worktree(&r, b, "agent-9");
    commit_in(&wt_b, "b.md");
    let wt_c = fold_worktree(&r, c, "agent-10");
    let tips =
        |r: &Repo| [b, c].map(|br| git(&r.work, &["rev-parse", &format!("refs/heads/{br}")]));
    let before = tips(&r);
    for id in ["20261027-b-fold", "20261027-c-fold"] {
        let ran = clm_finish(&r, id);
        assert_eq!(ran.code, Some(0), "{}", ran.stderr);
        assert_eq!(ran.stdout, "", "{id}");
    }
    assert!(wt_b.exists() && wt_c.exists(), "both worktrees stay");
    assert_eq!(tips(&r), before, "both local branches stay");
}

#[test]
fn scenario_landed_fold_with_uncommitted_work() {
    // req:orchestrator.triage/draft-finish@dbfa74f2
    let r = with_items();
    let id = "20261027-d-fold";
    let branch = format!("spec/{id}");
    let wt = fold_worktree(&r, &branch, "agent-11");
    commit_in(&wt, "fold.md");
    land_merge(&r, &branch);
    fs::write(wt.join("fold.md"), "changed\n").unwrap();
    push_files(&r.work, &format!("autosync/{branch}"), &[("s.md", "s")]);
    push_files(&r.work, &branch, &[("ahead.md", "ahead")]);
    let remote =
        |r: &Repo| [branch.clone(), format!("autosync/{branch}")].map(|b| remote_tip(r, &b));
    let before = (remote(&r), local_branch(&r, &branch));
    let ran = clm_finish(&r, id);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(
        ran.stdout.contains(&format!("kept {}", wt.display())),
        "{}",
        ran.stdout
    );
    assert!(wt.exists(), "the worktree with uncommitted work stays");
    assert_eq!((remote(&r), local_branch(&r, &branch)), before);
}

#[test]
fn a_landed_fold_branch_on_origin_is_deleted_only_when_in_main() {
    // req:orchestrator.triage/draft-finish@dbfa74f2
    let r = with_items();
    let id = "20261027-e-fold";
    let branch = format!("spec/{id}");
    let wt = fold_worktree(&r, &branch, "agent-12");
    commit_in(&wt, "fold.md");
    git(&wt, &["push", "-q", "origin", &branch]);
    land_merge(&r, &branch);
    let ran = clm_finish(&r, id);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(!wt.exists());
    assert_eq!(remote_tip(&r, &branch), None, "{}", ran.stdout);
}

#[test]
fn scenario_draft_waiting_for_review() {
    // req:orchestrator.triage/draft-finish@dbfa74f2
    let r = with_items();
    let id = "20261027-y";
    let wt = draft_worktree(&r, id, "agent-4");
    push_files(
        &r.work,
        &format!("spec/{id}"),
        &[(&format!("changes/{id}/change.md"), &change("proposed"))],
    );
    let tip = remote_tip(&r, &format!("spec/{id}"));
    let ran = clm_finish(&r, id);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(place_names(&r).is_empty(), "{:?}", place_names(&r));
    assert!(wt.exists(), "the waiting draft's worktree stays");
    assert_eq!(remote_tip(&r, &format!("spec/{id}")), tip);
    assert_eq!(ran.stdout.trim(), format!("freed {id}"));
}

#[test]
fn finishing_a_draft_with_no_place_exits_0() {
    // req:orchestrator.triage/draft-finish@dbfa74f2
    let r = with_items();
    let ran = clm_finish(&r, "20261027-none");
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert_eq!(ran.stdout, "");
}

/// Writes the place `id` for a draft, taken `hours` ago, naming `branches`.
fn place_taken(r: &Repo, id: &str, hours: u64, branches: &[&str]) {
    let common = git(
        &r.work,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    );
    let dir = PathBuf::from(common.trim()).join("clm/builds");
    fs::create_dir_all(&dir).unwrap();
    let secs = format!("@{}", now() - hours * 3600);
    let out = output(Command::new("date").args(["-u", "-d", &secs, "+%Y-%m-%dT%H:%M:%SZ"]));
    let when = String::from_utf8(out.stdout).unwrap();
    let mut text = format!("kind draft\ntaken {}\n", when.trim());
    for b in branches {
        text.push_str(&format!("branch {b}\n"));
    }
    fs::write(dir.join(id), text).unwrap();
}

#[test]
fn scenario_worktree_left_behind_by_its_moved_branch() {
    // req:orchestrator.triage/draft-finish@dbfa74f2
    let r = with_items();
    let id = "20261027-m";
    let wt = draft_worktree(&r, id, "agent-m");
    git(&r.work, &["worktree", "lock", wt.to_str().unwrap()]);
    // The branch moves on to a child of its tip, made in another worktree.
    let other = TempDir::new().unwrap();
    let elsewhere = other.path().join("wt");
    let path = elsewhere.to_str().unwrap();
    git(
        &r.work,
        &["worktree", "add", "-q", "--detach", path, "origin/main"],
    );
    write(&elsewhere, "child.md", "child\n");
    git(&elsewhere, &["add", "-A"]);
    git(&elsewhere, &["commit", "-qm", "child"]);
    let to = format!("HEAD:refs/heads/spec/{id}");
    git(&elsewhere, &["push", "-q", "origin", &to]);
    let child = git(&elsewhere, &["rev-parse", "HEAD"]);
    git(
        &r.work,
        &["update-ref", &format!("refs/heads/spec/{id}"), child.trim()],
    );
    git(&r.work, &["worktree", "remove", "--force", path]);
    write(
        &r.work,
        &format!("changes/{id}/change.md"),
        &change("applied"),
    );
    push_main(&r.work, "landed");
    let ran = clm_finish(&r, id);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(
        ran.stdout.contains(&format!("removed {}", wt.display())),
        "{}",
        ran.stdout
    );
    assert!(!wt.exists(), "the worktree is gone");
    assert_eq!(local_branch(&r, &format!("spec/{id}")), "");
}

#[test]
fn scenario_staged_work_not_in_history_kept() {
    // req:orchestrator.triage/draft-finish@dbfa74f2
    let r = with_items();
    let id = "20261027-n";
    let wt = draft_worktree(&r, id, "agent-n");
    write(&wt, "staged.md", "staged\n");
    git(&wt, &["add", "staged.md"]);
    write(
        &r.work,
        &format!("changes/{id}/change.md"),
        &change("applied"),
    );
    push_main(&r.work, "landed");
    let ran = clm_finish(&r, id);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(
        ran.stdout.contains(&format!("kept {}", wt.display())),
        "{}",
        ran.stdout
    );
    assert_eq!(git(&wt, &["status", "--porcelain"]).trim(), "A  staged.md");
}

#[test]
fn scenario_landed_drafts_claims_released() {
    // req:orchestrator.triage/draft-finish@dbfa74f2
    let r = with_items();
    let id = "20261027-r";
    place_taken(
        &r,
        id,
        1,
        &[&format!("spec/{id}"), "propose/a", "propose/b"],
    );
    push_propose(&r.work, "a", 1);
    push_propose(&r.work, "b", 1);
    let resolves = change_md("applied", "resolves: [a]\n");
    write(&r.work, &format!("changes/{id}/change.md"), &resolves);
    push_main(&r.work, "landed");
    let ran = clm_finish(&r, id);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let lines: Vec<&str> = ran.stdout.lines().collect();
    let at = |s: &str| lines.iter().position(|l| *l == s);
    assert!(at("released propose/a").is_some(), "{}", ran.stdout);
    assert!(at("released propose/b").is_some(), "{}", ran.stdout);
    let freed = at(&format!("freed {id}"));
    assert!(at("released propose/b") < freed, "{}", ran.stdout);
    assert_eq!(remote_tip(&r, "propose/a"), None);
    assert_eq!(remote_tip(&r, "propose/b"), None);
}

#[test]
fn scenario_moved_items_claim_kept_while_the_change_waits() {
    // req:orchestrator.triage/draft-finish@dbfa74f2
    let r = with_items();
    let id = "20261027-w";
    place_taken(
        &r,
        id,
        1,
        &[&format!("spec/{id}"), "propose/a", "propose/b"],
    );
    push_propose(&r.work, "a", 1);
    push_propose(&r.work, "b", 1);
    let resolves = change_md("applied", "resolves: [a]\n");
    let moved = "---\ntitle: B\nkind: bug\ncomponents: [check]\n---\nBody b.\n";
    push_files(
        &r.work,
        &format!("spec/{id}"),
        &[
            (&format!("changes/{id}/change.md"), &resolves),
            ("backlog/b.md", moved),
            // An item the change resolves doesn't keep its claim, edited or not.
            ("backlog/a.md", moved),
        ],
    );
    let kept = remote_tip(&r, "propose/b");
    let ran = clm_finish(&r, id);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert_eq!(remote_tip(&r, "propose/b"), kept);
    assert_eq!(remote_tip(&r, "propose/a"), None);
}

#[test]
fn scenario_another_sessions_later_claim_kept() {
    // req:orchestrator.triage/draft-finish@dbfa74f2
    let r = with_items();
    let id = "20261027-s";
    place_taken(&r, id, 6, &[&format!("spec/{id}"), "propose/a"]);
    push_propose(&r.work, "a", 1);
    let kept = remote_tip(&r, "propose/a");
    let ran = clm_draft_env(&r, &fake(), &["--finish", id, "--expiry", "4"], &[]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(place_names(&r).is_empty(), "{:?}", place_names(&r));
    assert_eq!(remote_tip(&r, "propose/a"), kept);
}

#[test]
fn scenario_no_place_no_release() {
    // req:orchestrator.triage/draft-finish@dbfa74f2
    let r = with_items();
    let id = "20261027-q";
    push_propose(&r.work, "a", 1);
    write(
        &r.work,
        &format!("changes/{id}/change.md"),
        &change("applied"),
    );
    push_main(&r.work, "landed");
    let kept = remote_tip(&r, "propose/a");
    let ran = clm_finish(&r, id);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert_eq!(remote_tip(&r, "propose/a"), kept);
}

#[test]
fn clm_triage_prints_the_lines_and_exits_0() {
    // req:orchestrator.triage/triage@91b10ab7
    let triage = |root: &std::path::Path| {
        output(
            Command::new(env!("CARGO_BIN_EXE_clm"))
                .args(["triage", "--expiry", "4", "--root"])
                .arg(root),
        )
    };
    let r = with_items();
    let out = triage(&r.work);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "group bug:check [check] P3 1 a\ngroup bug: [] P5 0 b\n"
    );
    let empty = repo();
    let out = triage(&empty.work);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "");
}

#[test]
fn clm_draft_reads_relative_answers_from_where_it_runs() {
    // req:orchestrator.triage/draft-answers@ebde7319
    let r = with_items();
    write(&r.work, ".clm/answers.md", "b: keep it small\n");
    let ran = clm_draft(&r, &fake(), &["--answers", ".clm/answers.md", "b"]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let p = prompt_of(&r, &ran);
    assert!(
        p.contains("## The human's answers\n\nb: keep it small\n"),
        "{p}"
    );
    let ran = clm_draft(&r, &fake(), &["--answers", "missing.md", "a"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert_eq!(ran.stdout, "");
    assert!(ran.stderr.contains("missing.md"), "{}", ran.stderr);
    assert!(remote_tip(&r, "propose/a").is_none());
}

#[test]
fn clm_draft_reads_answers_from_a_subdirectory_it_runs_in() {
    // req:orchestrator.triage/draft-answers@ebde7319
    // With no `--root`, `clm` finds the repository above the subdirectory; a relative
    // path is still read against the subdirectory.
    let r = with_items();
    write(&r.work, "sub/answers.md", "b: from the subdirectory\n");
    write(&r.work, "answers.md", "b: from the root\n");
    let out = output(
        Command::new(env!("CARGO_BIN_EXE_clm"))
            .args(["draft", "--answers", "answers.md", "b"])
            .current_dir(r.work.join("sub"))
            .env("XDG_CONFIG_HOME", r.work.join(".no-config")),
    );
    let ran = Ran {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    };
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let p = prompt_of(&r, &ran);
    assert!(p.contains("b: from the subdirectory"), "{p}");
    assert!(!p.contains("b: from the root"), "{p}");
}

#[test]
fn clm_draft_answers_conflict_with_finish_and_fold() {
    // req:orchestrator.triage/draft-answers@ebde7319
    let r = with_items();
    write(&r.work, "answers.md", "x\n");
    for args in [
        &["--answers", "answers.md", "--finish", "20261027-a"][..],
        &["--answers", "answers.md", "--fold", "20261027-a"][..],
    ] {
        let ran = clm_draft(&r, &fake(), args);
        assert_eq!(ran.code, Some(2), "{args:?}: {}", ran.stderr);
        assert!(ran.stderr.contains("cannot be used with"), "{}", ran.stderr);
    }
}

/// The prompt file `clm draft` wrote for change `id`.
fn prompt_file(r: &Repo, id: &str) -> PathBuf {
    r.work.join(".clm/prompts/draft").join(format!("{id}.md"))
}

/// The prompt `clm draft` wrote for the change on stderr's last line.
fn prompt_of(r: &Repo, ran: &Ran) -> String {
    let file = prompt_file(r, &drafted_id(ran));
    fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()))
}

#[test]
fn scenario_launcher() {
    // req:orchestrator.triage/draft-prompt-file@d84d9985
    let r = repo();
    write(
        &r.work,
        "backlog/a.md",
        "---\ntitle: A\nkind: bug\n---\nbody-of-a\n",
    );
    push_main(&r.work, "items");
    let ran = clm_draft(&r, &fake(), &["a"]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let file = prompt_file(&r, &drafted_id(&ran)).canonicalize().unwrap();
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
    assert!(!ran.stdout.contains("body-of-a"), "{}", ran.stdout);
    assert!(fs::read_to_string(&file).unwrap().contains("body-of-a"));
}

#[test]
fn scenario_earlier_prompt_replaced() {
    // req:orchestrator.triage/draft-prompt-file@d84d9985
    let (r, f) = (with_items(), fake());
    let id = format!("{}-a", today());
    write(
        &r.work,
        &format!(".clm/prompts/draft/{id}.md"),
        "stale-earlier-prompt\n",
    );
    let ran = clm_draft(&r, &f, &["a"]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let text = prompt_of(&r, &ran);
    assert!(!text.contains("stale-earlier-prompt"), "{text}");
}

#[test]
fn scenario_no_prompts_directory() {
    // req:orchestrator.triage/draft-prompt-file@d84d9985
    let (r, f) = (with_items(), fake());
    write(&r.work, ".clm/prompts", "a file\n");
    let ran = clm_draft(&r, &f, &["a"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert_eq!(ran.stdout, "");
    assert!(ran.stderr.contains(".clm/prompts"), "{}", ran.stderr);
    assert!(remote_tip(&r, "propose/a").is_none(), "no claim");
    assert!(place_names(&r).is_empty(), "{:?}", place_names(&r));
}

#[test]
fn a_prompt_that_cant_be_written_frees_the_place() {
    // req:orchestrator.triage/draft-prompt-file@d84d9985
    let (r, f) = (with_items(), fake());
    // A directory where the file goes, for either date the run may see.
    let dates = [today(), today()];
    for d in &dates {
        fs::create_dir_all(r.work.join(format!(".clm/prompts/draft/{d}-a.md/x"))).unwrap();
    }
    let ran = clm_draft(&r, &f, &["a"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert_eq!(ran.stdout, "");
    assert!(ran.stderr.contains("-a.md"), "{}", ran.stderr);
    assert!(place_names(&r).is_empty(), "{:?}", place_names(&r));
}
