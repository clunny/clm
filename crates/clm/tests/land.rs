//! `clm land` as a process (specs/land/SPEC.md): what only the binary shows, its own copy
//! and what it prints on stderr while it waits.

#[path = "../../clm-land/tests/common/mod.rs"]
mod common;

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use clm_ledger::Body;
use common::Fixture;

/// `clm land <args>` as the binary at `exe`, in `dir`, with the fixture's variables and
/// `vars`, and nothing of this process's own `clm` or cargo settings.
fn land(f: &Fixture, exe: &Path, dir: &Path, args: &[&str], vars: &[(&str, &str)]) -> Command {
    let mut command = Command::new(exe);
    command.arg("land").args(args).current_dir(dir);
    for (key, _) in std::env::vars() {
        if key.starts_with("CLM_") || key.starts_with("CARGO") || key == "CI" {
            command.env_remove(key);
        }
    }
    command.env_remove("XDG_CONFIG_HOME");
    for (key, value) in f.vars(vars) {
        command.env(key, value);
    }
    command.stdin(Stdio::null());
    command
}

fn bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_clm"))
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// A fixture whose `main` holds a root spec, so the real gate and `clm check` pass on it.
fn with_spec() -> Fixture {
    let f = Fixture::new("");
    f.advance_main(
        "specs/SPEC.md",
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    );
    let work = f.work();
    f.git(&work, &["fetch", "-q", "origin"]);
    f.git(&work, &["merge", "-q", "--ff-only", "origin/main"]);
    f.write("gate-out", "code\n");
    f
}

#[test]
fn check_deletes_the_binary() {
    // req:land/land-binary-copy@7e2d6c4e
    let f = with_spec();
    let scratch = f.path("clm-scratch");
    fs::copy(bin(), &scratch).unwrap();
    f.write("check-deletes", &scratch.display().to_string());
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let out = land(
        &f,
        &scratch,
        &f.work(),
        &["impl/a"],
        &[("CLM_GATE", ""), ("CLM_CHECK", "")],
    )
    .output()
    .unwrap();
    assert!(!scratch.exists(), "the check didn't delete the binary");
    let stdout = text(&out.stdout);
    assert!(out.status.success(), "{stdout}{}", text(&out.stderr));
    let main = f.remote("main");
    assert_eq!(stdout.lines().next(), Some(&*format!("landed {main} code")));
}

#[test]
fn copy_removed() {
    // req:land/land-binary-copy@7e2d6c4e
    let f = with_spec();
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let out = land(
        &f,
        bin(),
        &f.work(),
        &["impl/a"],
        &[("CLM_GATE", ""), ("CLM_CHECK", "")],
    )
    .output()
    .unwrap();
    assert!(out.status.success(), "{}", text(&out.stderr));
    f.assert_no_scratch();
    // And after a land that fails.
    f.write("check-fails", "");
    f.branch("impl/b", &[("c.txt", "c\n")]);
    let out = land(
        &f,
        bin(),
        &f.work(),
        &["impl/b"],
        &[("CLM_GATE", ""), ("CLM_CHECK", "")],
    )
    .output()
    .unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stderr));
    f.assert_no_scratch();
}

/// The first `git` on this process's `PATH`.
fn real_git() -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|d| d.join("git"))
        .find(|p| p.is_file())
        .expect("git on PATH")
}

/// A `PATH` whose first `git` runs `before` (sh, with `$REAL` the real `git`; it may exit),
/// then the real `git`, then `after` (with `$status` the real one's exit status).
fn fake_git(f: &Fixture, before: &str, after: &str) -> String {
    let bin_dir = f.path("fake-git");
    fs::create_dir_all(&bin_dir).unwrap();
    let script = format!(
        "#!/bin/sh\nREAL='{}'\n{before}\n\"$REAL\" \"$@\"\nstatus=$?\n{after}\nexit $status\n",
        real_git().display()
    );
    common::write_exec(&bin_dir.join("git"), &script);
    let path = std::env::join_paths(
        std::iter::once(bin_dir).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    path.to_str().unwrap().to_owned()
}

/// Shell that exits 1 when the command is a `push` naming `main` as its destination.
const PUSH_TO_MAIN_FAILS: &str = "if [ \"$1\" = push ]; then for a in \"$@\"; do \
     case \"$a\" in *:refs/heads/main) echo 'fake refusal' >&2; exit 1;; esac; done; fi";

#[test]
fn refetch_fails() {
    // req:land/land-push-retry@94d61d9c
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let old = f.remote("main");
    // The push to `main` is refused before it reaches `origin`, and the refetch of `main`
    // fails, so the push may have gone through for all `clm land` knows.
    let fetch_fails = "if [ \"$1\" = fetch ]; then for a in \"$@\"; do \
         case \"$a\" in *refs/heads/main:*) exit 1;; esac; done; fi";
    let path = fake_git(&f, &format!("{PUSH_TO_MAIN_FAILS}\n{fetch_fails}"), "");
    let out = land(&f, bin(), &f.work(), &["impl/a"], &[("PATH", &path)])
        .output()
        .unwrap();
    let stderr = text(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("fake refusal"), "{stderr}");
    assert!(!stderr.contains("may be transient"), "{stderr}");
    assert!(!stderr.contains("offline"), "{stderr}");
    assert_eq!(f.remote("main"), old);
    assert!(f.landed().is_empty());
}

#[test]
fn pushed_though_git_failed() {
    // req:land/land-push-retry@94d61d9c
    let f = Fixture::new("");
    let head = f.branch("impl/a", &[("b.txt", "b\n")]);
    // A `git` that pushes to `main` and then reports a failure whatever happened.
    let path = fake_git(&f, "", PUSH_TO_MAIN_FAILS);
    let out = land(&f, bin(), &f.work(), &["impl/a"], &[("PATH", &path)])
        .output()
        .unwrap();
    let stdout = text(&out.stdout);
    assert!(out.status.success(), "{stdout}{}", text(&out.stderr));
    let main = f.remote("main");
    assert_eq!(stdout.lines().next(), Some(&*format!("landed {main} code")));
    assert_eq!(f.parents(&f.path("origin.git"), &main)[1], head);
    let landed: Vec<_> = f
        .landed()
        .into_iter()
        .filter(|b| matches!(b, Body::Landed { commit, .. } if *commit == main))
        .collect();
    assert_eq!(landed.len(), 1);
}

/// Waits for `path` to hold `needle`, up to a minute.
fn wait_for(path: &Path, needle: &str) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while !fs::read_to_string(path)
        .unwrap_or_default()
        .contains(needle)
    {
        assert!(
            Instant::now() < deadline,
            "{} never held {needle}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// `child`'s output, killing it if it runs past a minute.
fn finish(mut child: Child) -> Output {
    let deadline = Instant::now() + Duration::from_secs(60);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            child.kill().unwrap();
            panic!("`clm land` never finished");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    child.wait_with_output().unwrap()
}

/// `clm land` of `branch` from the linked worktree `dir`, its stderr to the file `err`.
fn spawn_land(f: &Fixture, dir: &Path, err: &str) -> Child {
    let err = File::create(f.path(err)).unwrap();
    land(f, bin(), dir, &[], &[])
        .stdout(Stdio::piped())
        .stderr(err)
        .spawn()
        .unwrap()
}

/// Both lands succeeded; returns the second's stderr.
fn both_landed(f: &Fixture, first: Child, second: Child) -> String {
    let (first, second) = (finish(first), finish(second));
    let a_err = fs::read_to_string(f.path("a.err")).unwrap();
    assert!(first.status.success(), "{a_err}");
    let b_err = fs::read_to_string(f.path("b.err")).unwrap();
    assert!(second.status.success(), "{b_err}");
    assert_eq!(f.parents(&f.path("origin.git"), "main").len(), 2);
    b_err
}

#[test]
fn overlapping_lands() {
    // req:land/land-target-lock@ca751f80
    let f = Fixture::new("");
    f.write("hold", "");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f.branch("impl/b", &[("c.txt", "c\n")]);
    let a = f.linked("a", "impl/a");
    let b = f.linked("b", "impl/b");
    let first = spawn_land(&f, &a, "a.err");
    wait_for(&f.path("check-order.log"), "start");
    let second = spawn_land(&f, &b, "b.err");
    wait_for(&f.path("b.err"), "waiting");
    f.write("release", "");
    let b_err = both_landed(&f, first, second);
    let target = f.work().join(".clm/land");
    let waiting = format!("waiting for another land's check on {}", target.display());
    assert_eq!(b_err.matches(&waiting).count(), 1, "{b_err}");
    let a_err = fs::read_to_string(f.path("a.err")).unwrap();
    assert!(!a_err.contains("waiting"), "{a_err}");
    // Every start is followed by its own end before any other start. The lock is held
    // until the first land's push, so the waiting land finds `main` moved and checks once.
    let order = f.lines("check-order.log");
    assert_eq!(order.len(), 4, "{order:?}");
    for pair in order.chunks(2) {
        let start = pair[0].strip_prefix("start ").unwrap();
        assert_eq!(pair[1], format!("end {start}"), "{order:?}");
    }
}

#[test]
fn held_through_the_gate_on_the_merge() {
    // req:land/land-target-lock@ca751f80
    let f = Fixture::new("");
    f.write("check-sleeps", "2");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f.branch("impl/b", &[("c.txt", "c\n")]);
    let a = f.linked("a", "impl/a");
    let b = f.linked("b", "impl/b");
    let first = spawn_land(&f, &a, "a.err");
    let second = spawn_land(&f, &b, "b.err");
    both_landed(&f, first, second);
    // Each land's two gates fall around its own check, never inside the other land's.
    let order = f.lines("order.log");
    let [g1, c1, g2, g3, c2, g4] = &order[..] else {
        panic!("{order:?}")
    };
    assert_eq!([g1, g2, g3, g4], ["gate"; 4], "{order:?}");
    let checked = |c: &str| c.strip_prefix("check ").map(str::to_owned);
    let mut branches = [checked(c1), checked(c2)];
    branches.sort();
    assert_eq!(
        branches,
        [Some("impl/a".to_owned()), Some("impl/b".to_owned())],
        "{order:?}"
    );
}

#[test]
fn checked_code_takes_no_lock() {
    // req:land/land-target-lock@ca751f80
    let f = Fixture::new("");
    f.branch("spec/x", &[("specs/x/SPEC.md", "# x\n")]);
    let key = clm_ledger::code_key(&f.work(), "origin/main").unwrap();
    f.append(
        &f.work(),
        Body::Checked {
            code_key: key,
            command: f.check_command(),
        },
    );
    // This process holds the lock, which the `clm land` process can't share.
    let home = f.work().join(".clm");
    fs::create_dir_all(&home).unwrap();
    let held = File::create(home.join("land.lock")).unwrap();
    held.lock().unwrap();
    let child = land(&f, bin(), &f.work(), &["spec/x"], &[])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let out = finish(child);
    drop(held);
    let stderr = text(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert!(!stderr.contains("waiting"), "{stderr}");
}

#[test]
fn check_only_checks_the_working_tree() {
    // req:land/check-only@938e24c2
    let f = Fixture::new("");
    let work = f.work();
    // An uncommitted edit: the check runs on the working tree as it stands.
    fs::write(work.join("a.txt"), "edited\n").unwrap();
    let head = f.rev("HEAD");
    let main = f.remote("main");
    let out = land(&f, bin(), &work, &["--check-only"], &[])
        .env("CI", "true")
        .output()
        .unwrap();
    let stderr = text(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    // The edit keeps the check from counting for a land, and it is said before the verdict.
    assert_eq!(
        text(&out.stdout).lines().collect::<Vec<_>>(),
        [
            "not recorded: the working tree has uncommitted changes",
            "checks pass"
        ]
    );
    // In the working tree, outside CI, building where the caller's environment says.
    let pwd = fs::canonicalize(&work).unwrap();
    assert_eq!(f.lines("check-pwd.log"), [pwd.display().to_string()]);
    assert_eq!(
        f.lines("check-env.log"),
        ["CI=unset CARGO_TARGET_DIR=unset"]
    );
    // Then `clm check`, with its detectors.
    let clm_check = f.lines("clm-check.log");
    assert_eq!(clm_check.len(), 1, "{clm_check:?}");
    assert!(clm_check[0].ends_with(' '), "{clm_check:?}");
    // Nothing landed, committed or recorded.
    assert_eq!(f.rev("HEAD"), head);
    assert_eq!(f.remote("main"), main);
    let status = f.git(&work, &["status", "--porcelain", "--untracked-files=no"]);
    assert_eq!(status, "M a.txt");
    let ledgers = f.git(
        &work,
        &[
            "for-each-ref",
            "refs/heads/clm/ledger",
            "refs/remotes/origin/clm/ledger",
        ],
    );
    assert!(ledgers.is_empty(), "{ledgers}");
}

#[test]
fn selection_fails_says_why() {
    // req:land/selected-tests@912a8d72
    let f = Fixture::new("");
    f.write("affected-fails", "");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let out = land(&f, bin(), &f.work(), &["impl/a"], &[])
        .output()
        .unwrap();
    let stderr = text(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    // The first non-empty line of its stderr, trimmed, on one line.
    let line = "clm affected failed: no graph here; the check runs every test";
    assert!(stderr.lines().any(|l| l == line), "{stderr}");
    assert_eq!(f.lines("check-selection.log"), ["CLM_SELECTION=unset"]);
}

#[test]
fn readers_selection_fails_says_why() {
    // req:land/selected-tests@912a8d72
    let f = Fixture::new("");
    f.write("affected-fails", "");
    f.branch("impl/a", &[("ARCHITECTURE.md", "# x\n")]);
    f.mark_checked("origin/main");
    let out = land(&f, bin(), &f.work(), &["impl/a"], &[])
        .output()
        .unwrap();
    let stderr = text(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    let line = "clm affected failed: no graph here; the check runs every test";
    assert!(stderr.lines().any(|l| l == line), "{stderr}");
    assert_eq!(f.lines("check-selection.log"), ["CLM_SELECTION=unset"]);
}

#[test]
fn check_log_cant_be_created() {
    // req:land/check-log@6fefb1a1
    let f = Fixture::new("");
    f.write("check-prints", "echo checked\n");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let home = f.work().join(".clm");
    fs::create_dir_all(&home).unwrap();
    fs::write(home.join("land-logs"), "a file").unwrap();
    let out = land(&f, bin(), &f.work(), &["impl/a"], &[])
        .output()
        .unwrap();
    let stderr = text(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    let path = fs::canonicalize(f.work())
        .unwrap()
        .join(".clm/land-logs/impl/a.log");
    let warning = format!(
        "clm land: warning: can't write the check's log at {}",
        path.display()
    );
    assert!(stderr.contains(&warning), "{stderr}");
    // The check ran, its output on stderr all the same, and the branch landed.
    assert!(stderr.lines().any(|l| l == "checked"), "{stderr}");
    assert_eq!(f.lines("check.log").len(), 1);
    assert!(text(&out.stdout).starts_with("landed "), "{stderr}");
}

#[test]
fn check_output_reaches_stderr_in_order() {
    // req:land/check-log@6fefb1a1
    let f = Fixture::new("");
    f.write(
        "check-prints",
        "for i in $(seq 6); do if [ $((i % 2)) = 1 ]; then echo $i; else echo $i >&2; fi; done\n",
    );
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let out = land(&f, bin(), &f.work(), &["impl/a"], &[])
        .output()
        .unwrap();
    let stderr = text(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    let numbers: Vec<&str> = stderr
        .lines()
        .filter(|l| l.len() == 1 && l.chars().all(|c| c.is_ascii_digit()))
        .collect();
    assert_eq!(numbers, ["1", "2", "3", "4", "5", "6"], "{stderr}");
    assert!(text(&out.stdout).is_empty() || text(&out.stdout).starts_with("landed "));
}
