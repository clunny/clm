//! `pr_open` keeps a body that reached no pull request (host/keep-unopened-body), against a
//! fake `az` and a fake `gh`.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use clm_config::Host;
use clm_host::{Error, pr_open};
use tempfile::TempDir;

/// The tests put fakes first on the process's `PATH`, one test at a time.
static PATH_LOCK: Mutex<()> = Mutex::new(());

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

fn script(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// Runs `body` with a fake `az` (no active pull request; `pr create` exits 1 printing
/// `TF401398`) and a fake `gh` (`pr view` finds nothing; `pr create` prints a URL) first on
/// `PATH`, in a repository whose `.clm/pr-body.md` holds `## Review`.
fn with_fakes(body: impl FnOnce(&Path)) {
    let _one = PATH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let bin = TempDir::new().unwrap();
    script(
        &bin.path().join("az"),
        "#!/bin/sh\ncase \"$*\" in\n*'pr list'*) echo '[]';;\n\
         *) echo TF401398 >&2; exit 1;;\nesac\n",
    );
    script(
        &bin.path().join("gh"),
        "#!/bin/sh\ncase \"$*\" in\n*'pr view'*) echo 'no pull requests found' >&2; exit 1;;\n\
         *) echo https://github.com/o/r/pull/9;;\nesac\n",
    );
    let old = std::env::var("PATH").unwrap();
    // SAFETY: every test that touches the environment holds PATH_LOCK.
    unsafe { std::env::set_var("PATH", format!("{}:{old}", bin.path().display())) };
    let repo = TempDir::new().unwrap();
    git(repo.path(), &["init", "-q", "-b", "main"]);
    git(
        repo.path(),
        &["commit", "-q", "--allow-empty", "-m", "first"],
    );
    fs::create_dir(repo.path().join(".clm")).unwrap();
    fs::write(repo.path().join(".clm/pr-body.md"), "## Review").unwrap();
    body(repo.path());
    unsafe { std::env::set_var("PATH", old) };
}

fn open(host: Host, root: &Path) -> Result<Option<String>, Error> {
    pr_open(
        host,
        root,
        Some("impl/a"),
        Some("x"),
        Some(&root.join(".clm/pr-body.md")),
    )
}

fn kept(common: &Path) -> PathBuf {
    common.join("clm/pr-bodies/impl-a.md")
}

fn message(e: Error) -> String {
    match e {
        Error::Refused(why) | Error::Git(why) | Error::Nowhere(why) => why,
    }
}

#[test]
fn the_host_refuses() {
    // req:host/keep-unopened-body@a8937350
    with_fakes(|root| {
        let why = message(open(Host::Azure, root).unwrap_err());
        let path = kept(&root.join(".git").canonicalize().unwrap());
        assert!(why.contains("TF401398"), "{why}");
        assert!(
            why.ends_with(&format!("; the body is kept at {}", path.display())),
            "{why}"
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "## Review");
    });
}

#[test]
fn a_later_failure_replaces_the_copy() {
    // req:host/keep-unopened-body@a8937350
    with_fakes(|root| {
        open(Host::Azure, root).unwrap_err();
        fs::write(root.join(".clm/pr-body.md"), "## Second").unwrap();
        open(Host::Azure, root).unwrap_err();
        let path = kept(&root.join(".git").canonicalize().unwrap());
        assert_eq!(fs::read_to_string(path).unwrap(), "## Second");
    });
}

#[test]
fn from_a_linked_worktree() {
    // req:host/keep-unopened-body@a8937350
    with_fakes(|root| {
        let linked = TempDir::new().unwrap();
        let tree = linked.path().join("tree");
        git(
            root,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "other",
                tree.to_str().unwrap(),
            ],
        );
        fs::create_dir(tree.join(".clm")).unwrap();
        fs::write(tree.join(".clm/pr-body.md"), "## Review").unwrap();
        let why = message(open(Host::Azure, &tree).unwrap_err());
        let path = kept(&root.join(".git").canonicalize().unwrap());
        assert!(why.ends_with(&path.display().to_string()), "{why}");
        git(
            root,
            &["worktree", "remove", "--force", tree.to_str().unwrap()],
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "## Review");
    });
}

#[test]
fn the_body_drawn_from_the_branch_is_kept() {
    // req:host/keep-unopened-body@a8937350
    with_fakes(|root| {
        git(root, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
        git(root, &["switch", "-q", "-c", "impl/a"]);
        git(
            root,
            &["commit", "-q", "--allow-empty", "-m", "Do the thing"],
        );
        // `az` refuses, and the body drawn from the commits is kept.
        let why = message(pr_open(Host::Azure, root, None, None, None).unwrap_err());
        assert!(why.contains("; the body is kept at "), "{why}");
        assert_eq!(
            fs::read_to_string(kept(&root.join(".git").canonicalize().unwrap())).unwrap(),
            "- Do the thing\n"
        );
    });
}

#[test]
fn opened() {
    // req:host/keep-unopened-body@a8937350
    with_fakes(|root| {
        let got = open(Host::Github, root).unwrap();
        assert_eq!(got.as_deref(), Some("https://github.com/o/r/pull/9"));
        assert!(!root.join(".git/clm/pr-bodies").exists());
    });
}

#[test]
fn refusals_before_the_host_keep_nothing() {
    // req:host/keep-unopened-body@a8937350
    with_fakes(|root| {
        // A detached HEAD, a missing file and a blank one reach no host.
        git(root, &["checkout", "-q", "--detach"]);
        pr_open(Host::Azure, root, None, Some("x"), None).unwrap_err();
        let missing = pr_open(
            Host::Azure,
            root,
            Some("impl/a"),
            Some("x"),
            Some(Path::new("nope.md")),
        );
        assert!(!message(missing.unwrap_err()).contains("kept"));
        fs::write(root.join(".clm/pr-body.md"), "  \n").unwrap();
        assert!(!message(open(Host::Azure, root).unwrap_err()).contains("kept"));
        // `git` and `none` ask no host.
        assert_eq!(open(Host::Git, root).unwrap(), None);
        assert_eq!(open(Host::None, root).unwrap(), None);
        assert!(!root.join(".git/clm/pr-bodies").exists());
    });
}
