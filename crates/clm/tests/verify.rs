//! `clm verify` as the binary dispatches it (`verify-command`).

use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn clm(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap()
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
}

/// A repository `clm init` has just set up.
fn initialised() -> TempDir {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"]);
    let init = clm(
        dir.path(),
        &["init", "--lang", "rust", "--no-detectors", "--host", "none"],
    );
    assert!(init.status.success(), "{init:?}");
    dir
}

const OFF: &str = "verify off: no [verify] in origin/main:clm.toml\n";

/// A set-up repository with `[verify]` on, so each flag reaches its own mode.
fn verifying() -> TempDir {
    let dir = initialised();
    let toml = dir.path().join("clm.toml");
    let mut text = std::fs::read_to_string(&toml).unwrap();
    text.push_str("\n[verify]\ncheck = \"true\"\nmode = \"local\"\ntriggers = [\"work-done\"]\n");
    std::fs::write(&toml, text).unwrap();
    git(dir.path(), &["add", "-A"]);
    git(
        dir.path(),
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@e",
            "commit",
            "-qm",
            "i",
        ],
    );
    // `clm verify` reads its settings from `origin/main`.
    git(dir.path(), &["push", "-q", "origin", "main"]);
    dir
}

fn verify_as_writer(dir: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("CLM_WRITER", "t")
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

// req:clm/verify-command@57b06e54
#[test]
fn each_flag_reaches_its_own_mode() {
    let dir = verifying();
    let run = |args: &[&str]| verify_as_writer(dir.path(), args);
    let (code, out, _) = run(&["verify", "--due"]);
    assert_eq!(
        (code, out.as_str()),
        (Some(0), "verify not-due no trigger holds for 1 lands\n")
    );
    let (code, out, _) = run(&["verify", "--due", "--event", "work-done", "--dry-run"]);
    assert!(out.starts_with("verify due"), "{out}");
    assert_eq!(code, Some(0));
    let (code, out, _) = run(&["verify", "--due", "--dry-run"]);
    assert_eq!((code, out.starts_with("verify not-due")), (Some(3), true));
    let (code, out, _) = run(&["verify", "--import"]);
    assert_eq!(
        (code, out.as_str()),
        (Some(0), "verify nothing to import\n")
    );
    let (code, out, _) = run(&["verify", "--commit", "HEAD"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.starts_with("verify passed "), "{out}");
    let (code, _, err) = run(&["verify", "--bisect", "HEAD"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("nothing to bisect"), "{err}");
}

// req:clm/verify-command@57b06e54
#[test]
fn a_fresh_project_is_off_from_a_subdirectory() {
    let dir = initialised();
    let out = clm(&dir.path().join("specs"), &["verify", "--due"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), OFF);
}

// req:clm/verify-command@57b06e54
#[test]
fn a_subdirectory_reads_the_project_settings() {
    let dir = verifying();
    let (code, out, _) = verify_as_writer(&dir.path().join("specs"), &["verify", "--import"]);
    assert_eq!(
        (code, out.as_str()),
        (Some(0), "verify nothing to import\n")
    );
}

// req:clm/verify-command@57b06e54
#[test]
fn help_lists_the_command() {
    let dir = initialised();
    let help = clm(dir.path(), &["--help"]);
    let text = String::from_utf8_lossy(&help.stdout);
    assert!(
        text.lines().any(|l| l.trim_start().starts_with("verify ")),
        "{text}"
    );
}

/// The start of a marker citing a requirement of the fixture's component `a`, built so the
/// literal never appears whole in this file.
const MARK: &str = concat!("req", ":a/");

/// Waits up to twenty seconds for `path` to exist.
fn wait_for(path: &Path) -> bool {
    for _ in 0..400 {
        if path.exists() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    path.exists()
}

/// Whether the process `pid` still runs: a zombie, which has ended and waits only to be
/// reaped, doesn't.
fn running(pid: &str) -> bool {
    std::fs::read_to_string(format!("/proc/{}/stat", pid.trim())).is_ok_and(|stat| {
        stat.rsplit(')')
            .next()
            .is_some_and(|s| !s.trim_start().starts_with('Z'))
    })
}

fn kill(pid: u32, signal: &str) {
    let _ = Command::new("kill")
        .args([signal, &pid.to_string()])
        .status();
}

fn write_exec(path: &Path, text: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, text).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// A set-up repository whose component `a` holds requirements `one` and `two`, each cited
/// by a test, landed on `origin/main` with a `[verify]` whose `check` and `mutate` run the
/// two scripts given, written outside the repository in the second directory returned.
fn mutating(check: &str, mutate: &str) -> (TempDir, TempDir) {
    let dir = initialised();
    let bin = TempDir::new().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("specs/a")).unwrap();
    std::fs::write(
        root.join("specs/a/SPEC.md"),
        "---\ntitle: A\nstatus: active\nowners: []\npaths: [\"crates/a/**\"]\n---\n\n\
         ## Contract\nc\n\n## Design\nd\n\n## Requirements\n\n\
         ### Requirement: one\nIt SHALL do one.\n\n#### Scenario: s\n- GIVEN a\n- WHEN b\n- THEN c\n\n\
         ### Requirement: two\nIt SHALL do two.\n\n#### Scenario: s\n- GIVEN a\n- WHEN b\n- THEN c\n",
    )
    .unwrap();
    let hashes = clm(root, &["hashes", "a"]);
    let hashes = String::from_utf8_lossy(&hashes.stdout).into_owned();
    let version = |slug: &str| {
        hashes
            .lines()
            .find_map(|l| l.strip_prefix(&format!("requirement:{slug} ")))
            .unwrap_or_else(|| panic!("{hashes}"))[..8]
            .to_string()
    };
    std::fs::create_dir_all(root.join("crates/a/tests")).unwrap();
    std::fs::write(root.join("crates/a/src.rs"), "1\n").unwrap();
    for slug in ["one", "two"] {
        std::fs::write(
            root.join(format!("crates/a/tests/{slug}.rs")),
            format!(
                "#[test]\nfn proves_{slug}() {{\n    // {MARK}{slug}@{}\n}}\n",
                version(slug)
            ),
        )
        .unwrap();
    }
    let check_path = bin.path().join("check");
    write_exec(&check_path, &format!("#!/bin/sh\n{check}\n"));
    let mutate_path = bin.path().join("mutate");
    write_exec(&mutate_path, &format!("#!/bin/sh\n{mutate}\n"));
    let toml = root.join("clm.toml");
    let mut text = std::fs::read_to_string(&toml).unwrap();
    text.push_str(&format!(
        "\n[verify]\ncheck = \"{}\"\nmode = \"local\"\ntriggers = []\nevery = {{ lands = 1 }}\n\
         mutate = \"{}\"\n",
        check_path.display(),
        mutate_path.display()
    ));
    std::fs::write(&toml, text).unwrap();
    git(root, &["add", "-A"]);
    git(
        root,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@e",
            "commit",
            "-qm",
            "a",
        ],
    );
    git(root, &["push", "-q", "origin", "main"]);
    (dir, bin)
}

fn spawn_verify(dir: &Path) -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(["verify", "--due"])
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("CLM_WRITER", "t")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap()
}

// req:clm/verify-command@57b06e54
#[test]
fn stopped_by_hand() {
    let marks = TempDir::new().unwrap();
    let m = marks.path().display();
    // It gives up after about 20 seconds, so a `clm verify` that ignores the signal fails
    // here rather than hanging.
    let (dir, _bin) = mutating(
        "true",
        &format!(
            "echo $$ > '{m}/leader'\necho 'a/one killed' >> \"$CLM_MUTATE_RESULT\"\n\
             : > '{m}/written'\ni=0\nwhile [ $i -lt 400 ]; do sleep 0.05; i=$((i + 1)); done"
        ),
    );
    let started = std::time::Instant::now();
    let child = spawn_verify(dir.path());
    assert!(
        wait_for(&marks.path().join("written")),
        "the line was never written"
    );
    kill(child.id(), "-TERM");
    // A further signal during the pass changes nothing.
    kill(child.id(), "-INT");
    let out = child.wait_with_output().unwrap();
    assert!(
        started.elapsed() < std::time::Duration::from_secs(15),
        "{:?}",
        started.elapsed()
    );
    let (stdout, stderr) = (
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert_eq!(out.status.code(), Some(143), "{stdout}{stderr}");
    assert!(stdout.contains("verify passed"), "{stdout}");
    assert!(stderr.contains("mutation: stopped"), "{stderr}");
    let leader = std::fs::read_to_string(marks.path().join("leader")).unwrap();
    assert!(!running(&leader), "the command still runs");
    let fold = clm_ledger::fold(dir.path()).unwrap();
    let mut records: Vec<_> = fold.mutations.into_values().flatten().collect();
    records.sort_by(|a, b| a.at.cmp(&b.at));
    match records.pop().map(|r| r.body) {
        Some(clm_ledger::Body::Mutated { killed, left, .. }) => {
            assert_eq!(killed, ["a/one"]);
            assert_eq!(left, ["a/two"]);
        }
        other => panic!("{other:?}"),
    }
}

// req:clm/verify-command@57b06e54
#[test]
fn a_signal_before_the_pass_ends_it_as_any_process() {
    use std::os::unix::process::ExitStatusExt;
    let marks = TempDir::new().unwrap();
    let m = marks.path().display();
    let (dir, _bin) = mutating(
        &format!(
            ": > '{m}/checking'\ni=0\nwhile [ $i -lt 400 ]; do sleep 0.05; i=$((i + 1)); done"
        ),
        &format!(": > '{m}/mutated'"),
    );
    let child = spawn_verify(dir.path());
    assert!(
        wait_for(&marks.path().join("checking")),
        "the check never ran"
    );
    kill(child.id(), "-TERM");
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.signal(), Some(15), "{out:?}");
    assert!(!marks.path().join("mutated").exists());
}
