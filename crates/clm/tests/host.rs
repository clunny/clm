//! `clm page`, `clm branch delete` and `clm issue` (specs/host/SPEC.md), with a fake `gh`
//! first on `PATH`.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use common::*;
use tempfile::TempDir;

/// A fake `gh` whose `pr view` prints `$FAKE_PR_VIEW` (failing when it is unset), whose
/// `pr create` and `issue create` print a URL, or exit `$FAKE_CREATE_EXIT` when set; every
/// call is logged to `$FAKE_OUT/gh`, one per line.
struct Gh {
    dir: TempDir,
}

fn fake_gh() -> Gh {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("bin")).unwrap();
    write_exec(
        &dir.path().join("bin/gh"),
        "#!/bin/sh\necho \"$*\" >> \"$FAKE_OUT/gh\"\n\
         if [ \"$2\" = create ] && [ -n \"$FAKE_CREATE_EXIT\" ]; then \
         echo 'create failed' >&2; exit \"$FAKE_CREATE_EXIT\"; fi\n\
         case \"$1 $2\" in\n\
         'pr view') [ -z \"$FAKE_VIEW_ERROR\" ] || { echo \"$FAKE_VIEW_ERROR\" >&2; exit 1; }; \
         [ -n \"$FAKE_PR_VIEW\" ] || { echo 'no pull requests found' >&2; exit 1; }; \
         printf '%s\\n' \"$FAKE_PR_VIEW\";;\n\
         'pr create') echo 'Creating pull request'; echo https://github.com/o/r/pull/9;;\n\
         'issue create') echo https://github.com/o/r/issues/3;;\nesac\n",
    );
    Gh { dir }
}

impl Gh {
    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.dir.path().join("gh"))
            .unwrap_or_default()
            .lines()
            .map(String::from)
            .collect()
    }
}

/// `clm <args>` in `dir` with `gh` first on `PATH` and no user settings: exit code, stdout
/// and stderr.
fn clm(dir: &Path, gh: &Gh, args: &[&str], env: &[(&str, &str)]) -> (Option<i32>, String, String) {
    let path = format!(
        "{}:{}",
        gh.dir.path().join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = output(
        Command::new(env!("CARGO_BIN_EXE_clm"))
            .args(args)
            .current_dir(dir)
            .env("PATH", path)
            .env("XDG_CONFIG_HOME", gh.dir.path())
            .env("FAKE_OUT", gh.dir.path())
            .envs(env.iter().copied()),
    );
    (
        out.status.code(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

/// A checkout whose `clm.toml` sets `host`, on branch `spec/x`, holding `body.md`.
fn checkout(host: &str) -> Repo {
    let r = repo();
    write(&r.work, "clm.toml", &format!("host = \"{host}\"\n"));
    write(&r.work, "body.md", "The body\n");
    git(&r.work, &["checkout", "-q", "-b", "spec/x"]);
    r
}

fn tip(dir: &Path, rev: &str) -> String {
    git(dir, &["rev-parse", rev]).trim().to_string()
}

/// Pushes a new commit to `origin`'s `impl/a` and returns its sha.
fn push_impl_a(r: &Repo, file: &str) -> String {
    write(&r.work, file, "x\n");
    git(&r.work, &["add", file]);
    git(&r.work, &["commit", "-q", "-m", file]);
    git(
        &r.work,
        &["push", "-q", "-f", "origin", "HEAD:refs/heads/impl/a"],
    );
    tip(&r.work, "HEAD")
}

/// Pushes `branch` to `origin` from a second clone, as another session would, with a new
/// commit holding `file`, and returns its sha. This clone doesn't fetch it.
fn other_clone_push(r: &Repo, branch: &str, file: &str) -> String {
    let dir = TempDir::new().unwrap();
    let other = dir.path().join("other");
    git(
        dir.path(),
        &[
            "clone",
            "-q",
            r.origin.to_str().unwrap(),
            other.to_str().unwrap(),
        ],
    );
    git(&other, &["checkout", "-q", "-B", branch, "origin/main"]);
    write(&other, file, "other\n");
    git(&other, &["add", file]);
    git(&other, &["commit", "-q", "-m", file]);
    git(
        &other,
        &[
            "push",
            "-q",
            "-f",
            "origin",
            &format!("HEAD:refs/heads/{branch}"),
        ],
    );
    tip(&other, "HEAD")
}

fn on_origin(r: &Repo, branch: &str) -> String {
    git(
        &r.origin,
        &[
            "for-each-ref",
            "--format=%(objectname)",
            &format!("refs/heads/{branch}"),
        ],
    )
    .trim()
    .to_string()
}

#[test]
fn scenario_branch_deleted() {
    // req:host/branch-delete@18b4bda5
    for host in ["github", "git", "none"] {
        let (r, gh) = (checkout(host), fake_gh());
        let s = push_impl_a(&r, "a.txt");
        git(&r.work, &["push", "-q", "origin", "spec/x"]);
        let (code, _, err) = clm(
            &r.work,
            &gh,
            &["branch", "delete", "impl/a", "--lease", &s],
            &[],
        );
        assert_eq!(code, Some(0), "{host}: {err}");
        let (code, _, err) = clm(&r.work, &gh, &["branch", "delete", "spec/x"], &[]);
        assert_eq!(code, Some(0), "{host}: {err}");
        let left = git(
            &r.origin,
            &["for-each-ref", "refs/heads/impl/a", "refs/heads/spec/x"],
        );
        assert!(left.is_empty(), "{host}: {left}");
        assert!(gh.calls().is_empty(), "{:?}", gh.calls());
    }
}

#[test]
fn scenario_branch_another_sessions() {
    // req:host/branch-delete@18b4bda5
    let (r, gh) = (checkout("github"), fake_gh());
    let u = other_clone_push(&r, "impl/b", "b.txt");
    let v = other_clone_push(&r, "spec/y", "y.txt");
    git(&r.work, &["fetch", "-q", "origin"]);
    let (code, _, err) = clm(
        &r.work,
        &gh,
        &["branch", "delete", "impl/b", "--lease", &u],
        &[],
    );
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("didn't push"), "{err}");
    let (code, _, err) = clm(&r.work, &gh, &["branch", "delete", "spec/y"], &[]);
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("didn't push"), "{err}");
    assert_eq!(on_origin(&r, "impl/b"), u);
    assert_eq!(on_origin(&r, "spec/y"), v);
}

#[test]
fn branch_delete_takes_an_abbreviated_lease_of_the_pushed_tip() {
    // req:host/branch-delete@18b4bda5
    let (r, gh) = (checkout("github"), fake_gh());
    let tip = push_impl_a(&r, "a.txt");
    let (code, _, err) = clm(
        &r.work,
        &gh,
        &["branch", "delete", "impl/a", "--lease", &tip[..7]],
        &[],
    );
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(on_origin(&r, "impl/a"), "");
}

#[test]
fn branch_delete_refuses_a_lease_that_names_no_pushed_tip() {
    // req:host/branch-delete@18b4bda5
    let (r, gh) = (checkout("github"), fake_gh());
    let old = push_impl_a(&r, "a.txt");
    let new = push_impl_a(&r, "b.txt");
    for lease in [&old[..7], "HEAD", "origin/impl/a", "deadbeef", "not a hash"] {
        let (code, _, err) = clm(
            &r.work,
            &gh,
            &["branch", "delete", "impl/a", "--lease", lease],
            &[],
        );
        assert_eq!(code, Some(1), "{lease}: {err}");
        assert!(err.contains("didn't push"), "{lease}: {err}");
        assert_eq!(on_origin(&r, "impl/a"), new, "{lease}");
    }
}

#[test]
fn branch_delete_names_a_lease_that_is_not_the_pushed_tip() {
    // req:host/branch-delete@18b4bda5
    let (r, gh) = (checkout("github"), fake_gh());
    let old = push_impl_a(&r, "a.txt");
    let new = push_impl_a(&r, "b.txt");
    let (code, _, err) = clm(
        &r.work,
        &gh,
        &["branch", "delete", "impl/a", "--lease", &old],
        &[],
    );
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("didn't push"), "{err}");
    assert_eq!(on_origin(&r, "impl/a"), new);
}

#[test]
fn scenario_branch_never_fetched() {
    // req:host/branch-delete@18b4bda5
    let (r, gh) = (checkout("github"), fake_gh());
    let z = other_clone_push(&r, "spec/z", "z.txt");
    let (code, _, err) = clm(&r.work, &gh, &["branch", "delete", "spec/z"], &[]);
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("origin/spec/z"), "{err}");
    assert!(err.contains("didn't push"), "{err}");
    assert_eq!(on_origin(&r, "spec/z"), z);
}

#[test]
fn scenario_branch_main_refused() {
    // req:host/branch-delete@18b4bda5
    let (r, gh) = (checkout("github"), fake_gh());
    let m = on_origin(&r, "main");
    let (code, _, err) = clm(
        &r.work,
        &gh,
        &["branch", "delete", "main", "--lease", &m],
        &[],
    );
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("main"), "{err}");
    assert_eq!(on_origin(&r, "main"), m);
}

#[test]
fn branch_delete_refuses_a_ledger_branch() {
    // req:host/branch-delete@18b4bda5
    let (r, gh) = (checkout("github"), fake_gh());
    git(
        &r.work,
        &["push", "-q", "origin", "HEAD:refs/heads/clm/ledger/p/c"],
    );
    let (code, _, err) = clm(&r.work, &gh, &["branch", "delete", "clm/ledger/p/c"], &[]);
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("clm/ledger/p/c"), "{err}");
    assert!(!on_origin(&r, "clm/ledger/p/c").is_empty());
}

#[test]
fn scenario_branch_moved() {
    // req:host/branch-delete@18b4bda5
    let (r, gh) = (checkout("github"), fake_gh());
    let s = push_impl_a(&r, "a.txt");
    let t = other_clone_push(&r, "impl/a", "b.txt");
    let (code, _, err) = clm(
        &r.work,
        &gh,
        &["branch", "delete", "impl/a", "--lease", &s],
        &[],
    );
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("impl/a"), "{err}");
    assert_eq!(tip(&r.origin, "refs/heads/impl/a"), t);
}

#[test]
fn branch_moved_keeps_its_message() {
    // req:host/shared-plumbing@a65259c7
    let (r, gh) = (checkout("github"), fake_gh());
    let s = push_impl_a(&r, "a.txt");
    other_clone_push(&r, "impl/a", "b.txt");
    let (code, _, err) = clm(
        &r.work,
        &gh,
        &["branch", "delete", "impl/a", "--lease", &s],
        &[],
    );
    assert_eq!(code, Some(1), "{err}");
    let message = format!("`impl/a` on origin is no longer at {s}; left it");
    assert!(err.contains(&message), "{err}");
}

#[test]
fn branch_delete_without_origin_has_nothing_to_do() {
    // req:host/branch-delete@18b4bda5
    let (r, gh) = (checkout("none"), fake_gh());
    git(&r.work, &["remote", "remove", "origin"]);
    let (code, _, err) = clm(
        &r.work,
        &gh,
        &["branch", "delete", "impl/a", "--lease", "abc"],
        &[],
    );
    assert_eq!(code, Some(0), "{err}");
}

#[test]
fn branch_delete_of_a_branch_already_gone_has_nothing_to_do() {
    // req:host/branch-delete@18b4bda5
    let (r, gh) = (checkout("github"), fake_gh());
    let s = tip(&r.work, "HEAD");
    let (code, _, err) = clm(
        &r.work,
        &gh,
        &["branch", "delete", "impl/gone", "--lease", &s],
        &[],
    );
    assert_eq!(code, Some(0), "{err}");
}

/// Commits `file` on the checked-out branch of `r.work`.
fn commit_file(r: &Repo, file: &str) {
    write(&r.work, file, "x\n");
    git(&r.work, &["add", file]);
    git(&r.work, &["commit", "-q", "-m", file]);
}

#[test]
fn scenario_push_rebased_branch() {
    // req:host/branch-push@2c314eee
    for host in ["github", "git", "none"] {
        let (r, gh) = (checkout(host), fake_gh());
        git(&r.work, &["checkout", "-q", "-b", "impl/a"]);
        commit_file(&r, "a.txt");
        git(&r.work, &["push", "-q", "origin", "impl/a"]);
        git(&r.work, &["commit", "-q", "--amend", "-m", "again"]);
        let t = tip(&r.work, "HEAD");
        let (code, out, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
        assert_eq!(code, Some(0), "{host}: {err}");
        assert_eq!(out, format!("pushed impl/a {t}\n"));
        assert_eq!(on_origin(&r, "impl/a"), t);
        let upstream = git(&r.work, &["rev-parse", "--abbrev-ref", "impl/a@{upstream}"]);
        assert_eq!(upstream.trim(), "origin/impl/a");
        assert!(gh.calls().is_empty());
        // Already there: pushing again succeeds.
        let (code, _, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
        assert_eq!(code, Some(0), "{host}: {err}");
    }
}

#[test]
fn scenario_push_new_branch() {
    // req:host/branch-push@2c314eee
    let (r, gh) = (checkout("github"), fake_gh());
    let s = tip(&r.work, "HEAD");
    let (code, _, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(on_origin(&r, "spec/x"), s);
    let upstream = git(&r.work, &["rev-parse", "--abbrev-ref", "spec/x@{upstream}"]);
    assert_eq!(upstream.trim(), "origin/spec/x");
}

#[test]
fn scenario_push_new_branch_refused_when_origin_has_it() {
    // req:host/branch-push@2c314eee
    let (r, gh) = (checkout("github"), fake_gh());
    let u = other_clone_push(&r, "spec/x", "u.txt");
    let (code, _, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
    assert_eq!(code, Some(1), "{err}");
    assert_eq!(on_origin(&r, "spec/x"), u);
}

#[test]
fn scenario_push_moved_since_last_fetch() {
    // req:host/branch-push@2c314eee
    let (r, gh) = (checkout("github"), fake_gh());
    git(&r.work, &["checkout", "-q", "-b", "impl/a"]);
    commit_file(&r, "a.txt");
    git(&r.work, &["push", "-q", "origin", "impl/a"]);
    git(&r.work, &["commit", "-q", "--amend", "-m", "again"]);
    let u = other_clone_push(&r, "impl/a", "u.txt");
    let (code, out, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
    assert_eq!(code, Some(1), "{err}");
    assert!(format!("{out}{err}").contains("stale info"), "{out}{err}");
    assert_eq!(on_origin(&r, "impl/a"), u);
}

#[test]
fn scenario_push_fetched_but_never_held() {
    // req:host/branch-push@2c314eee
    let (r, gh) = (checkout("github"), fake_gh());
    git(&r.work, &["checkout", "-q", "-b", "impl/a"]);
    commit_file(&r, "a.txt");
    let u = other_clone_push(&r, "impl/a", "u.txt");
    git(&r.work, &["fetch", "-q", "origin"]);
    let (code, _, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
    assert_eq!(code, Some(1), "{err}");
    assert_eq!(on_origin(&r, "impl/a"), u);
}

#[test]
fn scenario_push_main_refused() {
    // req:host/branch-push@2c314eee
    let (r, gh) = (checkout("github"), fake_gh());
    let main = on_origin(&r, "main");
    git(&r.work, &["checkout", "-q", "main"]);
    commit_file(&r, "m.txt");
    let (code, _, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("main"), "{err}");
    assert_eq!(on_origin(&r, "main"), main);

    git(&r.work, &["checkout", "-q", "--detach"]);
    let (code, _, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("detached"), "{err}");

    git(&r.work, &["checkout", "-q", "-b", "clm/ledger/p/c"]);
    let (code, _, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
    assert_eq!(code, Some(1), "{err}");
    assert!(on_origin(&r, "clm/ledger/p/c").is_empty());
    assert_eq!(on_origin(&r, "main"), main);
}

#[test]
fn push_prints_the_remotes_lines_and_succeeds_when_origin_has_the_commit() {
    // req:host/branch-push@2c314eee
    let (r, gh) = (checkout("github"), fake_gh());
    // origin already holds the commit, and this clone has no tracking ref for it.
    git(&r.work, &["push", "-q", "origin", "spec/x"]);
    git(&r.work, &["update-ref", "-d", "refs/remotes/origin/spec/x"]);
    let (code, _, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
    assert_eq!(code, Some(0), "{err}");
    // A remote that refuses: its lines are printed.
    pre_receive(&r, "echo 'remote says no' >&2; exit 1\n");
    commit_file(&r, "n.txt");
    let (code, out, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
    assert_eq!(code, Some(1), "{err}");
    assert!(
        format!("{out}{err}").contains("remote says no"),
        "{out}{err}"
    );
}

#[test]
fn push_without_origin_exits_1() {
    // req:host/branch-push@2c314eee
    let (r, gh) = (checkout("none"), fake_gh());
    git(&r.work, &["remote", "remove", "origin"]);
    let (code, _, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
    assert_eq!(code, Some(1), "{err}");
}

const OPEN_ARGS: [&str; 7] = [
    "pr",
    "open",
    "spec/x",
    "--title",
    "x",
    "--body-file",
    "body.md",
];

#[test]
fn scenario_page_open_github() {
    // req:host/pr-open@db347388
    let (r, gh) = (checkout("github"), fake_gh());
    let (code, out, err) = clm(&r.work, &gh, &OPEN_ARGS, &[]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "https://github.com/o/r/pull/9\n");
    let creates: Vec<_> = gh
        .calls()
        .into_iter()
        .filter(|c| c.starts_with("pr create"))
        .collect();
    assert_eq!(
        creates,
        ["pr create --base main --head spec/x --title x --body-file body.md"]
    );
}

#[test]
fn scenario_page_already_open() {
    // req:host/pr-open@db347388
    let (r, gh) = (checkout("github"), fake_gh());
    let view = r#"{"url":"https://github.com/o/r/pull/4","state":"OPEN","headRefName":"spec/x"}"#;
    let (code, out, err) = clm(&r.work, &gh, &OPEN_ARGS, &[("FAKE_PR_VIEW", view)]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "https://github.com/o/r/pull/4\n");
    assert!(
        !gh.calls().iter().any(|c| c.starts_with("pr create")),
        "{:?}",
        gh.calls()
    );
}

#[test]
fn page_open_after_a_merged_page_opens_another() {
    // req:host/pr-open@db347388
    let (r, gh) = (checkout("github"), fake_gh());
    let view = r#"{"url":"https://github.com/o/r/pull/4","state":"MERGED","headRefName":"spec/x"}"#;
    let (code, out, err) = clm(&r.work, &gh, &OPEN_ARGS, &[("FAKE_PR_VIEW", view)]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "https://github.com/o/r/pull/9\n");
}

#[test]
fn page_open_exits_2_when_gh_fails() {
    // req:host/pr-open@db347388
    let (r, gh) = (checkout("github"), fake_gh());
    let (code, _, err) = clm(&r.work, &gh, &OPEN_ARGS, &[("FAKE_CREATE_EXIT", "1")]);
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("create failed"), "{err}");
}

#[test]
fn scenario_page_open_no_host() {
    // req:host/pr-open@db347388
    for host in ["none", "git"] {
        let (r, gh) = (checkout(host), fake_gh());
        let (code, out, err) = clm(&r.work, &gh, &OPEN_ARGS, &[]);
        assert_eq!(code, Some(0), "{host}: {err}");
        assert_eq!(out, "none\n");
        assert!(gh.calls().is_empty(), "{:?}", gh.calls());
    }
}

const PR_7: &str = r#"{"number":7,"state":"OPEN","headRefName":"spec/x"}"#;
const BODY_ARGS: [&str; 3] = ["pr", "body", "body.md"];

#[test]
fn scenario_page_body_updated() {
    // req:host/pr-body@7e8f221a
    let (r, gh) = (checkout("github"), fake_gh());
    let (code, out, err) = clm(&r.work, &gh, &BODY_ARGS, &[("FAKE_PR_VIEW", PR_7)]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "updated #7\n");
    let writes: Vec<_> = gh
        .calls()
        .into_iter()
        .filter(|c| !c.starts_with("pr view"))
        .collect();
    assert_eq!(
        writes,
        ["api -X PATCH repos/{owner}/{repo}/pulls/7 -F body=@body.md"]
    );
}

#[test]
fn scenario_page_body_blank_file() {
    // req:host/pr-body@7e8f221a
    let (r, gh) = (checkout("github"), fake_gh());
    write(&r.work, "body.md", " \n\t\n");
    let (code, _, err) = clm(&r.work, &gh, &BODY_ARGS, &[("FAKE_PR_VIEW", PR_7)]);
    assert_eq!(code, Some(1), "{err}");
    fs::remove_file(r.work.join("body.md")).unwrap();
    let (code, _, err) = clm(&r.work, &gh, &BODY_ARGS, &[("FAKE_PR_VIEW", PR_7)]);
    assert_eq!(code, Some(1), "{err}");
    assert!(gh.calls().is_empty(), "{:?}", gh.calls());
}

#[test]
fn page_body_without_an_open_page_exits_1() {
    // req:host/pr-body@7e8f221a
    let (r, gh) = (checkout("github"), fake_gh());
    let merged = r#"{"number":7,"state":"MERGED","headRefName":"spec/x"}"#;
    let other = r#"{"number":8,"state":"OPEN","headRefName":"spec/y"}"#;
    for env in [
        vec![("FAKE_PR_VIEW", merged)],
        vec![("FAKE_PR_VIEW", other)],
        vec![],
    ] {
        let (code, _, err) = clm(&r.work, &gh, &BODY_ARGS, &env);
        assert_eq!(code, Some(1), "{env:?}: {err}");
        assert!(err.contains("spec/x"), "{err}");
    }
    git(&r.work, &["checkout", "-q", "--detach"]);
    let (code, _, err) = clm(&r.work, &gh, &BODY_ARGS, &[("FAKE_PR_VIEW", PR_7)]);
    assert_eq!(code, Some(1), "{err}");
    assert!(
        !gh.calls().iter().any(|c| c.starts_with("api")),
        "{:?}",
        gh.calls()
    );
}

#[test]
fn a_failing_gh_pr_view_exits_2_and_opens_nothing() {
    // req:host/pr-body@7e8f221a
    // req:host/pr-open@db347388
    let (r, gh) = (checkout("github"), fake_gh());
    let env = [("FAKE_VIEW_ERROR", "HTTP 401: Bad credentials")];
    let (code, _, err) = clm(&r.work, &gh, &BODY_ARGS, &env);
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("Bad credentials"), "{err}");
    let (code, _, err) = clm(&r.work, &gh, &OPEN_ARGS, &env);
    assert_eq!(code, Some(2), "{err}");
    assert!(
        !gh.calls().iter().any(|c| !c.starts_with("pr view")),
        "{:?}",
        gh.calls()
    );
}

#[test]
fn unloadable_settings_exit_2() {
    let (r, gh) = (checkout("github"), fake_gh());
    write(&r.work, "clm.toml", "host = \"nowhere\"\n");
    let (code, _, err) = clm(
        &r.work,
        &gh,
        &["branch", "delete", "impl/a", "--lease", "abc"],
        &[],
    );
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("clm.toml"), "{err}");
}

#[test]
fn page_body_without_a_host_does_nothing() {
    // req:host/pr-body@7e8f221a
    for host in ["none", "git"] {
        let (r, gh) = (checkout(host), fake_gh());
        let (code, out, err) = clm(&r.work, &gh, &BODY_ARGS, &[]);
        assert_eq!((code, out.as_str()), (Some(0), ""), "{host}: {err}");
        assert!(gh.calls().is_empty(), "{:?}", gh.calls());
    }
}

const ISSUE_ARGS: [&str; 7] = [
    "issue",
    "--title",
    "x",
    "--body-file",
    "r.md",
    "--repo",
    "clunny/clm",
];

#[test]
fn scenario_issue_filed() {
    // req:host/issue@02ae5818
    let (r, gh) = (checkout("github"), fake_gh());
    write(&r.work, "r.md", "The report\n");
    let (code, out, err) = clm(&r.work, &gh, &ISSUE_ARGS, &[]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "https://github.com/o/r/issues/3\n");
    assert_eq!(
        gh.calls(),
        ["issue create --repo clunny/clm --title x --body-file r.md"]
    );
}

#[test]
fn scenario_issue_nowhere_to_file() {
    // req:host/issue@02ae5818
    for host in ["git", "none"] {
        let (r, gh) = (checkout(host), fake_gh());
        write(&r.work, "r.md", "The report\n");
        let (code, out, err) = clm(&r.work, &gh, &ISSUE_ARGS, &[]);
        assert_eq!(code, Some(1), "{host}: {err}");
        assert_eq!(out, "The report\n");
        assert!(err.contains("nowhere to file it"), "{err}");
        assert!(gh.calls().is_empty(), "{:?}", gh.calls());
    }
}

#[test]
fn scenario_pr_with_no_host() {
    // req:clm/renamed-commands@f600d2ff
    let (r, gh) = (checkout("none"), fake_gh());
    let (code, out, err) = clm(&r.work, &gh, &OPEN_ARGS, &[]);
    assert_eq!((code, out.as_str()), (Some(0), "none\n"), "{err}");
    assert!(gh.calls().is_empty(), "{:?}", gh.calls());
    // A failure names the command as it was run.
    let (r, gh) = (checkout("github"), fake_gh());
    let (_, _, err) = clm(&r.work, &gh, &OPEN_ARGS, &[("FAKE_CREATE_EXIT", "1")]);
    assert!(err.contains("clm pr open"), "{err}");
    let env = [("FAKE_PR_VIEW", PR_7)];
    let (code, out, err) = clm(&r.work, &gh, &BODY_ARGS, &env);
    assert_eq!((code, out.as_str()), (Some(0), "updated #7\n"), "{err}");
    fs::remove_file(r.work.join("body.md")).unwrap();
    let (_, _, err) = clm(&r.work, &gh, &BODY_ARGS, &env);
    assert!(err.contains("clm pr body"), "{err}");
}

/// A fake `az` beside the fake `gh`: `repos pr list` prints `[]`, `repos pr create` prints a
/// pull request 12, and `$FAKE_AZ_FAIL` makes every call print it on stderr and exit 1. Every
/// call is logged to `$FAKE_OUT/az`.
fn fake_az(gh: &Gh) {
    write_exec(
        &gh.dir.path().join("bin/az"),
        "#!/bin/sh\necho \"$*\" >> \"$FAKE_OUT/az\"\n\
         [ -z \"$FAKE_AZ_FAIL\" ] || { echo \"$FAKE_AZ_FAIL\" >&2; exit 1; }\n\
         case \"$*\" in\n\
         *'repos pr list'*) echo '[]';;\n\
         *'repos pr create'*) echo \
         '{\"pullRequestId\":12,\"repository\":{\"webUrl\":\"https://dev.azure.com/o/p/_git/r\"}}';;\n\
         esac\n",
    );
}

fn az_calls(gh: &Gh) -> Vec<String> {
    fs::read_to_string(gh.dir.path().join("az"))
        .unwrap_or_default()
        .lines()
        .map(String::from)
        .collect()
}

#[test]
fn scenario_azure_pull_request_through_the_setting() {
    // req:host/azure-selected@ef11e15d
    let (r, gh) = (checkout("azure"), fake_gh());
    fake_az(&gh);
    let (code, out, err) = clm(&r.work, &gh, &OPEN_ARGS, &[]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "https://dev.azure.com/o/p/_git/r/pullrequest/12\n");
    assert!(
        az_calls(&gh).iter().any(|c| c.contains("repos pr create")),
        "{:?}",
        az_calls(&gh)
    );
    assert!(gh.calls().is_empty(), "{:?}", gh.calls());
}

#[test]
fn scenario_azure_not_signed_in() {
    // req:host/azure-selected@ef11e15d
    let (r, gh) = (checkout("azure"), fake_gh());
    fake_az(&gh);
    let env = [("FAKE_AZ_FAIL", "Please run 'az login'")];
    let (code, out, err) = clm(&r.work, &gh, &OPEN_ARGS, &env);
    assert_eq!(code, Some(1), "{out}{err}");
    assert!(format!("{out}{err}").contains("az login"), "{out}{err}");
}

#[test]
fn scenario_azure_issue_through_the_setting() {
    // req:host/azure-selected@ef11e15d
    let (r, gh) = (checkout("azure"), fake_gh());
    fake_az(&gh);
    write(&r.work, "r.md", "The report\n");
    let (code, out, err) = clm(&r.work, &gh, &ISSUE_ARGS, &[]);
    assert_eq!(code, Some(1), "{err}");
    assert_eq!(out, "The report\n");
    assert!(err.contains("nowhere to file it"), "{err}");
    assert!(gh.calls().is_empty() && az_calls(&gh).is_empty());
}
