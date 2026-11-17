//! `clm land` and `clm reconnect` as calls of `clm-land` (`land/commands-in-crate`), and the
//! tests of `land`'s requirements that ran the `clm` binary, moved here to call it.

mod common;

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use clm_ledger::Body;
use common::Fixture;

/// What a call wrote and returned.
struct Out {
    code: u8,
    stdout: String,
    stderr: String,
}

/// `clm land <args>` as `clm-land`'s call, in `root` as `crates/clm` finds it from `dir`, with
/// the fixture's variables and `vars`.
fn land(f: &Fixture, dir: &Path, args: &[&str], vars: &[(&str, &str)]) -> Out {
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let code = clm_land::land_command(args, dir, &f.env(vars), &mut stdout, &mut stderr);
    Out {
        code,
        stdout: text(&stdout),
        stderr: text(&stderr),
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Sets `name` to `value` on this process for the length of `run`: `git` and the commands a
/// land starts inherit the process's environment, not the lookup. The tests run one per
/// process (`ci/nextest-runner`), so no other test sees it.
fn with_var<R>(name: &str, value: &str, run: impl FnOnce() -> R) -> R {
    let old = std::env::var_os(name);
    // SAFETY: one test per process; nothing else reads the environment meanwhile.
    unsafe { std::env::set_var(name, value) };
    let done = run();
    match old {
        // SAFETY: as above.
        Some(old) => unsafe { std::env::set_var(name, old) },
        None => unsafe { std::env::remove_var(name) },
    }
    done
}

#[test]
fn a_command_called_in_a_test() {
    // req:land/commands-in-crate@6ed92c77
    let f = Fixture::new("");
    f.write("check-prints", "echo checked\n");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let out = land(&f, &f.work(), &["impl/a"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let main = f.remote("main");
    assert_eq!(out.stdout, format!("landed {main} code\n"));
    // The check's output reaches the call's error stream, not the process's.
    assert!(out.stderr.lines().any(|l| l == "checked"), "{}", out.stderr);
    // A refusal: its reason on the error stream with the command's prefix, status 1.
    f.write("check-fails", "");
    f.branch("impl/b", &[("c.txt", "c\n")]);
    let out = land(&f, &f.work(), &["impl/b"], &[]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(out.stdout.is_empty(), "{}", out.stdout);
    let last = out.stderr.lines().rfind(|l| l.starts_with("clm land: "));
    assert!(last.is_some(), "{}", out.stderr);
}

#[test]
fn what_the_gate_and_clm_check_print_reaches_the_error_stream() {
    // req:land/commands-in-crate@6ed92c77
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    // The fixture's gate and `clm check`, saying something first.
    let noisy = |name: &str, real: &str, say: &str| {
        let path = f.path(name);
        let script = format!(
            "#!/bin/sh\n{say}\nexec '{}' \"$@\"\n",
            f.path(real).display()
        );
        common::write_exec(&path, &script);
        path.display().to_string()
    };
    let gate = noisy("noisy-gate", "bin/gate", "echo gate-said >&2");
    let check = noisy(
        "noisy-check",
        "bin/clm-check",
        "echo check-out; echo check-err >&2",
    );
    let out = land(
        &f,
        &f.work(),
        &["impl/a"],
        &[("CLM_GATE", &gate), ("CLM_CHECK", &check)],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.stdout, format!("landed {} code\n", f.remote("main")));
    let lines: Vec<&str> = out.stderr.lines().collect();
    assert!(lines.contains(&"gate-said"), "{}", out.stderr);
    // Its stdout and its stderr both, in the order it wrote them.
    let at = lines
        .iter()
        .position(|l| *l == "check-out")
        .expect(&out.stderr);
    assert_eq!(lines.get(at + 1), Some(&"check-err"), "{}", out.stderr);
}

#[test]
fn the_root_option_overrides_the_root() {
    // req:land/commands-in-crate@6ed92c77
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let work = f.work();
    let given = work.to_str().unwrap();
    let out = land(&f, &f.path("nowhere"), &["--root", given, "impl/a"], &[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.stdout, format!("landed {} code\n", f.remote("main")));
}

#[test]
fn reconnect_called_in_a_test() {
    // req:land/commands-in-crate@6ed92c77
    let f = Fixture::new("");
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let code = clm_land::reconnect_command(
        Vec::<&str>::new(),
        &f.work(),
        &f.env(&[]),
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(code, 0, "{}", text(&stderr));
    assert_eq!(text(&stdout), "reconnected 0 lands\n");
    assert!(stderr.is_empty(), "{}", text(&stderr));
}

#[test]
fn arguments_it_cant_parse() {
    // req:land/commands-in-crate@6ed92c77
    let f = Fixture::new("");
    let main = f.remote("main");
    let out = land(&f, &f.work(), &["--no-such-option"], &[]);
    assert_eq!(out.code, 2);
    assert!(out.stdout.is_empty(), "{}", out.stdout);
    assert!(
        out.stderr
            .contains("unexpected argument '--no-such-option'"),
        "{}",
        out.stderr
    );
    assert!(out.stderr.contains("Usage: clm land"), "{}", out.stderr);
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let code = clm_land::reconnect_command(
        ["--no-such-option"],
        &f.work(),
        &f.env(&[]),
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(code, 2);
    assert!(
        text(&stderr).contains("Usage: clm reconnect"),
        "{}",
        text(&stderr)
    );
    // The test's process goes on, and nothing landed.
    assert_eq!(f.remote("main"), main);
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
    let out = with_var("PATH", &path, || {
        land(&f, &f.work(), &["impl/a"], &[("PATH", &path)])
    });
    let stderr = out.stderr;
    assert_eq!(out.code, 2, "{stderr}");
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
    let out = with_var("PATH", &path, || {
        land(&f, &f.work(), &["impl/a"], &[("PATH", &path)])
    });
    let stdout = out.stdout;
    assert_eq!(out.code, 0, "{stdout}{}", out.stderr);
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

#[test]
fn checked_code_takes_no_lock() {
    // req:land/land-target-lock@017c470a
    let f = Fixture::new("");
    f.branch("spec/x", &[("specs/x/SPEC.md", "# x\n")]);
    let key = clm_ledger::code_key(&f.work(), "origin/main").unwrap();
    f.append(
        &f.work(),
        Body::Checked {
            code_key: key,
            command: f.check_command(),
            basis: None,
        },
    );
    // This test holds the lock through a file of its own, which the land's can't share.
    let home = f.work().join(".clm");
    fs::create_dir_all(&home).unwrap();
    let held = File::create(home.join("land.lock")).unwrap();
    held.lock().unwrap();
    // On a thread of its own, so a land that waits for the lock fails the test, not hangs it.
    let (work, env) = (f.work(), f.env(&[]));
    let (sent, got) = mpsc::channel();
    std::thread::spawn(move || {
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        let code = clm_land::land_command(["spec/x"], &work, &env, &mut stdout, &mut stderr);
        let _ = sent.send((code, text(&stderr)));
    });
    let (code, stderr) = got
        .recv_timeout(Duration::from_secs(60))
        .expect("`clm land` never finished");
    drop(held);
    assert_eq!(code, 0, "{stderr}");
    assert!(!stderr.contains("waiting"), "{stderr}");
}

/// A stderr the test reads while the call still writes to it.
#[derive(Clone, Default)]
struct Shared(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for Shared {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_waiting_land_says_so() {
    // req:land/land-target-lock@017c470a
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    // Another process's land holds the lock, through a file of this test's own.
    let home = f.work().join(".clm");
    fs::create_dir_all(&home).unwrap();
    let held = File::create(home.join("land.lock")).unwrap();
    held.lock().unwrap();
    let stderr = Shared::default();
    let (work, env, mut err) = (f.work(), f.env(&[]), stderr.clone());
    let (sent, got) = mpsc::channel();
    std::thread::spawn(move || {
        let mut stdout = Vec::new();
        let code = clm_land::land_command(["impl/a"], &work, &env, &mut stdout, &mut err);
        let _ = sent.send(code);
    });
    let waiting = format!(
        "waiting for another land's check on {}: 1 ahead",
        home.join("land").display()
    );
    let said = || text(&stderr.0.lock().unwrap());
    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    while !said().contains(&waiting) {
        assert!(std::time::Instant::now() < deadline, "{}", said());
        std::thread::sleep(Duration::from_millis(20));
    }
    // It waits for the lock, and lands once it is free.
    assert!(got.try_recv().is_err(), "{}", said());
    drop(held);
    let code = got
        .recv_timeout(Duration::from_secs(60))
        .expect("`clm land` never finished");
    assert_eq!(code, 0, "{}", said());
    assert_eq!(
        said().matches("waiting for another land's check").count(),
        1
    );
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
    let out = with_var("CI", "true", || land(&f, &work, &["--check-only"], &[]));
    let stderr = out.stderr;
    assert_eq!(out.code, 0, "{stderr}");
    // The edit keeps the check from counting for a land, and it is said before the verdict.
    assert_eq!(
        out.stdout.lines().collect::<Vec<_>>(),
        [
            "not recorded: the working tree has uncommitted changes",
            "checks pass"
        ]
    );
    // In the working tree, outside CI, with no `CLM_LAND_DIR`.
    let pwd = fs::canonicalize(&work).unwrap();
    assert_eq!(f.lines("check-pwd.log"), [pwd.display().to_string()]);
    assert_eq!(f.lines("check-env.log"), ["CI=unset CLM_LAND_DIR=unset"]);
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
    let out = land(&f, &f.work(), &["impl/a"], &[]);
    let stderr = out.stderr;
    assert_eq!(out.code, 0, "{stderr}");
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
    let out = land(&f, &f.work(), &["impl/a"], &[]);
    let stderr = out.stderr;
    assert_eq!(out.code, 0, "{stderr}");
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
    let out = land(&f, &f.work(), &["impl/a"], &[]);
    let stderr = out.stderr;
    assert_eq!(out.code, 0, "{stderr}");
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
    assert!(out.stdout.starts_with("landed "), "{stderr}");
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
    let out = land(&f, &f.work(), &["impl/a"], &[]);
    let stderr = out.stderr;
    assert_eq!(out.code, 0, "{stderr}");
    let numbers: Vec<&str> = stderr
        .lines()
        .filter(|l| l.len() == 1 && l.chars().all(|c| c.is_ascii_digit()))
        .collect();
    assert_eq!(numbers, ["1", "2", "3", "4", "5", "6"], "{stderr}");
    assert!(out.stdout.is_empty() || out.stdout.starts_with("landed "));
}

/// A `PATH` whose `git` fails a `fetch` naming `origin` the way a lost ref race does: every
/// one when `always`, otherwise the first only (a file `lost-race` remembers it).
fn fetch_loses_race(f: &Fixture, always: bool) -> String {
    let mark = f.path("lost-race");
    let once = if always {
        String::new()
    } else {
        format!(
            "[ -e '{m}' ] && continue; touch '{m}'; ",
            m = mark.display()
        )
    };
    let before = format!(
        "if [ \"$1\" = fetch ]; then for a in \"$@\"; do if [ \"$a\" = origin ]; then {once}\
         echo \"error: cannot lock ref 'refs/remotes/origin/main'\" >&2; exit 1; fi; done; fi"
    );
    fake_git(f, &before, "")
}

#[test]
fn fetch_loses_a_ref_race() {
    // req:land/offline-land@42e20984
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let path = fetch_loses_race(&f, false);
    let out = with_var("PATH", &path, || {
        land(&f, &f.work(), &["impl/a"], &[("PATH", &path)])
    });
    let (stdout, stderr) = (out.stdout, out.stderr);
    assert_eq!(out.code, 0, "{stdout}{stderr}");
    let main = f.remote("main");
    assert_eq!(stdout.lines().next(), Some(&*format!("landed {main} code")));
    assert!(!stderr.contains("offline"), "{stderr}");
}

#[test]
fn fetch_keeps_failing_with_origin_reachable() {
    // req:land/offline-land@42e20984
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let old = f.remote("main");
    let before = f.landed().len();
    let path = fetch_loses_race(&f, true);
    let out = with_var("PATH", &path, || {
        land(&f, &f.work(), &["impl/a"], &[("PATH", &path)])
    });
    let stderr = out.stderr;
    assert_eq!(out.code, 1, "{stderr}");
    assert!(stderr.contains("cannot lock ref"), "{stderr}");
    assert!(!stderr.contains("offline"), "{stderr}");
    assert_eq!(f.remote("main"), old);
    assert_eq!(f.landed().len(), before);
}
