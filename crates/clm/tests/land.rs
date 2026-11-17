//! `clm land` as a process (specs/land/SPEC.md): what only the binary shows, its own copy
//! and two lands running at once. The rest of `land`'s tests call `clm-land`'s call in its
//! own crate (`land/commands-in-crate`).

#[path = "../../clm-land/tests/common/mod.rs"]
mod common;

use std::fs::{self, File};
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

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
    // req:clm/land-as-a-process@ac304254
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
    // req:clm/land-as-a-process@ac304254
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
    // req:clm/land-as-a-process@ac304254
    // req:land/land-target-lock@2dadb710
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
fn a_callers_target_takes_the_lock_too() {
    // req:clm/land-as-a-process@ac304254
    // req:land/land-target-lock@2dadb710
    let f = Fixture::new("");
    f.write("hold", "");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f.branch("impl/b", &[("c.txt", "c\n")]);
    let a = f.linked("a", "impl/a");
    let b = f.linked("b", "impl/b");
    let spawn = |dir: &Path, err: &str, target: &str| {
        let err = File::create(f.path(err)).unwrap();
        let target = f.path(target);
        land(
            &f,
            bin(),
            dir,
            &[],
            &[("CARGO_TARGET_DIR", target.to_str().unwrap())],
        )
        .stdout(Stdio::piped())
        .stderr(err)
        .spawn()
        .unwrap()
    };
    let first = spawn(&a, "a.err", "ta");
    wait_for(&f.path("check-order.log"), "start");
    let second = spawn(&b, "b.err", "tb");
    wait_for(&f.path("b.err"), "waiting");
    f.write("release", "");
    let b_err = both_landed(&f, first, second);
    let waiting = format!(
        "waiting for another land's check on {}",
        f.work().join(".clm/land").display()
    );
    assert_eq!(b_err.matches(&waiting).count(), 1, "{b_err}");
    let order = f.lines("check-order.log");
    assert_eq!(order.len(), 4, "{order:?}");
    for pair in order.chunks(2) {
        let start = pair[0].strip_prefix("start ").unwrap();
        assert_eq!(pair[1], format!("end {start}"), "{order:?}");
    }
}

#[test]
fn held_through_the_gate_on_the_merge() {
    // req:clm/land-as-a-process@ac304254
    // req:land/land-target-lock@2dadb710
    let f = Fixture::new("");
    f.write("check-sleeps", "2");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f.branch("impl/b", &[("c.txt", "c\n")]);
    let a = f.linked("a", "impl/a");
    let b = f.linked("b", "impl/b");
    let first = spawn_land(&f, &a, "a.err");
    // Two lands started at once may be taken together on one check (`land/batched-lands`),
    // so the second starts once the first's check runs, and queues while that check sleeps.
    wait_for(&f.path("check-order.log"), "start");
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
