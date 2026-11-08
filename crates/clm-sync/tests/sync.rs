//! `clm sync` and `clm autosync` against a bare `origin` and a work clone.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clm_sync::{Error, Report, autosync};
use tempfile::TempDir;

#[path = "../../clm-git/tests/exec/mod.rs"]
mod exec;
use exec::write_exec;

/// `config`'s default `health` settings, as the `clm` crate would pass them in; `clm sync`
/// reads only `reconsider` from them.
fn health() -> clm_health::Settings {
    clm_health::Settings {
        reconsider: 3,
        scenarios: 6,
        exceptions: 4,
        stale_days: 30,
        lcov: None,
        component_size: clm_check::Budgets {
            spec_tokens: 12000,
            code_tokens: 40000,
        },
    }
}

/// `clm sync` with the default `health` settings.
fn sync(root: &Path) -> Result<Report, Error> {
    clm_sync::sync(root, |_: &Path| Ok(health()))
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

fn write(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn read(dir: &Path, rel: &str) -> String {
    fs::read_to_string(dir.join(rel)).unwrap()
}

fn commit(dir: &Path, msg: &str) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-qm", msg]);
}

/// Commits only `rel`, leaving anything else uncommitted as it is.
fn commit_only(dir: &Path, rel: &str, msg: &str) {
    git(dir, &["add", "--", rel]);
    git(dir, &["commit", "-qm", msg]);
}

fn requirement(slug: &str) -> String {
    format!("\n### Requirement: {slug}\nIt SHALL {slug}.\n\n#### Scenario: s\n- ok\n")
}

/// Node `a` with Contract `contract` and requirements `reqs`, at `status`.
fn node_a(status: &str, contract: &str, reqs: &[&str]) -> String {
    let mut s = format!(
        "---\ntitle: A\nstatus: {status}\nowners: []\n---\n\n## Contract\n{contract}\n\n\
         ## Design\nd\n\n## Requirements\n"
    );
    for r in reqs {
        s.push_str(&requirement(r));
    }
    s
}

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n";

struct Repo {
    _dirs: (TempDir, TempDir),
    origin: PathBuf,
    work: PathBuf,
}

/// A clone of a bare `origin` whose `main` has node `a` with requirement `p`, and HEAD on a
/// fresh branch `branch`.
fn repo(branch: &str) -> Repo {
    let (o, w) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let (origin, work) = (o.path().to_path_buf(), w.path().join("work"));
    git(&origin, &["init", "-q", "--bare", "-b", "main"]);
    git(
        w.path(),
        &[
            "clone",
            "-q",
            origin.to_str().unwrap(),
            work.to_str().unwrap(),
        ],
    );
    git(&work, &["config", "user.name", "t"]);
    git(&work, &["config", "user.email", "t@t"]);
    write(&work, "specs/SPEC.md", ROOT);
    write(&work, "specs/a/SPEC.md", &node_a("specd", "c", &["p"]));
    write(&work, "src/lib.rs", "one\ntwo\nthree\n");
    commit(&work, "base");
    git(&work, &["push", "-q", "origin", "HEAD:main"]);
    git(&work, &["checkout", "-q", "-b", branch]);
    Repo {
        _dirs: (o, w),
        origin,
        work,
    }
}

/// Commits `files` on `origin/main` from a second clone, as another PR merging would.
fn main_moves(r: &Repo, files: &[(&str, &str)]) {
    let other = r.work.with_file_name("other");
    if !other.exists() {
        git(
            r.work.parent().unwrap(),
            &["clone", "-q", r.origin.to_str().unwrap(), "other"],
        );
        git(&other, &["config", "user.name", "o"]);
        git(&other, &["config", "user.email", "o@o"]);
    }
    git(&other, &["pull", "-q", "origin", "main"]);
    for (rel, text) in files {
        write(&other, rel, text);
    }
    commit(&other, "main moves");
    git(&other, &["push", "-q", "origin", "HEAD:main"]);
}

fn origin_main(r: &Repo) -> String {
    git(&r.origin, &["rev-parse", "main"])
}

fn origin_refs(r: &Repo) -> String {
    git(&r.origin, &["for-each-ref"])
}

/// Runs `clm sync`, asserting the branch came out up to date and passing; its lines.
fn synced(r: &Repo) -> Vec<String> {
    let report = sync(&r.work).unwrap();
    assert!(report.ok, "{:?}", report.lines);
    report.lines
}

/// Runs `clm sync`, asserting something was left for the caller; its lines.
fn unfinished(r: &Repo) -> Vec<String> {
    let report = sync(&r.work).unwrap();
    assert!(!report.ok, "{:?}", report.lines);
    report.lines
}

fn rebasing(r: &Repo) -> bool {
    let path = git(&r.work, &["rev-parse", "--git-path", "rebase-merge"]);
    r.work.join(path).exists()
}

/// A spec branch whose change `y` has `delta` for node `a`, applied with `clm apply` as
/// `propose` does.
fn applied_spec_branch(r: &Repo, delta: &str) {
    apply_change(r, "y", "", delta);
}

/// Adds change `id`, with `extra` frontmatter lines and `delta` for node `a`, applies it and
/// commits.
fn apply_change(r: &Repo, id: &str, extra: &str, delta: &str) {
    write(
        &r.work,
        &format!("changes/{id}/change.md"),
        &format!("---\nstatus: proposed\n{extra}---\n\n# {id}\n"),
    );
    write(&r.work, &format!("changes/{id}/deltas/a.md"), delta);
    clm_apply::apply(&r.work, id).unwrap();
    commit(&r.work, &format!("Apply {id}"));
}

fn modifies_contract(r: &Repo, text: &str) -> String {
    let base = clm_model::load(&r.work).0.nodes["a"]
        .hashes
        .contract
        .clone();
    format!(
        "---\ncomponent: a\nbase: {{ contract: \"{base}\" }}\n---\n## MODIFIED Contract\n{text}\n"
    )
}

fn adds_x() -> String {
    format!(
        "---\ncomponent: a\n---\n## ADDED Requirements\n{}",
        requirement("x")
    )
}

#[test]
fn behind_main() {
    // req:sync/rebase@3ba762e9
    let r = repo("topic");
    write(&r.work, "src/new.rs", "new\n");
    commit(&r.work, "work");
    main_moves(&r, &[("src/other.rs", "other\n")]);
    let main = origin_main(&r);
    let refs = origin_refs(&r);

    assert_eq!(synced(&r), Vec::<String>::new());
    assert_eq!(git(&r.work, &["rev-parse", "HEAD^"]), main);
    assert_eq!(git(&r.work, &["log", "-1", "--format=%s"]), "work");
    assert_eq!(origin_refs(&r), refs, "sync never pushes");
}

#[test]
fn real_conflict_then_resume() {
    // req:sync/rebase@3ba762e9
    let r = repo("topic");
    write(&r.work, "src/lib.rs", "one\nTWO\nthree\n");
    commit(&r.work, "work");
    main_moves(&r, &[("src/lib.rs", "one\n2\nthree\n")]);

    assert_eq!(unfinished(&r), ["conflict src/lib.rs"]);
    assert!(rebasing(&r), "the rebase is left in progress");

    // Resolved and staged, a second run continues the rebase rather than restarting it.
    write(&r.work, "src/lib.rs", "one\nTWO and 2\nthree\n");
    git(&r.work, &["add", "src/lib.rs"]);
    assert_eq!(synced(&r), Vec::<String>::new());
    assert!(!rebasing(&r));
    assert_eq!(read(&r.work, "src/lib.rs"), "one\nTWO and 2\nthree\n");
    assert_eq!(git(&r.work, &["rev-parse", "HEAD^"]), origin_main(&r));
}

#[test]
fn refuses_a_dirty_tree_main_and_a_detached_head() {
    // req:sync/rebase@3ba762e9
    let r = repo("impl/a");
    main_moves(&r, &[("src/other.rs", "other\n")]);
    write(&r.work, "src/lib.rs", "edited\n");
    assert_refused(&r, &["uncommitted src/lib.rs"]);

    git(&r.work, &["checkout", "-q", "--", "src/lib.rs"]);
    git(&r.work, &["checkout", "-q", "main"]);
    assert!(matches!(sync(&r.work), Err(Error::Refused(m)) if m.contains("main")));
    git(&r.work, &["checkout", "-q", "--detach", "HEAD"]);
    assert!(matches!(sync(&r.work), Err(Error::Refused(m)) if m.contains("detached")));
}

/// The working tree's and the index's state, contents included.
fn tree_state(r: &Repo) -> [String; 4] {
    [
        git(&r.work, &["for-each-ref", "refs/heads"]),
        git(&r.work, &["status", "--porcelain", "--untracked-files=all"]),
        git(&r.work, &["diff"]),
        git(&r.work, &["diff", "--cached"]),
    ]
}

/// Runs `clm sync`, asserting it refused with `lines` and changed nothing but `origin`'s
/// remote-tracking refs: no branch, index or working-tree change, nothing under `.git/clm/`.
fn assert_refused(r: &Repo, lines: &[&str]) {
    let before = tree_state(r);
    assert_eq!(unfinished(r), lines);
    assert_eq!(tree_state(r), before);
    let clm = git(&r.work, &["rev-parse", "--git-path", "clm"]);
    assert!(!r.work.join(clm).exists(), "nothing under .git/clm/");
    assert_eq!(
        git(&r.work, &["rev-parse", "origin/main"]),
        origin_main(r),
        "it fetched before refusing"
    );
}

#[test]
fn an_edit_to_a_file_main_changed_stops_it() {
    // req:sync/rebase@3ba762e9
    let r = repo("spec/x");
    main_moves(&r, &[("src/lib.rs", "one\n2\nthree\n")]);
    write(&r.work, "src/lib.rs", "one\nTWO\nthree\n");
    assert_refused(&r, &["uncommitted src/lib.rs"]);
}

#[test]
fn an_unrelated_edit_is_carried_through() {
    // req:sync/rebase@3ba762e9
    let r = repo("spec/x");
    write(&r.work, ".claude/settings.json", "{}\n");
    commit(&r.work, "settings");
    main_moves(&r, &[("src/other.rs", "other\n")]);
    write(&r.work, ".claude/settings.json", "{\"mine\": 1}\n");
    let stashes = git(&r.work, &["stash", "list"]);

    assert_eq!(synced(&r), Vec::<String>::new());
    assert_eq!(git(&r.work, &["rev-parse", "HEAD^"]), origin_main(&r));
    assert_eq!(read(&r.work, ".claude/settings.json"), "{\"mine\": 1}\n");
    assert_eq!(
        git(&r.work, &["status", "--porcelain"]),
        "M .claude/settings.json",
        "unstaged"
    );
    assert_eq!(
        git(&r.work, &["show", "HEAD:.claude/settings.json"]),
        "{}",
        "no commit holds it"
    );
    assert_eq!(git(&r.work, &["stash", "list"]), stashes);
}

#[test]
fn an_uncommitted_spec_edit_stops_it() {
    // req:sync/rebase@3ba762e9
    let r = repo("spec/x");
    write(&r.work, "backlog/q.md", "---\ntitle: Q\nkind: idea\n---\n");
    commit(&r.work, "q");
    main_moves(&r, &[("src/other.rs", "other\n")]);
    write(&r.work, "backlog/q.md", "---\ntitle: Q2\nkind: idea\n---\n");
    assert_refused(&r, &["uncommitted backlog/q.md"]);
}

#[test]
fn an_uncommitted_change_edit_stops_it() {
    // req:sync/rebase@3ba762e9
    let r = repo("spec/x");
    write(&r.work, "changes/x/change.md", "draft\n");
    commit(&r.work, "x");
    main_moves(&r, &[("src/other.rs", "other\n")]);
    write(&r.work, "changes/x/change.md", "draft 2\n");
    assert_refused(&r, &["uncommitted changes/x/change.md"]);
}

#[test]
fn a_staged_change_stops_it() {
    // req:sync/rebase@3ba762e9
    let r = repo("spec/x");
    write(&r.work, ".claude/settings.json", "{}\n");
    commit(&r.work, "settings");
    main_moves(&r, &[("src/other.rs", "other\n")]);
    write(&r.work, ".claude/settings.json", "{\"mine\": 1}\n");
    git(&r.work, &["add", ".claude/settings.json"]);
    assert_refused(&r, &["uncommitted .claude/settings.json"]);
}

#[test]
fn without_a_merge_base_every_edit_stops_it() {
    // req:sync/rebase@3ba762e9
    let r = repo("spec/x");
    git(&r.work, &["checkout", "-q", "--orphan", "spec/y"]);
    commit(&r.work, "unrelated history");
    write(&r.work, "src/lib.rs", "edited\n");
    assert_refused(&r, &["uncommitted src/lib.rs"]);
}

#[test]
fn an_edit_is_set_aside_during_a_conflict() {
    // req:sync/rebase@3ba762e9
    let r = repo("spec/x");
    write(&r.work, ".claude/settings.json", "{}\n");
    write(&r.work, "src/lib.rs", "one\nTWO\nthree\n");
    commit(&r.work, "work");
    main_moves(&r, &[("src/lib.rs", "one\n2\nthree\n")]);
    write(&r.work, ".claude/settings.json", "{\"mine\": 1}\n");

    assert_eq!(unfinished(&r), ["conflict src/lib.rs"]);
    write(&r.work, "src/lib.rs", "one\nTWO and 2\nthree\n");
    git(&r.work, &["add", "src/lib.rs"]);
    assert_eq!(synced(&r), Vec::<String>::new());
    assert_eq!(read(&r.work, ".claude/settings.json"), "{\"mine\": 1}\n");
    assert_eq!(
        git(&r.work, &["status", "--porcelain"]),
        "M .claude/settings.json"
    );
}

#[test]
fn untracked_output_and_worktrees_dont_stop_it() {
    // req:sync/rebase@3ba762e9
    let r = repo("topic");
    write(&r.work, "src/new.rs", "new\n");
    commit(&r.work, "work");
    main_moves(&r, &[("src/other.rs", "other\n")]);
    write(&r.work, "test-results/report.txt", "report\n");
    write(&r.work, ".claude/worktrees/other/README.md", "readme\n");

    assert_eq!(synced(&r), Vec::<String>::new());
    assert_eq!(git(&r.work, &["rev-parse", "HEAD^"]), origin_main(&r));
    assert_eq!(read(&r.work, "test-results/report.txt"), "report\n");
    assert_eq!(
        read(&r.work, ".claude/worktrees/other/README.md"),
        "readme\n"
    );
    assert_eq!(
        git(&r.work, &["status", "--porcelain"]),
        "?? .claude/\n?? test-results/"
    );
}

#[test]
fn an_untracked_delta_stops_it() {
    // req:sync/rebase@3ba762e9
    let r = repo("topic");
    main_moves(&r, &[("src/other.rs", "other\n")]);
    write(&r.work, "changes/x/deltas/a.md", "delta\n");
    let head = git(&r.work, &["rev-parse", "HEAD"]);
    let status = git(&r.work, &["status", "--porcelain"]);

    assert_eq!(unfinished(&r), ["uncommitted changes/x/deltas/a.md"]);
    assert_eq!(git(&r.work, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&r.work, &["status", "--porcelain"]), status);
}

#[test]
fn uncommitted_work_is_named_path_by_path() {
    // req:sync/rebase@3ba762e9
    let r = repo("topic");
    // An ignored file under `specs/` isn't work in progress.
    write(&r.work, ".git/info/exclude", "*.tmp\n");
    write(&r.work, "specs/a/scratch.tmp", "x\n");
    git(&r.work, &["mv", "src/lib.rs", "src/moved.rs"]);
    write(&r.work, "src/staged.rs", "s\n");
    git(&r.work, &["add", "src/staged.rs"]);
    fs::remove_file(r.work.join("specs/SPEC.md")).unwrap();
    write(&r.work, "specs/b/SPEC.md", "new\n");

    assert_eq!(
        unfinished(&r),
        [
            "uncommitted specs/SPEC.md",
            "uncommitted specs/b/SPEC.md",
            "uncommitted src/lib.rs",
            "uncommitted src/moved.rs",
            "uncommitted src/staged.rs",
        ]
    );
}

#[test]
fn an_untracked_file_main_adds_fails_naming_it() {
    // req:sync/rebase@3ba762e9
    let r = repo("topic");
    main_moves(&r, &[("test-results/report.txt", "theirs\n")]);
    write(&r.work, "test-results/report.txt", "mine\n");

    assert!(matches!(sync(&r.work), Err(Error::Git(m)) if m.contains("test-results/report.txt")));
    assert_eq!(read(&r.work, "test-results/report.txt"), "mine\n");
}

#[test]
fn applied_specs_are_regenerated_on_the_new_main() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/y");
    applied_spec_branch(&r, &adds_x());
    // `main` adds `a/y` to the same file, so a text merge of `specs/` would conflict.
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "c", &["p", "y"]))],
    );

    assert_eq!(synced(&r), Vec::<String>::new());
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(
        spec.contains("Requirement: x") && spec.contains("Requirement: y"),
        "{spec}"
    );
    assert!(!rebasing(&r));
    assert_eq!(git(&r.work, &["status", "--porcelain"]), "");
    assert_eq!(git(&r.work, &["log", "-1", "--format=%s"]), "Re-apply y");
    assert!(clm_check::run(&r.work).is_empty());
    assert!(read(&r.work, "changes/y/change.md").contains("status: applied"));
}

/// A delta of `a` that rewrites its Design to `text`, with base `base`.
fn design_delta(base: &str, text: &str) -> String {
    format!("---\ncomponent: a\nbase: {{ design: \"{base}\" }}\n---\n## MODIFIED Design\n{text}\n")
}

fn design_hash(r: &Repo) -> String {
    let graph = clm_model::load(&r.work).0;
    clm_apply::hashes(&graph.nodes["a"])
        .into_iter()
        .find(|(e, _)| e == "design")
        .unwrap()
        .1
}

#[test]
fn a_change_built_on_another_sorts_first() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/x");
    let three = node_a("specd", "c", &["p"]).replace("## Design\nd", "## Design\none\ntwo\nthree");
    main_moves(&r, &[("specs/a/SPEC.md", &three)]);
    git(&r.work, &["pull", "-q", "--rebase", "origin", "main"]);
    let h1 = design_hash(&r);
    apply_change(&r, "x", "", &design_delta(&h1, "X1\ntwo\nthree"));
    let h2 = design_hash(&r);
    // `w` sorts before `x` but is based on the Design after `x`.
    write(
        &r.work,
        "changes/w/change.md",
        "---\nstatus: applied\n---\n\n# w\n",
    );
    write(
        &r.work,
        "changes/w/deltas/a.md",
        &design_delta(&h2, "X1\nW2\nthree"),
    );
    commit(&r.work, "Add w");
    let b = "---\ntitle: B\nstatus: specd\nowners: []\n---\n\n## Contract\nb\n\n## Design\nd\n\n## Requirements\n";
    main_moves(
        &r,
        &[("specs/b/SPEC.md", &format!("{b}{}", requirement("q")))],
    );

    let report = sync(&r.work).unwrap();
    let out = report.lines.join("\n");
    assert!(!out.contains("apply/stale-base"), "{out}");
    assert!(!out.contains("changed design of a"), "{out}");
    assert!(report.ok, "{out}");
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(spec.contains("## Design\nX1\nW2\nthree\n"), "{spec}");
    assert!(read(&r.work, "specs/b/SPEC.md").contains("Requirement: q"));
}

#[test]
fn applied_assets_survive_regeneration() {
    // req:sync/reapply@01d24eb6 req:apply/apply-assets@786adb62
    let r = repo("spec/y");
    main_moves(&r, &[("specs/a/assets/old.png", "png")]);
    git(&r.work, &["pull", "-q", "--rebase", "origin", "main"]);
    let old = clm_model::asset_hash(b"png");
    write(&r.work, "changes/y/assets/a/m.html", "<p>m</p>\n");
    applied_spec_branch(
        &r,
        &format!("---\ncomponent: a\n---\n## REMOVED Assets\n- `old.png` <!-- base: {old} -->\n"),
    );
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "c", &["p", "y"]))],
    );

    assert_eq!(synced(&r), Vec::<String>::new());
    assert_eq!(read(&r.work, "specs/a/assets/m.html"), "<p>m</p>\n");
    assert!(!r.work.join("specs/a/assets/old.png").exists());
    assert!(read(&r.work, "specs/a/SPEC.md").contains("Requirement: y"));
    assert_eq!(git(&r.work, &["status", "--porcelain"]), "");
}

#[test]
fn a_stale_delta_stops_before_writing_specs() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/y");
    let graph = clm_model::load(&r.work).0;
    let base = graph.nodes["a"].hashes.contract.clone();
    let delta = format!(
        "---\ncomponent: a\nbase: {{ contract: \"{base}\" }}\n---\n## MODIFIED Contract\nmine\n"
    );
    applied_spec_branch(&r, &delta);
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "theirs", &["p"]))],
    );

    let left = unfinished(&r);
    assert!(
        left.iter().any(|l| l.contains("apply/stale-base")),
        "{left:?}"
    );
    // The rebase kept `main`'s `specs/`; the regenerate step wrote nothing over it.
    let main_spec = git(&r.origin, &["show", "main:specs/a/SPEC.md"]);
    assert_eq!(read(&r.work, "specs/a/SPEC.md").trim(), main_spec);
    assert_ne!(git(&r.work, &["log", "-1", "--format=%s"]), "Re-apply y");
}

#[test]
fn a_finish_branch_keeps_its_status_edit() {
    // req:sync/reapply@01d24eb6
    let r = repo("impl/y/finish");
    write(&r.work, "specs/a/SPEC.md", &node_a("active", "c", &["p"]));
    commit(&r.work, "finish");
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "c", &["p", "y"]))],
    );

    assert_eq!(synced(&r), Vec::<String>::new());
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(spec.contains("status: active"), "{spec}");
    assert!(spec.contains("Requirement: y"), "{spec}");
    assert_eq!(git(&r.work, &["log", "-1", "--format=%s"]), "finish");
}

#[test]
fn an_applied_change_already_on_main_is_not_regenerated() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/y");
    applied_spec_branch(&r, &adds_x());
    git(&r.work, &["push", "-q", "origin", "HEAD:main"]);
    // Its finish branch: `y` is on `main`, so this branch doesn't add it.
    git(&r.work, &["checkout", "-q", "-b", "impl/y/finish"]);
    let spec = read(&r.work, "specs/a/SPEC.md").replace("status: specd", "status: active");
    write(&r.work, "specs/a/SPEC.md", &spec);
    commit(&r.work, "finish");
    main_moves(&r, &[("src/other.rs", "other\n")]);

    assert_eq!(synced(&r), Vec::<String>::new());
    assert_eq!(read(&r.work, "specs/a/SPEC.md"), spec);
    assert_eq!(git(&r.work, &["log", "-1", "--format=%s"]), "finish");
}

#[test]
fn a_change_main_archived_is_not_reapplied() {
    // req:sync/reapply@01d24eb6
    let r = repo("topic");
    applied_spec_branch(&r, &adds_x());
    git(&r.work, &["push", "-q", "origin", "HEAD:main"]);
    // The branch is cut while `y` is `applied` on `main`, and does unrelated work.
    write(&r.work, "src/new.rs", "new\n");
    commit(&r.work, "work");
    // `main` then builds and archives `y`, as its task and `finish` PRs do.
    main_moves(&r, &[("src/x.rs", concat!("// req", ":a/x\n"))]);
    let other = r.work.with_file_name("other");
    fs::create_dir_all(other.join("changes/archive")).unwrap();
    git(&other, &["mv", "changes/y", "changes/archive/y"]);
    let md = read(&other, "changes/archive/y/change.md").replace("status: applied", "status: done");
    write(&other, "changes/archive/y/change.md", &md);
    commit(&other, "Archive y");
    git(&other, &["push", "-q", "origin", "HEAD:main"]);

    assert_eq!(synced(&r), Vec::<String>::new());
    assert_eq!(git(&r.work, &["log", "-1", "--format=%s"]), "work");
    let main_spec = git(&r.origin, &["show", "main:specs/a/SPEC.md"]);
    assert_eq!(read(&r.work, "specs/a/SPEC.md").trim(), main_spec);
}

#[test]
fn a_change_main_removed_is_not_reapplied() {
    // req:sync/reapply@01d24eb6
    let r = repo("topic");
    applied_spec_branch(&r, &adds_x());
    git(&r.work, &["push", "-q", "origin", "HEAD:main"]);
    write(&r.work, "src/new.rs", "new\n");
    commit(&r.work, "work");
    // `main` drops `y` outright, leaving `specs/` as it was before `y`.
    main_moves(&r, &[("specs/a/SPEC.md", &node_a("specd", "c", &["p"]))]);
    let other = r.work.with_file_name("other");
    git(&other, &["rm", "-rq", "changes/y"]);
    commit(&other, "Drop y");
    git(&other, &["push", "-q", "origin", "HEAD:main"]);

    assert_eq!(synced(&r), Vec::<String>::new());
    assert_eq!(git(&r.work, &["log", "-1", "--format=%s"]), "work");
    assert!(!read(&r.work, "specs/a/SPEC.md").contains("Requirement: x"));
    assert!(!r.work.join("changes/y").exists());
}

#[test]
fn a_branch_without_an_applied_change_never_regenerates() {
    // req:sync/reapply@01d24eb6
    let r = repo("topic");
    // A proposed change and a hand edit to `specs/`: neither may be regenerated away.
    write(
        &r.work,
        "changes/y/change.md",
        "---\nstatus: proposed\n---\n\n# y\n",
    );
    write(&r.work, "changes/y/deltas/a.md", &adds_x());
    write(&r.work, "specs/a/SPEC.md", &node_a("specd", "c2", &["p"]));
    commit(&r.work, "work");
    main_moves(&r, &[("src/other.rs", "other\n")]);

    assert_eq!(synced(&r), Vec::<String>::new());
    assert!(read(&r.work, "specs/a/SPEC.md").contains("c2"));
    assert!(!read(&r.work, "specs/a/SPEC.md").contains("Requirement: x"));
    assert_eq!(git(&r.work, &["log", "-1", "--format=%s"]), "work");
}

#[test]
fn the_result_is_judged_by_check() {
    // req:sync/judged-by-check@5c30baa7
    let r = repo("topic");
    // The marker is built so this file never cites `a/p` itself.
    write(
        &r.work,
        "src/test.rs",
        &format!("// {}\n", concat!("req", ":a/p")),
    );
    commit(&r.work, "test a/p");
    main_moves(&r, &[("specs/a/SPEC.md", &node_a("specd", "c", &[]))]);

    let left = unfinished(&r);
    assert!(!rebasing(&r), "the rebase completed");
    assert!(
        left.iter().any(|l| l.contains("check/dangling-markers")),
        "{left:?}"
    );
}

/// The version `id` has in the work clone's checkout.
fn version(r: &Repo, id: &str) -> String {
    let (graph, violations) = clm_model::load(&r.work);
    assert!(violations.is_empty(), "{violations:?}");
    graph.requirement(id).unwrap().version().to_string()
}

/// A test file citing `id` at `version`, built so this file never cites it itself.
fn citing(id: &str, version: &str) -> String {
    format!("// {}:{id}@{version}\n", "req")
}

/// Node `a`'s SPEC.md with requirement `a/p` modified.
fn a_with_p_modified(r: &Repo) -> String {
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(spec.contains("It SHALL p."), "{spec}");
    spec.replace("It SHALL p.", "It SHALL p, and more.")
}

/// Brings the work clone's branch up to `origin/main` with plain git.
fn pull(r: &Repo) {
    git(&r.work, &["pull", "-q", "--rebase", "origin", "main"]);
}

/// The line that tells the session what to read after `spec changed a`.
fn names_pending(line: &str) {
    assert!(
        line.contains("clm pending a") && line.contains("git diff"),
        "{line}"
    );
}

#[test]
fn spec_changed_when_a_requirement_changed_on_main() {
    // req:sync/spec-changed@14245755
    let r = repo("impl/a");
    write(&r.work, "src/new.rs", "new\n");
    commit(&r.work, "work");
    let old = version(&r, "a/p");
    main_moves(&r, &[("specs/a/SPEC.md", &a_with_p_modified(&r))]);

    let left = unfinished(&r);
    assert_eq!(git(&r.work, &["rev-parse", "HEAD^"]), origin_main(&r));
    let new = version(&r, "a/p");
    assert_ne!(old, new);
    assert_eq!(
        left[..3],
        [
            "spec changed a".to_string(),
            "changed specs/a/SPEC.md".into(),
            format!("requirement a/p {old} -> {new}"),
        ]
    );
    names_pending(&left[3]);
    assert_eq!(left.len(), 4, "{left:?}");
}

#[test]
fn spec_changed_is_reported_with_the_check_result() {
    // req:sync/spec-changed@14245755
    let r = repo("impl/a");
    // A marker nothing defines: `clm check` must still run and report it (step 7).
    write(&r.work, "src/test.rs", &format!("// {}:a/zz\n", "req"));
    commit(&r.work, "dangling");
    main_moves(&r, &[("specs/a/SPEC.md", &a_with_p_modified(&r))]);

    let left = unfinished(&r);
    assert_eq!(left[0], "spec changed a");
    assert!(
        left.iter().any(|l| l.contains("check/dangling-markers")),
        "{left:?}"
    );
}

#[test]
fn quiet_on_the_next_run() {
    // req:sync/spec-changed@14245755
    let r = repo("impl/a");
    main_moves(&r, &[("specs/a/SPEC.md", &a_with_p_modified(&r))]);
    assert!(!sync(&r.work).unwrap().ok);

    assert_eq!(synced(&r), Vec::<String>::new());
}

#[test]
fn a_run_stopped_by_a_stale_delta_still_owes_the_report() {
    // req:sync/spec-changed@14245755
    let r = repo("impl/a");
    apply_change(&r, "y", "", &modifies_contract(&r, "mine"));
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "theirs", &["p"]))],
    );
    let left = unfinished(&r);
    assert!(
        left.iter().any(|l| l.contains("apply/stale-base")),
        "{left:?}"
    );
    assert!(
        !left.iter().any(|l| l.starts_with("spec changed")),
        "{left:?}"
    );

    // The session drops its change rather than rebase its delta.
    git(&r.work, &["rm", "-rq", "changes/y"]);
    git(&r.work, &["checkout", "-q", "origin/main", "--", "specs"]);
    commit(&r.work, "drop y");
    let left = unfinished(&r);
    assert_eq!(left[..2], ["spec changed a", "changed specs/a/SPEC.md"]);
}

#[test]
fn requirements_that_appeared_or_went() {
    // req:sync/spec-changed@14245755
    let r = repo("impl/a");
    let old = version(&r, "a/p");
    main_moves(&r, &[("specs/a/SPEC.md", &node_a("specd", "c", &["y"]))]);

    let left = unfinished(&r);
    let new = version(&r, "a/y");
    assert_eq!(
        left[..4],
        [
            "spec changed a".to_string(),
            "changed specs/a/SPEC.md".into(),
            format!("requirement a/p {old} -> removed"),
            format!("requirement a/y added -> {new}"),
        ]
    );
}

const B: &str = "---\ntitle: B\nstatus: active\nowners: []\n---\n\n## Contract\nb\n";

/// Makes `a` depend on a new node `b` on `origin/main`, and pulls.
fn a_depends_on_b(r: &Repo) {
    let a = node_a("specd", "c", &["p"]).replace("owners: []\n", "owners: []\ndepends_on: [b]\n");
    main_moves(r, &[("specs/b/SPEC.md", B), ("specs/a/SPEC.md", &a)]);
    pull(r);
}

#[test]
fn dependency_contract_changed() {
    // req:sync/spec-changed@14245755
    let r = repo("impl/a");
    a_depends_on_b(&r);
    main_moves(&r, &[("specs/b/SPEC.md", &B.replace("\nb\n", "\nb2\n"))]);

    let left = unfinished(&r);
    assert_eq!(left[..2], ["spec changed a", "changed specs/b/SPEC.md"]);
    names_pending(&left[2]);
    assert_eq!(left.len(), 3, "{left:?}");
}

#[test]
fn asset_changed() {
    // req:sync/spec-changed@14245755
    let r = repo("impl/a");
    main_moves(&r, &[("specs/a/assets/m.html", "<p>m</p>\n")]);
    pull(&r);
    main_moves(&r, &[("specs/a/assets/m.html", "<p>m2</p>\n")]);

    let left = unfinished(&r);
    assert_eq!(
        left[..2],
        ["spec changed a", "changed specs/a/assets/m.html"]
    );
    assert_eq!(left.len(), 3, "{left:?}");
}

#[test]
fn unrelated_change_on_main() {
    // req:sync/spec-changed@14245755
    let r = repo("impl/a");
    main_moves(&r, &[("specs/b/SPEC.md", B)]);
    assert_eq!(synced(&r), Vec::<String>::new());

    // `b` exists now, off `a`'s bundle.
    main_moves(&r, &[("specs/b/SPEC.md", &B.replace("\nb\n", "\nb2\n"))]);
    assert_eq!(synced(&r), Vec::<String>::new());
}

/// `CAPABILITIES.md` holding capabilities `c` and `d`, whose statements are `c` and `d`.
fn capabilities(c: &str, d: &str) -> String {
    let entry = |slug: &str, text: &str| {
        format!("### Capability: {slug}\n{text}\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n")
    };
    format!("## Capabilities\n\n{}\n{}", entry("c", c), entry("d", d))
}

/// Makes `a` serve `capability/c` on `origin/main`, with `CAPABILITIES.md`, and pulls.
fn a_serves_c(r: &Repo) {
    let a = node_a("specd", "c", &["p"])
        .replace("owners: []\n", "owners: []\nserves: [capability/c]\n");
    main_moves(
        r,
        &[
            ("CAPABILITIES.md", &capabilities("C", "D")),
            ("specs/a/SPEC.md", &a),
        ],
    );
    pull(r);
}

#[test]
fn spec_changed_reports_capabilities() {
    // req:sync/spec-changed@14245755
    // req:sync/extract-through-layer@5c278f6c
    let r = repo("impl/a");
    a_serves_c(&r);
    main_moves(
        &r,
        &[("CAPABILITIES.md", &capabilities("C, reworded", "D"))],
    );

    let left = unfinished(&r);
    assert_eq!(left[..2], ["spec changed a", "changed CAPABILITIES.md"]);
    names_pending(&left[2]);
    assert_eq!(left.len(), 3, "{left:?}");
}

/// On `impl/<node>`, where `node` builds the capability tests, `main` rewords capability `d`,
/// which no node serves (`a` serves `c`, so its rule is traced); the run reports it.
fn capability_tests_node_sees_any_capability(r: &Repo, node: &str) {
    a_serves_c(r);
    main_moves(r, &[("CAPABILITIES.md", &capabilities("C", "D, reworded"))]);

    let left = unfinished(r);
    assert_eq!(
        left[..2],
        [
            format!("spec changed {node}"),
            "changed CAPABILITIES.md".into()
        ]
    );
    assert!(
        left[2].contains(&format!("clm pending {node}")) && left[2].contains("git diff"),
        "{left:?}"
    );
    assert_eq!(left.len(), 3, "{left:?}");
}

#[test]
fn spec_changed_on_the_capability_node() {
    // req:sync/spec-changed@14245755
    let r = repo("impl/capability");
    let node = "---\ntitle: Capabilities\nstatus: specd\nowners: []\n---\n\n## Contract\nk\n\n\
                ## Design\nd\n\n## Requirements\n";
    main_moves(&r, &[("specs/capability/SPEC.md", node)]);
    capability_tests_node_sees_any_capability(&r, "capability");
}

#[test]
fn spec_changed_on_the_root_without_a_capability_node() {
    // req:sync/spec-changed@14245755
    // With no capability node, the root builds the capability tests.
    let r = repo("impl/clm");
    capability_tests_node_sees_any_capability(&r, "clm");
}

#[test]
fn unserved_capability_changed() {
    // req:sync/spec-changed@14245755
    let r = repo("impl/a");
    // `a` serves an architecture requirement, so it serves no capability and is traced.
    let a = node_a("specd", "c", &["p"])
        .replace("owners: []\n", "owners: []\nserves: [architecture/z]\n");
    let arch = "## Constraints\n\n### Constraint: z\nIt SHALL hold.\n\n#### Scenario: s\n- ok\n\n#### Untested\nA judgement.\n";
    main_moves(&r, &[("ARCHITECTURE.md", arch), ("specs/a/SPEC.md", &a)]);
    pull(&r);
    main_moves(&r, &[("CAPABILITIES.md", &capabilities("C", "D"))]);
    assert_eq!(synced(&r), Vec::<String>::new());

    main_moves(
        &r,
        &[("CAPABILITIES.md", &capabilities("C, reworded", "D"))],
    );
    assert_eq!(synced(&r), Vec::<String>::new());
}

#[test]
fn unserved_capability_changed_with_the_node() {
    // req:sync/spec-changed@14245755
    let r = repo("impl/a");
    a_serves_c(&r);
    let a = read(&r.work, "specs/a/SPEC.md").replace("## Contract\nc\n", "## Contract\nc2\n");
    main_moves(
        &r,
        &[
            ("CAPABILITIES.md", &capabilities("C", "D, reworded")),
            ("specs/a/SPEC.md", &a),
        ],
    );

    let left = unfinished(&r);
    assert_eq!(left[..2], ["spec changed a", "changed specs/a/SPEC.md"]);
    names_pending(&left[2]);
    assert_eq!(left.len(), 3, "{left:?}");
}

#[test]
fn no_spec_changed_report_off_node_branches() {
    // req:sync/spec-changed@14245755
    for branch in ["spec/x", "topic", "impl/x/a"] {
        let r = repo(branch);
        main_moves(&r, &[("specs/a/SPEC.md", &a_with_p_modified(&r))]);
        let lines = synced(&r);
        assert!(
            !lines.iter().any(|l| l.starts_with("spec changed")),
            "{branch}: {lines:?}"
        );
    }
}

/// On `main`: applied change `x` adding `a/x`, with `resolves: [q]` and `backlog/q.md`,
/// plus `extra` delta text. The branch is `branch`, cut after that.
fn with_applied_x(branch: &str, extra: &str) -> Repo {
    let r = repo(branch);
    write(&r.work, "backlog/q.md", "---\ntitle: Q\nkind: idea\n---\n");
    apply_change(&r, "x", "resolves: [q]\n", &format!("{}{extra}", adds_x()));
    git(&r.work, &["push", "-q", "origin", "HEAD:main"]);
    r
}

#[test]
fn branch_completes_a_change() {
    // req:sync/archive-complete@aaaf914b
    let r = with_applied_x("impl/a", "");
    write(&r.work, "src/test.rs", &citing("a/x", &version(&r, "a/x")));
    commit(&r.work, "test a/x");

    assert_eq!(synced(&r), ["archived x"]);
    assert_eq!(git(&r.work, &["log", "-1", "--format=%s"]), "Archive x");
    assert_eq!(git(&r.work, &["status", "--porcelain"]), "");
    let md = git(&r.work, &["show", "HEAD:changes/archive/x/change.md"]);
    assert!(md.contains("status: done"), "{md}");
    assert!(!r.work.join("changes/x").exists());
    assert!(!r.work.join("backlog/q.md").exists());
    assert!(clm_check::run(&r.work).is_empty());
}

#[test]
fn a_repo_without_a_backlog_archives_too() {
    // req:sync/archive-complete@aaaf914b
    let r = repo("impl/a");
    apply_change(&r, "x", "", &adds_x());
    git(&r.work, &["push", "-q", "origin", "HEAD:main"]);
    assert!(!r.work.join("backlog").exists());
    write(&r.work, "src/test.rs", &citing("a/x", &version(&r, "a/x")));
    commit(&r.work, "test a/x");

    assert_eq!(synced(&r), ["archived x"]);
    assert_eq!(git(&r.work, &["status", "--porcelain"]), "");
    assert!(r.work.join("changes/archive/x/change.md").exists());
}

#[test]
fn an_uncited_requirement_holds_it() {
    // req:sync/archive-complete@aaaf914b
    let r = with_applied_x("impl/a", "");
    write(&r.work, "src/test.rs", &citing("a/p", &version(&r, "a/p")));
    commit(&r.work, "test a/p only");

    let lines = synced(&r);
    assert!(
        !lines.iter().any(|l| l.starts_with("archived")),
        "{lines:?}"
    );
    assert!(r.work.join("changes/x/change.md").exists());
    assert!(r.work.join("backlog/q.md").exists());
}

#[test]
fn obsolete_marker_holds_it() {
    // req:sync/archive-complete@aaaf914b
    let r = repo("impl/a");
    let p = clm_model::load(&r.work)
        .0
        .requirement("a/p")
        .unwrap()
        .hash
        .clone();
    apply_change(
        &r,
        "x",
        "",
        &format!(
            "{}\n## REMOVED Requirements\n\n### Requirement: p <!-- base: {p} -->\n",
            adds_x()
        ),
    );
    git(&r.work, &["push", "-q", "origin", "HEAD:main"]);
    let x = version(&r, "a/x");
    write(
        &r.work,
        "src/test.rs",
        &format!("{}// {}:a/p\n", citing("a/x", &x), "req"),
    );
    commit(&r.work, "tests");

    let lines = synced(&r);
    assert!(
        !lines.iter().any(|l| l.starts_with("archived")),
        "{lines:?}"
    );
    assert!(r.work.join("changes/x/change.md").exists());
}

#[test]
fn completed_by_another_branch_s_merge() {
    // req:sync/archive-complete@aaaf914b
    let r = with_applied_x("impl/b", "");
    write(&r.work, "src/new.rs", "new\n");
    commit(&r.work, "work");
    main_moves(&r, &[("src/test.rs", &citing("a/x", &version(&r, "a/x")))]);

    assert_eq!(synced(&r), ["archived x"]);
    assert!(r.work.join("changes/archive/x/change.md").exists());
    assert!(!r.work.join("changes/x").exists());
}

#[test]
fn unapplied_change_left_alone() {
    // req:sync/archive-complete@aaaf914b
    let r = repo("impl/a");
    let p = clm_model::load(&r.work)
        .0
        .requirement("a/p")
        .unwrap()
        .hash
        .clone();
    write(
        &r.work,
        "changes/y/change.md",
        "---\nstatus: proposed\n---\n\n# y\n",
    );
    write(
        &r.work,
        "changes/y/deltas/a.md",
        &format!(
            "---\ncomponent: a\n---\n## MODIFIED Requirements\n\n### Requirement: p <!-- base: {p} -->\nIt SHALL p more.\n\n#### Scenario: s\n- ok\n"
        ),
    );
    write(&r.work, "src/test.rs", &citing("a/p", &version(&r, "a/p")));
    commit(&r.work, "propose y, test a/p");

    let lines = synced(&r);
    assert!(
        !lines.iter().any(|l| l.starts_with("archived")),
        "{lines:?}"
    );
    assert!(r.work.join("changes/y/change.md").exists());
}

#[test]
fn node_activated() {
    // req:sync/archive-complete@aaaf914b
    let r = repo("impl/a");
    write(&r.work, "src/test.rs", &citing("a/p", &version(&r, "a/p")));
    commit(&r.work, "test a/p");

    let lines = synced(&r);
    assert_eq!(git(&r.work, &["log", "-1", "--format=%s"]), "Activate a");
    let spec = git(&r.work, &["show", "HEAD:specs/a/SPEC.md"]);
    assert!(spec.contains("status: active"), "{spec}");
    assert_eq!(git(&r.work, &["status", "--porcelain"]), "");
    assert!(lines.iter().any(|l| l == "activated a"), "{lines:?}");
}

#[test]
fn activation_rides_the_archive_commit() {
    // req:sync/archive-complete@aaaf914b
    let r = with_applied_x("impl/a", "");
    let (p, x) = (version(&r, "a/p"), version(&r, "a/x"));
    write(
        &r.work,
        "src/test.rs",
        &format!("{}{}", citing("a/p", &p), citing("a/x", &x)),
    );
    commit(&r.work, "tests");

    assert_eq!(synced(&r), ["activated a", "archived x"]);
    assert_eq!(git(&r.work, &["log", "-1", "--format=%s"]), "Archive x");
    assert_eq!(
        git(&r.work, &["log", "-2", "--format=%s"]),
        "Archive x\ntests"
    );
    let spec = git(&r.work, &["show", "HEAD:specs/a/SPEC.md"]);
    assert!(spec.contains("status: active"), "{spec}");
}

#[test]
fn an_untracked_test_neither_archives_nor_activates() {
    // req:sync/archive-complete@aaaf914b
    let r = with_applied_x("impl/a", "");
    let (p, x) = (version(&r, "a/p"), version(&r, "a/x"));
    // Markers count only at `HEAD`: an untracked test isn't on the branch.
    write(
        &r.work,
        "src/test.rs",
        &format!("{}{}", citing("a/p", &p), citing("a/x", &x)),
    );

    let lines = sync(&r.work).unwrap().lines;
    assert!(
        !lines
            .iter()
            .any(|l| l.starts_with("archived") || l.starts_with("activated")),
        "{lines:?}"
    );
    assert!(r.work.join("changes/x/change.md").exists());
    assert!(read(&r.work, "specs/a/SPEC.md").contains("status: specd"));
}

#[test]
fn an_untracked_backlog_item_is_left_alone() {
    // req:sync/archive-complete@aaaf914b
    let r = repo("impl/a");
    write(&r.work, "backlog/q.md", "---\ntitle: Q\nkind: idea\n---\n");
    write(&r.work, "backlog/u.md", "---\ntitle: U\nkind: idea\n---\n");
    apply_change(&r, "x", "resolves: [q, u]\n", &adds_x());
    git(&r.work, &["rm", "-q", "--cached", "backlog/u.md"]);
    git(&r.work, &["commit", "-qm", "untrack u"]);
    git(&r.work, &["push", "-q", "origin", "HEAD:main"]);
    write(&r.work, "src/test.rs", &citing("a/x", &version(&r, "a/x")));
    commit_only(&r.work, "src/test.rs", "test a/x");

    let lines = sync(&r.work).unwrap().lines;
    assert!(lines.iter().any(|l| l == "archived x"), "{lines:?}");
    assert!(!r.work.join("backlog/q.md").exists());
    assert_eq!(
        read(&r.work, "backlog/u.md"),
        "---\ntitle: U\nkind: idea\n---\n"
    );
    assert_eq!(
        git(&r.work, &["status", "--porcelain", "-uall"]),
        "?? backlog/u.md"
    );
}

#[test]
fn regeneration_reads_the_branch_not_untracked_files() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/y");
    applied_spec_branch(&r, &adds_x());
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "c", &["p", "y"]))],
    );
    // A half-written backlog item the session hasn't committed.
    write(&r.work, "backlog/draft.md", "not frontmatter\n");

    sync(&r.work).unwrap();
    assert_eq!(git(&r.work, &["log", "-1", "--format=%s"]), "Re-apply y");
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(
        spec.contains("Requirement: x") && spec.contains("Requirement: y"),
        "{spec}"
    );
}

#[test]
fn nothing_archived_or_activated_off_impl_branches() {
    // req:sync/archive-complete@aaaf914b
    for branch in ["spec/y", "topic"] {
        let r = with_applied_x(branch, "");
        let (p, x) = (version(&r, "a/p"), version(&r, "a/x"));
        write(
            &r.work,
            "src/test.rs",
            &format!("{}{}", citing("a/p", &p), citing("a/x", &x)),
        );
        commit(&r.work, "tests");

        assert_eq!(synced(&r), Vec::<String>::new(), "{branch}");
        assert!(r.work.join("changes/x/change.md").exists(), "{branch}");
        assert!(read(&r.work, "specs/a/SPEC.md").contains("status: specd"));
    }
}

fn subject(r: &Repo) -> String {
    git(&r.work, &["log", "-1", "--format=%s"])
}

fn filed(lines: &[String]) -> bool {
    lines.iter().any(|l| l.starts_with("filed"))
}

#[test]
fn settings_are_loaded_on_every_branch() {
    // req:sync/judged-by-check@5c30baa7
    let r = repo("spec/x");
    main_moves(&r, &[("src/other.rs", "other\n")]);
    let mut calls = 0;
    let report = clm_sync::sync(&r.work, |_: &Path| {
        calls += 1;
        Err("clm.toml: unknown field `new`".into())
    })
    .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(report.lines, ["clm.toml: unknown field `new`"]);
    assert!(!report.ok);
    assert_eq!(git(&r.work, &["rev-parse", "HEAD"]), origin_main(&r));
}

#[test]
fn settings_cant_be_loaded() {
    // req:sync/judged-by-check@5c30baa7
    let r = repo("impl/a");
    let head = git(&r.work, &["rev-parse", "HEAD"]);

    let report = clm_sync::sync(&r.work, |_: &Path| Err("clm.toml: bad".into())).unwrap();
    assert_eq!(report.lines, ["clm.toml: bad"]);
    assert!(!report.ok);
    assert_eq!(git(&r.work, &["rev-parse", "HEAD"]), head);
}

#[test]
fn the_settings_error_comes_before_the_check() {
    // req:sync/judged-by-check@5c30baa7
    let r = repo("topic");
    write(
        &r.work,
        "src/test.rs",
        &format!("// {}\n", concat!("req", ":a/p")),
    );
    commit(&r.work, "test a/p");
    main_moves(&r, &[("specs/a/SPEC.md", &node_a("specd", "c", &[]))]);

    let report = clm_sync::sync(&r.work, |_: &Path| Err("settings: bad".into())).unwrap();
    assert_eq!(report.lines[0], "settings: bad", "{:?}", report.lines);
    assert!(
        report.lines[1..]
            .iter()
            .any(|l| l.contains("check/dangling-markers")),
        "the check still runs: {:?}",
        report.lines
    );
}

#[test]
fn the_settings_are_loaded_from_head_in_a_temporary_worktree() {
    // req:sync/judged-by-check@5c30baa7
    let r = repo("spec/x");
    main_moves(&r, &[("src/other.rs", "other\n")]);
    write(&r.work, "notes.txt", "untracked\n");
    let mut seen = None;
    let report = clm_sync::sync(&r.work, |root: &Path| {
        seen = Some((
            root.to_path_buf(),
            root.join("specs/SPEC.md").exists(),
            root.join("notes.txt").exists(),
        ));
        Err(format!(
            "{}: unknown field",
            root.join("clm.toml").display()
        ))
    })
    .unwrap();
    let (root, tracked, untracked) = seen.unwrap();
    assert!(!root.starts_with(&r.work), "{}", root.display());
    assert!(tracked && !untracked);
    assert!(!root.exists(), "the temporary worktree is removed");
    // Named by its path in the repository, never under the temporary directory.
    assert_eq!(
        report.lines,
        [format!(
            "{}: unknown field",
            r.work.join("clm.toml").display()
        )]
    );
}

/// A test file citing `a/missing`, a requirement no node has.
fn cites_missing() -> String {
    format!("// {}\n", concat!("req", ":a/missing"))
}

#[test]
fn an_uncommitted_marker_is_not_judged() {
    // req:sync/judged-by-check@5c30baa7
    let r = repo("spec/x");
    main_moves(&r, &[("src/other.rs", "other\n")]);
    write(&r.work, "src/lib.rs", &cites_missing());

    assert_eq!(synced(&r), Vec::<String>::new());
    assert_eq!(read(&r.work, "src/lib.rs"), cites_missing());
}

#[test]
fn a_committed_fault_is_judged_despite_an_uncommitted_fix() {
    // req:sync/judged-by-check@5c30baa7
    let r = repo("spec/x");
    write(&r.work, "src/lib.rs", &cites_missing());
    commit(&r.work, "cite a/missing");
    main_moves(&r, &[("src/other.rs", "other\n")]);
    write(&r.work, "src/lib.rs", "fixed\n");

    let left = unfinished(&r);
    assert!(
        left.iter()
            .any(|l| l.starts_with("src/lib.rs:") && l.contains("check/dangling-markers")),
        "{left:?}"
    );
    assert_eq!(read(&r.work, "src/lib.rs"), "fixed\n");
    assert_eq!(git(&r.work, &["status", "--porcelain"]), "M src/lib.rs");
}

#[test]
fn an_untracked_test_file_is_not_judged() {
    // req:sync/judged-by-check@5c30baa7
    let r = repo("spec/x");
    main_moves(&r, &[("src/other.rs", "other\n")]);
    write(&r.work, "tests/new.rs", &cites_missing());

    assert_eq!(synced(&r), Vec::<String>::new());
    assert_eq!(read(&r.work, "tests/new.rs"), cites_missing());
    assert_eq!(git(&r.work, &["status", "--porcelain"]), "?? tests/");
}

fn autosync_tip(r: &Repo, branch: &str) -> Option<String> {
    let out = Command::new("git")
        .args([
            "rev-parse",
            "--verify",
            "-q",
            &format!("refs/heads/autosync/{branch}"),
        ])
        .current_dir(&r.origin)
        .output()
        .unwrap();
    out.status
        .success()
        .then(|| String::from_utf8(out.stdout).unwrap().trim().to_string())
}

#[test]
fn autosync_pushes_uncommitted_spec_work_only() {
    // req:sync/autosync@9720eb37
    let r = repo("spec/x");
    write(&r.work, "changes/x/deltas/a.md", &adds_x());
    write(&r.work, "notes.txt", "private\n");
    write(&r.work, "src/lib.rs", "edited\n");
    let status = git(&r.work, &["status", "--porcelain"]);
    let head = git(&r.work, &["rev-parse", "HEAD"]);

    autosync(&r.work).unwrap();
    let tip = autosync_tip(&r, "spec/x").expect("pushed");
    assert_eq!(git(&r.origin, &["rev-parse", &format!("{tip}^")]), head);
    let files = git(&r.origin, &["ls-tree", "-r", "--name-only", &tip]);
    assert!(
        files.lines().any(|f| f == "changes/x/deltas/a.md"),
        "{files}"
    );
    assert!(!files.lines().any(|f| f == "notes.txt"), "{files}");
    let lib = git(&r.origin, &["show", &format!("{tip}:src/lib.rs")]);
    assert_eq!(
        lib, "one\ntwo\nthree",
        "files outside the spec paths stay as committed"
    );
    assert_eq!(git(&r.work, &["status", "--porcelain"]), status);
    assert_eq!(git(&r.work, &["rev-parse", "HEAD"]), head);
}

#[test]
fn autosync_snapshots_capabilities() {
    // req:sync/autosync@9720eb37
    let r = repo("spec/x");
    write(&r.work, "CAPABILITIES.md", "## Capabilities\n");

    autosync(&r.work).unwrap();
    let tip = autosync_tip(&r, "spec/x").expect("pushed");
    let files = git(&r.origin, &["ls-tree", "-r", "--name-only", &tip]);
    assert!(files.lines().any(|f| f == "CAPABILITIES.md"), "{files}");
}

#[test]
fn autosync_pushes_nothing_new() {
    // req:sync/autosync@9720eb37
    let r = repo("spec/x");
    write(&r.work, "changes/x/deltas/a.md", &adds_x());
    autosync(&r.work).unwrap();
    let first = autosync_tip(&r, "spec/x").unwrap();
    // Replace the remote snapshot: an unneeded second push would put it back.
    git(
        &r.origin,
        &["update-ref", "refs/heads/autosync/spec/x", "main"],
    );
    autosync(&r.work).unwrap();
    assert_ne!(autosync_tip(&r, "spec/x").unwrap(), first, "nothing pushed");
}

#[test]
fn autosync_never_signs_its_snapshot() {
    // req:sync/autosync@9720eb37
    let r = repo("spec/x");
    // Signing that would fail (or prompt) on every turn.
    git(&r.work, &["config", "commit.gpgsign", "true"]);
    git(&r.work, &["config", "gpg.program", "false"]);
    write(&r.work, "changes/x/deltas/a.md", &adds_x());
    autosync(&r.work).unwrap();
    assert!(autosync_tip(&r, "spec/x").is_some());
}

#[test]
fn autosync_with_nothing_uncommitted_pushes_head() {
    // req:sync/autosync@9720eb37
    let r = repo("spec/x");
    write(&r.work, "changes/x/deltas/a.md", &adds_x());
    commit(&r.work, "delta");
    autosync(&r.work).unwrap();
    assert_eq!(
        autosync_tip(&r, "spec/x").unwrap(),
        git(&r.work, &["rev-parse", "HEAD"])
    );
}

#[test]
fn autosync_ignores_other_branches() {
    // req:sync/autosync@9720eb37
    let r = repo("impl/x/a");
    write(&r.work, "changes/x/deltas/a.md", &adds_x());
    let refs = origin_refs(&r);
    autosync(&r.work).unwrap();
    assert_eq!(origin_refs(&r), refs);
}

#[test]
fn autosync_skips_a_tree_already_on_the_branch() {
    // req:sync/autosync@9720eb37
    let r = repo("spec/x");
    write(&r.work, "changes/x/deltas/a.md", &adds_x());
    commit(&r.work, "delta");
    git(&r.work, &["push", "-q", "origin", "spec/x"]);
    autosync(&r.work).unwrap();
    assert_eq!(autosync_tip(&r, "spec/x"), None);
}

#[test]
fn a_change_with_links_is_regenerated() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/y");
    write(
        &r.work,
        "backlog/foo.md",
        "---\ntitle: Foo\nkind: idea\n---\n",
    );
    apply_change(&r, "y", "resolves: [foo]\n", &adds_x());
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "c", &["p", "y"]))],
    );

    assert_eq!(synced(&r), Vec::<String>::new());
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(
        spec.contains("Requirement: x") && spec.contains("Requirement: y"),
        "{spec}"
    );
}

#[test]
fn an_item_blocking_a_requirement_the_change_adds_is_regenerated() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/y");
    applied_spec_branch(&r, &adds_x());
    // `a/x` is in the branch's `specs/` but not `main`'s, which the regenerate step
    // starts from.
    write(
        &r.work,
        "backlog/q.md",
        "---\ntitle: Q\nkind: human\nblocks: [a/x]\n---\n",
    );
    commit(&r.work, "File q");
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "c", &["p", "y"]))],
    );

    assert_eq!(synced(&r), Vec::<String>::new());
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(
        spec.contains("Requirement: x") && spec.contains("Requirement: y"),
        "{spec}"
    );
}

#[test]
fn an_item_the_change_doesnt_fix_is_judged_by_check() {
    // req:sync/reapply@01d24eb6 req:sync/judged-by-check@5c30baa7
    let r = repo("spec/y");
    applied_spec_branch(&r, &adds_x());
    write(
        &r.work,
        "backlog/q.md",
        "---\ntitle: Q\nkind: human\nblocks: [a/nope]\n---\n",
    );
    commit(&r.work, "File q");
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "c", &["p", "y"]))],
    );

    let left = unfinished(&r);
    assert!(left.iter().any(|l| l.contains("backlog/q.md")), "{left:?}");
    // Regeneration went ahead; only the check result holds the branch.
    assert!(read(&r.work, "specs/a/SPEC.md").contains("Requirement: x"));
    assert_eq!(git(&r.work, &["log", "-1", "--format=%s"]), "Re-apply y");
}

#[test]
fn a_broken_delta_of_the_change_still_stops_it() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/y");
    applied_spec_branch(&r, &adds_x());
    write(&r.work, "changes/y/deltas/b.md", "no frontmatter\n");
    commit(&r.work, "Break y");
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "c", &["p", "y"]))],
    );

    let left = unfinished(&r);
    assert!(
        left.iter().any(|l| l.contains("changes/y/deltas/b.md")),
        "{left:?}"
    );
    // The valid delta for `a` never reached `specs/` either.
    assert!(!read(&r.work, "specs/a/SPEC.md").contains("Requirement: x"));
    assert_ne!(git(&r.work, &["log", "-1", "--format=%s"]), "Re-apply y");
}

#[test]
fn a_stale_second_change_writes_nothing() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/y");
    apply_change(&r, "y1", "", &adds_x());
    let delta = modifies_contract(&r, "mine");
    apply_change(&r, "y2", "", &delta);
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "theirs", &["p"]))],
    );

    let left = unfinished(&r);
    assert!(
        left.iter().any(|l| l.contains("apply/stale-base")),
        "{left:?}"
    );
    // `y1` re-applies cleanly, but nothing reaches `specs/` while `y2` is stale.
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(!spec.contains("Requirement: x"), "{spec}");
}

#[test]
fn a_spec_file_main_deleted_is_regenerated_not_reported() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/y");
    applied_spec_branch(&r, &adds_x());
    main_moves(&r, &[("src/other.rs", "other\n")]);
    let other = r.work.with_file_name("other");
    git(&other, &["rm", "-q", "specs/a/SPEC.md"]);
    git(&other, &["commit", "-qm", "drop a"]);
    git(&other, &["push", "-q", "origin", "HEAD:main"]);

    let left = unfinished(&r);
    assert!(!rebasing(&r), "the modify/delete conflict is settled");
    assert!(!left.iter().any(|l| l.starts_with("conflict")), "{left:?}");
    assert!(
        !left.is_empty(),
        "the delta's node is gone, so it can't apply"
    );
}

#[test]
fn a_resume_regenerates_against_the_main_it_rebased_onto() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/y");
    write(&r.work, "src/lib.rs", "one\nTWO\nthree\n");
    commit(&r.work, "work");
    applied_spec_branch(&r, &adds_x());
    main_moves(
        &r,
        &[
            ("src/lib.rs", "one\n2\nthree\n"),
            ("specs/a/SPEC.md", &node_a("specd", "c", &["p", "y"])),
        ],
    );
    assert_eq!(unfinished(&r), ["conflict src/lib.rs"]);
    let onto = origin_main(&r);

    // `main` moves again before the resume; the resume must not pick that up.
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "c", &["p", "y", "z"]))],
    );
    write(&r.work, "src/lib.rs", "one\nTWO and 2\nthree\n");
    git(&r.work, &["add", "src/lib.rs"]);
    assert_eq!(synced(&r), Vec::<String>::new());
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(
        spec.contains("Requirement: x") && spec.contains("Requirement: y"),
        "{spec}"
    );
    assert!(!spec.contains("Requirement: z"), "{spec}");
    assert_eq!(git(&r.work, &["rev-parse", "HEAD~3"]), onto);
}

#[test]
fn a_change_set_back_to_proposed_keeps_mains_specs() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/y");
    let before = read(&r.work, "specs/a/SPEC.md");
    applied_spec_branch(&r, &adds_x());
    // The author sets `y` back to proposed, to edit its deltas and apply again.
    write(&r.work, "specs/a/SPEC.md", &before);
    let md = read(&r.work, "changes/y/change.md").replace("status: applied", "status: proposed");
    write(&r.work, "changes/y/change.md", &md);
    commit(&r.work, "Unapply y");
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "c", &["p", "w"]))],
    );

    let lines = synced(&r);
    assert!(
        !lines.iter().any(|l| l.starts_with("conflict")),
        "{lines:?}"
    );
    assert!(!rebasing(&r));
    let main_spec = git(&r.origin, &["show", "main:specs/a/SPEC.md"]);
    assert_eq!(read(&r.work, "specs/a/SPEC.md").trim(), main_spec);
    assert!(read(&r.work, "changes/y/change.md").contains("status: proposed"));
}

#[test]
fn a_specs_conflict_beside_a_real_one_is_resolved() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/y");
    write(&r.work, "src/lib.rs", "one\nTWO\nthree\n");
    write(
        &r.work,
        "changes/y/change.md",
        "---\nstatus: proposed\n---\n\n# y\n",
    );
    write(&r.work, "changes/y/deltas/a.md", &adds_x());
    clm_apply::apply(&r.work, "y").unwrap();
    commit(&r.work, "Apply y, and work");
    main_moves(
        &r,
        &[
            ("src/lib.rs", "one\n2\nthree\n"),
            ("specs/a/SPEC.md", &node_a("specd", "c", &["p", "y"])),
        ],
    );

    assert_eq!(unfinished(&r), ["conflict src/lib.rs"]);
    let unmerged = git(&r.work, &["diff", "--name-only", "--diff-filter=U"]);
    assert_eq!(unmerged, "src/lib.rs");
    write(&r.work, "src/lib.rs", "one\nTWO and 2\nthree\n");
    git(&r.work, &["add", "src/lib.rs"]);
    assert_eq!(synced(&r), Vec::<String>::new());
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(
        spec.contains("Requirement: x") && spec.contains("Requirement: y"),
        "{spec}"
    );
}

#[test]
fn a_specs_modify_delete_beside_a_real_conflict_is_settled_first() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/y");
    write(&r.work, "src/lib.rs", "one\nTWO\nthree\n");
    write(
        &r.work,
        "changes/y/change.md",
        "---\nstatus: proposed\n---\n\n# y\n",
    );
    write(&r.work, "changes/y/deltas/a.md", &adds_x());
    clm_apply::apply(&r.work, "y").unwrap();
    commit(&r.work, "Apply y, and work");
    // A conflict the driver can't take: `main` deletes the file the branch modified.
    main_moves(&r, &[("src/lib.rs", "one\n2\nthree\n")]);
    let other = r.work.with_file_name("other");
    git(&other, &["rm", "-q", "specs/a/SPEC.md"]);
    git(&other, &["commit", "-qm", "drop a"]);
    git(&other, &["push", "-q", "origin", "HEAD:main"]);

    assert_eq!(unfinished(&r), ["conflict src/lib.rs"]);
    let unmerged = git(&r.work, &["diff", "--name-only", "--diff-filter=U"]);
    assert_eq!(unmerged, "src/lib.rs");
    assert!(!r.work.join("specs/a/SPEC.md").exists());
}

#[test]
fn a_branch_adding_no_change_reports_its_specs_conflict() {
    // req:sync/reapply@01d24eb6
    let r = repo("topic");
    write(&r.work, "specs/a/SPEC.md", &node_a("specd", "mine", &["p"]));
    commit(&r.work, "edit a's Contract");
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "theirs", &["p"]))],
    );

    assert_eq!(unfinished(&r), ["conflict specs/a/SPEC.md"]);
    assert!(rebasing(&r));
}

#[test]
fn a_change_whose_frontmatter_doesnt_parse_still_counts_as_added() {
    // req:sync/reapply@01d24eb6
    let r = repo("topic");
    write(
        &r.work,
        "changes/y/change.md",
        "---\nstatus: [not, a, status\n---\n",
    );
    write(&r.work, "specs/a/SPEC.md", &node_a("specd", "mine", &["p"]));
    commit(&r.work, "work");
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "theirs", &["p"]))],
    );

    let left = unfinished(&r);
    assert!(!left.iter().any(|l| l.starts_with("conflict")), "{left:?}");
    assert!(!rebasing(&r));
    // Kept, never regenerated: the change isn't applied.
    let main_spec = git(&r.origin, &["show", "main:specs/a/SPEC.md"]);
    assert_eq!(read(&r.work, "specs/a/SPEC.md").trim(), main_spec);
}

/// The subjects of the commits on the branch since `origin/main`, oldest first.
fn subjects_since_main(r: &Repo) -> Vec<String> {
    git(
        &r.work,
        &["log", "--reverse", "--format=%s", "origin/main..HEAD"],
    )
    .lines()
    .map(str::to_string)
    .collect()
}

#[test]
fn a_resume_keeps_mains_specs_for_a_change_that_isnt_applied() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/y");
    write(&r.work, "src/lib.rs", "one\nTWO\nthree\n");
    write(
        &r.work,
        "changes/y/change.md",
        "---\nstatus: proposed\n---\n\n# y\n",
    );
    write(&r.work, "specs/a/SPEC.md", &node_a("specd", "mine", &["p"]));
    commit(&r.work, "work");
    main_moves(
        &r,
        &[
            ("src/lib.rs", "one\n2\nthree\n"),
            ("specs/a/SPEC.md", &node_a("specd", "theirs", &["p"])),
        ],
    );

    assert_eq!(unfinished(&r), ["conflict src/lib.rs"]);
    write(&r.work, "src/lib.rs", "one\nTWO and 2\nthree\n");
    git(&r.work, &["add", "src/lib.rs"]);
    let lines = synced(&r);
    assert!(
        !lines.iter().any(|l| l.starts_with("conflict")),
        "{lines:?}"
    );
    let main_spec = git(&r.origin, &["show", "main:specs/a/SPEC.md"]);
    assert_eq!(read(&r.work, "specs/a/SPEC.md").trim(), main_spec);
    assert_eq!(git(&r.work, &["log", "-1", "--format=%s"]), "work");
}

/// Sets change `id`'s status to `status` and moves it to `changes/archive/<id>/` in `dir`,
/// committing the result as `Archive <id>`.
fn archive_in(dir: &Path, id: &str, status: &str) {
    fs::create_dir_all(dir.join("changes/archive")).unwrap();
    git(
        dir,
        &[
            "mv",
            &format!("changes/{id}"),
            &format!("changes/archive/{id}"),
        ],
    );
    let path = format!("changes/archive/{id}/change.md");
    let md = read(dir, &path).replace("status: applied", &format!("status: {status}"));
    write(dir, &path, &md);
    commit(dir, &format!("Archive {id}"));
}

#[test]
fn a_change_archived_on_its_spec_branch_is_reapplied() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/x");
    apply_change(&r, "x", "", &adds_x());
    archive_in(&r.work, "x", "done");
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "c", &["p", "y"]))],
    );

    assert_eq!(synced(&r), Vec::<String>::new());
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(
        spec.contains("Requirement: x") && spec.contains("Requirement: y"),
        "{spec}"
    );
    assert!(read(&r.work, "changes/archive/x/change.md").contains("status: done"));
    assert!(!r.work.join("changes/x").exists());
    assert_eq!(git(&r.work, &["status", "--porcelain"]), "");
    assert_eq!(git(&r.work, &["log", "-1", "--format=%s"]), "Re-apply x");
}

#[test]
fn a_resume_regenerates_a_change_archived_on_the_branch() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/x");
    write(&r.work, "src/lib.rs", "one\nTWO\nthree\n");
    commit(&r.work, "work");
    apply_change(&r, "x", "", &adds_x());
    archive_in(&r.work, "x", "done");
    main_moves(
        &r,
        &[
            ("src/lib.rs", "one\n2\nthree\n"),
            ("specs/a/SPEC.md", &node_a("specd", "c", &["p", "y"])),
        ],
    );
    assert_eq!(unfinished(&r), ["conflict src/lib.rs"]);

    write(&r.work, "src/lib.rs", "one\nTWO and 2\nthree\n");
    git(&r.work, &["add", "src/lib.rs"]);
    assert_eq!(synced(&r), Vec::<String>::new());
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(
        spec.contains("Requirement: x") && spec.contains("Requirement: y"),
        "{spec}"
    );
    assert_eq!(git(&r.work, &["log", "-1", "--format=%s"]), "Re-apply x");
}

#[test]
fn an_archived_change_is_merged() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/x");
    main_moves(
        &r,
        &[(
            "specs/a/SPEC.md",
            &node_a("specd", "one\ntwo\nthree", &["p"]),
        )],
    );
    pull(&r);
    let delta = modifies_contract(&r, "ONE\ntwo\nthree");
    apply_change(&r, "x", "", &delta);
    archive_in(&r.work, "x", "done");
    main_moves(
        &r,
        &[(
            "specs/a/SPEC.md",
            &node_a("specd", "one\ntwo\nTHREE", &["p"]),
        )],
    );

    assert_eq!(
        synced(&r),
        ["merged contract of a into changes/archive/x/deltas/a.md"]
    );
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(spec.contains("## Contract\nONE\ntwo\nTHREE\n"), "{spec}");
    assert_eq!(
        subjects_since_main(&r),
        ["Apply x", "Archive x", "Merge main into x", "Re-apply x"]
    );
}

#[test]
fn an_archived_change_that_isnt_done_keeps_mains_specs() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/x");
    apply_change(&r, "x", "", &adds_x());
    archive_in(&r.work, "x", "abandoned");
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "c", &["p", "y"]))],
    );

    let report = sync(&r.work).unwrap();
    assert!(
        !report.lines.iter().any(|l| l.starts_with("conflict")),
        "{:?}",
        report.lines
    );
    assert!(!rebasing(&r));
    assert_ne!(git(&r.work, &["log", "-1", "--format=%s"]), "Re-apply x");
    let main_spec = git(&r.origin, &["show", "main:specs/a/SPEC.md"]);
    assert_eq!(read(&r.work, "specs/a/SPEC.md").trim(), main_spec);
}

#[test]
fn live_and_archived_changes_are_reapplied_in_id_order() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/x");
    // The archived change sorts first, the live one after it.
    apply_change(&r, "a1", "", &adds_x());
    archive_in(&r.work, "a1", "done");
    let adds_v = format!(
        "---\ncomponent: a\n---\n## ADDED Requirements\n{}",
        requirement("v")
    );
    apply_change(&r, "b1", "", &adds_v);
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "c", &["p", "y"]))],
    );

    assert_eq!(synced(&r), Vec::<String>::new());
    assert_eq!(
        git(&r.work, &["log", "-1", "--format=%s"]),
        "Re-apply a1, b1"
    );
    let spec = read(&r.work, "specs/a/SPEC.md");
    for slug in ["x", "v", "y"] {
        assert!(spec.contains(&format!("Requirement: {slug}")), "{spec}");
    }
}

#[test]
fn a_branch_archiving_a_change_main_applied_doesnt_reapply_it() {
    // req:sync/reapply@01d24eb6
    let r = repo("impl/b");
    let delta = modifies_contract(&r, "mine");
    apply_change(&r, "y", "", &delta);
    git(&r.work, &["push", "-q", "origin", "HEAD:main"]);
    archive_in(&r.work, "y", "done");
    let spec = read(&r.work, "specs/a/SPEC.md");
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &format!("{spec}{}", requirement("w")))],
    );

    let lines = synced(&r);
    assert!(
        !lines
            .iter()
            .any(|l| l.starts_with("main changed") || l.contains("apply/")),
        "{lines:?}"
    );
    let subjects = subjects_since_main(&r);
    assert!(
        !subjects
            .iter()
            .any(|s| s.starts_with("Re-apply") || s.starts_with("Merge main into")),
        "{subjects:?}"
    );
    let main_spec = git(&r.origin, &["show", "main:specs/a/SPEC.md"]);
    assert_eq!(read(&r.work, "specs/a/SPEC.md").trim(), main_spec);
}

/// Installs a `post-checkout` hook in the work clone that appends the directory it runs in
/// to a log outside the repository; the log's path.
fn log_checkouts(r: &Repo) -> PathBuf {
    let log = r.work.with_file_name("checkouts.log");
    let hook = r.work.join(git(
        &r.work,
        &["rev-parse", "--git-path", "hooks/post-checkout"],
    ));
    fs::create_dir_all(hook.parent().unwrap()).unwrap();
    write_exec(
        &hook,
        &format!("#!/bin/sh\npwd -P >> '{}'\n", log.display()),
    );
    log
}

/// Asserts the log isn't empty and names only the work clone or a `sync-*` worktree.
fn only_checkouts_of_the_repo(r: &Repo, log: &Path) {
    let text = fs::read_to_string(log).unwrap_or_default();
    assert!(!text.is_empty(), "no checkout logged");
    let work = fs::canonicalize(&r.work).unwrap();
    for line in text.lines() {
        let dir = Path::new(line);
        let scratch = dir.ancestors().any(|a| {
            a.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("sync-"))
        });
        assert!(dir == work || scratch, "a checkout elsewhere: {line}");
    }
}

#[test]
fn no_post_checkout_hook_when_regenerating() {
    // req:sync/extract-through-layer@5c278f6c
    let r = repo("spec/x");
    apply_change(&r, "x", "", &adds_x());
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "c", &["p", "y"]))],
    );
    let log = log_checkouts(&r);

    assert_eq!(synced(&r), Vec::<String>::new());
    assert_eq!(git(&r.work, &["log", "-1", "--format=%s"]), "Re-apply x");
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(spec.contains("Requirement: x") && spec.contains("Requirement: y"));
    only_checkouts_of_the_repo(&r, &log);
}

#[test]
fn no_post_checkout_hook_when_comparing_a_nodes_spec() {
    // req:sync/extract-through-layer@5c278f6c
    let r = repo("impl/a");
    write(&r.work, "src/new.rs", "new\n");
    commit(&r.work, "work");
    main_moves(&r, &[("specs/a/SPEC.md", &a_with_p_modified(&r))]);
    let log = log_checkouts(&r);

    let left = unfinished(&r);
    assert_eq!(left[0], "spec changed a", "{left:?}");
    only_checkouts_of_the_repo(&r, &log);
}

/// A `proposed` record's line: `id`, written by `writer`, for `slug`.
fn proposed(id: &str, writer: &str, slug: &str, title: &str, nodes: &[&str], body: &str) -> String {
    serde_json::json!({
        "id": id,
        "kind": "proposed",
        "writer": writer,
        "at": "2026-10-28T10:00:00.000Z",
        "slug": slug,
        "title": title,
        "body": body,
        "nodes": nodes,
    })
    .to_string()
}

fn withdrawn(id: &str, writer: &str, proposal: &str) -> String {
    serde_json::json!({
        "id": id,
        "kind": "withdrawn",
        "writer": writer,
        "at": "2026-10-28T11:00:00.000Z",
        "proposal": proposal,
    })
    .to_string()
}

/// Pushes `clm/ledger/<writer>` to `origin`, holding `records.jsonl` with `lines`, from a
/// repository of its own, as the web app's writer would.
fn ledger(r: &Repo, writer: &str, lines: &[String]) {
    let dir = r
        .work
        .with_file_name(format!("ledger-{}", writer.replace('/', "-")));
    git(
        r.work.parent().unwrap(),
        &["init", "-q", dir.to_str().unwrap()],
    );
    git(&dir, &["config", "user.name", "l"]);
    git(&dir, &["config", "user.email", "l@l"]);
    write(&dir, "records.jsonl", &format!("{}\n", lines.join("\n")));
    commit(&dir, "records");
    git(
        &dir,
        &[
            "push",
            "-q",
            r.origin.to_str().unwrap(),
            &format!("HEAD:refs/heads/clm/ledger/{writer}"),
        ],
    );
}

const EXPORT: &str = "---\ntitle: \"Export CSV\"\nkind: idea\ncomponents: [a]\n---\n\nLet me export.\n\n\
                      Proposed in the web app by alice/web at 2026-10-28T10:00:00.000Z (record p1).\n";

fn export_csv(body: &str) -> String {
    proposed(
        "p1",
        "alice/web",
        "export-csv",
        "Export CSV",
        &["a", "gone"],
        body,
    )
}

#[test]
fn a_proposal_is_filed() {
    // req:sync/file-proposal-items@e75d388f
    let r = repo("impl/a");
    ledger(&r, "alice/web", &[export_csv("Let me export.\n\n")]);

    let report = sync(&r.work).unwrap();
    assert_eq!(report.lines, ["filed backlog/export-csv.md"]);
    assert!(report.ok, "{report:?}");
    assert_eq!(subject(&r), "File export-csv");
    assert_eq!(
        git(&r.work, &["show", "--name-status", "--format=", "HEAD"]),
        "A\tbacklog/export-csv.md"
    );
    assert_eq!(read(&r.work, "backlog/export-csv.md"), EXPORT);
    assert_eq!(git(&r.work, &["status", "--porcelain"]), "");
    assert!(clm_check::run(&r.work).is_empty());
    // Once filed, it is an item: the next run files nothing.
    assert_eq!(synced(&r), Vec::<String>::new());
}

#[test]
fn a_proposal_files_the_same_bytes_on_two_branches() {
    // req:sync/file-proposal-items@e75d388f
    let mut files = Vec::new();
    for branch in ["impl/a", "impl/b"] {
        let r = repo(branch);
        ledger(&r, "alice/web", &[export_csv("Let me export.")]);
        synced(&r);
        files.push(read(&r.work, "backlog/export-csv.md"));
    }
    assert_eq!(files[0], files[1]);
    assert_eq!(files[0], EXPORT);
}

#[test]
fn withdrawn_filed_or_resolved_proposals_are_not_filed() {
    // req:sync/file-proposal-items@e75d388f
    let r = repo("impl/a");
    write(
        &r.work,
        "backlog/s2.md",
        "---\ntitle: S2\nkind: idea\n---\n",
    );
    write(
        &r.work,
        "changes/archive/20260101-x/change.md",
        "---\nstatus: done\nresolves: [s3]\n---\n\n# x\n",
    );
    commit(&r.work, "s2 and an archived change");
    let head = git(&r.work, &["rev-parse", "HEAD"]);
    ledger(
        &r,
        "alice/web",
        &[
            proposed("p1", "alice/web", "s1", "S1", &["a"], "one"),
            withdrawn("w1", "alice/web", "p1"),
            proposed("p2", "alice/web", "s2", "S2 again", &["a"], "two"),
            proposed("p3", "alice/web", "s3", "S3", &["a"], "three"),
        ],
    );

    let lines = synced(&r);
    assert!(!filed(&lines), "{lines:?}");
    assert_eq!(git(&r.work, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        read(&r.work, "backlog/s2.md"),
        "---\ntitle: S2\nkind: idea\n---\n"
    );
}

#[test]
fn an_untracked_item_keeps_its_proposal_unfiled() {
    // req:sync/file-proposal-items@e75d388f
    let r = repo("impl/a");
    ledger(
        &r,
        "alice/web",
        &[proposed("p1", "alice/web", "s1", "S1", &["a"], "one")],
    );
    write(&r.work, "backlog/s1.md", "mine\n");

    let lines = synced(&r);
    assert!(!filed(&lines), "{lines:?}");
    assert_eq!(read(&r.work, "backlog/s1.md"), "mine\n");
}

#[test]
fn a_slug_that_isnt_kebab_case_is_skipped_with_a_warning() {
    // req:sync/file-proposal-items@e75d388f
    let r = repo("impl/a");
    ledger(
        &r,
        "alice/web",
        &[proposed("p9", "alice/web", "../escape", "Bad", &["a"], "x")],
    );

    let report = sync(&r.work).unwrap();
    assert!(report.ok, "{report:?}");
    assert!(!filed(&report.lines), "{report:?}");
    assert!(
        report.warnings.iter().any(|w| w.contains("p9")),
        "{report:?}"
    );
    assert!(!r.work.join("escape.md").exists());
}

#[test]
fn a_skipped_proposals_warning_carries_no_command_name() {
    // req:sync/warnings-name-the-command@32d2f35d
    let r = repo("impl/a");
    ledger(
        &r,
        "alice/web",
        &[proposed("p9", "alice/web", "Not Kebab", "Bad", &["a"], "x")],
    );

    let report = sync(&r.work).unwrap();
    let skipped: Vec<_> = report
        .warnings
        .iter()
        .filter(|w| w.starts_with("skipped proposal"))
        .collect();
    assert_eq!(skipped.len(), 1, "{report:?}");
    assert!(skipped[0].contains("p9"), "{report:?}");
    assert!(
        report.warnings.iter().all(|w| !w.starts_with("clm")),
        "{report:?}"
    );
}

#[test]
fn the_crate_names_no_command_in_a_string() {
    // req:sync/warnings-name-the-command@32d2f35d
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in fs::read_dir(&src).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap();
        for (n, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap();
            for name in ["rebase", "sync"] {
                let named = format!("clm {name}");
                assert!(
                    !(code.contains('"') && code.contains(&named)),
                    "{}:{} names `{named}` in a string",
                    path.display(),
                    n + 1
                );
            }
        }
    }
}

#[test]
fn no_proposal_items_off_impl_branches() {
    // req:sync/file-proposal-items@e75d388f
    let r = repo("spec/y");
    ledger(
        &r,
        "alice/web",
        &[proposed("p4", "alice/web", "s4", "S4", &["a"], "four")],
    );

    let lines = synced(&r);
    assert!(!filed(&lines), "{lines:?}");
    assert!(!r.work.join("backlog/s4.md").exists());
}

/// `repo(branch)` whose nodes `a` (requirements `p` and `x`) and `b` (requirement `y`) are on
/// `origin/main` and the branch, with `flags` committed `friction` items naming `id` at its
/// current version.
fn flagged(branch: &str, id: &str, flags: usize) -> Repo {
    let r = repo(branch);
    write(
        &r.work,
        "specs/a/SPEC.md",
        &node_a("specd", "c", &["p", "x"]),
    );
    write(
        &r.work,
        "specs/b/SPEC.md",
        &node_a("specd", "c", &["y"]).replace("title: A", "title: B"),
    );
    commit(&r.work, "a/x and b/y");
    let at = version(&r, id);
    let node = id.split_once('/').unwrap().0;
    for n in 1..=flags {
        write(
            &r.work,
            &format!("backlog/friction-{}-{n}.md", id.replace('/', "-")),
            &format!(
                "---\ntitle: \"Hard {n}\"\nkind: friction\ncomponents: [{node}]\n\
                 requirement: {id}@{at}\n---\n\nHard to build.\n"
            ),
        );
    }
    commit(&r.work, "flags");
    git(&r.work, &["push", "-q", "origin", "HEAD:main"]);
    r
}

#[test]
fn a_requirement_at_the_threshold_gets_a_reconsider_item() {
    // req:sync/file-reconsider-item@1f67b16b
    let r = flagged("impl/a", "a/x", 3);

    assert_eq!(synced(&r), ["filed backlog/reconsider-a-x.md"]);
    assert_eq!(subject(&r), "File reconsider-a-x");
    assert_eq!(
        git(&r.work, &["show", "--name-status", "--format=", "HEAD"]),
        "A\tbacklog/reconsider-a-x.md"
    );
    assert_eq!(git(&r.work, &["status", "--porcelain"]), "");
    let item = read(&r.work, "backlog/reconsider-a-x.md");
    for want in [
        "title: \"Reconsider a/x\"",
        "kind: question",
        "components: [a]",
        "scores 3",
        "backlog/friction-a-x-1.md",
        "backlog/friction-a-x-2.md",
        "backlog/friction-a-x-3.md",
    ] {
        assert!(item.contains(want), "{want}: {item}");
    }
    assert!(clm_check::run(&r.work).is_empty());
}

#[test]
fn a_reconsider_item_is_filed_after_archiving_and_proposals() {
    // req:sync/file-reconsider-item@1f67b16b
    let r = flagged("impl/a", "a/x", 3);
    let adds_z = format!(
        "---\ncomponent: a\n---\n## ADDED Requirements\n{}",
        requirement("z")
    );
    apply_change(&r, "z", "", &adds_z);
    write(&r.work, "src/test.rs", &citing("a/z", &version(&r, "a/z")));
    commit(&r.work, "test a/z");
    ledger(&r, "alice/web", &[export_csv("Let me export.\n\n")]);

    assert_eq!(
        synced(&r),
        [
            "archived z",
            "filed backlog/export-csv.md",
            "filed backlog/reconsider-a-x.md"
        ]
    );
    assert_eq!(
        git(&r.work, &["log", "-3", "--format=%s"]),
        "File reconsider-a-x\nFile export-csv\nArchive z"
    );
}

#[test]
fn a_reconsider_item_is_filed_after_a_spec_change() {
    // req:sync/file-reconsider-item@1f67b16b
    let r = flagged("impl/a", "a/x", 3);
    main_moves(&r, &[("specs/a/SPEC.md", &a_with_p_modified(&r))]);

    let left = unfinished(&r);
    assert_eq!(left[0], "spec changed a");
    assert_eq!(left.last().unwrap(), "filed backlog/reconsider-a-x.md");
    assert_eq!(subject(&r), "File reconsider-a-x");
}

#[test]
fn a_failed_commit_leaves_no_reconsider_item() {
    // req:sync/file-reconsider-item@1f67b16b
    let r = flagged("impl/a", "a/x", 3);
    let hook = r.work.join(git(
        &r.work,
        &["rev-parse", "--git-path", "hooks/pre-commit"],
    ));
    fs::create_dir_all(hook.parent().unwrap()).unwrap();
    write_exec(&hook, "#!/bin/sh\nexit 1\n");

    assert!(matches!(sync(&r.work), Err(Error::Git(_))));
    assert!(!r.work.join("backlog/reconsider-a-x.md").exists());
    assert_eq!(git(&r.work, &["status", "--porcelain"]), "");
}

#[test]
fn a_reconsider_item_already_filed_is_left_alone() {
    // req:sync/file-reconsider-item@1f67b16b
    let r = flagged("impl/a", "a/x", 3);
    write(
        &r.work,
        "backlog/reconsider-a-x.md",
        "---\ntitle: Mine\nkind: question\n---\n",
    );
    commit(&r.work, "filed by hand");

    let lines = synced(&r);
    assert!(!filed(&lines), "{lines:?}");
    assert_eq!(
        read(&r.work, "backlog/reconsider-a-x.md"),
        "---\ntitle: Mine\nkind: question\n---\n"
    );
}

#[test]
fn a_requirement_below_the_threshold_gets_none() {
    // req:sync/file-reconsider-item@1f67b16b
    let r = flagged("impl/a", "a/x", 2);

    let lines = synced(&r);
    assert!(!filed(&lines), "{lines:?}");
}

#[test]
fn another_nodes_requirement_gets_none() {
    // req:sync/file-reconsider-item@1f67b16b
    let r = flagged("impl/a", "b/y", 3);

    let lines = synced(&r);
    assert!(!filed(&lines), "{lines:?}");
    assert!(!r.work.join("backlog/reconsider-b-y.md").exists());
}

#[test]
fn no_reconsider_item_off_node_branches() {
    // req:sync/file-reconsider-item@1f67b16b
    let r = flagged("spec/y", "a/x", 3);

    let lines = synced(&r);
    assert!(!filed(&lines), "{lines:?}");
}

#[test]
fn no_reconsider_item_when_the_settings_cant_be_loaded() {
    // req:sync/file-reconsider-item@1f67b16b
    let r = flagged("impl/a", "a/x", 3);

    let report = clm_sync::sync(&r.work, |_: &Path| Err("clm.toml: bad".into())).unwrap();
    assert_eq!(report.lines, ["clm.toml: bad"]);
    assert!(!r.work.join("backlog/reconsider-a-x.md").exists());
}

#[test]
fn an_untracked_reconsider_item_counts_as_filed() {
    // req:sync/file-reconsider-item@1f67b16b
    let r = flagged("impl/a", "a/x", 3);
    write(&r.work, "backlog/reconsider-a-x.md", "mine\n");

    let lines = synced(&r);
    assert!(!filed(&lines), "{lines:?}");
    assert_eq!(read(&r.work, "backlog/reconsider-a-x.md"), "mine\n");
}

#[test]
fn a_proposal_title_is_written_as_a_json_string() {
    // req:sync/file-proposal-items@e75d388f
    let r = repo("impl/a");
    let title = "Say \"hi\": now";
    ledger(
        &r,
        "alice/web",
        &[proposed("p1", "alice/web", "say-hi", title, &[], "")],
    );

    assert_eq!(synced(&r), ["filed backlog/say-hi.md"]);
    let item = read(&r.work, "backlog/say-hi.md");
    assert!(
        item.starts_with("---\ntitle: \"Say \\\"hi\\\": now\"\nkind: idea\ncomponents: []\n---\n"),
        "{item}"
    );
    let graph = clm_model::load(&r.work).0;
    assert_eq!(graph.backlog["say-hi"].frontmatter.title, title);
}

#[test]
fn a_graph_with_violations_files_no_proposal() {
    // req:sync/file-proposal-items@e75d388f
    let r = repo("impl/a");
    write(&r.work, "backlog/broken.md", "---\nkind: idea\n---\n");
    commit(&r.work, "an item without a title");
    ledger(
        &r,
        "alice/web",
        &[proposed("p1", "alice/web", "s1", "S1", &["a"], "one")],
    );

    let lines = unfinished(&r);
    assert!(!filed(&lines), "{lines:?}");
    assert!(!r.work.join("backlog/s1.md").exists());
}

/// A repo whose `main` holds change `y`, `proposed`, with a delta adding `a/x`, and `spec/z`
/// cut from it.
fn main_holds_proposed_y() -> Repo {
    let r = repo("spec/z");
    write(
        &r.work,
        "changes/y/change.md",
        "---\nstatus: proposed\n---\n\n# y\n",
    );
    write(&r.work, "changes/y/deltas/a.md", &adds_x());
    commit(&r.work, "Propose y");
    git(&r.work, &["push", "-q", "origin", "HEAD:main"]);
    r
}

#[test]
fn a_branch_applies_a_change_main_holds() {
    // req:sync/reapply@01d24eb6
    let r = main_holds_proposed_y();
    clm_apply::apply(&r.work, "y").unwrap();
    write(
        &r.work,
        "changes/z/change.md",
        "---\nstatus: applied\n---\n\n# z\n",
    );
    commit(&r.work, "Apply y, add z");
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "c", &["p", "w"]))],
    );

    let lines = synced(&r);
    assert!(!lines.iter().any(|l| l.contains("conflict")), "{lines:?}");
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(
        spec.contains("Requirement: x") && spec.contains("Requirement: w"),
        "{spec}"
    );
    assert!(read(&r.work, "changes/y/change.md").contains("status: applied"));
}

#[test]
fn a_branch_applies_and_archives_a_change_main_holds() {
    // req:sync/reapply@01d24eb6
    let r = main_holds_proposed_y();
    clm_apply::apply(&r.work, "y").unwrap();
    commit(&r.work, "Apply y");
    archive_in(&r.work, "y", "done");
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "c", &["p", "w"]))],
    );

    let lines = synced(&r);
    assert!(!lines.iter().any(|l| l.contains("conflict")), "{lines:?}");
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(
        spec.contains("Requirement: x") && spec.contains("Requirement: w"),
        "{spec}"
    );
    assert!(read(&r.work, "changes/archive/y/change.md").contains("status: done"));
}

#[test]
fn a_change_main_holds_applied_is_not_applied_again() {
    // req:sync/reapply@01d24eb6
    let r = repo("topic");
    applied_spec_branch(&r, &adds_x());
    git(&r.work, &["push", "-q", "origin", "HEAD:main"]);
    write(&r.work, "src/new.rs", "new\n");
    commit(&r.work, "work");
    main_moves(&r, &[("src/other.rs", "other\n")]);

    assert_eq!(synced(&r), Vec::<String>::new());
    assert_eq!(git(&r.work, &["log", "-1", "--format=%s"]), "work");
}

#[test]
fn a_change_main_holds_proposed_stays_unregenerated_while_proposed() {
    // req:sync/reapply@01d24eb6
    let r = main_holds_proposed_y();
    write(&r.work, "specs/a/SPEC.md", &node_a("specd", "c2", &["p"]));
    commit(&r.work, "hand edit");
    main_moves(&r, &[("src/other.rs", "other\n")]);

    assert_eq!(synced(&r), Vec::<String>::new());
    assert!(read(&r.work, "specs/a/SPEC.md").contains("c2"));
    assert_eq!(git(&r.work, &["log", "-1", "--format=%s"]), "hand edit");
}

#[test]
fn a_branch_applies_a_change_main_holds_approved() {
    // req:sync/reapply@01d24eb6
    let r = repo("spec/z");
    write(
        &r.work,
        "changes/y/change.md",
        "---\nstatus: approved\n---\n\n# y\n",
    );
    write(&r.work, "changes/y/deltas/a.md", &adds_x());
    commit(&r.work, "Propose y");
    git(&r.work, &["push", "-q", "origin", "HEAD:main"]);
    clm_apply::apply(&r.work, "y").unwrap();
    commit(&r.work, "Apply y");
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "c", &["p", "w"]))],
    );

    synced(&r);
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(
        spec.contains("Requirement: x") && spec.contains("Requirement: w"),
        "{spec}"
    );
}

#[test]
fn a_change_archived_abandoned_is_not_applied() {
    // req:sync/reapply@01d24eb6
    let r = main_holds_proposed_y();
    let md = read(&r.work, "changes/y/change.md").replace("proposed", "applied");
    write(&r.work, "changes/y/change.md", &md);
    archive_in(&r.work, "y", "abandoned");
    main_moves(&r, &[("src/other.rs", "other\n")]);

    let _ = synced(&r);
    assert!(!read(&r.work, "specs/a/SPEC.md").contains("Requirement: x"));
}

const RENAME_P: &str =
    "---\nstatus: proposed\nrenames:\n  - requirement: a/p\n    to: q\n---\n\n## Intent\nRename.\n";

/// A branch holding the committed result of `clm rename r` (`a/p` to `a/q`).
fn rename_branch() -> Repo {
    let r = repo("spec/r");
    write(&r.work, "changes/r/change.md", RENAME_P);
    git(&r.work, &["add", "-A"]);
    clm_rename::rename(&r.work, "r").unwrap();
    commit(&r.work, "Rename r");
    r
}

#[test]
fn main_moved_under_a_rename() {
    // req:sync/regenerate-renames@0a813763
    let r = rename_branch();
    // `main` adds a requirement `w` to the node and a backlog item blocking `a/p`.
    main_moves(
        &r,
        &[
            ("specs/a/SPEC.md", &node_a("specd", "c", &["p", "w"])),
            (
                "backlog/b.md",
                "---\ntitle: B\nkind: question\ncomponents: [a]\nblocks: [a/p]\n---\nWhy?\n",
            ),
        ],
    );
    let main = origin_main(&r);

    synced(&r);

    assert_eq!(git(&r.work, &["rev-parse", "HEAD^"]), main);
    assert_eq!(git(&r.work, &["log", "-1", "--format=%s"]), "Rename r");
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(spec.contains("Requirement: q") && spec.contains("Requirement: w"));
    assert!(!spec.contains("Requirement: p"));
    assert!(read(&r.work, "backlog/b.md").contains("blocks: [a/q]"));
    let tracked = git(&r.work, &["ls-tree", "-r", "--name-only", "HEAD"]);
    assert!(tracked.contains("changes/archive/r/change.md"), "{tracked}");
    assert_eq!(git(&r.work, &["status", "--porcelain"]), "");
    assert_eq!(origin_main(&r), main, "never pushes");
}

/// Runs `git` in `w` with the git directory `g` named in the environment.
fn git_env(w: &Path, g: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(w)
        .env("GIT_DIR", g)
        .env("GIT_WORK_TREE", w)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

#[test]
fn a_clone_the_environment_names() {
    // req:sync/regenerate-renames@0a813763
    // The child half: the test binary runs itself with the variables set, so no other
    // test in this process sees them.
    if let Ok(work) = std::env::var("CLM_SYNC_TEST_WORK") {
        let report = sync(Path::new(&work)).unwrap();
        assert!(report.ok, "{:?}", report.lines);
        return;
    }
    let r = rename_branch();
    main_moves(&r, &[("src/other.rs", "other\n")]);
    let (w, g) = (r.work.clone(), r._dirs.1.path().join("g"));
    fs::rename(w.join(".git"), &g).unwrap();
    let main = origin_main(&r);
    let config = fs::read(g.join("config")).unwrap();

    let out = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "a_clone_the_environment_names", "--nocapture"])
        .env("CLM_SYNC_TEST_WORK", &w)
        .env("GIT_DIR", &g)
        .env("GIT_WORK_TREE", &w)
        .output()
        .unwrap();

    assert!(out.status.success(), "{out:?}");
    assert_eq!(git_env(&w, &g, &["rev-parse", "HEAD^"]), main);
    assert_eq!(git_env(&w, &g, &["log", "-1", "--format=%s"]), "Rename r");
    assert_eq!(git_env(&w, &g, &["status", "--porcelain"]), "");
    assert_eq!(fs::read(g.join("config")).unwrap(), config);
    assert_eq!(origin_main(&r), main, "never pushes");
}

#[test]
fn a_defect_report_survives_and_a_second_run_changes_nothing() {
    // req:sync/regenerate-renames@0a813763
    let r = rename_branch();
    write(
        &r.work,
        "backlog/d.md",
        "---\ntitle: D\nkind: bug\ncomponents: [a]\n---\nBroken.\n",
    );
    commit(&r.work, "Report d");
    main_moves(&r, &[("src/other.rs", "other\n")]);

    synced(&r);
    let head = git(&r.work, &["rev-parse", "HEAD"]);

    assert!(read(&r.work, "backlog/d.md").contains("Broken."));
    assert_eq!(
        git(&r.work, &["rev-list", "--count", "origin/main..HEAD"]),
        "1"
    );
    let moves = git(&r.work, &["reflog", "show", "HEAD"]);
    synced(&r);
    assert_eq!(git(&r.work, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&r.work, &["reflog", "show", "HEAD"]), moves);
}

#[test]
fn a_hand_edit_on_a_rename_branch() {
    // req:sync/regenerate-renames@0a813763
    let r = rename_branch();
    write(&r.work, "src/lib.rs", "one\ntwo\nthree\nfour\n");
    commit(&r.work, "hand edit");
    main_moves(&r, &[("src/other.rs", "other\n")]);
    let head = git(&r.work, &["rev-parse", "HEAD"]);

    let lines = unfinished(&r);

    assert_eq!(lines, ["not a rename branch src/lib.rs"]);
    assert_eq!(git(&r.work, &["rev-parse", "HEAD"]), head);
}

#[test]
fn a_rename_main_refuses_leaves_the_branch() {
    // req:sync/regenerate-renames@0a813763
    let r = rename_branch();
    // `main` gains a `q` of its own, so `a/p` to `a/q` can no longer be carried out.
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "c", &["p", "q"]))],
    );
    let head = git(&r.work, &["rev-parse", "HEAD"]);

    let lines = unfinished(&r);

    assert!(lines.len() == 1 && lines[0].contains("a/q"), "{lines:?}");
    assert_eq!(git(&r.work, &["rev-parse", "HEAD"]), head);
}

#[test]
fn proposals_are_filed_in_slug_order() {
    // req:sync/file-proposal-items@e75d388f
    let r = repo("impl/a");
    ledger(
        &r,
        "alice/web",
        &[
            proposed("p1", "alice/web", "zebra", "Z", &["a"], "z"),
            proposed("p2", "alice/web", "apple", "A", &["a"], "a"),
        ],
    );
    let report = sync(&r.work).unwrap();
    assert_eq!(
        report.lines,
        ["filed backlog/apple.md", "filed backlog/zebra.md"]
    );
}
