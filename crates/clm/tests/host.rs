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

#[test]
fn scenario_branch_deleted() {
    // req:host/branch-delete@d8b1bee9
    for host in ["github", "git", "none"] {
        let (r, gh) = (checkout(host), fake_gh());
        let s = push_impl_a(&r, "a.txt");
        let (code, _, err) = clm(
            &r.work,
            &gh,
            &["branch", "delete", "impl/a", "--lease", &s],
            &[],
        );
        assert_eq!(code, Some(0), "{host}: {err}");
        let left = git(&r.origin, &["for-each-ref", "refs/heads/impl/a"]);
        assert!(left.is_empty(), "{host}: {left}");
        assert!(gh.calls().is_empty(), "{:?}", gh.calls());
    }
}

#[test]
fn scenario_branch_moved() {
    // req:host/branch-delete@d8b1bee9
    let (r, gh) = (checkout("github"), fake_gh());
    let s = push_impl_a(&r, "a.txt");
    let t = push_impl_a(&r, "b.txt");
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
    push_impl_a(&r, "b.txt");
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
    // req:host/branch-delete@d8b1bee9
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
    // req:host/branch-delete@d8b1bee9
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
    // req:host/branch-delete@d8b1bee9
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
