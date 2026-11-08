//! `clm/none-remote-before-commands` as the binary runs it: a `host = "none"` clone gets its
//! own remote before a command runs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use clm_ledger::{Body, LandKind};
use tempfile::TempDir;

const NONE: &str = "host = \"none\"\n\n[land]\ncheck = \"true\"\n";

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_owned()
}

/// `clm <args>` in `dir`, with nothing of this process's own `clm` settings.
fn clm(dir: &Path, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_clm"));
    command.args(args).current_dir(dir);
    for (key, _) in std::env::vars() {
        if key.starts_with("CLM_") || key == "CI" {
            command.env_remove(key);
        }
    }
    command
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("XDG_CONFIG_HOME", dir.join("no-config"))
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// A CLM repository `work` on `main` whose `clm.toml` is `config`, with no remote.
fn repo(config: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let work = dir.path().canonicalize().unwrap().join("work");
    fs::create_dir(&work).unwrap();
    git(&work, &["init", "-q", "-b", "main"]);
    git(&work, &["config", "user.name", "Christopher Lunny"]);
    git(&work, &["config", "user.email", "t@example.com"]);
    fs::create_dir(work.join("specs")).unwrap();
    fs::write(
        work.join("specs/SPEC.md"),
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    )
    .unwrap();
    fs::write(work.join("clm.toml"), config).unwrap();
    fs::write(
        work.join(".gitignore"),
        "clm.local.toml\n.clm/\n.claude/worktrees/\n",
    )
    .unwrap();
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-q", "-m", "init"]);
    (dir, work)
}

fn remotes(work: &Path) -> String {
    git(work, &["remote"])
}

// req:clm/none-remote-before-commands@2ab520dd
#[test]
fn a_repository_set_up_before_gets_its_remote_and_reconnects() {
    let (_dir, work) = repo(NONE);
    // A land made with no remote, as an older `clm` did.
    fs::write(work.join("b.txt"), "b\n").unwrap();
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-q", "-m", "land"]);
    let landed = git(&work, &["rev-parse", "HEAD"]);
    let body = Body::Landed {
        commit: landed.clone(),
        branch: "impl/a".into(),
        kind: LandKind::Code,
        review: false,
        fingerprint: "f".into(),
        code_key: None,
        changes: vec![],
    };
    clm_ledger::append(&work, body, &|_| None).unwrap();
    assert_eq!(remotes(&work), "");

    let next = clm(&work, &["next"]);
    let stderr = err(&next);
    let remote = git(&work, &["remote", "get-url", "origin"]);
    assert!(remote.ends_with("clm/remote.git"), "{remote}");
    assert!(
        stderr.contains(&format!(
            "origin: {remote} (the clone's own remote, host = \"none\")"
        )),
        "{stderr}"
    );
    assert!(stderr.contains("clm reconnect"), "{stderr}");
    assert_eq!(
        git(&work, &["ls-remote", "origin"]),
        "",
        "nothing was pushed"
    );

    let reconnect = clm(&work, &["reconnect"]);
    assert_eq!(
        out(&reconnect).trim(),
        "reconnected 1 lands",
        "{}",
        err(&reconnect)
    );
    assert_eq!(err(&reconnect), "", "the remote is already the clone's own");
    assert_eq!(
        git(&work, &["ls-remote", "origin", "refs/heads/main"])
            .split_whitespace()
            .next(),
        Some(landed.as_str())
    );
    assert_ne!(
        git(&work, &["ls-remote", "origin", "refs/heads/clm/ledger/*"]),
        ""
    );
}

// req:clm/none-remote-before-commands@2ab520dd
#[test]
fn the_loop_runs_with_no_host() {
    let (_dir, work) = repo(NONE);
    // The first command gives the clone its remote; `reconnect` puts `main` on it.
    clm(&work, &["next"]);
    let reconnect = clm(&work, &["reconnect"]);
    assert!(reconnect.status.success(), "{}", err(&reconnect));
    git(&work, &["fetch", "-q", "origin"]);

    let tree = work.parent().unwrap().join("wt");
    git(
        &work,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "impl/a",
            tree.to_str().unwrap(),
            "origin/main",
        ],
    );
    fs::write(tree.join("c.txt"), "c\n").unwrap();
    git(&tree, &["add", "-A"]);
    git(&tree, &["commit", "-q", "-m", "c"]);

    let rebase = clm(&tree, &["rebase"]);
    assert_eq!(
        out(&rebase).lines().last(),
        Some("rebase ok"),
        "{}{}",
        out(&rebase),
        err(&rebase)
    );
    let push = clm(&tree, &["branch", "push"]);
    let sha = git(&tree, &["rev-parse", "HEAD"]);
    assert_eq!(
        out(&push).trim(),
        format!("pushed impl/a {sha}"),
        "{}",
        err(&push)
    );
    let land = clm(&tree, &["land", "impl/a"]);
    let landed = out(&land);
    assert!(landed.starts_with("landed "), "{landed}{}", err(&land));
    assert!(landed.trim().ends_with(" code"), "{landed}");
    let main = landed.split_whitespace().nth(1).unwrap();
    assert_eq!(
        git(&work, &["ls-remote", "origin", "refs/heads/main"])
            .split_whitespace()
            .next(),
        Some(main)
    );
}

// req:clm/none-remote-before-commands@2ab520dd
#[test]
fn a_real_origin_under_none_is_left_and_warned_about() {
    let (dir, work) = repo(NONE);
    let bare = dir.path().join("elsewhere.git");
    git(
        dir.path(),
        &["init", "-q", "--bare", bare.to_str().unwrap()],
    );
    git(&work, &["remote", "add", "origin", bare.to_str().unwrap()]);
    git(&work, &["push", "-q", "origin", "main"]);
    let next = clm(&work, &["next"]);
    assert_eq!(remotes(&work), "origin");
    assert_eq!(
        git(&work, &["remote", "get-url", "origin"]),
        bare.to_str().unwrap()
    );
    let stderr = err(&next);
    assert!(stderr.contains("host = \"git\""), "{stderr}");
    assert!(!stderr.contains("the clone's own remote"), "{stderr}");
}

// req:clm/none-remote-before-commands@2ab520dd
#[test]
fn hooks_and_the_guide_create_nothing() {
    let (_dir, work) = repo(NONE);
    let guide = clm(&work, &["guide", "quickstart"]);
    assert!(guide.status.success(), "{}", err(&guide));
    let mut hook = Command::new(env!("CARGO_BIN_EXE_clm"));
    hook.args(["hook", "session-start"])
        .current_dir(&work)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(remotes(&work), "");
}

// req:clm/none-remote-before-commands@2ab520dd
#[test]
fn other_hosts_and_other_directories_create_nothing() {
    let (_dir, work) = repo("host = \"git\"\n");
    clm(&work, &["next"]);
    assert_eq!(remotes(&work), "");

    // Settings that don't load.
    let (_dir, bad) = repo("host = \"nowhere\"\n");
    clm(&bad, &["next"]);
    assert_eq!(remotes(&bad), "");

    // A git repository that isn't under CLM.
    let (_dir, plain) = repo(NONE);
    git(&plain, &["rm", "-q", "-r", "specs"]);
    git(&plain, &["commit", "-q", "-m", "no specs"]);
    clm(&plain, &["check"]);
    assert_eq!(remotes(&plain), "");
    let elsewhere = TempDir::new().unwrap();
    clm(
        elsewhere.path(),
        &["next", "--root", plain.to_str().unwrap()],
    );
    assert_eq!(remotes(&plain), "");
}

// req:clm/none-remote-before-commands@2ab520dd
#[test]
fn a_command_given_a_root_works_for_that_root_and_a_subdirectory_for_its_repository() {
    let (_dir, work) = repo(NONE);
    let elsewhere = TempDir::new().unwrap();
    clm(
        elsewhere.path(),
        &["next", "--root", work.to_str().unwrap()],
    );
    assert_eq!(remotes(&work), "origin");

    let (_dir, work) = repo(NONE);
    fs::create_dir(work.join("specs/sub")).unwrap();
    clm(&work.join("specs/sub"), &["status"]);
    assert_eq!(remotes(&work), "origin");
}

// req:clm/none-remote-before-commands@2ab520dd
#[test]
fn a_command_that_needs_no_migration_still_keeps_the_remote_but_a_blocked_one_does_not() {
    let (_dir, work) = repo("host = \"none\"\nupgraded = \"99990101-from-the-future\"\n");
    let land = clm(&work, &["land", "--check-only"]);
    assert_eq!(land.status.code(), Some(4));
    assert_eq!(remotes(&work), "", "a command the upgrade gate stopped");
    clm(&work, &["status"]);
    assert_eq!(remotes(&work), "origin");
}
