//! `clm pr`, `clm branch` and `clm issue` as `clm-host`'s calls (`host/commands-in-crate`),
//! with a fake `gh` first on `PATH`: what each writes to its streams and the status it returns.

#[path = "../../clm-git/tests/exec/mod.rs"]
mod exec;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

use exec::write_exec;
use tempfile::TempDir;

/// A clone of a bare `origin` whose `main` holds one commit.
struct Repo {
    _dirs: (TempDir, TempDir),
    origin: PathBuf,
    work: PathBuf,
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
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

fn write(dir: &Path, rel: &str, content: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn repo() -> Repo {
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
    git(&work, &["config", "user.name", "t"]);
    git(&work, &["config", "user.email", "t@t"]);
    write(&work, "specs/SPEC.md", "---\ntitle: T\n---\n");
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-qm", "base"]);
    git(&work, &["push", "-q", "origin", "HEAD:main"]);
    Repo {
        _dirs: (o, w),
        origin,
        work,
    }
}

/// A `pre-receive` hook on `origin` running `body`.
fn pre_receive(r: &Repo, body: &str) {
    write_exec(
        &r.origin.join("hooks/pre-receive"),
        &format!("#!/bin/sh\n{body}"),
    );
}

/// One of `clm-host`'s calls, as these tests pass it its arguments.
type Call = fn(
    Vec<String>,
    &Path,
    &dyn Fn(&str) -> Option<String>,
    &mut dyn std::io::Write,
    &mut dyn std::io::Write,
) -> u8;

/// Puts back the directory and the variables a call changed, also when it panics.
struct Restore {
    cwd: PathBuf,
    vars: Vec<(String, Option<std::ffi::OsString>)>,
}

impl Drop for Restore {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.cwd);
        // SAFETY: as where `clm` sets them.
        unsafe {
            for (k, v) in &self.vars {
                match v {
                    Some(v) => std::env::set_var(k, v),
                    None => std::env::remove_var(k),
                }
            }
        }
    }
}

/// The call's environment is this process's: one call at a time sets it.
static CALL: Mutex<()> = Mutex::new(());

/// `clm <args>` as `clm-host`'s call, with `dir` as the root it is given and the directory it
/// runs in, `gh` first on `PATH`, no user settings and `env` set for the length of the call:
/// exit code, stdout and stderr. The binary's run set these on its own process; the call's
/// `gh`, `az` and `git` inherit this process's, so the test sets them here.
fn clm(dir: &Path, gh: &Gh, args: &[&str], env: &[(&str, &str)]) -> (Option<i32>, String, String) {
    let _one = CALL.lock().unwrap_or_else(|e| e.into_inner());
    let path = std::env::var("PATH").unwrap_or_default();
    let mut vars = vec![
        (
            "PATH",
            format!("{}:{path}", gh.dir.path().join("bin").display()),
        ),
        ("XDG_CONFIG_HOME", gh.dir.path().display().to_string()),
        ("FAKE_OUT", gh.dir.path().display().to_string()),
    ];
    vars.extend(env.iter().map(|(k, v)| (*k, v.to_string())));
    let _restore = Restore {
        cwd: std::env::current_dir().unwrap(),
        vars: vars
            .iter()
            .map(|(k, _)| (k.to_string(), std::env::var_os(k)))
            .collect(),
    };
    // SAFETY: the tests run one per process (`ci/nextest-runner`), and under `cargo test`
    // every call that changes the environment holds CALL.
    unsafe {
        for (k, v) in &vars {
            std::env::set_var(k, v);
        }
    }
    std::env::set_current_dir(dir).unwrap();
    let (name, rest) = args.split_first().unwrap();
    let call: Call = match *name {
        "pr" => clm_host::pr_command,
        "issue" => clm_host::issue_command,
        "branch" => clm_host::branch_command,
        other => panic!("not a command of clm-host: {other}"),
    };
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let code = call(
        rest.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
        dir,
        &|key| std::env::var(key).ok(),
        &mut stdout,
        &mut stderr,
    );
    (
        Some(i32::from(code)),
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

#[test]
fn scenario_a_command_called_in_a_test() {
    // req:host/commands-in-crate@f798e133
    let (r, gh) = (checkout("github"), fake_gh());
    let (code, out, err) = clm(&r.work, &gh, &OPEN_ARGS, &[]);
    assert_eq!(
        (code, out.as_str(), err.as_str()),
        (Some(0), "https://github.com/o/r/pull/9\n", "")
    );
    let env = [("FAKE_CREATE_EXIT", "1")];
    let (code, out, err) = clm(&r.work, &gh, &OPEN_ARGS, &env);
    assert_eq!((code, out.as_str()), (Some(2), ""), "{err}");
    assert!(err.starts_with("clm pr open: "), "{err}");
    git(&r.work, &["checkout", "-q", "main"]);
    let (code, out, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
    assert_eq!((code, out.as_str()), (Some(1), ""), "{err}");
    assert!(err.starts_with("clm branch push: "), "{err}");
    git(&r.work, &["checkout", "-q", "spec/x"]);
    let (code, out, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
    let sha = git(&r.work, &["rev-parse", "HEAD"]);
    assert_eq!(
        (code, out, err),
        (Some(0), format!("pushed spec/x {sha}"), String::new())
    );
}

#[test]
fn scenario_arguments_it_cant_parse() {
    // req:host/commands-in-crate@f798e133
    let (r, gh) = (checkout("github"), fake_gh());
    for args in [
        &["pr", "open", "--nope"][..],
        &["pr"],
        &["branch", "push", "--nope"],
        &["issue", "--title", "x"],
    ] {
        let (code, out, err) = clm(&r.work, &gh, args, &[]);
        assert_eq!((code, out.as_str()), (Some(2), ""), "{args:?}: {err}");
        assert!(err.contains("Usage:"), "{args:?}: {err}");
    }
    assert!(gh.calls().is_empty(), "{:?}", gh.calls());
}

#[test]
fn the_given_root_overrides_the_found_one() {
    // req:host/commands-in-crate@f798e133
    let (r, gh) = (checkout("none"), fake_gh());
    let elsewhere = TempDir::new().unwrap();
    write(elsewhere.path(), "clm.toml", "host = \"nowhere\"\n");
    let root = r.work.display().to_string();
    let _one = CALL.lock().unwrap_or_else(|e| e.into_inner());
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = clm_host::pr_command(
        ["open", "--root", &root],
        elsewhere.path(),
        &|_| None,
        &mut out,
        &mut err,
    );
    let err = String::from_utf8(err).unwrap();
    assert_eq!(
        (code, String::from_utf8(out).unwrap()),
        (0, "none\n".into()),
        "{err}"
    );
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = clm_host::issue_command(
        ["--title", "x", "--body-file", "r.md", "--repo", "o/r"],
        elsewhere.path(),
        &|_| None,
        &mut out,
        &mut err,
    );
    let err = String::from_utf8(err).unwrap();
    assert_eq!(code, 2, "{err}");
    assert!(
        err.starts_with("clm issue: ") && err.contains("clm.toml"),
        "{err}"
    );
    // The settings are read through the lookup, not the process's environment: a user file
    // that sets `host`, which only the repository may set, is found only through it.
    let user = TempDir::new().unwrap();
    write(user.path(), "clm/config.toml", "host = \"git\"\n");
    let xdg = user.path().display().to_string();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = clm_host::pr_command(
        ["open", "--root", &root],
        elsewhere.path(),
        &|key| (key == "XDG_CONFIG_HOME").then(|| xdg.clone()),
        &mut out,
        &mut err,
    );
    let err = String::from_utf8(err).unwrap();
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("config.toml"), "{err}");
    assert!(gh.calls().is_empty(), "{:?}", gh.calls());
}

#[test]
fn scenario_settings_that_dont_load() {
    // req:host/commands-in-crate@f798e133
    let user = TempDir::new().unwrap();
    write(user.path(), "clm/config.toml", "host = \"git\"\n");
    let xdg = user.path().display().to_string();
    let commands: [(&str, &[&str]); 5] = [
        ("pr open", &["pr", "open"]),
        ("pr body", &["pr", "body", "body.md"]),
        ("issue", &ISSUE_ARGS),
        ("branch push", &["branch", "push"]),
        ("branch delete", &["branch", "delete", "impl/a"]),
    ];
    for (command, args) in commands {
        // A `clm.toml` naming no host CLM knows.
        let (r, gh) = (checkout("nowhere"), fake_gh());
        let (code, out, err) = clm(&r.work, &gh, args, &[]);
        assert_eq!((code, out.as_str()), (Some(2), ""), "{command}: {err}");
        assert!(err.starts_with(&format!("clm {command}: ")), "{err}");
        assert!(err.contains("clm.toml"), "{command}: {err}");
        // A user settings file that sets `host`.
        let (r, gh) = (checkout("github"), fake_gh());
        let tip_before = on_origin(&r, "spec/x");
        let (code, out, err) = clm(&r.work, &gh, args, &[("XDG_CONFIG_HOME", &xdg)]);
        assert_eq!((code, out.as_str()), (Some(2), ""), "{command}: {err}");
        assert!(err.starts_with(&format!("clm {command}: ")), "{err}");
        assert!(err.contains("config.toml"), "{command}: {err}");
        assert!(gh.calls().is_empty(), "{command}: {:?}", gh.calls());
        assert_eq!(on_origin(&r, "spec/x"), tip_before, "{command}");
    }
}

#[test]
fn scenario_help() {
    // req:host/commands-in-crate@f798e133
    let pr = "Open the branch's pull request against `main` on the host, or find the open one, and print its URL, or `none` without a host";
    let cases: [(&[&str], &str, &str); 7] = [
        (
            &["pr", "--help"],
            "A branch's pull request on the host the `host` setting names",
            "Usage: clm pr <COMMAND>",
        ),
        (&["pr", "open", "--help"], pr, "Usage: clm pr open "),
        (
            &["pr", "body", "--help"],
            "Set the body of the current branch's open pull request to a file's contents",
            "Usage: clm pr body ",
        ),
        (
            &["issue", "--help"],
            "File an issue where the host the `host` setting names keeps them and print its URL; without a host, print the report and exit 1",
            "Usage: clm issue ",
        ),
        (
            &["branch", "--help"],
            "Remote branches, through the host adapter the `host` setting names",
            "Usage: clm branch <COMMAND>",
        ),
        (
            &["branch", "push", "--help"],
            "Push the checked-out branch to `origin`, overwriting only what this clone last saw",
            "Usage: clm branch push ",
        ),
        (
            &["branch", "delete", "--help"],
            "Delete `origin`'s branch when its tip is still `--lease` (default: the tip last fetched) and this clone pushed it, on every host",
            "Usage: clm branch delete ",
        ),
    ];
    let (r, gh) = (checkout("github"), fake_gh());
    for (args, about, usage) in cases {
        let (code, out, err) = clm(&r.work, &gh, args, &[]);
        assert_eq!((code, err.as_str()), (Some(0), ""), "{args:?}");
        let mut lines = out.lines();
        assert_eq!(lines.next(), Some(about), "{args:?}: {out}");
        assert_eq!(lines.next(), Some(""), "{args:?}: {out}");
        let line = lines.next().unwrap_or_default();
        assert!(line.starts_with(usage), "{args:?}: {out}");
        if usage.ends_with('>') {
            assert_eq!(line, usage, "{args:?}");
        }
    }
    assert!(gh.calls().is_empty(), "{:?}", gh.calls());
}

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
         if [ \"$2\" = create ] && [ -n \"$FAKE_CREATE_SILENT\" ]; then exit 0; fi\n\
         case \"$1 $2\" in\n\
         'pr view') [ -z \"$FAKE_VIEW_ERROR\" ] || { echo \"$FAKE_VIEW_ERROR\" >&2; exit 1; }; \
         [ -n \"$FAKE_PR_VIEW\" ] || { echo 'no pull requests found' >&2; exit 1; }; \
         printf '%s\\n' \"$FAKE_PR_VIEW\";;\n\
         'pr create') while [ $# -gt 0 ]; do \
         [ \"$1\" = --body-file ] && cp \"$2\" \"$FAKE_OUT/body\"; shift; done; \
         echo 'Creating pull request'; echo https://github.com/o/r/pull/9;;\n\
         'issue create') echo https://github.com/o/r/issues/3;;\n\
         'api -X') for a in \"$@\"; do case \"$a\" in body=@*) \
         cp \"${a#body=@}\" \"$FAKE_OUT/body\";; esac; done;;\nesac\n",
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
    // req:host/branch-delete@f8e36cf0
    for host in ["github", "git", "none"] {
        let (r, gh) = (checkout(host), fake_gh());
        let s = push_impl_a(&r, "a.txt");
        let t = tip(&r.work, "spec/x");
        git(&r.work, &["checkout", "-q", "spec/x"]);
        let (code, _, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
        assert_eq!(code, Some(0), "{host}: {err}");
        // Another session's fetch stored the tip first: the newest entry is a fetch's.
        fetch_stored_first(&r, "spec/x", &t);
        git(&r.work, &["checkout", "-q", "--detach"]);
        git(&r.work, &["branch", "-q", "-D", "spec/x"]);
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
        assert_eq!(pushed_record(&r, "spec/x"), "", "{host}");
        assert!(gh.calls().is_empty(), "{:?}", gh.calls());
    }
}

/// Rewrites `refs/remotes/origin/<branch>` at `sha` so that its one reflog entry is a
/// fetch's, as when another session's fetch stored the pushed tip before the push did.
fn fetch_stored_first(r: &Repo, branch: &str, sha: &str) {
    let tracking = format!("refs/remotes/origin/{branch}");
    git(&r.work, &["update-ref", "-d", &tracking]);
    git(
        &r.work,
        &[
            "update-ref",
            "--create-reflog",
            "-m",
            "fetch origin: storing head",
            &tracking,
            sha,
        ],
    );
    let newest = git(
        &r.work,
        &["reflog", "show", "-n", "1", "--format=%gs", &tracking],
    );
    assert_eq!(newest.trim(), "fetch origin: storing head");
}

/// The commit `refs/clm/pushed/<branch>` names, or nothing without that ref.
fn pushed_record(r: &Repo, branch: &str) -> String {
    git(
        &r.work,
        &[
            "for-each-ref",
            "--format=%(objectname)",
            &format!("refs/clm/pushed/{branch}"),
        ],
    )
    .trim()
    .to_string()
}

#[test]
fn scenario_branch_another_sessions() {
    // req:host/branch-delete@f8e36cf0
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
    // req:host/branch-delete@f8e36cf0
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
    // req:host/branch-delete@f8e36cf0
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
    // req:host/branch-delete@f8e36cf0
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
    // req:host/branch-delete@f8e36cf0
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
    // req:host/branch-delete@f8e36cf0
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
    // req:host/branch-delete@f8e36cf0
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
    // req:host/branch-delete@f8e36cf0
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
    // req:host/branch-delete@f8e36cf0
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

/// `impl/a` checked out at a new commit and pushed with `clm branch push`: its sha.
fn branch_push_impl_a(r: &Repo, gh: &Gh) -> String {
    git(&r.work, &["checkout", "-q", "-b", "impl/a"]);
    commit_file(r, "a.txt");
    let (code, _, err) = clm(&r.work, gh, &["branch", "push"], &[]);
    assert_eq!(code, Some(0), "{err}");
    let s = tip(&r.work, "HEAD");
    assert_eq!(pushed_record(r, "impl/a"), s);
    s
}

#[test]
fn scenario_branch_delete_the_remote_refuses_the_delete() {
    // req:host/branch-delete@f8e36cf0
    let (r, gh) = (checkout("github"), fake_gh());
    let s = branch_push_impl_a(&r, &gh);
    pre_receive(&r, "echo 'remote says no' >&2\nexit 1\n");
    let (code, _, err) = clm(&r.work, &gh, &["branch", "delete", "impl/a"], &[]);
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("remote says no"), "{err}");
    assert!(!err.contains("offline"), "{err}");
    assert_eq!(on_origin(&r, "impl/a"), s);
    assert_eq!(pushed_record(&r, "impl/a"), s);
}

#[test]
fn scenario_branch_delete_origin_cant_be_reached() {
    // req:host/branch-delete@f8e36cf0
    let (r, gh) = (checkout("github"), fake_gh());
    let s = branch_push_impl_a(&r, &gh);
    let gone = r.origin.join("no-such-dir");
    git(
        &r.work,
        &["remote", "set-url", "origin", gone.to_str().unwrap()],
    );
    let (code, _, err) = clm(&r.work, &gh, &["branch", "delete", "impl/a"], &[]);
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("offline: origin can't be reached; "), "{err}");
    assert_eq!(pushed_record(&r, "impl/a"), s);
    assert_eq!(on_origin(&r, "impl/a"), s);
}

#[test]
fn scenario_branch_delete_call_the_call_land_makes() {
    // req:host/branch-delete-call@eb352a4b
    use clm_config::Host as Setting;
    for (host, setting) in [
        ("github", Setting::Github),
        ("azure", Setting::Azure),
        ("git", Setting::Git),
        ("none", Setting::None),
    ] {
        let adapter = clm_host::adapter(setting);
        let r = checkout(host);
        // `impl/a`, pushed by another clone: no record here, and none is read.
        let s = other_clone_push(&r, "impl/a", "a.txt");
        // `impl/b`, moved on origin from `s` to `t`.
        git(&r.origin, &["update-ref", "refs/heads/impl/b", &s]);
        let t = other_clone_push(&r, "impl/b", "b.txt");
        // A ref that merely ends in `refs/heads/impl/gone`.
        git(
            &r.origin,
            &["update-ref", "refs/x/refs/heads/impl/gone", &s],
        );
        let m = on_origin(&r, "main");
        git(&r.origin, &["update-ref", "refs/heads/clm/ledger/p/c", &m]);
        git(&r.origin, &["update-ref", "refs/heads/impl/c", &m]);
        pre_receive(
            &r,
            "while read -r old new name; do\n\
             [ \"$name\" = refs/heads/impl/c ] && { echo 'remote says no' >&2; exit 1; }\n\
             done\nexit 0\n",
        );
        let delete = |branch: &str, lease: &str| adapter.branch_delete(&r.work, branch, lease);
        assert!(delete("impl/a", &s).is_ok(), "{host}");
        assert_eq!(on_origin(&r, "impl/a"), "", "{host}");
        match delete("impl/b", &s) {
            Err(clm_host::Error::Refused(why)) => assert_eq!(
                why,
                format!("`impl/b` on origin is no longer at {s}; left it"),
                "{host}"
            ),
            other => panic!("{host}: {other:?}"),
        }
        assert_eq!(on_origin(&r, "impl/b"), t, "{host}");
        assert!(delete("impl/gone", &s).is_ok(), "{host}");
        for (branch, lease) in [("main", m.as_str()), ("clm/ledger/p/c", m.as_str())] {
            let got = delete(branch, lease);
            assert!(
                matches!(got, Err(clm_host::Error::Refused(_))),
                "{host}: {got:?}"
            );
            assert_eq!(on_origin(&r, branch), m, "{host} {branch}");
        }
        match delete("impl/c", &m) {
            Err(clm_host::Error::Refused(why)) => {
                assert!(why.contains("remote says no"), "{host}: {why}");
                assert!(!why.contains("offline"), "{host}: {why}");
            }
            other => panic!("{host}: {other:?}"),
        }
        assert_eq!(on_origin(&r, "impl/c"), m, "{host}");
        // A clone with no `origin`.
        git(&r.work, &["remote", "remove", "origin"]);
        assert!(delete("impl/a", &s).is_ok(), "{host}");
        let got = delete("main", &m);
        assert!(
            matches!(got, Err(clm_host::Error::Refused(_))),
            "{host}: {got:?}"
        );
        // A clone whose `origin` can't be reached.
        let gone = r.origin.join("no-such-dir");
        git(
            &r.work,
            &["remote", "add", "origin", gone.to_str().unwrap()],
        );
        match delete("impl/a", &s) {
            Err(clm_host::Error::Refused(why)) => assert!(
                why.starts_with("offline: origin can't be reached"),
                "{host}: {why}"
            ),
            other => panic!("{host}: {other:?}"),
        }
    }
}

#[test]
fn branch_delete_without_origin_has_nothing_to_do() {
    // req:host/branch-delete@f8e36cf0
    let (r, gh) = (checkout("none"), fake_gh());
    git(&r.work, &["update-ref", "refs/clm/pushed/impl/a", "HEAD"]);
    git(&r.work, &["remote", "remove", "origin"]);
    let (code, _, err) = clm(
        &r.work,
        &gh,
        &["branch", "delete", "impl/a", "--lease", "abc"],
        &[],
    );
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(pushed_record(&r, "impl/a"), "");
}

#[test]
fn branch_delete_of_a_branch_already_gone_has_nothing_to_do() {
    // req:host/branch-delete@f8e36cf0
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

#[test]
fn branch_delete_of_a_branch_found_gone_drops_its_record() {
    // req:host/branch-delete@f8e36cf0
    let (r, gh) = (checkout("github"), fake_gh());
    let (code, _, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(pushed_record(&r, "spec/x"), tip(&r.work, "HEAD"));
    git(&r.origin, &["update-ref", "-d", "refs/heads/spec/x"]);
    let (code, _, err) = clm(&r.work, &gh, &["branch", "delete", "spec/x"], &[]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(pushed_record(&r, "spec/x"), "");
}

/// Commits `file` on the checked-out branch of `r.work`.
fn commit_file(r: &Repo, file: &str) {
    write(&r.work, file, "x\n");
    git(&r.work, &["add", file]);
    git(&r.work, &["commit", "-q", "-m", file]);
}

#[test]
fn scenario_push_rebased_branch() {
    // req:host/branch-push@e99935c6
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
        assert_eq!(pushed_record(&r, "impl/a"), t, "{host}");
        assert!(gh.calls().is_empty());
        // Already there: pushing again succeeds.
        let (code, _, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
        assert_eq!(code, Some(0), "{host}: {err}");
    }
}

#[test]
fn scenario_push_another_clones_tip_again() {
    // req:host/branch-push@e99935c6
    let (r, gh) = (checkout("github"), fake_gh());
    let u = other_clone_push(&r, "impl/b", "b.txt");
    git(&r.work, &["fetch", "-q", "origin"]);
    git(
        &r.work,
        &["checkout", "-q", "-b", "impl/b", "origin/impl/b"],
    );
    let (code, out, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, format!("pushed impl/b {u}\n"));
    assert_eq!(pushed_record(&r, "impl/b"), "");
    let (code, _, err) = clm(&r.work, &gh, &["branch", "delete", "impl/b"], &[]);
    assert_eq!(code, Some(1), "{err}");
    assert_eq!(on_origin(&r, "impl/b"), u);
}

#[test]
fn push_records_only_a_tip_it_moved_origin_to() {
    // req:host/branch-push@e99935c6
    let (r, gh) = (checkout("github"), fake_gh());
    // origin already holds the commit and the clone has no tracking ref: nothing moved.
    git(&r.work, &["push", "-q", "origin", "spec/x"]);
    git(&r.work, &["update-ref", "-d", "refs/remotes/origin/spec/x"]);
    let (code, _, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(pushed_record(&r, "spec/x"), "");
    // A push that moved it records the tip; a refused one leaves the record as it was.
    commit_file(&r, "n.txt");
    let (code, _, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
    assert_eq!(code, Some(0), "{err}");
    let moved = tip(&r.work, "HEAD");
    assert_eq!(pushed_record(&r, "spec/x"), moved);
    pre_receive(&r, "exit 1\n");
    commit_file(&r, "m.txt");
    let (code, _, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
    assert_eq!(code, Some(1), "{err}");
    assert_eq!(pushed_record(&r, "spec/x"), moved);
}

#[test]
fn scenario_push_new_branch() {
    // req:host/branch-push@e99935c6
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
    // req:host/branch-push@e99935c6
    let (r, gh) = (checkout("github"), fake_gh());
    let u = other_clone_push(&r, "spec/x", "u.txt");
    let (code, _, err) = clm(&r.work, &gh, &["branch", "push"], &[]);
    assert_eq!(code, Some(1), "{err}");
    assert_eq!(on_origin(&r, "spec/x"), u);
}

#[test]
fn scenario_push_moved_since_last_fetch() {
    // req:host/branch-push@e99935c6
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
    // req:host/branch-push@e99935c6
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
    // req:host/branch-push@e99935c6
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
    // req:host/branch-push@e99935c6
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
    // req:host/branch-push@e99935c6
    let (r, gh) = (checkout("git"), fake_gh());
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
    // req:host/pr-open@f06b4b0e
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
    // req:host/pr-open@f06b4b0e
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
    // req:host/pr-open@f06b4b0e
    let (r, gh) = (checkout("github"), fake_gh());
    let view = r#"{"url":"https://github.com/o/r/pull/4","state":"MERGED","headRefName":"spec/x"}"#;
    let (code, out, err) = clm(&r.work, &gh, &OPEN_ARGS, &[("FAKE_PR_VIEW", view)]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "https://github.com/o/r/pull/9\n");
}

#[test]
fn page_open_exits_2_when_gh_fails() {
    // req:host/pr-open@f06b4b0e
    // req:host/keep-unopened-body@471ee956
    // req:host/host-tool-failure@5ca96db4
    let (r, gh) = (checkout("github"), fake_gh());
    let (code, _, err) = clm(&r.work, &gh, &OPEN_ARGS, &[("FAKE_CREATE_EXIT", "1")]);
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("create failed"), "{err}");
    assert!(
        err.trim_end().ends_with("pr-bodies/spec-x.markdown"),
        "{err}"
    );
}

/// `branch` made from `main` with a commit beyond it holding `file`, checked out.
fn branch_beyond_main(r: &Repo, branch: &str, file: &str) {
    git(&r.work, &["checkout", "-q", "-B", branch, "main"]);
    commit_file(r, file);
}

#[test]
fn scenario_keep_two_branches_two_copies() {
    // req:host/keep-unopened-body@471ee956
    let (r, gh) = (checkout("github"), fake_gh());
    let bodies = r.work.join(".git/clm/pr-bodies");
    write(&bodies, "spec-a-b.md", "old");
    write(&r.work, "one.md", "one");
    write(&r.work, "two.md", "two");
    let fail = [("FAKE_CREATE_EXIT", "1")];
    for (branch, file) in [("spec/a-b", "one.md"), ("spec-a/b", "two.md")] {
        branch_beyond_main(&r, branch, &format!("{}.txt", branch.replace('/', "_")));
        let args = ["pr", "open", branch, "--title", "x", "--body-file", file];
        let (code, _, err) = clm(&r.work, &gh, &args, &fail);
        assert_eq!(code, Some(2), "{branch}: {err}");
        assert!(err.contains("the body is kept at"), "{branch}: {err}");
    }
    let read = |name: &str| fs::read_to_string(bodies.join(name)).unwrap();
    assert_eq!(read("spec-a~b.markdown"), "one");
    assert_eq!(read("spec~a-b.markdown"), "two");
    assert_eq!(read("spec-a-b.md"), "old");
}

#[test]
fn scenario_keep_nothing_worth_keeping() {
    // req:host/keep-unopened-body@471ee956
    let (r, gh) = (checkout("github"), fake_gh());
    branch_beyond_main(&r, "impl/a", "a.txt");
    let args = [
        "pr",
        "open",
        "impl/a",
        "--title",
        "x",
        "--body-file",
        "body.md",
    ];
    let fail = [("FAKE_CREATE_EXIT", "1")];
    let kept = r.work.join(".git/clm/pr-bodies/impl-a.markdown");
    write(&r.work, "body.md", "  \n\n \t\n");
    let (code, _, err) = clm(&r.work, &gh, &args, &fail);
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("create failed"), "{err}");
    assert!(!err.contains("the body is kept at"), "{err}");
    assert!(!kept.exists());
    // A good body, with a file where the copies' directory would be.
    write(&r.work, "body.md", "## Review\n");
    write(&r.work, ".git/clm/pr-bodies", "in the way");
    let (code, _, err) = clm(&r.work, &gh, &args, &fail);
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("create failed"), "{err}");
    assert!(!err.contains("the body is kept at"), "{err}");
    assert!(!kept.exists());
}

#[test]
fn scenario_page_open_no_host() {
    // req:host/pr-open-defaults@17fa5999
    for host in ["none", "git"] {
        let (r, gh) = (checkout(host), fake_gh());
        let (code, out, err) = clm(&r.work, &gh, &OPEN_ARGS, &[]);
        assert_eq!(code, Some(0), "{host}: {err}");
        assert_eq!(out, "none\n");
        assert!(gh.calls().is_empty(), "{:?}", gh.calls());
    }
}

const ROOT_SPEC: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n";

/// Component `a`'s spec holding a requirement for each of `slugs`.
fn component_a(slugs: &[&str]) -> String {
    let mut out = "---\ntitle: a\nstatus: active\nowners: []\npaths: [\"src/a/**\"]\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n".to_string();
    for slug in slugs {
        out.push_str(&format!(
            "\n### Requirement: {slug}\n{slug} SHALL hold.\n\n#### Scenario: {slug}\n- GIVEN g\n- WHEN w\n- THEN t\n"
        ));
    }
    out
}

/// A checkout whose `clm.toml` sets `host = "github"` and whose `origin/main` holds component
/// `a` with requirements `x` and `y`, on `branch`, which commits `a` holding `head` instead.
fn spec_edit(branch: &str, head: &[&str]) -> Repo {
    let r = repo();
    write(&r.work, "specs/SPEC.md", ROOT_SPEC);
    write(&r.work, "specs/a/SPEC.md", &component_a(&["x", "y"]));
    git(&r.work, &["add", "-A"]);
    git(&r.work, &["commit", "-qm", "a"]);
    git(&r.work, &["push", "-q", "origin", "HEAD:main"]);
    git(&r.work, &["checkout", "-q", "-b", branch]);
    write(&r.work, "specs/a/SPEC.md", &component_a(head));
    git(&r.work, &["add", "-A"]);
    git(&r.work, &["commit", "-qm", "edit a"]);
    write(&r.work, "clm.toml", "host = \"github\"\n");
    write(&r.work, "body.md", "The body\n");
    r
}

const CUTS: &str = "## Cuts\n- specs/a/SPEC.md: gate/cut: requirement a/x is gone\n\n";

#[test]
fn scenario_pr_open_a_branch_with_cuts() {
    // req:host/pr-open@f06b4b0e
    let (r, gh) = (spec_edit("spec/x", &["y"]), fake_gh());
    let (code, out, err) = clm(&r.work, &gh, &OPEN_ARGS, &[]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "https://github.com/o/r/pull/9\n");
    assert_eq!(body(&gh), format!("{CUTS}The body\n"));
    assert_eq!(
        fs::read_to_string(r.work.join("body.md")).unwrap(),
        "The body\n"
    );
}

#[test]
fn scenario_pr_open_a_branch_without_cuts() {
    // req:host/pr-open@f06b4b0e
    let (r, gh) = (spec_edit("spec/y", &["x", "y", "z"]), fake_gh());
    let args = [
        "pr",
        "open",
        "spec/y",
        "--title",
        "y",
        "--body-file",
        "body.md",
    ];
    let (code, out, err) = clm(&r.work, &gh, &args, &[]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "https://github.com/o/r/pull/9\n");
    assert_eq!(body(&gh), "The body\n");
    assert_eq!(
        creates(&gh),
        ["pr create --base main --head spec/y --title y --body-file body.md"]
    );
}

#[test]
fn pr_open_exits_1_when_the_gate_cant_be_evaluated() {
    // req:host/pr-open@f06b4b0e
    // req:host/keep-unopened-body@471ee956
    let (r, gh) = (checkout("github"), fake_gh());
    let args = [
        "pr",
        "open",
        "spec/nowhere",
        "--title",
        "x",
        "--body-file",
        "body.md",
    ];
    let (code, _, err) = clm(&r.work, &gh, &args, &[]);
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("gate") && !err.contains("kept"), "{err}");
    assert!(creates(&gh).is_empty(), "{:?}", gh.calls());
}

#[test]
fn a_drawn_body_starts_with_the_cuts_too() {
    // req:host/pr-open@f06b4b0e
    let (r, gh) = (spec_edit("spec/x", &["y"]), fake_gh());
    let (code, _, err) = clm(&r.work, &gh, &["pr", "open", "spec/x"], &[]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(body(&gh), format!("{CUTS}- edit a\n"));
}

#[test]
fn the_gate_waits_for_the_checks_before_it() {
    // req:host/pr-open@f06b4b0e
    // req:host/pr-body@42b28fb0
    let (r, gh) = (checkout("github"), fake_gh());
    git(&r.work, &["update-ref", "-d", "refs/remotes/origin/main"]);
    // Already open: nothing is sent, so the gate isn't asked.
    let view = r#"{"url":"https://github.com/o/r/pull/4","state":"OPEN","headRefName":"spec/x"}"#;
    let (code, out, err) = clm(&r.work, &gh, &OPEN_ARGS, &[("FAKE_PR_VIEW", view)]);
    assert_eq!(
        (code, out.as_str()),
        (Some(0), "https://github.com/o/r/pull/4\n"),
        "{err}"
    );
    // A blank body and a branch with no open pull request refuse for that, not the gate.
    write(&r.work, "body.md", " \n");
    let (code, _, err) = clm(&r.work, &gh, &BODY_ARGS, &[("FAKE_PR_VIEW", PR_7)]);
    assert_eq!(code, Some(1), "{err}");
    assert!(!err.contains("gate"), "{err}");
    write(&r.work, "body.md", "The body\n");
    let (code, _, err) = clm(&r.work, &gh, &BODY_ARGS, &[]);
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("no open PR") && !err.contains("gate"), "{err}");
}

#[test]
fn a_body_that_fails_to_open_is_kept_with_its_cuts() {
    // req:host/pr-open@f06b4b0e
    // req:host/keep-unopened-body@471ee956
    let (r, gh) = (spec_edit("spec/x", &["y"]), fake_gh());
    let (code, _, err) = clm(&r.work, &gh, &OPEN_ARGS, &[("FAKE_CREATE_EXIT", "1")]);
    assert_eq!(code, Some(2), "{err}");
    let (_, kept) = err.split_once("the body is kept at ").expect(&err);
    let kept = fs::read_to_string(kept.trim()).unwrap();
    assert_eq!(kept, format!("{CUTS}The body\n"));
}

/// A `body.md` that starts with a stale `## Cuts` section, as one read back from a pull
/// request does.
const STALE: &str = "## Cuts\n- an older line\n\n## Intent\nThe intent\n";

#[test]
fn scenario_pr_open_a_body_that_already_holds_the_section() {
    // req:host/pr-open@f06b4b0e
    let (r, gh) = (spec_edit("spec/x", &["y"]), fake_gh());
    write(&r.work, "body.md", STALE);
    let (code, _, err) = clm(&r.work, &gh, &OPEN_ARGS, &[]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(body(&gh), format!("{CUTS}## Intent\nThe intent\n"));
}

#[test]
fn a_stale_section_goes_from_a_branch_without_cuts() {
    // req:host/pr-open@f06b4b0e
    let (r, gh) = (spec_edit("spec/y", &["x", "y", "z"]), fake_gh());
    write(&r.work, "body.md", STALE);
    let args = [
        "pr",
        "open",
        "spec/y",
        "--title",
        "y",
        "--body-file",
        "body.md",
    ];
    let (code, _, err) = clm(&r.work, &gh, &args, &[]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(body(&gh), "## Intent\nThe intent\n");
    // The section runs to its first blank line, or to the body's end; a line holding only
    // whitespace ends it too.
    write(&r.work, "body.md", "## Cuts\n- old\n \t\n## Intent\n");
    let (code, _, err) = clm(&r.work, &gh, &args, &[]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(body(&gh), "## Intent\n");
    // A body that only mentions the heading later keeps it, and goes as given.
    write(&r.work, "body.md", "## Intent\n## Cuts\n- kept\n");
    let (code, _, err) = clm(&r.work, &gh, &args, &[]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(body(&gh), "## Intent\n## Cuts\n- kept\n");
    assert!(
        creates(&gh)
            .last()
            .is_some_and(|c| c.ends_with("--body-file body.md")),
        "{:?}",
        creates(&gh)
    );
}

#[test]
fn a_kept_body_holds_the_section_once() {
    // req:host/keep-unopened-body@471ee956
    // req:host/pr-open@f06b4b0e
    // A kept copy sent again, whose host fails before the gate runs: kept with one section.
    let (r, gh) = (spec_edit("spec/x", &["y"]), fake_gh());
    write(&r.work, "body.md", STALE);
    let env = [("FAKE_VIEW_ERROR", "HTTP 401: Bad credentials")];
    let (code, _, err) = clm(&r.work, &gh, &OPEN_ARGS, &env);
    assert_eq!(code, Some(2), "{err}");
    let (_, kept) = err.split_once("the body is kept at ").expect(&err);
    let kept = fs::read_to_string(kept.trim()).unwrap();
    assert_eq!(kept, format!("{CUTS}## Intent\nThe intent\n"));
}

#[test]
fn scenario_pr_body_a_body_read_back_from_the_pull_request() {
    // req:host/pr-body@42b28fb0
    let (r, gh) = (spec_edit("spec/x", &["y"]), fake_gh());
    write(&r.work, "body.md", STALE);
    let (code, out, err) = clm(&r.work, &gh, &BODY_ARGS, &[("FAKE_PR_VIEW", PR_7)]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "updated #7\n");
    assert_eq!(body(&gh), format!("{CUTS}## Intent\nThe intent\n"));
}

#[test]
fn scenario_gh_fails_to_open_a_pull_request() {
    // req:host/host-tool-failure@5ca96db4
    let (r, gh) = (checkout("github"), fake_gh());
    let (code, _, err) = clm(&r.work, &gh, &OPEN_ARGS, &[("FAKE_CREATE_EXIT", "1")]);
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("create failed"), "{err}");
}

#[test]
fn scenario_gh_fails_to_file_an_issue() {
    // req:host/host-tool-failure@5ca96db4
    let (r, gh) = (checkout("github"), fake_gh());
    write(&r.work, "r.md", "The report\n");
    let args = [
        "issue",
        "--title",
        "x",
        "--body-file",
        "r.md",
        "--repo",
        "o/r",
    ];
    let (code, _, err) = clm(&r.work, &gh, &args, &[("FAKE_CREATE_EXIT", "1")]);
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("create failed"), "{err}");
}

#[test]
fn scenario_az_fails_to_set_a_body() {
    // req:host/host-tool-failure@5ca96db4
    let (r, gh) = (checkout("azure"), fake_gh());
    fake_az(&gh);
    write(&r.work, "body.md", "## Intent\n");
    let env = [("FAKE_AZ_FAIL", "Please run 'az login'")];
    let (code, _, err) = clm(&r.work, &gh, &BODY_ARGS, &env);
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("az login"), "{err}");
}

#[test]
fn scenario_an_answer_that_cant_be_read() {
    // req:host/host-tool-failure@5ca96db4
    let (r, gh) = (checkout("github"), fake_gh());
    write(&r.work, "body.md", "## Intent\n");
    let (code, _, err) = clm(&r.work, &gh, &BODY_ARGS, &[("FAKE_PR_VIEW", "not json")]);
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("bad JSON"), "{err}");
}

#[test]
fn an_answer_without_a_url_exits_2() {
    // req:host/host-tool-failure@5ca96db4
    // `gh pr create` and `gh issue create` exit 0 printing nothing: no URL to read.
    let (r, gh) = (checkout("github"), fake_gh());
    write(&r.work, "r.md", "The report\n");
    let env = [("FAKE_CREATE_SILENT", "1")];
    for args in [&OPEN_ARGS[..], &ISSUE_ARGS[..]] {
        let (code, out, err) = clm(&r.work, &gh, args, &env);
        assert_eq!((code, out.as_str()), (Some(2), ""), "{args:?}: {err}");
        assert!(err.contains("no URL"), "{err}");
    }
}

#[test]
fn a_body_holding_only_a_stale_section_is_refused() {
    // req:host/pr-body@42b28fb0
    // Once its section goes, nothing is left: blank, so nothing is called.
    let (r, gh) = (checkout("github"), fake_gh());
    write(&r.work, "body.md", "## Cuts\n- an older line\n");
    let (code, _, err) = clm(&r.work, &gh, &BODY_ARGS, &[("FAKE_PR_VIEW", PR_7)]);
    assert_eq!(code, Some(1), "{err}");
    assert!(gh.calls().is_empty(), "{:?}", gh.calls());
}

#[test]
fn a_section_with_no_blank_line_runs_to_the_bodys_end() {
    // req:host/pr-open@f06b4b0e
    let (r, gh) = (spec_edit("spec/x", &["y"]), fake_gh());
    write(&r.work, "body.md", "## Cuts\n- an older line\n- another");
    let (code, _, err) = clm(&r.work, &gh, &OPEN_ARGS, &[]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(body(&gh), CUTS);
}

#[test]
fn a_commands_own_refusals_stay_exit_1() {
    // req:host/host-tool-failure@5ca96db4
    // A blank body file and a branch with no pull request, on both hosts that have a tool.
    for host in ["github", "azure"] {
        let (r, gh) = (checkout(host), fake_gh());
        fake_az(&gh);
        let (code, _, err) = clm(&r.work, &gh, &BODY_ARGS, &[]);
        assert_eq!(code, Some(1), "{host}: {err}");
        write(&r.work, "body.md", " \n");
        let (code, _, err) = clm(&r.work, &gh, &BODY_ARGS, &[("FAKE_PR_VIEW", PR_7)]);
        assert_eq!(code, Some(1), "{host}: {err}");
    }
}

#[test]
fn a_body_kept_before_the_gate_ran_holds_its_cuts() {
    // req:host/keep-unopened-body@471ee956
    // `gh pr view` fails before the body is sent: the kept copy still starts with the cuts.
    let (r, gh) = (spec_edit("spec/x", &["y"]), fake_gh());
    let env = [("FAKE_VIEW_ERROR", "HTTP 401: Bad credentials")];
    let (code, _, err) = clm(&r.work, &gh, &OPEN_ARGS, &env);
    assert_eq!(code, Some(2), "{err}");
    let (_, kept) = err.split_once("the body is kept at ").expect(&err);
    let kept = fs::read_to_string(kept.trim()).unwrap();
    assert_eq!(kept, format!("{CUTS}The body\n"));
}

#[test]
fn a_body_whose_gate_cant_be_evaluated_is_not_kept_after_a_host_failure() {
    // req:host/keep-unopened-body@471ee956
    let (r, gh) = (checkout("github"), fake_gh());
    git(&r.work, &["update-ref", "-d", "refs/remotes/origin/main"]);
    let env = [("FAKE_VIEW_ERROR", "HTTP 401: Bad credentials")];
    let (code, _, err) = clm(&r.work, &gh, &OPEN_ARGS, &env);
    assert_eq!(code, Some(2), "{err}");
    assert!(!err.contains("kept"), "{err}");
}

#[test]
fn scenario_pr_body_a_branch_with_cuts() {
    // req:host/pr-body@42b28fb0
    let (r, gh) = (spec_edit("spec/x", &["y"]), fake_gh());
    let (code, out, err) = clm(&r.work, &gh, &BODY_ARGS, &[("FAKE_PR_VIEW", PR_7)]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "updated #7\n");
    assert_eq!(body(&gh), format!("{CUTS}The body\n"));
}

#[test]
fn pr_body_exits_1_when_the_gate_cant_be_evaluated() {
    // req:host/pr-body@42b28fb0
    let (r, gh) = (checkout("github"), fake_gh());
    git(&r.work, &["update-ref", "-d", "refs/remotes/origin/main"]);
    let (code, _, err) = clm(&r.work, &gh, &BODY_ARGS, &[("FAKE_PR_VIEW", PR_7)]);
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("gate"), "{err}");
    assert!(
        !gh.calls().iter().any(|c| c.starts_with("api")),
        "{:?}",
        gh.calls()
    );
}

/// The `pr create` calls `gh` got.
fn creates(gh: &Gh) -> Vec<String> {
    gh.calls()
        .into_iter()
        .filter(|c| c.starts_with("pr create"))
        .collect()
}

/// `checkout("github")` on `spec/x`, holding a change whose `## Intent` is `intent`.
fn spec_branch(intent: &str) -> Repo {
    let r = checkout("github");
    write(
        &r.work,
        "changes/x/change.md",
        &format!("---\nstatus: proposed\n---\n\n## Intent\n{intent}\n## Approaches\nNone.\n"),
    );
    git(&r.work, &["add", "-A"]);
    git(&r.work, &["commit", "-qm", "the change"]);
    r
}

/// The body file the last `pr create` or `api` call was given.
fn body(gh: &Gh) -> String {
    fs::read_to_string(gh.dir.path().join("body")).unwrap()
}

#[test]
fn scenario_pr_open_a_spec_branchs_change() {
    // req:host/pr-open-defaults@17fa5999
    let r = spec_branch("Skills fail. They name a bare command.\n");
    let gh = fake_gh();
    let (code, _, err) = clm(&r.work, &gh, &["pr", "open"], &[]);
    assert_eq!(code, Some(0), "{err}");
    let creates = creates(&gh);
    assert_eq!(creates.len(), 1, "{creates:?}");
    assert!(
        creates[0]
            .starts_with("pr create --base main --head spec/x --title Skills fail. --body-file "),
        "{creates:?}"
    );
    assert_eq!(body(&gh), "Skills fail. They name a bare command.");
}

#[test]
fn pr_open_title_is_the_first_sentence_of_the_first_paragraph() {
    // req:host/pr-open-defaults@17fa5999
    for (intent, title) in [
        ("Is it so? Yes.\n", "Is it so?"),
        (
            "No stop here\nacross lines\n\nSecond. Para.\n",
            "No stop here across lines",
        ),
        ("Version 1.2 ships. More.\n", "Version 1.2 ships."),
        ("First line\n   \nNext. Para.\n", "First line"),
    ] {
        let r = spec_branch(intent);
        let gh = fake_gh();
        let (code, _, err) = clm(&r.work, &gh, &["pr", "open"], &[]);
        assert_eq!(code, Some(0), "{err}");
        let want = format!("--head spec/x --title {title} --body-file ");
        assert!(creates(&gh)[0].contains(&want), "{:?}", creates(&gh));
    }
}

#[test]
fn pr_open_blank_intent_falls_back_to_commits() {
    // req:host/pr-open-defaults@17fa5999
    let r = spec_branch("  \n");
    let gh = fake_gh();
    let (code, _, err) = clm(&r.work, &gh, &["pr", "open"], &[]);
    assert_eq!(code, Some(0), "{err}");
    assert!(creates(&gh)[0].contains("--title the change --body-file "));
    assert_eq!(body(&gh), "- the change\n");
}

#[test]
fn scenario_pr_open_a_branch_without_a_change() {
    // req:host/pr-open-defaults@17fa5999
    let (r, gh) = (checkout("github"), fake_gh());
    git(&r.work, &["checkout", "-q", "-b", "fix/y"]);
    for subject in ["first", "second"] {
        git(&r.work, &["commit", "-q", "--allow-empty", "-m", subject]);
    }
    let (code, _, err) = clm(&r.work, &gh, &["pr", "open"], &[]);
    assert_eq!(code, Some(0), "{err}");
    assert!(creates(&gh)[0].contains("--head fix/y --title first --body-file "));
    assert_eq!(body(&gh), "- first\n- second\n");
}

#[test]
fn pr_open_describes_the_branch_it_is_given() {
    // req:host/pr-open-defaults@17fa5999
    let r = spec_branch("Skills fail. They name a bare command.\n");
    git(&r.work, &["checkout", "-q", "-b", "fix/y", "origin/main"]);
    git(&r.work, &["commit", "-q", "--allow-empty", "-m", "other"]);
    git(&r.work, &["checkout", "-q", "--detach"]);
    let gh = fake_gh();
    let (code, _, err) = clm(&r.work, &gh, &["pr", "open", "spec/x"], &[]);
    assert_eq!(code, Some(0), "{err}");
    assert!(creates(&gh)[0].contains("--head spec/x --title Skills fail. --body-file "));
    assert_eq!(body(&gh), "Skills fail. They name a bare command.");
}

#[test]
fn pr_open_spec_branch_without_a_change_uses_commits() {
    // req:host/pr-open-defaults@17fa5999
    let (r, gh) = (checkout("github"), fake_gh());
    git(&r.work, &["checkout", "-q", "-b", "spec/nochange"]);
    git(&r.work, &["commit", "-q", "--allow-empty", "-m", "only"]);
    let (code, _, err) = clm(&r.work, &gh, &["pr", "open"], &[]);
    assert_eq!(code, Some(0), "{err}");
    assert!(creates(&gh)[0].contains("--head spec/nochange --title only --body-file "));
    assert_eq!(body(&gh), "- only\n");
}

#[test]
fn scenario_pr_open_given_arguments_override() {
    // req:host/pr-open-defaults@17fa5999
    let r = spec_branch("Skills fail. They name a bare command.\n");
    let gh = fake_gh();
    let (code, _, err) = clm(
        &r.work,
        &gh,
        &["pr", "open", "--title", "mine", "--body-file", "body.md"],
        &[],
    );
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(
        creates(&gh),
        ["pr create --base main --head spec/x --title mine --body-file body.md"]
    );
}

#[test]
fn pr_open_each_argument_overrides_alone() {
    // req:host/pr-open-defaults@17fa5999
    let r = spec_branch("Skills fail. They name a bare command.\n");
    let gh = fake_gh();
    let (code, _, err) = clm(&r.work, &gh, &["pr", "open", "--title", "mine"], &[]);
    assert_eq!(code, Some(0), "{err}");
    assert!(creates(&gh)[0].contains("--title mine --body-file "));
    assert_eq!(body(&gh), "Skills fail. They name a bare command.");
    let gh = fake_gh();
    let args = ["pr", "open", "--body-file", "body.md"];
    let (code, _, err) = clm(&r.work, &gh, &args, &[]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(
        creates(&gh),
        ["pr create --base main --head spec/x --title Skills fail. --body-file body.md"]
    );
}

#[test]
fn scenario_pr_open_nothing_to_describe() {
    // req:host/pr-open-defaults@17fa5999
    let (r, gh) = (checkout("github"), fake_gh());
    git(&r.work, &["checkout", "-q", "-b", "fix/z"]);
    let (code, _, err) = clm(&r.work, &gh, &["pr", "open"], &[]);
    assert_eq!(code, Some(1), "{err}");
    assert!(gh.calls().is_empty(), "{:?}", gh.calls());
}

#[test]
fn scenario_pr_open_detached_head() {
    // req:host/pr-open-defaults@17fa5999
    let (r, gh) = (checkout("github"), fake_gh());
    git(&r.work, &["checkout", "-q", "--detach"]);
    let (code, _, err) = clm(&r.work, &gh, &["pr", "open"], &[]);
    assert_eq!(code, Some(1), "{err}");
    assert!(gh.calls().is_empty(), "{:?}", gh.calls());
}

#[test]
fn scenario_pr_open_defaults_no_host() {
    // req:host/pr-open-defaults@17fa5999
    for host in ["none", "git"] {
        let (r, gh) = (checkout(host), fake_gh());
        git(&r.work, &["checkout", "-q", "--detach"]);
        for args in [&["pr", "open"][..], &OPEN_ARGS] {
            let (code, out, err) = clm(&r.work, &gh, args, &[]);
            assert_eq!(code, Some(0), "{host} {args:?}: {err}");
            assert_eq!(out, "none\n");
        }
        assert!(gh.calls().is_empty(), "{:?}", gh.calls());
    }
}

/// `branch` from `main`, checked out, with one commit `subject` adding a change for each of
/// `changes`, whose `## Intent` reads `intent`.
fn branch_adding(r: &Repo, branch: &str, changes: &[&str], intent: &str, subject: &str) {
    git(&r.work, &["checkout", "-q", "-B", branch, "main"]);
    for id in changes {
        write(
            &r.work,
            &format!("changes/{id}/change.md"),
            &format!("---\nstatus: proposed\n---\n\n## Intent\n{intent}\n"),
        );
    }
    git(&r.work, &["add", "changes"]);
    git(&r.work, &["commit", "-q", "--allow-empty", "-m", subject]);
}

#[test]
fn scenario_pr_open_the_change_the_branch_adds() {
    // req:host/pr-open-defaults@17fa5999
    /// A branch, the changes it adds, their intent, then the title and body `gh` gets and
    /// the branch's one commit's subject.
    type Case<'a> = (&'a str, &'a [&'a str], &'a str, &'a str, &'a str, &'a str);
    let r = checkout("github");
    let cases: [Case; 3] = [
        (
            "fix/y",
            &["y"],
            "Fix it. Now.",
            "Fix it.",
            "Fix it. Now.",
            "y",
        ),
        (
            "spec/x",
            &["other"],
            "Other. Words.",
            "Other.",
            "Other. Words.",
            "x",
        ),
        (
            "fix/w",
            &["w1", "w2"],
            "W. Words.",
            "both",
            "- both\n",
            "both",
        ),
    ];
    for (branch, changes, intent, title, want_body, subject) in cases {
        branch_adding(&r, branch, changes, intent, subject);
        let gh = fake_gh();
        let (code, _, err) = clm(&r.work, &gh, &["pr", "open"], &[]);
        assert_eq!(code, Some(0), "{branch}: {err}");
        let want = format!("--head {branch} --title {title} --body-file ");
        assert!(creates(&gh)[0].contains(&want), "{:?}", creates(&gh));
        assert_eq!(body(&gh), want_body, "{branch}");
    }
    // `spec/x` whose change `origin/main` already holds, with one commit `edit` beyond it.
    branch_adding(&r, "spec/x", &["x"], "Held. Already.", "the change");
    git(&r.work, &["push", "-q", "origin", "spec/x:main"]);
    git(&r.work, &["fetch", "-q", "origin"]);
    git(&r.work, &["commit", "-q", "--allow-empty", "-m", "edit"]);
    let gh = fake_gh();
    let (code, _, err) = clm(&r.work, &gh, &["pr", "open"], &[]);
    assert_eq!(code, Some(0), "{err}");
    assert!(creates(&gh)[0].contains("--title edit --body-file "));
    assert_eq!(body(&gh), "- edit\n");
}

#[test]
fn scenario_pr_open_no_origin_main() {
    // req:host/pr-open-defaults@17fa5999
    let (r, gh) = (checkout("github"), fake_gh());
    git(&r.work, &["checkout", "-q", "-b", "fix/z"]);
    git(&r.work, &["commit", "-q", "--allow-empty", "-m", "z"]);
    git(&r.work, &["update-ref", "-d", "refs/remotes/origin/main"]);
    let (code, _, err) = clm(&r.work, &gh, &["pr", "open"], &[]);
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("origin/main"), "{err}");
    assert!(err.contains("fatal"), "{err}");
    assert!(gh.calls().is_empty(), "{:?}", gh.calls());
}

const PR_7: &str = r#"{"number":7,"state":"OPEN","headRefName":"spec/x"}"#;
const BODY_ARGS: [&str; 3] = ["pr", "body", "body.md"];

#[test]
fn scenario_page_body_updated() {
    // req:host/pr-body@42b28fb0
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
    // req:host/pr-body@42b28fb0
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
    // req:host/pr-body@42b28fb0
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
    // req:host/pr-body@42b28fb0
    // req:host/pr-open@f06b4b0e
    // req:host/keep-unopened-body@471ee956
    let (r, gh) = (checkout("github"), fake_gh());
    let env = [("FAKE_VIEW_ERROR", "HTTP 401: Bad credentials")];
    let (code, _, err) = clm(&r.work, &gh, &BODY_ARGS, &env);
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("Bad credentials"), "{err}");
    // `pr open` keeps the body it couldn't send, and exits 2 naming the copy.
    let (code, _, err) = clm(&r.work, &gh, &OPEN_ARGS, &env);
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("Bad credentials"), "{err}");
    assert!(
        err.trim_end().ends_with("pr-bodies/spec-x.markdown"),
        "{err}"
    );
    assert!(
        !gh.calls().iter().any(|c| !c.starts_with("pr view")),
        "{:?}",
        gh.calls()
    );
}

#[test]
fn page_body_without_a_host_does_nothing() {
    // req:host/pr-body@42b28fb0
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
    // req:host/issue@f4bf6ce7
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
    // req:host/issue@f4bf6ce7
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
fn scenario_issue_an_unreadable_report() {
    // req:host/issue@f4bf6ce7
    for host in ["git", "none"] {
        let (r, gh) = (checkout(host), fake_gh());
        let (code, out, err) = clm(&r.work, &gh, &ISSUE_ARGS, &[]);
        assert_eq!((code, out.as_str()), (Some(1), ""), "{host}: {err}");
        assert!(err.contains("r.md"), "{host}: {err}");
        assert!(!err.contains("nowhere to file"), "{host}: {err}");
    }
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
    // req:host/azure-selected@0ab29923
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
    // req:host/azure-selected@0ab29923
    let (r, gh) = (checkout("azure"), fake_gh());
    fake_az(&gh);
    let env = [("FAKE_AZ_FAIL", "Please run 'az login'")];
    let (code, out, err) = clm(&r.work, &gh, &OPEN_ARGS, &env);
    assert_eq!(code, Some(2), "{out}{err}");
    assert!(format!("{out}{err}").contains("az login"), "{out}{err}");
}

#[test]
fn scenario_azure_issue_through_the_setting() {
    // req:host/azure-selected@0ab29923
    let (r, gh) = (checkout("azure"), fake_gh());
    fake_az(&gh);
    write(&r.work, "r.md", "The report\n");
    let (code, out, err) = clm(&r.work, &gh, &ISSUE_ARGS, &[]);
    assert_eq!(code, Some(1), "{err}");
    assert_eq!(out, "The report\n");
    assert!(err.contains("nowhere to file it"), "{err}");
    assert!(gh.calls().is_empty() && az_calls(&gh).is_empty());
}
