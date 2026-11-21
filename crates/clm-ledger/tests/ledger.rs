//! `clm ledger compact`, `verify` and `show`, called through `clm-ledger`'s call
//! (`commands-in-crate`).

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use clm_ledger::{Body, LandKind};
use common::{clone_with_origin, commit, env, repo, sh};

/// `clm ledger <args> --root <root>` through `clm-ledger`'s call, with `HOME` at `home` and no
/// `XDG_CONFIG_HOME` or `CLM_WRITER`, in the lookup and on the process for the length of the
/// call, since the `git` it starts inherits the process's: exit code, stdout, stderr.
fn clm(root: &Path, home: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let ["ledger", args @ ..] = args else {
        panic!("not a `clm ledger` command: {args:?}");
    };
    let mut line: Vec<&str> = args.to_vec();
    line.extend(["--root", root.to_str().unwrap()]);
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = with_vars(&isolated(home), || {
        clm_ledger::ledger_command(line, root, &env(home, &[]), &mut out, &mut err)
    });
    (
        Some(i32::from(code)),
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

/// The variables a call runs under: `HOME` at `home`, and no `XDG_CONFIG_HOME` or
/// `CLM_WRITER`, so it reads no developer's config.
fn isolated(home: &Path) -> [(&'static str, Option<&std::ffi::OsStr>); 3] {
    [
        ("HOME", Some(home.as_os_str())),
        ("XDG_CONFIG_HOME", None),
        ("CLM_WRITER", None),
    ]
}

/// Runs `run` with each of `vars` set, or removed for `None`, on this process. Tests run one
/// per process (`ci/nextest-runner`), so no other test sees them; the old values come back when
/// `run` returns or panics.
fn with_vars<R>(vars: &[(&str, Option<&std::ffi::OsStr>)], run: impl FnOnce() -> R) -> R {
    struct Restore(Vec<(String, Option<std::ffi::OsString>)>);
    impl Drop for Restore {
        fn drop(&mut self) {
            for (name, old) in self.0.drain(..) {
                match old {
                    // SAFETY: one test per process; nothing else reads the environment meanwhile.
                    Some(old) => unsafe { std::env::set_var(&name, old) },
                    // SAFETY: as above.
                    None => unsafe { std::env::remove_var(&name) },
                }
            }
        }
    }
    let _restore = Restore(
        vars.iter()
            .map(|(name, _)| (name.to_string(), std::env::var_os(name)))
            .collect(),
    );
    for (name, value) in vars {
        match value {
            // SAFETY: as above.
            Some(value) => unsafe { std::env::set_var(name, value) },
            // SAFETY: as above.
            None => unsafe { std::env::remove_var(name) },
        }
    }
    run()
}

#[test]
fn scenario_a_command_called_in_a_test() {
    // req:ledger/commands-in-crate@61f9b60a
    // `clm ledger verify` on a `main` with an unrecorded land: its line on the output buffer,
    // nothing on the error buffer, and 1, as the command prints and exits.
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0", "Merge b1"]);
    record_land(dir.path(), &merges[0]);
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!((code, err.as_str()), (Some(1), ""), "{out}");
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(
        out.starts_with("main: ledger/unrecorded-land: ") && out.contains(&merges[1]),
        "{out}"
    );
}

#[test]
fn the_root_option_overrides_the_root_it_is_given() {
    // req:ledger/commands-in-crate@61f9b60a
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0", "Merge b1"]);
    record_land(dir.path(), &merges[0]);
    let elsewhere = tempfile::tempdir().unwrap();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let line = ["verify", "--root", dir.path().to_str().unwrap()];
    let code = clm_ledger::ledger_command(
        line,
        elsewhere.path(),
        &env(dir.path(), &[]),
        &mut out,
        &mut err,
    );
    let out = String::from_utf8(out).unwrap();
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("ledger/unrecorded-land"), "{out}");
    // Without `--root`, the root it is given.
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = clm_ledger::ledger_command(
        ["verify"],
        dir.path(),
        &env(dir.path(), &[]),
        &mut out,
        &mut err,
    );
    assert_eq!(code, 1);
    assert!(
        String::from_utf8(out)
            .unwrap()
            .contains("ledger/unrecorded-land")
    );
}

#[test]
fn scenario_what_it_prints_where() {
    // req:ledger/commands-in-crate@61f9b60a
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let env = env(root, alices(root));
    let first = append(root, approved("f"), &env).unwrap();
    let branch = format!("clm/ledger/{}", first.record.writer);
    append_line(root, &branch, "not json");
    let (code, out, err) = clm(root, root, &["ledger", "show"]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(
        out,
        format!(
            "approved f spec/x {} {}\n",
            first.record.writer, first.record.at
        )
    );
    let warning = format!("warning: {branch} line 2: skipped, not a record: ");
    assert_eq!(err.lines().count(), 1, "{err}");
    assert!(err.starts_with(&warning), "{err}");
    let (code, out, err) = clm(root, root, &["ledger", "verify", "--since", "nowhere"]);
    assert_eq!((code, out.as_str()), (Some(2), ""), "{err}");
    assert!(err.starts_with("clm ledger: "), "{err}");
    assert!(err.contains("nowhere"), "git's message: {err}");
}

#[test]
fn scenario_help() {
    // req:ledger/commands-in-crate@61f9b60a
    for (args, description, usage) in [
        (
            &["--help"][..],
            "The ledger of approvals, checks and lands: print it, audit `main` against it, or \
             prune your own superseded records",
            "Usage: clm ledger <COMMAND>",
        ),
        (
            &["compact", "--help"][..],
            "Rewrite your own ledger without approvals of landed diffs and checks older than \
             the newest land, push it, and print how many records went",
            "Usage: clm ledger compact [OPTIONS]",
        ),
        (
            &["verify", "--help"][..],
            "Report each commit on `main` no `landed` record names, merge or not, and each \
             land no `checked` record covers; exit 1 when there is one. Doesn't fetch",
            "Usage: clm ledger verify [OPTIONS]",
        ),
        (
            &["show", "--help"][..],
            "Print every record, one per line in time order. Doesn't fetch",
            "Usage: clm ledger show [OPTIONS]",
        ),
    ] {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = clm_ledger::ledger_command(
            args.iter().copied(),
            Path::new("."),
            &|_| None,
            &mut out,
            &mut err,
        );
        let out = String::from_utf8(out).unwrap();
        assert_eq!(code, 0, "{args:?}");
        assert!(err.is_empty(), "{args:?}");
        let lines: Vec<&str> = out.lines().take(3).collect();
        assert_eq!(lines, [description, "", usage], "{args:?}:\n{out}");
    }
}

/// The diff's fingerprint against `base` in `dir`, which `clm fingerprint` prints, from the
/// gate's own call.
fn fingerprint(dir: &Path, base: &str) -> String {
    clm_gate::fingerprint(dir, base).unwrap_or_else(|e| panic!("fingerprint: {e}"))
}

fn approved(fingerprint: &str) -> Body {
    Body::Approved {
        fingerprint: fingerprint.into(),
        branch: "spec/x".into(),
        change: Some("x".into()),
    }
}

fn checked(key: &str) -> Body {
    Body::Checked {
        code_key: key.into(),
        command: "check".into(),
        basis: None,
    }
}

/// A code land of `commit`, its code key `key`, or none for a record written before the
/// field.
fn landed(commit: &str, fingerprint: &str, key: Option<&str>) -> Body {
    Body::Landed {
        commit: commit.into(),
        branch: "impl/x".into(),
        kind: LandKind::Code,
        review: false,
        fingerprint: fingerprint.into(),
        code_key: key.map(str::to_string),
        changes: vec![],
        check_seconds: None,
        steps: vec![],
        recheck: None,
        recheck_files: None,
    }
}

fn records(dir: &Path, rev: &str) -> String {
    sh(dir, &["show", &format!("{rev}:records.jsonl")])
}

thread_local! {
    /// The latest `at` of the records this test appended through `append`.
    static LATEST: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// `clm_ledger::append`, noting the record's `at` for `tick`.
fn append(
    root: &Path,
    entry: impl Into<clm_ledger::Entry>,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<clm_ledger::Appended, clm_ledger::Error> {
    let appended = clm_ledger::append(root, entry, env)?;
    LATEST.with_borrow_mut(|latest| {
        if appended.record.at > *latest {
            latest.clone_from(&appended.record.at);
        }
    });
    Ok(appended)
}

/// So the next record's `at` is later than every record's this test appended: waits until the
/// wall clock reads later than the latest, however far the clock stepped back since.
fn tick() {
    let latest = LATEST.with_borrow(Clone::clone);
    let start = std::time::Instant::now();
    while at_now() <= latest {
        assert!(
            start.elapsed() < std::time::Duration::from_secs(30),
            "the clock stayed before {latest}"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

/// The wall clock as a record's `at`, `YYYY-MM-DDTHH:MM:SS.mmmZ`.
fn at_now() -> String {
    let since = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let secs = since.as_secs();
    // Howard Hinnant's `civil_from_days`.
    let z = (secs / 86_400) as i64 + 719_468;
    let (era, doe) = (z.div_euclid(146_097), z.rem_euclid(146_097));
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let rem = secs % 86_400;
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60,
        since.subsec_millis()
    )
}

#[test]
fn compact_prunes_a_landed_approval_and_older_checks() {
    // req:ledger/compact@59996cc0
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let b = clone_with_origin(home, "b", "B");
    let mine = append(&a, approved("f"), &env(home, &[])).unwrap();
    append(&a, checked("k"), &env(home, &[])).unwrap();
    tick();
    let theirs = append(&b, landed("c", "f", Some("m")), &env(home, &[])).unwrap();
    let origin = home.join("origin.git");
    let their_branch = format!("clm/ledger/{}", theirs.record.writer);
    let their_tip = sh(&origin, &["rev-parse", &their_branch]);

    let (code, out, err) = clm(&a, home, &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 2");
    let my_branch = format!("clm/ledger/{}", mine.record.writer);
    assert_eq!(records(&a, &my_branch), "");
    assert_eq!(records(&origin, &my_branch), "", "the result is pushed");
    assert_eq!(sh(&origin, &["rev-parse", &their_branch]), their_tip);
    // Rewritten as a new commit on the tip, never a force push.
    assert_eq!(sh(&origin, &["rev-list", "--count", &my_branch]), "3");
}

#[cfg(unix)]
#[test]
fn compact_warns_when_origin_refuses_the_push() {
    // req:ledger/compact@59996cc0
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let mine = append(&a, approved("f"), &env(home, &[])).unwrap();
    tick();
    append(&a, landed("c", "f", Some("m")), &env(home, &[])).unwrap();
    let origin = home.join("origin.git");
    let hook = origin.join("hooks/pre-receive");
    std::fs::create_dir_all(hook.parent().unwrap()).unwrap();
    std::fs::write(&hook, "#!/bin/sh\nexit 1\n").unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let (code, out, err) = clm(&a, home, &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 1");
    let branch = format!("clm/ledger/{}", mine.record.writer);
    assert!(
        err.contains(&format!("isn't on origin yet: git push origin {branch}")),
        "{err}"
    );
    assert!(!records(&a, &branch).contains("\"approved\""));
}

#[test]
fn compact_keeps_an_open_approval() {
    // req:ledger/compact@59996cc0
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let b = clone_with_origin(home, "b", "B");
    let mine = append(&a, approved("g"), &env(home, &[])).unwrap();
    tick();
    append(&b, landed("c", "other", Some("m")), &env(home, &[])).unwrap();
    let branch = format!("clm/ledger/{}", mine.record.writer);
    let before = sh(&a, &["rev-parse", &branch]);
    let (code, out, err) = clm(&a, home, &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 0");
    assert!(records(&a, &branch).contains("\"fingerprint\":\"g\""));
    assert_eq!(sh(&a, &["rev-parse", &branch]), before, "nothing to commit");
}

#[test]
fn compact_keeps_lands_and_newer_checks() {
    // req:ledger/compact@59996cc0
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let env = env(home, &[]);
    let mine = append(&a, landed("c", "f", Some("m")), &env).unwrap();
    tick();
    append(&a, checked("newer"), &env).unwrap();
    let (code, out, err) = clm(&a, home, &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 0");
    let branch = format!("clm/ledger/{}", mine.record.writer);
    let text = records(&a, &branch);
    for kind in ["landed", "checked"] {
        assert!(
            text.contains(&format!("\"kind\":\"{kind}\"")),
            "{kind}: {text}"
        );
    }
}

/// Commits `line` to the end of `branch`'s `records.jsonl`, as an older `clm` wrote it.
fn append_line(dir: &Path, branch: &str, line: &str) {
    let text = format!("{}\n{line}\n", records(dir, branch).trim_end());
    let file = dir.join("line.tmp");
    std::fs::write(&file, text).unwrap();
    let blob = sh(dir, &["hash-object", "-w", file.to_str().unwrap()]);
    std::fs::remove_file(&file).unwrap();
    let mut mktree = Command::new("git")
        .arg("mktree")
        .current_dir(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::io::Write::write_all(
        &mut mktree.stdin.take().unwrap(),
        format!("100644 blob {blob}\trecords.jsonl\n").as_bytes(),
    )
    .unwrap();
    let tree = String::from_utf8(mktree.wait_with_output().unwrap().stdout).unwrap();
    let tree = tree.trim();
    let commit = sh(dir, &["commit-tree", tree, "-p", branch, "-m", "old line"]);
    sh(
        dir,
        &["update-ref", &format!("refs/heads/{branch}"), &commit],
    );
}

#[test]
fn compact_keeps_a_line_it_cannot_parse() {
    // req:ledger/compact@59996cc0
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let env = env(home, &[]);
    let mine = append(&a, approved("f"), &env).unwrap();
    let branch = format!("clm/ledger/{}", mine.record.writer);
    append_line(&a, &branch, "not json");
    tick();
    append(&a, landed("c", "f", Some("m")), &env).unwrap();
    let (code, out, err) = clm(&a, home, &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 1");
    let text = records(&a, &branch);
    assert!(text.lines().any(|l| l == "not json"), "{text}");
    assert!(!text.contains(r#""kind":"approved""#), "{text}");
}

#[test]
fn compact_drops_reviewed_lines() {
    // req:ledger/compact@59996cc0
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let env = env(home, &[]);
    let mine = append(&a, approved("g"), &env).unwrap();
    let branch = format!("clm/ledger/{}", mine.record.writer);
    let reviewed = r#"{"id":"r1","writer":"x/1","at":"2026-01-01T00:00:00.000Z","kind":"reviewed","fingerprint":"f"}"#;
    append_line(&a, &branch, reviewed);
    let (code, out, err) = clm(&a, home, &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 1");
    let text = records(&a, &branch);
    assert!(!text.contains("reviewed"), "{text}");
    assert!(text.contains(r#""kind":"approved""#), "{text}");
}

#[test]
fn compact_keeps_a_lands_check() {
    // req:ledger/compact@59996cc0
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0"]);
    // `checked k`, then the land naming `k`.
    record_land(dir.path(), &merges[0]);
    let verify = || clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!(verify().1, "");
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 0");
    let fold = clm_ledger::fold(dir.path()).unwrap();
    let key = clm_ledger::code_key(dir.path(), &merges[0]).unwrap();
    assert!(fold.checks.contains_key(&key), "{fold:?}");
    let (code, out, _) = verify();
    assert_eq!((code, out.as_str()), (Some(0), ""));
}

#[test]
fn compact_keeps_an_older_lands_check() {
    // req:ledger/compact@59996cc0
    let dir = tempfile::tempdir().unwrap();
    let merges = history_holding(dir.path(), &[GUIDE_PAGE], &["Merge b0", "Merge b1"]);
    let env = env(dir.path(), &[]);
    let key = legacy_key(dir.path(), &merges[0]);
    append(dir.path(), checked(&key), &env).unwrap();
    // Written before `landed` records named their code key: checked under its legacy key.
    append(dir.path(), landed(&merges[0], "f", None), &env).unwrap();
    tick();
    append(dir.path(), landed(&merges[1], "g", Some("m")), &env).unwrap();
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 0");
    assert!(
        clm_ledger::fold(dir.path())
            .unwrap()
            .checks
            .contains_key(&key)
    );
}

#[test]
fn compact_keeps_every_check_when_a_lands_key_is_unknown() {
    // req:ledger/compact@59996cc0
    // A land without `code_key` whose commit isn't in this clone: its key can't be
    // computed, so no check is dropped, though the landed approval still is.
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0"]);
    let env = env(dir.path(), &[]);
    append(dir.path(), approved("f"), &env).unwrap();
    append(dir.path(), checked("old"), &env).unwrap();
    tick();
    let missing = "0123456789abcdef0123456789abcdef01234567";
    append(dir.path(), landed(missing, "f", None), &env).unwrap();
    append(dir.path(), landed(&merges[0], "g", Some("m")), &env).unwrap();
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 1");
    // This repository has no `origin`, so the push's warning follows.
    let warning = format!(
        "warning: kept every `checked` record: landed {missing} isn't in this clone, so its \
         code key can't be computed; fetch it and compact again"
    );
    assert_eq!(err.lines().next(), Some(warning.as_str()), "{err}");
    let fold = clm_ledger::fold(dir.path()).unwrap();
    assert!(fold.checks.contains_key("old"), "{fold:?}");
    assert!(fold.approvals.is_empty(), "{fold:?}");
}

/// A repository with no remote whose `main` has a merge commit per name, each from its own
/// branch: the merge commits, oldest first.
fn history(dir: &Path, merges: &[&str]) -> Vec<String> {
    history_holding(dir, &[], merges)
}

/// As `history`, with `main`'s first commit also holding `files`.
fn history_holding(dir: &Path, files: &[(&str, &str)], merges: &[&str]) -> Vec<String> {
    repo(dir, "T");
    std::fs::write(dir.join("clm.toml"), "host = \"none\"\n").unwrap();
    commit(dir, files, "files");
    let mut out = Vec::new();
    for (i, message) in merges.iter().enumerate() {
        let branch = format!("b{i}");
        sh(dir, &["switch", "-q", "-c", &branch]);
        commit(dir, &[(&format!("src/{i}.rs"), "x\n")], "work");
        sh(dir, &["switch", "-q", "main"]);
        sh(dir, &["merge", "-q", "--no-ff", "-m", message, &branch]);
        out.push(sh(dir, &["rev-parse", "HEAD"]));
    }
    out
}

/// A guide page, code when the `landed` records without a `code_key` were written, doc-only now.
const GUIDE_PAGE: (&str, &str) = ("docs/guide/intro.md", "intro\n");

/// `commit`'s legacy key (`legacy-code-key`): SHA-256 over `git ls-tree -r -z` without the
/// entries the list as it was named, which kept `docs/guide/` as code. The live key leaves the
/// guide page out, so the two differ.
fn legacy_key(dir: &Path, commit: &str) -> String {
    use sha2::{Digest, Sha256};
    let out = Command::new("git")
        .args(["ls-tree", "-r", "-z", commit])
        .current_dir(dir)
        .output()
        .unwrap();
    let mut hasher = Sha256::new();
    for entry in out.stdout.split_inclusive(|b| *b == 0) {
        let line = String::from_utf8_lossy(entry);
        let path = line.split_once('\t').unwrap().1.trim_end_matches('\0');
        let doc = ["specs/", "changes/", "backlog/", "docs/"]
            .iter()
            .any(|p| path.starts_with(p))
            && !path.starts_with("docs/guide/")
            || ["ARCHITECTURE.md", "README.md", "CLAUDE.md"].contains(&path);
        if !doc {
            hasher.update(entry);
        }
    }
    let key: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_ne!(key, clm_ledger::code_key(dir, commit).unwrap());
    key
}

/// `landed` and `checked` records for `commit`.
fn record_land(dir: &Path, commit: &str) {
    let key = clm_ledger::code_key(dir, commit).unwrap();
    let env = env(dir, &[]);
    append(dir, checked(&key), &env).unwrap();
    tick();
    append(dir, landed(commit, "f", Some(&key)), &env).unwrap();
}

#[test]
fn verify_clean_history() {
    // req:ledger/verify@9fb2cb6b
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0", "Merge b1"]);
    // The first land written before `landed` records named their code key: its key is
    // computed from the commit.
    let env = env(dir.path(), &[]);
    let key = clm_ledger::code_key(dir.path(), &merges[0]).unwrap();
    append(dir.path(), checked(&key), &env).unwrap();
    append(dir.path(), landed(&merges[0], "f", None), &env).unwrap();
    record_land(dir.path(), &merges[1]);
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn verify_reads_the_recorded_key() {
    // req:ledger/verify@9fb2cb6b
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0"]);
    let env = env(dir.path(), &[]);
    append(dir.path(), checked("k"), &env).unwrap();
    append(dir.path(), landed(&merges[0], "f", Some("k")), &env).unwrap();
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn verify_reads_the_legacy_key_of_a_land_recorded_without_one() {
    // req:ledger/verify@9fb2cb6b
    let dir = tempfile::tempdir().unwrap();
    let merges = history_holding(dir.path(), &[GUIDE_PAGE], &["Merge b0"]);
    let vars = env(dir.path(), &[]);
    let legacy = legacy_key(dir.path(), &merges[0]);
    append(dir.path(), checked(&legacy), &vars).unwrap();
    append(dir.path(), landed(&merges[0], "f", None), &vars).unwrap();
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");

    // Only today's key checked: the land is unchecked.
    let dir = tempfile::tempdir().unwrap();
    let merges = history_holding(dir.path(), &[GUIDE_PAGE], &["Merge b0"]);
    let vars = env(dir.path(), &[]);
    let live = clm_ledger::code_key(dir.path(), &merges[0]).unwrap();
    append(dir.path(), checked(&live), &vars).unwrap();
    append(dir.path(), landed(&merges[0], "f", None), &vars).unwrap();
    let (code, out, _) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains("ledger/unchecked-land"), "{out}");
}

#[test]
fn verify_reports_a_push_behind_clms_back() {
    // req:ledger/verify@9fb2cb6b
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0", "Merge b1"]);
    record_land(dir.path(), &merges[0]);
    let (code, out, _) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}");
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(lines.len(), 1, "{out}");
    assert!(lines[0].contains("ledger/unrecorded-land"), "{out}");
    assert!(lines[0].contains(&merges[1]), "{out}");
    // `--since` the unrecorded merge leaves nothing after it.
    let (code, out, _) = clm(
        dir.path(),
        dir.path(),
        &["ledger", "verify", "--since", &merges[1]],
    );
    assert_eq!((code, out.as_str()), (Some(0), ""));
}

#[test]
fn verify_starts_from_the_first_land() {
    // req:ledger/verify@9fb2cb6b
    // Merges from before the ledger existed aren't reported.
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0", "Merge b1"]);
    record_land(dir.path(), &merges[1]);
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn verify_reports_a_direct_push() {
    // req:ledger/verify@9fb2cb6b
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0"]);
    record_land(dir.path(), &merges[0]);
    // Only a subject that starts `Release v` is a release.
    let pushed = commit(
        dir.path(),
        &[("src/direct.rs", "x\n")],
        "Prep Release v1.0.0",
    );
    let (code, out, _) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}");
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(lines.len(), 1, "{out}");
    assert!(
        lines[0].contains("ledger/unrecorded-land") && lines[0].contains(&pushed),
        "{out}"
    );
}

#[test]
fn verify_exempts_releases() {
    // req:ledger/verify@9fb2cb6b
    // A release merge, and a version bump that fast-forwarded onto `main`.
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0", "Release v0.9.0"]);
    record_land(dir.path(), &merges[0]);
    commit(dir.path(), &[("Cargo.toml", "v\n")], "Release v0.10.0");
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn verify_where_the_audit_starts() {
    // req:ledger/verify@9fb2cb6b
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let merges = history(root, &["Merge m0", "Merge m1"]);
    let (m0, m1) = (&merges[0], &merges[1]);
    let vars = env(root, &[]);
    append(root, landed(m1, "f", Some("k")), &vars).unwrap();
    let (code, out, err) = clm(root, root, &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{err}");
    assert_eq!(
        out,
        format!(
            "main: ledger/unchecked-land: landed {m1}: no `checked` record names its code key k\n"
        )
    );
    assert!(!out.contains(m0.as_str()), "{out}");
    append(root, checked("k"), &vars).unwrap();
    let p = commit(root, &[("src/hotfix.rs", "x\n")], "hotfix");
    let (code, out, err) = clm(root, root, &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{err}");
    assert_eq!(
        out,
        format!(
            "main: ledger/unrecorded-land: commit {p} (hotfix) has no `landed` record: it is on \
             main without `clm land`\n"
        )
    );
    // A `main` whose ledger's only `landed` record names a commit not on its chain.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    history(root, &["Merge m0"]);
    let vars = env(root, &[]);
    append(root, checked("k"), &vars).unwrap();
    let elsewhere = "0123456789abcdef0123456789abcdef01234567";
    append(root, landed(elsewhere, "f", Some("k")), &vars).unwrap();
    let (code, out, err) = clm(root, root, &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[cfg(unix)]
#[test]
fn verify_an_audit_that_cannot_run() {
    // req:ledger/verify@9fb2cb6b
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signed_main(root, "Christopher Lunny");
    let land = hard_land(root, &s.merge, &s.fingerprint);
    append_signed(root, "christopher-lunny", &s.christopher, "clm-land", land);
    let git = common::git_alone();
    let mut vars = isolated(root).to_vec();
    vars.push(("PATH", Some(git.path().as_os_str())));
    let found = with_vars(&vars, || clm_ledger::on_main(root));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].path, Path::new("main"), "{found:?}");
    assert_eq!(found[0].rule, "ledger/verify", "{found:?}");
    assert!(
        found[0]
            .message
            .starts_with("can't audit main against the ledger: "),
        "{found:?}"
    );
}

#[test]
fn verify_findings_offered_to_the_check_on_main_only() {
    // req:ledger/verify@9fb2cb6b
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0", "Merge b1"]);
    let vars = isolated(dir.path());
    // On `main` with no ledger branch, it offers none.
    let found = with_vars(&vars, || clm_ledger::on_main(dir.path()));
    assert!(found.is_empty(), "{found:?}");
    record_land(dir.path(), &merges[0]);
    let found = with_vars(&vars, || clm_ledger::on_main(dir.path()));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].path, Path::new("main"), "{found:?}");
    assert_eq!(found[0].rule, "ledger/unrecorded-land", "{found:?}");
    assert!(found[0].message.contains(&merges[1]), "{found:?}");
    // With another branch checked out, it offers none.
    sh(dir.path(), &["switch", "-q", "-c", "other"]);
    let found = with_vars(&vars, || clm_ledger::on_main(dir.path()));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn verify_reports_a_land_no_check_names() {
    // req:ledger/verify@9fb2cb6b
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0", "Merge b1"]);
    record_land(dir.path(), &merges[0]);
    let env = env(dir.path(), &[]);
    // The commit's computed key is checked, but the land's own key `k` isn't.
    let computed = clm_ledger::code_key(dir.path(), &merges[1]).unwrap();
    append(dir.path(), checked(&computed), &env).unwrap();
    append(dir.path(), landed(&merges[1], "f", Some("k")), &env).unwrap();
    let (code, out, _) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}");
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(
        out.contains("ledger/unchecked-land") && out.contains(&merges[1]),
        "{out}"
    );
}

#[test]
fn show_lists_the_fold_in_time_order() {
    // req:ledger/show@6e2fd27f
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), "T");
    std::fs::write(dir.path().join("clm.toml"), "host = \"none\"\n").unwrap();
    let env = env(dir.path(), &[]);
    let a = append(dir.path(), approved("f"), &env).unwrap();
    tick();
    let l = append(dir.path(), landed("c", "f", None), &env).unwrap();
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "show"]);
    assert_eq!(code, Some(0), "{err}");
    let w = &a.record.writer;
    assert_eq!(
        out,
        format!(
            "approved f spec/x {w} {}\nlanded c impl/x code - {w} {}\n",
            a.record.at, l.record.at
        )
    );
}

#[test]
fn show_prints_whether_a_land_needed_review() {
    // req:ledger/show@6e2fd27f
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), "T");
    std::fs::write(dir.path().join("clm.toml"), "host = \"none\"\n").unwrap();
    let env = env(dir.path(), &[]);
    let mut body = landed("c", "f", None);
    if let Body::Landed {
        kind,
        review,
        branch,
        ..
    } = &mut body
    {
        (*kind, *review, *branch) = (LandKind::Spec, true, "spec/x".into());
    }
    let l = append(dir.path(), body, &env).unwrap();
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "show"]);
    assert_eq!(code, Some(0), "{err}");
    let w = &l.record.writer;
    assert_eq!(
        out,
        format!("landed c spec/x spec review {w} {}\n", l.record.at)
    );
}

#[test]
fn show_sorts_across_ledgers() {
    // req:ledger/show@6e2fd27f
    // Writer `b`'s record is older, though its ledger is read after `a`'s.
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let b = clone_with_origin(home, "b", "B");
    append(&b, approved("old"), &env(home, &[])).unwrap();
    tick();
    append(&a, approved("new"), &env(home, &[])).unwrap();
    sh(&a, &["fetch", "-q", "origin"]);
    let (code, out, err) = clm(&a, home, &["ledger", "show"]);
    assert_eq!(code, Some(0), "{err}");
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{out}");
    assert!(
        lines[0].starts_with("approved old ") && lines[1].starts_with("approved new "),
        "{out}"
    );
}

/// A fresh ed25519 key at `dir/<name>`: its path and its public half.
fn keygen(dir: &Path, name: &str) -> (PathBuf, String) {
    let key = dir.join(name);
    let ok = Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-C", name, "-f"])
        .arg(&key)
        .status()
        .unwrap()
        .success();
    assert!(ok, "ssh-keygen -t ed25519");
    let public = std::fs::read_to_string(key.with_extension("pub")).unwrap();
    (key, public.trim().to_string())
}

/// A repository with no remote whose `main` holds `approval = "other"` and
/// `docs/allowed_signers` listing `christopher-lunny` and `dana`, then the merge of a branch
/// authored by `author`.
struct SignedMain {
    _keys: tempfile::TempDir,
    christopher: PathBuf,
    dana: PathBuf,
    merge: String,
    /// The branch's diff's fingerprint, as `clm land` computed it.
    fingerprint: String,
}

fn signed_main(dir: &Path, author: &str) -> SignedMain {
    repo(dir, "T");
    let keys = tempfile::tempdir().unwrap();
    let (christopher, christopher_pub) = keygen(keys.path(), "christopher");
    let (dana, dana_pub) = keygen(keys.path(), "dana");
    let signers = format!("christopher-lunny {christopher_pub}\ndana {dana_pub}\n");
    commit(
        dir,
        &[
            ("clm.toml", "host = \"none\"\napproval = \"other\"\n"),
            ("docs/allowed_signers", &signers),
        ],
        "signers",
    );
    let (merge, fingerprint) = merge_branch(dir, "b", author);
    SignedMain {
        _keys: keys,
        christopher,
        dana,
        merge,
        fingerprint,
    }
}

/// Branch `name` off `main` with one commit authored by `author`, merged into `main`: the
/// merge commit and the branch's fingerprint against `main`.
fn merge_branch(dir: &Path, name: &str, author: &str) -> (String, String) {
    sh(dir, &["switch", "-q", "-c", name]);
    sh(dir, &["config", "user.name", author]);
    commit(dir, &[(&format!("specs/{name}.md"), "x\n")], "work");
    sh(dir, &["config", "user.name", "T"]);
    let out = fingerprint(dir, "main");
    sh(dir, &["switch", "-q", "main"]);
    sh(
        dir,
        &[
            "merge",
            "-q",
            "--no-ff",
            "-m",
            &format!("Merge {name}"),
            name,
        ],
    );
    (sh(dir, &["rev-parse", "HEAD"]), out.trim().to_string())
}

/// `body` signed under `namespace` by writer `person` with `key`, and appended.
fn append_signed(dir: &Path, person: &str, key: &Path, namespace: &str, body: Body) {
    sh(dir, &["config", "gpg.format", "ssh"]);
    sh(dir, &["config", "user.signingkey", key.to_str().unwrap()]);
    let global = dir.join(".git/no-gitconfig");
    let vars = [
        ("CLM_WRITER", person),
        ("GIT_CONFIG_GLOBAL", global.to_str().unwrap()),
    ];
    let env = env(dir, &vars);
    let signed = clm_ledger::sign(dir, body, namespace, &env).unwrap();
    append(dir, signed, &env).unwrap();
}

/// A spec land of `commit` that needed review, naming `fingerprint`, its code key checked.
fn hard_land(dir: &Path, commit: &str, fingerprint: &str) -> Body {
    let key = clm_ledger::code_key(dir, commit).unwrap();
    append(dir, checked(&key), &env(dir, &[])).unwrap();
    Body::Landed {
        commit: commit.into(),
        branch: "spec/x".into(),
        kind: LandKind::Spec,
        review: true,
        fingerprint: fingerprint.into(),
        code_key: Some(key),
        changes: vec![],
        check_seconds: None,
        steps: vec![],
        recheck: None,
        recheck_files: None,
    }
}

#[test]
fn verify_signed_and_approved_by_another() {
    // req:ledger/verify-signed-lands@68f42380
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signed_main(root, "Christopher Lunny");
    let land = hard_land(root, &s.merge, &s.fingerprint);
    append_signed(root, "christopher-lunny", &s.christopher, "clm-land", land);
    append_signed(
        root,
        "dana",
        &s.dana,
        "clm-approve",
        approved(&s.fingerprint),
    );
    let (code, out, err) = clm(root, root, &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn verify_judges_a_land_by_any_signed_record() {
    // req:ledger/verify-signed-lands@68f42380
    // A retried land left an unsigned record beside the signed one.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signed_main(root, "Christopher Lunny");
    let land = hard_land(root, &s.merge, &s.fingerprint);
    append(root, land.clone(), &env(root, &[])).unwrap();
    append_signed(root, "christopher-lunny", &s.christopher, "clm-land", land);
    append_signed(
        root,
        "dana",
        &s.dana,
        "clm-approve",
        approved(&s.fingerprint),
    );
    let (code, out, err) = clm(root, root, &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn verify_approval_in_anothers_name() {
    // req:ledger/verify-signed-lands@68f42380
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signed_main(root, "Christopher Lunny");
    let land = hard_land(root, &s.merge, &s.fingerprint);
    append_signed(root, "christopher-lunny", &s.christopher, "clm-land", land);
    // Named `dana`, signed with `christopher-lunny`'s key.
    append_signed(
        root,
        "dana",
        &s.christopher,
        "clm-approve",
        approved(&s.fingerprint),
    );
    let (code, out, _) = clm(root, root, &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}");
    assert_eq!(
        out,
        format!(
            "main: ledger/unapproved-land: landed {}: it needed review, but no `approved` \
             record for its fingerprint {} is signed by its writer and by someone other than \
             the lander and the authors\n",
            s.merge, s.fingerprint
        )
    );
}

#[test]
fn verify_author_name_changed() {
    // req:ledger/verify-signed-lands@68f42380
    // The branch's commits name `Zed`, but the one approval is the lander's own.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signed_main(root, "Zed");
    let land = hard_land(root, &s.merge, &s.fingerprint);
    append_signed(root, "christopher-lunny", &s.christopher, "clm-land", land);
    let approval = approved(&s.fingerprint);
    append_signed(
        root,
        "christopher-lunny",
        &s.christopher,
        "clm-approve",
        approval,
    );
    let (code, out, _) = clm(root, root, &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}");
    assert!(
        out.contains("ledger/unapproved-land") && out.contains(&s.merge),
        "{out}"
    );
}

#[test]
fn verify_an_authors_approval_counts_only_under_any() {
    // req:ledger/verify-signed-lands@68f42380
    // Another person lands, but the approval is by the branch's author.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signed_main(root, "Dana");
    let land = hard_land(root, &s.merge, &s.fingerprint);
    append_signed(root, "christopher-lunny", &s.christopher, "clm-land", land);
    append_signed(
        root,
        "dana",
        &s.dana,
        "clm-approve",
        approved(&s.fingerprint),
    );
    let (code, out, _) = clm(root, root, &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}");
    assert!(
        out.contains("ledger/unapproved-land") && out.contains(&s.merge),
        "{out}"
    );
    // Under `approval = "any"`, it counts.
    std::fs::write(root.join("clm.toml"), "host = \"none\"\n").unwrap();
    let tip = commit(root, &[], "any approval");
    let (merge, fingerprint) = merge_branch(root, "c", "Dana");
    let land = hard_land(root, &merge, &fingerprint);
    append_signed(root, "christopher-lunny", &s.christopher, "clm-land", land);
    append_signed(root, "dana", &s.dana, "clm-approve", approved(&fingerprint));
    let (_, out, _) = clm(root, root, &["ledger", "verify", "--since", &tip]);
    assert_eq!(out, "");
}

#[test]
fn verify_unsigned_land() {
    // req:ledger/verify-signed-lands@68f42380
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signed_main(root, "Christopher Lunny");
    let land = hard_land(root, &s.merge, &s.fingerprint);
    append(root, land, &env(root, &[])).unwrap();
    append_signed(
        root,
        "dana",
        &s.dana,
        "clm-approve",
        approved(&s.fingerprint),
    );
    let (code, out, _) = clm(root, root, &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}");
    assert_eq!(
        out,
        format!(
            "main: ledger/unsigned-land: landed {}: no `landed` record naming it is signed by \
             its writer\n",
            s.merge
        )
    );
}

#[test]
fn verify_approval_for_another_diff() {
    // req:ledger/verify-signed-lands@68f42380
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signed_main(root, "Christopher Lunny");
    let other = "0".repeat(64);
    let land = hard_land(root, &s.merge, &other);
    append_signed(root, "christopher-lunny", &s.christopher, "clm-land", land);
    append_signed(root, "dana", &s.dana, "clm-approve", approved(&other));
    let (code, out, _) = clm(root, root, &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}");
    assert_eq!(
        out,
        format!(
            "main: ledger/unapproved-land: landed {}: its `landed` record names fingerprint \
             {other}, but its parents give {}, so any approval was for another diff\n",
            s.merge, s.fingerprint
        )
    );
}

/// `signed_main`'s merge landed, signed and approved by another.
fn land_signed_main(root: &Path, s: &SignedMain) {
    let land = hard_land(root, &s.merge, &s.fingerprint);
    append_signed(root, "christopher-lunny", &s.christopher, "clm-land", land);
    append_signed(
        root,
        "dana",
        &s.dana,
        "clm-approve",
        approved(&s.fingerprint),
    );
}

/// A land of `commit` of `kind` that needed review, as `hard_land` gives it.
fn hard_land_of(root: &Path, commit: &str, fingerprint: &str, kind: LandKind) -> Body {
    let mut land = hard_land(root, commit, fingerprint);
    if let Body::Landed { kind: k, .. } = &mut land {
        *k = kind;
    }
    land
}

#[test]
fn verify_lands_no_approval_can_clear() {
    // req:ledger/verify-signed-lands@68f42380
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signed_main(root, "Christopher Lunny");
    land_signed_main(root, &s);
    // `p`: one parent.
    let p = commit(root, &[("specs/p.md", "p\n")], "p");
    let land = hard_land(root, &p, "f");
    append_signed(root, "christopher-lunny", &s.christopher, "clm-land", land);
    // `u`: a branch that shares no history with `main`.
    sh(root, &["switch", "-q", "--orphan", "u"]);
    commit(root, &[("u.txt", "u\n")], "unrelated");
    sh(root, &["switch", "-q", "main"]);
    let args = ["merge", "-q", "--no-ff", "--allow-unrelated-histories"];
    sh(root, &[&args[..], &["-m", "Merge u", "u"]].concat());
    let u = sh(root, &["rev-parse", "HEAD"]);
    let land = hard_land(root, &u, "f");
    append_signed(root, "christopher-lunny", &s.christopher, "clm-land", land);
    // `c`: a `code` land whose record says `review`, with no approval.
    let (c, c_diff) = merge_branch(root, "c", "Christopher Lunny");
    let land = hard_land_of(root, &c, &c_diff, LandKind::Code);
    append_signed(root, "christopher-lunny", &s.christopher, "clm-land", land);
    let (code, out, err) = clm(root, root, &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}{err}");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3, "{out}");
    assert_eq!(
        lines[0],
        format!(
            "main: ledger/unapproved-land: landed {p}: it needed review, but isn't a merge, so \
             no diff was approved"
        )
    );
    assert_eq!(
        lines[1],
        format!(
            "main: ledger/unapproved-land: landed {u}: its parents share no history, so no diff \
             was approved"
        )
    );
    // Audited for its approval, as a land that needed review.
    let c_line = format!(
        "main: ledger/unapproved-land: landed {c}: it needed review, but no `approved` record \
         for its fingerprint {c_diff} is signed by its writer"
    );
    assert!(lines[2].starts_with(&c_line), "{out}");

    // A `clm.toml` that doesn't parse in `c`'s first parent, and a counted approval of `c`.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signed_main(root, "Christopher Lunny");
    land_signed_main(root, &s);
    let broken = commit(root, &[("clm.toml", "not [toml\n")], "broken settings");
    // The branch mends the file, so the clone's own settings read again once it merges.
    sh(root, &["switch", "-q", "-c", "c"]);
    let settings = "host = \"none\"\napproval = \"other\"\n";
    commit(
        root,
        &[("clm.toml", settings), ("specs/c.md", "c\n")],
        "work",
    );
    let diff = fingerprint(root, "main");
    sh(root, &["switch", "-q", "main"]);
    sh(root, &["merge", "-q", "--no-ff", "-m", "Merge c", "c"]);
    let c = sh(root, &["rev-parse", "HEAD"]);
    let land = hard_land_of(root, &c, &diff, LandKind::Code);
    append_signed(root, "christopher-lunny", &s.christopher, "clm-land", land);
    append_signed(root, "dana", &s.dana, "clm-approve", approved(&diff));
    let (code, _, err) = clm(root, root, &["ledger", "verify"]);
    assert_eq!(code, Some(2), "{err}");
    let start = format!("clm ledger: {broken}:clm.toml: ");
    assert!(
        err.starts_with(&start) && err.len() > start.len() + 1,
        "{err}"
    );
}

#[test]
fn compact_keeps_a_signed_lands_approval() {
    // req:ledger/compact@59996cc0
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signed_main(root, "Christopher Lunny");
    let land = hard_land(root, &s.merge, &s.fingerprint);
    append_signed(root, "christopher-lunny", &s.christopher, "clm-land", land);
    append_signed(
        root,
        "dana",
        &s.dana,
        "clm-approve",
        approved(&s.fingerprint),
    );
    // `dana`'s approval of a diff that a land before the switch names still goes.
    let before = sh(root, &["rev-parse", &format!("{}^1", s.merge)]);
    let env = env(root, &[("CLM_WRITER", "dana")]);
    append(root, approved("old"), &env).unwrap();
    tick();
    let old = hard_land(root, &before, "old");
    append(root, old, &env).unwrap();
    // Compacting as `dana`.
    sh(root, &["config", "user.name", "Dana"]);
    let (code, out, err) = clm(root, root, &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 1");
    let fold = clm_ledger::fold(root).unwrap();
    assert!(fold.approvals.contains_key(&s.fingerprint), "{fold:?}");
    assert!(!fold.approvals.contains_key("old"), "{fold:?}");
    let (code, out, err) = clm(root, root, &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn compact_keeps_the_approval_of_a_land_that_needed_review_not_fetched() {
    // req:ledger/compact@59996cc0
    // Whether its first parent is signed can't be told, so the approval stays.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    history(root, &["Merge b0"]);
    let env = env(root, &[]);
    append(root, approved("f"), &env).unwrap();
    tick();
    let missing = "0123456789abcdef0123456789abcdef01234567";
    let land = Body::Landed {
        commit: missing.into(),
        branch: "spec/x".into(),
        kind: LandKind::Spec,
        review: true,
        fingerprint: "f".into(),
        code_key: Some("m".into()),
        changes: vec![],
        check_seconds: None,
        steps: vec![],
        recheck: None,
        recheck_files: None,
    };
    append(root, land, &env).unwrap();
    let (code, out, err) = clm(root, root, &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 0");
    assert_eq!(
        err,
        format!(
            "warning: kept the approval of f: landed {missing} isn't in this clone, so \
             whether `clm ledger verify` reads it can't be told; fetch it and compact again\n"
        )
    );
    assert!(clm_ledger::fold(root).unwrap().approvals.contains_key("f"));
}

#[test]
fn verify_before_the_switch() {
    // req:ledger/verify-signed-lands@68f42380
    // Unsigned lands that needed review, the last of which adds `docs/allowed_signers`.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let merges = history(root, &["Merge b0"]);
    let keys = tempfile::tempdir().unwrap();
    let (_, public) = keygen(keys.path(), "dana");
    sh(root, &["switch", "-q", "-c", "signers"]);
    let signers = format!("dana {public}\n");
    commit(root, &[("docs/allowed_signers", &signers)], "signers");
    sh(root, &["switch", "-q", "main"]);
    sh(
        root,
        &["merge", "-q", "--no-ff", "-m", "Merge signers", "signers"],
    );
    let switch = sh(root, &["rev-parse", "HEAD"]);
    let env = env(root, &[]);
    for (commit, fingerprint) in [(&merges[0], "f0"), (&switch, "f1")] {
        let land = hard_land(root, commit, fingerprint);
        append(root, approved(fingerprint), &env).unwrap();
        append(root, land, &env).unwrap();
    }
    let (code, out, err) = clm(root, root, &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

fn subscribed(endpoint: &str) -> Body {
    Body::Subscribed {
        endpoint: endpoint.into(),
        p256dh: "P256DH-SECRET".into(),
        auth: "AUTH-SECRET".into(),
    }
}

#[test]
fn show_prints_a_subscription_without_its_keys() {
    // req:ledger/subscription-records@94d71184
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    repo(root, "T");
    std::fs::write(root.join("clm.toml"), "host = \"none\"\n").unwrap();
    let env = env(root, &[("CLM_WRITER", "alice")]);
    let s = append(root, subscribed("https://push.example/s/2"), &env).unwrap();
    tick();
    let end = Body::Unsubscribed {
        subscription: s.record.id.clone(),
    };
    let u = append(root, end, &env).unwrap();
    let (code, out, err) = clm(root, root, &["ledger", "show"]);
    assert_eq!(code, Some(0), "{err}");
    let (id, w) = (&s.record.id, &s.record.writer);
    assert!(w.starts_with("alice/"), "{w}");
    assert_eq!(
        out,
        format!(
            "subscribed {id} https://push.example {w} {}\nunsubscribed {id} {w} {}\n",
            s.record.at, u.record.at
        )
    );
    assert!(
        !out.contains("SECRET") && !err.contains("SECRET"),
        "{out}{err}"
    );
}

#[test]
fn compact_keeps_subscription_records() {
    // req:ledger/subscription-records@94d71184
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let env = env(home, &[]);
    let s = append(&a, subscribed("https://push.example/s/1"), &env).unwrap();
    let end = Body::Unsubscribed {
        subscription: s.record.id.clone(),
    };
    append(&a, end, &env).unwrap();
    append(&a, approved("f"), &env).unwrap();
    tick();
    append(&a, landed("c", "f", Some("m")), &env).unwrap();
    let (code, out, err) = clm(&a, home, &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 1");
    let text = records(&a, &format!("clm/ledger/{}", s.record.writer));
    assert!(text.contains(r#""kind":"subscribed""#), "{text}");
    assert!(text.contains(r#""kind":"unsubscribed""#), "{text}");
    assert!(!text.contains(r#""kind":"approved""#), "{text}");
}

// `ledger/verify-recomputes-review`: a spec graph the gate can rate.

const ROOT_SPEC: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n\n## Contract\nr\n";

/// Node `a`'s spec with Contract `contract` and requirements `reqs`.
fn node_a(contract: &str, reqs: &[&str]) -> String {
    let mut s = format!(
        "---\ntitle: a\nstatus: specd\nowners: []\n---\n\n## Contract\n{contract}\n\n## Design\nd\n\n## Requirements\n"
    );
    for slug in reqs {
        s.push_str(&format!(
            "\n### Requirement: {slug}\nIt SHALL {slug}.\n\n#### Scenario: s\n- ok\n"
        ));
    }
    s
}

/// A repository with no remote whose `main` holds the root, node `a`, some code and, when
/// `signers`, `docs/allowed_signers` listing `christopher-lunny`: the keys' directory and
/// `christopher-lunny`'s key.
fn signed_main_with_specs(dir: &Path, signers: bool) -> (tempfile::TempDir, PathBuf) {
    repo(dir, "T");
    let keys = tempfile::tempdir().unwrap();
    let (christopher, christopher_pub) = keygen(keys.path(), "christopher");
    let a = node_a("c", &["r"]);
    let allowed = format!("christopher-lunny {christopher_pub}\n");
    let mut files = vec![
        ("clm.toml", "host = \"none\"\n"),
        ("specs/SPEC.md", ROOT_SPEC),
        ("specs/a/SPEC.md", a.as_str()),
        ("src/x.rs", "fn x() {}\n"),
    ];
    if signers {
        files.push(("docs/allowed_signers", &allowed));
    }
    commit(dir, &files, "base");
    (keys, christopher)
}

/// Branch `name` off `main` writing `files`, merged into `main`: the merge commit and the
/// branch's fingerprint against `main`.
fn merge_files(dir: &Path, name: &str, files: &[(&str, &str)]) -> (String, String) {
    merge_files_ending(dir, name, files, "")
}

/// `merge_files` with `ending` appended to the merge commit's message, as a paragraph of its
/// own when it isn't empty.
fn merge_files_ending(
    dir: &Path,
    name: &str,
    files: &[(&str, &str)],
    ending: &str,
) -> (String, String) {
    sh(dir, &["switch", "-q", "-c", name]);
    commit(dir, files, "work");
    let out = fingerprint(dir, "main");
    sh(dir, &["switch", "-q", "main"]);
    let mut message = format!("Merge {name}");
    if !ending.is_empty() {
        message.push_str(&format!("\n\n{ending}"));
    }
    sh(dir, &["merge", "-q", "--no-ff", "-m", &message, name]);
    (sh(dir, &["rev-parse", "HEAD"]), out.trim().to_string())
}

/// A land of `commit` recorded as `kind` and `review`, its code key checked.
fn land_as(dir: &Path, commit: &str, fingerprint: &str, kind: LandKind, review: bool) -> Body {
    let key = clm_ledger::code_key(dir, commit).unwrap();
    append(dir, checked(&key), &env(dir, &[])).unwrap();
    Body::Landed {
        commit: commit.into(),
        branch: "x".into(),
        kind,
        review,
        fingerprint: fingerprint.into(),
        code_key: Some(key),
        changes: vec![],
        check_seconds: None,
        steps: vec![],
        recheck: None,
        recheck_files: None,
    }
}

/// The merge of a branch that only edits `ARCHITECTURE.md`, which needs review.
fn architecture_change(dir: &Path) -> (String, String) {
    merge_files(dir, "architecture", &[("ARCHITECTURE.md", "invariants\n")])
}

/// The merge of a branch that only adds a requirement to `a`.
fn requirement_added(dir: &Path) -> (String, String) {
    merge_files(
        dir,
        "req",
        &[("specs/a/SPEC.md", &node_a("c", &["r", "t"]))],
    )
}

#[test]
fn review_recorded_as_none() {
    // req:ledger/verify-recomputes-review@730d0917
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_keys, christopher) = signed_main_with_specs(root, true);
    let (merge, fingerprint) = architecture_change(root);
    let land = land_as(root, &merge, &fingerprint, LandKind::Spec, false);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    let (code, out, err) = clm(root, root, &["ledger", "verify", "--review"]);
    assert_eq!(code, Some(1), "{out}{err}");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{out}");
    assert_eq!(
        lines[0],
        format!(
            "main: ledger/understated-land: landed {merge}: recorded spec, the gate gives spec \
             review"
        )
    );
    assert!(
        lines[1].contains("ledger/unapproved-land") && lines[1].contains(&merge),
        "{out}"
    );
}

#[test]
fn review_spec_recorded_as_code() {
    // req:ledger/verify-recomputes-review@730d0917
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_keys, christopher) = signed_main_with_specs(root, true);
    let (merge, fingerprint) = architecture_change(root);
    let land = land_as(root, &merge, &fingerprint, LandKind::Code, false);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    append_signed(
        root,
        "christopher-lunny",
        &christopher,
        "clm-approve",
        approved(&fingerprint),
    );
    let (code, out, err) = clm(root, root, &["ledger", "verify", "--review"]);
    assert_eq!(code, Some(1), "{out}{err}");
    assert_eq!(
        out,
        format!(
            "main: ledger/understated-land: landed {merge}: recorded code, the gate gives spec \
             review\n"
        )
    );
}

#[test]
fn review_honest_and_over_stated_lands() {
    // req:ledger/verify-recomputes-review@730d0917
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_keys, christopher) = signed_main_with_specs(root, true);
    let (soft, soft_fp) = requirement_added(root);
    let land = land_as(root, &soft, &soft_fp, LandKind::Spec, false);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    let (code_land, code_fp) = merge_files(root, "code", &[("src/x.rs", "fn x() { 1; }\n")]);
    let land = land_as(root, &code_land, &code_fp, LandKind::Code, false);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    let (code, out, err) = clm(root, root, &["ledger", "verify", "--review"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");

    // Over-stated: a diff that needs no review recorded as needing it, and approved.
    let (hard, hard_fp) = merge_files(
        root,
        "req2",
        &[("specs/a/SPEC.md", &node_a("c", &["r", "t", "u"]))],
    );
    let land = land_as(root, &hard, &hard_fp, LandKind::Spec, true);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    append_signed(
        root,
        "christopher-lunny",
        &christopher,
        "clm-approve",
        approved(&hard_fp),
    );
    let (code, out, err) = clm(root, root, &["ledger", "verify", "--review"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn review_highest_signed_record_is_the_recorded_one() {
    // req:ledger/verify-recomputes-review@730d0917
    // Two signed records name the land, `code` and `spec review`: the higher one is recorded.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_keys, christopher) = signed_main_with_specs(root, true);
    let (merge, fingerprint) = architecture_change(root);
    let low = land_as(root, &merge, &fingerprint, LandKind::Code, false);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", low);
    let high = land_as(root, &merge, &fingerprint, LandKind::Spec, true);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", high);
    append_signed(
        root,
        "christopher-lunny",
        &christopher,
        "clm-approve",
        approved(&fingerprint),
    );
    let (code, out, err) = clm(root, root, &["ledger", "verify", "--review"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn review_not_recomputed_without_the_flag() {
    // req:ledger/verify-recomputes-review@730d0917
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_keys, christopher) = signed_main_with_specs(root, true);
    let (merge, fingerprint) = architecture_change(root);
    let land = land_as(root, &merge, &fingerprint, LandKind::Spec, false);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    let (code, out, err) = clm(root, root, &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn review_before_the_switch() {
    // req:ledger/verify-recomputes-review@730d0917
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let _keys = signed_main_with_specs(root, false);
    let (merge, fingerprint) = architecture_change(root);
    let land = land_as(root, &merge, &fingerprint, LandKind::Spec, false);
    append(root, land, &env(root, &[])).unwrap();
    let (code, out, err) = clm(root, root, &["ledger", "verify", "--review"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn review_ranks_code_with_spec() {
    // req:ledger/verify-recomputes-review@730d0917
    // A backlog-only diff, `spec` to the gate, recorded as a code land: no finding.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_keys, christopher) = signed_main_with_specs(root, true);
    let item = "---\ntitle: X\nkind: idea\n---\n";
    let (merge, fingerprint) = merge_files(root, "item", &[("backlog/x.md", item)]);
    let land = land_as(root, &merge, &fingerprint, LandKind::Code, false);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    let (code, out, err) = clm(root, root, &["ledger", "verify", "--review"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn review_unsigned_land_is_not_recomputed() {
    // req:ledger/verify-recomputes-review@730d0917
    // On a signed `main`, an unsigned record understating a diff that needs review is only
    // unsigned.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let _keys = signed_main_with_specs(root, true);
    let (merge, fingerprint) = architecture_change(root);
    let land = land_as(root, &merge, &fingerprint, LandKind::Spec, false);
    append(root, land, &env(root, &[])).unwrap();
    let (code, out, err) = clm(root, root, &["ledger", "verify", "--review"]);
    assert_eq!(code, Some(1), "{out}{err}");
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(
        out.contains("ledger/unsigned-land") && out.contains(&merge),
        "{out}"
    );
}

/// The merge of a branch that only removes requirement `r` from `a`, its message ending with
/// `ending`: under the rules of version 1 a spec land that needs no review, under version 2
/// one that does, since it cuts a requirement.
fn requirement_removed(dir: &Path, ending: &str) -> (String, String) {
    merge_files_ending(
        dir,
        "cut",
        &[("specs/a/SPEC.md", &node_a("c", &[]))],
        ending,
    )
}

/// `clm ledger verify <args>`: exit code, each line's rule and the commit it names, and the
/// whole output.
fn findings(root: &Path, args: &[&str]) -> (Option<i32>, Vec<(String, String)>, String) {
    let mut line = vec!["ledger", "verify"];
    line.extend(args);
    let (code, out, err) = clm(root, root, &line);
    let found = out
        .lines()
        .map(|l| {
            let rule = l.split(": ").nth(1).unwrap_or(l).to_string();
            let commit = l
                .split_whitespace()
                .map(|w| w.trim_end_matches(':'))
                .find(|w| w.len() == 40 && w.chars().all(|c| c.is_ascii_hexdigit()))
                .unwrap_or("")
                .to_string();
            (rule, commit)
        })
        .collect();
    (code, found, format!("{out}{err}"))
}

/// The pairs `findings` gives, from `(rule, commit)`s.
fn found(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(rule, commit)| (rule.to_string(), commit.to_string()))
        .collect()
}

#[test]
fn review_a_removal_before_the_rule() {
    // req:ledger/verify-recomputes-review@730d0917
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_keys, christopher) = signed_main_with_specs(root, true);
    let (merge, fingerprint) = requirement_removed(root, "");
    let land = land_as(root, &merge, &fingerprint, LandKind::Spec, false);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    let (code, out, err) = clm(root, root, &["ledger", "verify", "--review"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn review_a_removal_under_the_rule() {
    // req:ledger/verify-recomputes-review@730d0917
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_keys, christopher) = signed_main_with_specs(root, true);
    let (merge, fingerprint) = requirement_removed(root, "Gate-Rules: 2");
    let land = land_as(root, &merge, &fingerprint, LandKind::Spec, false);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    let (code, out, err) = clm(root, root, &["ledger", "verify", "--review"]);
    assert_eq!(code, Some(1), "{out}{err}");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{out}");
    assert!(
        lines[0].contains("ledger/understated-land")
            && lines[0].contains(&merge)
            && lines[0].contains("recorded spec, the gate gives spec review"),
        "{out}"
    );
    assert!(
        lines[1].contains("ledger/unapproved-land") && lines[1].contains(&merge),
        "{out}"
    );
}

#[test]
fn review_the_highest_whole_trailer_counts() {
    // req:ledger/verify-recomputes-review@730d0917
    // Of these trailers only `  1 ` is, trimmed, a whole number of at least 1: version 1.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_keys, christopher) = signed_main_with_specs(root, true);
    let ending = "Gate-Rules: 2x\nGate-Rules: 0\nGate-Rules: +2\nGate-Rules:  1 ";
    let (merge, fingerprint) = requirement_removed(root, ending);
    let land = land_as(root, &merge, &fingerprint, LandKind::Spec, false);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    let (code, out, err) = clm(root, root, &["ledger", "verify", "--review"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");

    // Of two whole trailers, the higher counts, whatever their order.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_keys, christopher) = signed_main_with_specs(root, true);
    let (merge, fingerprint) = requirement_removed(root, "Gate-Rules: 1\nGate-Rules: 2");
    let land = land_as(root, &merge, &fingerprint, LandKind::Spec, false);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    let (code, findings, out) = findings(root, &["--review"]);
    assert_eq!(code, Some(1), "{out}");
    let want = [
        ("ledger/understated-land", merge.as_str()),
        ("ledger/unapproved-land", &merge),
    ];
    assert_eq!(findings, found(&want), "{out}");
}

#[test]
fn review_a_lower_trailer_after_a_higher_one() {
    // req:ledger/verify-recomputes-review@730d0917
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_keys, christopher) = signed_main_with_specs(root, true);
    let files = [("src/x.rs", "fn x() { 1; }\n")];
    let (first, first_fp) = merge_files_ending(root, "code", &files, "Gate-Rules: 2");
    let land = land_as(root, &first, &first_fp, LandKind::Code, false);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    let (second, second_fp) = requirement_removed(root, "Gate-Rules: 1");
    let land = land_as(root, &second, &second_fp, LandKind::Spec, false);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    let (code, findings, out) = findings(root, &["--review", "--since", &first]);
    assert_eq!(code, Some(1), "{out}");
    let want = [
        ("ledger/understated-land", second.as_str()),
        ("ledger/unapproved-land", &second),
    ];
    assert_eq!(findings, found(&want), "{out}");
}

#[test]
fn review_a_floor_counts_only_a_signed_land() {
    // req:ledger/verify-recomputes-review@730d0917
    // An earlier land no signed record names raises no floor: it is only unsigned.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_keys, christopher) = signed_main_with_specs(root, true);
    let files = [("src/x.rs", "fn x() { 1; }\n")];
    let (first, first_fp) = merge_files_ending(root, "code", &files, "Gate-Rules: 2");
    let land = land_as(root, &first, &first_fp, LandKind::Code, false);
    append(root, land, &env(root, &[])).unwrap();
    let (second, second_fp) = requirement_removed(root, "");
    let land = land_as(root, &second, &second_fp, LandKind::Spec, false);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    let (code, findings, out) = findings(root, &["--review"]);
    assert_eq!(code, Some(1), "{out}");
    assert_eq!(
        findings,
        found(&[("ledger/unsigned-land", &first)]),
        "{out}"
    );
}

/// A signed, checked code land whose merge commit ends with `Gate-Rules: ` and a number above
/// `clm-gate`'s `RULES`: the merge commit and that number.
fn rules_newer_than_this_clm(root: &Path, christopher: &Path) -> (String, u32) {
    let newer = clm_gate::RULES + 1;
    let files = [("src/x.rs", "fn x() { 2; }\n")];
    let ending = format!("Gate-Rules: {newer}");
    let (merge, fingerprint) = merge_files_ending(root, "newer", &files, &ending);
    let land = land_as(root, &merge, &fingerprint, LandKind::Code, false);
    append_signed(root, "christopher-lunny", christopher, "clm-land", land);
    (merge, newer)
}

#[test]
fn review_rules_newer_than_this_clm() {
    // req:ledger/verify-recomputes-review@730d0917
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_keys, christopher) = signed_main_with_specs(root, true);
    let (merge, newer) = rules_newer_than_this_clm(root, &christopher);
    let (code, out, err) = clm(root, root, &["ledger", "verify", "--review"]);
    assert_eq!(code, Some(1), "{out}{err}");
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(
        out.contains("ledger/unrated-land")
            && out.contains(&format!("landed {merge}: the gate can't rate it: "))
            && out.contains(&format!("version {newer}"))
            && out.contains(&format!("RULES ({})", clm_gate::RULES)),
        "{out}"
    );
}

#[test]
fn review_a_land_after_rules_newer_than_this_clm() {
    // req:ledger/verify-recomputes-review@730d0917
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_keys, christopher) = signed_main_with_specs(root, true);
    let (first, _) = rules_newer_than_this_clm(root, &christopher);
    let (second, second_fp) = architecture_change(root);
    let land = land_as(root, &second, &second_fp, LandKind::Spec, false);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    let (code, findings, out) = findings(root, &["--review"]);
    assert_eq!(code, Some(1), "{out}");
    let want = [
        ("ledger/unrated-land", first.as_str()),
        ("ledger/understated-land", &second),
        ("ledger/unapproved-land", &second),
    ];
    assert_eq!(findings, found(&want), "{out}");
}

#[test]
fn review_a_capped_floor_still_rates_later_lands() {
    // req:ledger/verify-recomputes-review@730d0917
    // After a land under rules newer than this clm the floor is `RULES`: a later removal is
    // rated under `RULES`, not reported as unrated.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_keys, christopher) = signed_main_with_specs(root, true);
    let (first, _) = rules_newer_than_this_clm(root, &christopher);
    let (second, second_fp) = requirement_removed(root, "");
    let land = land_as(root, &second, &second_fp, LandKind::Spec, false);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    let (code, findings, out) = findings(root, &["--review"]);
    assert_eq!(code, Some(1), "{out}");
    let want = [
        ("ledger/unrated-land", first.as_str()),
        ("ledger/understated-land", &second),
        ("ledger/unapproved-land", &second),
    ];
    assert_eq!(findings, found(&want), "{out}");
}

/// The merge of a branch adding `changes/x/change.md` as a gitlink to a commit the repository
/// doesn't hold, which the gate can't read: the merge commit, the branch's fingerprint and
/// the change's path.
fn unreadable_change(dir: &Path) -> (String, String, &'static str) {
    let change = "changes/x/change.md";
    sh(dir, &["switch", "-q", "-c", "unreadable"]);
    let missing = format!("160000,{},{change}", "1".repeat(40));
    sh(dir, &["update-index", "--add", "--cacheinfo", &missing]);
    sh(dir, &["commit", "-qm", "work"]);
    let out = fingerprint(dir, "main");
    sh(dir, &["switch", "-q", "main"]);
    sh(
        dir,
        &[
            "merge",
            "-q",
            "--no-ff",
            "-m",
            "Merge unreadable",
            "unreadable",
        ],
    );
    (
        sh(dir, &["rev-parse", "HEAD"]),
        out.trim().to_string(),
        change,
    )
}

#[test]
fn review_unrated_land() {
    // req:ledger/verify-unrated-land@c1f0cb74
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_keys, christopher) = signed_main_with_specs(root, true);
    let (first, first_fp, change) = unreadable_change(root);
    let land = land_as(root, &first, &first_fp, LandKind::Spec, false);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    let (second, second_fp) = architecture_change(root);
    let land = land_as(root, &second, &second_fp, LandKind::Spec, false);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    let (code, out, err) = clm(root, root, &["ledger", "verify", "--review"]);
    assert_eq!(code, Some(1), "{out}{err}");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3, "{out}");
    assert!(
        lines[0].contains("ledger/unrated-land")
            && lines[0].contains(&format!("landed {first}: the gate can't rate it: "))
            && lines[0].contains(change),
        "{out}"
    );
    assert!(
        lines[1].contains("ledger/understated-land") && lines[1].contains(&second),
        "{out}"
    );
    assert!(
        lines[2].contains("ledger/unapproved-land") && lines[2].contains(&second),
        "{out}"
    );
}

#[test]
fn review_unrated_land_judged_by_its_record() {
    // req:ledger/verify-unrated-land@c1f0cb74
    // Recorded as needing review, with no approval: judged by that record, the gate error
    // aside.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_keys, christopher) = signed_main_with_specs(root, true);
    let (first, first_fp, _) = unreadable_change(root);
    let land = land_as(root, &first, &first_fp, LandKind::Spec, true);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    let (code, out, err) = clm(root, root, &["ledger", "verify", "--review"]);
    assert_eq!(code, Some(1), "{out}{err}");
    let rules: Vec<&str> = out
        .lines()
        .map(|l| l.split(": ").nth(1).unwrap_or(l))
        .collect();
    assert_eq!(
        rules,
        ["ledger/unrated-land", "ledger/unapproved-land"],
        "{out}"
    );
}

#[test]
fn review_unrated_land_without_the_flag() {
    // req:ledger/verify-unrated-land@c1f0cb74
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (_keys, christopher) = signed_main_with_specs(root, true);
    let (first, first_fp, _) = unreadable_change(root);
    let land = land_as(root, &first, &first_fp, LandKind::Spec, false);
    append_signed(root, "christopher-lunny", &christopher, "clm-land", land);
    let (code, out, err) = clm(root, root, &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

fn proposed(slug: &str) -> Body {
    Body::Proposed {
        slug: slug.into(),
        title: "Export CSV".into(),
        body: "A CSV export.".into(),
        nodes: vec!["view".into()],
    }
}

/// A repository with no remote, writing as `alice`: the environment variables to pass.
fn alices(dir: &Path) -> &'static [(&'static str, &'static str)] {
    repo(dir, "T");
    std::fs::write(dir.join("clm.toml"), "host = \"none\"\n").unwrap();
    &[("CLM_WRITER", "alice")]
}

#[test]
fn show_prints_proposals() {
    // req:ledger/proposal-records@f2ac7e51
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let env = env(root, alices(root));
    let p = append(root, proposed("export-csv"), &env).unwrap();
    tick();
    let end = Body::Withdrawn {
        proposal: p.record.id.clone(),
    };
    let w = append(root, end, &env).unwrap();
    let (code, out, err) = clm(root, root, &["ledger", "show"]);
    assert_eq!(code, Some(0), "{err}");
    let (id, who) = (&p.record.id, &p.record.writer);
    assert!(who.starts_with("alice/"), "{who}");
    assert_eq!(
        out,
        format!(
            "proposed {id} export-csv {who} {}\nwithdrawn {id} {who} {}\n",
            p.record.at, w.record.at
        )
    );
}

#[test]
fn compact_keeps_proposal_records() {
    // req:ledger/proposal-records@f2ac7e51
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let env = env(home, &[]);
    let p = append(&a, proposed("dark-mode"), &env).unwrap();
    let end = Body::Withdrawn {
        proposal: p.record.id.clone(),
    };
    append(&a, end, &env).unwrap();
    append(&a, approved("f"), &env).unwrap();
    tick();
    append(&a, landed("c", "f", Some("m")), &env).unwrap();
    let (code, out, err) = clm(&a, home, &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 1");
    let text = records(&a, &format!("clm/ledger/{}", p.record.writer));
    assert!(text.contains(r#""kind":"proposed""#), "{text}");
    assert!(text.contains(r#""kind":"withdrawn""#), "{text}");
    assert!(!text.contains(r#""kind":"approved""#), "{text}");
}

fn answered(item: &str, answer: &str) -> Body {
    Body::Answered {
        item: item.into(),
        answer: answer.into(),
    }
}

#[test]
fn show_prints_answers() {
    // req:ledger/answer-records@96b5bcca
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let env = env(root, alices(root));
    let a = append(root, answered("q", "yes"), &env).unwrap();
    let (code, out, err) = clm(root, root, &["ledger", "show"]);
    assert_eq!(code, Some(0), "{err}");
    let (id, who) = (&a.record.id, &a.record.writer);
    assert!(who.starts_with("alice/"), "{who}");
    assert_eq!(out, format!("answered {id} q {who} {}\n", a.record.at));
}

#[test]
fn compact_keeps_answers() {
    // req:ledger/answer-records@96b5bcca
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let env = env(home, &[]);
    let r = append(&a, answered("q", "yes"), &env).unwrap();
    append(&a, approved("f"), &env).unwrap();
    tick();
    append(&a, landed("c", "f", Some("m")), &env).unwrap();
    let (code, out, err) = clm(&a, home, &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 1");
    let text = records(&a, &format!("clm/ledger/{}", r.record.writer));
    assert!(text.contains(r#""kind":"answered""#), "{text}");
}

fn commented(fingerprint: &str, target: Option<&str>, text: &str) -> Body {
    Body::Commented {
        branch: "spec/x".into(),
        fingerprint: fingerprint.into(),
        target: target.map(str::to_string),
        text: text.into(),
    }
}

fn changes_requested(fingerprint: &str, text: &str) -> Body {
    Body::ChangesRequested {
        branch: "spec/x".into(),
        fingerprint: fingerprint.into(),
        text: text.into(),
    }
}

#[test]
fn show_prints_feedback_on_one_line() {
    // req:ledger/feedback-records@9066f278
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let env = env(root, alices(root));
    let r = changes_requested("f", "  Split the\n   requirement  \n");
    let r = append(root, r, &env).unwrap();
    tick();
    // Each run of whitespace holding a line break is one space; others stay.
    let c = commented("f", None, "Why  this?\r\n\r\nAnd that.\u{2028}Done.");
    let c = append(root, c, &env).unwrap();
    tick();
    let t = append(root, commented("f", Some("n/r"), "ok"), &env).unwrap();
    let (code, out, err) = clm(root, root, &["ledger", "show"]);
    assert_eq!(code, Some(0), "{err}");
    let who = &r.record.writer;
    assert_eq!(
        out,
        format!(
            "changes_requested spec/x f {who} {} Split the requirement\n\
             commented spec/x f - {who} {} Why  this? And that. Done.\n\
             commented spec/x f n/r {who} {} ok\n",
            r.record.at, c.record.at, t.record.at
        )
    );
}

#[test]
fn compact_prunes_landed_feedback() {
    // req:ledger/feedback-records@9066f278
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let b = clone_with_origin(home, "b", "B");
    let env = env(home, &[]);
    let mine = append(&a, commented("f", Some("n/r"), "Why?"), &env).unwrap();
    append(&a, changes_requested("f", "Split it"), &env).unwrap();
    // Whatever its branch, feedback at a landed fingerprint goes.
    let elsewhere = Body::Commented {
        branch: "spec/y".into(),
        fingerprint: "g".into(),
        target: None,
        text: "Keep me".into(),
    };
    append(&a, elsewhere, &env).unwrap();
    tick();
    let mut land = landed("c", "f", Some("m"));
    if let Body::Landed { branch, .. } = &mut land {
        *branch = "spec/renamed".into();
    }
    append(&b, land, &env).unwrap();
    let (code, out, err) = clm(&a, home, &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 2");
    let text = records(&a, &format!("clm/ledger/{}", mine.record.writer));
    assert_eq!(text.lines().count(), 1, "{text}");
    assert!(text.contains("Keep me"), "{text}");
}

#[test]
fn compact_prunes_feedback_at_a_signed_lands_fingerprint() {
    // req:ledger/feedback-records@9066f278
    // The approval of a signed land that needed review stays for `verify`; feedback on that
    // diff still goes.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signed_main(root, "Christopher Lunny");
    let land = hard_land(root, &s.merge, &s.fingerprint);
    append_signed(root, "christopher-lunny", &s.christopher, "clm-land", land);
    append_signed(
        root,
        "dana",
        &s.dana,
        "clm-approve",
        approved(&s.fingerprint),
    );
    let env = env(root, &[("CLM_WRITER", "dana")]);
    let note = commented(&s.fingerprint, None, "Looks right");
    append(root, note, &env).unwrap();
    sh(root, &["config", "user.name", "Dana"]);
    let (code, out, err) = clm(root, root, &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 1");
    let fold = clm_ledger::fold(root).unwrap();
    assert!(fold.approvals.contains_key(&s.fingerprint), "{fold:?}");
    assert!(fold.comments.is_empty(), "{fold:?}");
}

fn effort(output: u64) -> Body {
    let usage = |input, output, cache_read, cache_write| clm_ledger::Usage {
        input,
        output,
        cache_read,
        cache_write,
    };
    Body::Effort {
        branch: "impl/a".into(),
        session: "s1".into(),
        models: [
            ("claude-opus-5-5".to_string(), usage(7, output, 60, 12)),
            ("claude-sonnet-5-5".to_string(), usage(3, 0, 40, 8)),
        ]
        .into(),
        active_seconds: 120,
    }
}

fn decided(capability: &str) -> Body {
    Body::Decided {
        capability: capability.into(),
        value: clm_ledger::Rating::Low,
        decision: clm_ledger::Decision::Cut,
        settles: vec![],
    }
}

#[test]
fn show_effort_and_decided() {
    // req:ledger/effort-records@828c1a9a
    // req:ledger/decision-records@9ca8cb93
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let env = env(root, alices(root));
    let e = append(root, effort(5), &env).unwrap();
    tick();
    let d = append(root, decided("capability/x"), &env).unwrap();
    let (code, out, err) = clm(root, root, &["ledger", "show"]);
    assert_eq!(code, Some(0), "{err}");
    let who = &e.record.writer;
    // Work tokens: input, output and cache writes over both models, 10 + 5 + 20.
    assert_eq!(
        out,
        format!(
            "effort impl/a s1 35 120s {who} {}\ndecided capability/x low cut {who} {}\n",
            e.record.at, d.record.at
        )
    );
}

#[test]
fn compact_drops_superseded_effort() {
    // req:ledger/effort-records@828c1a9a
    // req:ledger/decision-records@9ca8cb93
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let env = env(home, &[]);
    let early = append(&a, effort(5), &env).unwrap();
    append(&a, decided("capability/x"), &env).unwrap();
    tick();
    let late = append(&a, effort(50), &env).unwrap();
    append(&a, decided("capability/x"), &env).unwrap();
    assert_eq!(early.record.id, late.record.id);
    let (code, out, err) = clm(&a, home, &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 1");
    let text = records(&a, &format!("clm/ledger/{}", late.record.writer));
    assert_eq!(text.matches(r#""kind":"effort""#).count(), 1, "{text}");
    assert!(text.contains(&late.record.at), "{text}");
    assert!(!text.contains(&early.record.at), "{text}");
    assert_eq!(text.matches(r#""kind":"decided""#).count(), 2, "{text}");
}

/// A landed `main`, then a merge commit made by hand whose message carries the trailer
/// `Clm-By-Hand: by-hand`, its branch adding `files`: the merge commit.
fn by_hand_merge(dir: &Path, files: &[(&str, &str)]) -> String {
    by_hand_merge_on(dir, &[], files)
}

/// As `by_hand_merge`, `main`'s first commit also holding `on_main`.
fn by_hand_merge_on(dir: &Path, on_main: &[(&str, &str)], files: &[(&str, &str)]) -> String {
    let merges = history_holding(dir, on_main, &["Merge b0"]);
    record_land(dir, &merges[0]);
    sh(dir, &["switch", "-q", "-c", "refound"]);
    commit(dir, &[&[("src/r.rs", "x\n")], files].concat(), "work");
    sh(dir, &["switch", "-q", "main"]);
    let message = "Merge refound into main\n\nClm-By-Hand: by-hand\n";
    sh(dir, &["merge", "-q", "--no-ff", "-m", message, "refound"]);
    sh(dir, &["rev-parse", "HEAD"])
}

const ACCEPTED: (&str, &str) = (
    "docs/decisions/by-hand.md",
    "---\ntitle: \"By hand\"\ndate: 2026-10-30\nstatus: accepted\n---\n\nWhy.\n",
);

#[test]
fn by_hand_trailer_skips_commit() {
    // req:ledger/verify-by-hand-merge@1a3d7833
    let dir = tempfile::tempdir().unwrap();
    by_hand_merge(dir.path(), &[ACCEPTED]);
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn by_hand_trailer_needs_the_decision() {
    // req:ledger/verify-by-hand-merge@1a3d7833
    let dir = tempfile::tempdir().unwrap();
    let merge = by_hand_merge(dir.path(), &[]);
    let (code, out, _) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}");
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(
        out.contains("ledger/unrecorded-land") && out.contains(&merge),
        "{out}"
    );
    // A decision that isn't accepted counts for nothing either.
    let dir = tempfile::tempdir().unwrap();
    let proposed = (ACCEPTED.0, &*ACCEPTED.1.replace("accepted", "proposed"));
    let merge = by_hand_merge(dir.path(), &[proposed]);
    let (code, out, _) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains(&merge), "{out}");
}

#[test]
fn by_hand_trailer_needs_closed_frontmatter() {
    // req:ledger/verify-by-hand-merge@1a3d7833
    // The frontmatter never closes, so the body's `status: accepted` is no status.
    let dir = tempfile::tempdir().unwrap();
    let open = (ACCEPTED.0, "---\ntitle: \"By hand\"\n\nstatus: accepted\n");
    let merge = by_hand_merge(dir.path(), &[open]);
    let (code, out, _) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}");
    assert!(
        out.contains("ledger/unrecorded-land") && out.contains(&merge),
        "{out}"
    );
}

#[test]
fn by_hand_trailer_shut_in_a_signed_repository() {
    // req:ledger/verify-by-hand-merge@1a3d7833
    let dir = tempfile::tempdir().unwrap();
    let signers = (
        "docs/allowed_signers",
        "christopher-lunny ssh-ed25519 AAAA\n",
    );
    let merge = by_hand_merge_on(dir.path(), &[signers], &[ACCEPTED]);
    let (code, out, _) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}");
    assert!(
        out.lines()
            .any(|l| l.contains("ledger/unrecorded-land") && l.contains(&merge)),
        "{out}"
    );
}

#[test]
fn compact_settles_an_effort_tie() {
    // req:ledger/effort-records@828c1a9a
    // Two of the caller's own counts with one `id` and one `at`: one goes, whichever it is.
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let env = env(home, &[]);
    let first = append(&a, effort(5), &env).unwrap();
    let branch = format!("clm/ledger/{}", first.record.writer);
    let line = records(&a, &branch).replace(r#""output":5"#, r#""output":6"#);
    append_line(&a, &branch, &line);
    let (code, out, err) = clm(&a, home, &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 1");
    let text = records(&a, &branch);
    assert_eq!(text.matches(r#""kind":"effort""#).count(), 1, "{text}");
}

/// A composed `checked` record for `key`, built from the checks of `head` and `base`.
fn composed(key: &str, command: &str, head: &str, base: &str) -> Body {
    Body::Checked {
        code_key: key.into(),
        command: command.into(),
        basis: Some(clm_ledger::Basis {
            head: head.into(),
            base: base.into(),
        }),
    }
}

fn checked_by(key: &str, command: &str) -> Body {
    Body::Checked {
        code_key: key.into(),
        command: command.into(),
        basis: None,
    }
}

#[test]
fn verify_counts_a_composed_check() {
    // req:ledger/composed-check@af377cb9
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0"]);
    let env = env(dir.path(), &[]);
    append(dir.path(), checked_by("h", "x"), &env).unwrap();
    append(dir.path(), checked_by("b", "x"), &env).unwrap();
    append(dir.path(), composed("m", "x", "h", "b"), &env).unwrap();
    append(dir.path(), landed(&merges[0], "f", Some("m")), &env).unwrap();
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn verify_rejects_a_composed_check_whose_basis_is_missing() {
    // req:ledger/composed-check@af377cb9
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0"]);
    let env = env(dir.path(), &[]);
    append(dir.path(), checked_by("h", "x"), &env).unwrap();
    append(dir.path(), composed("m", "x", "h", "b"), &env).unwrap();
    append(dir.path(), landed(&merges[0], "f", Some("m")), &env).unwrap();
    let (code, out, _) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}");
    assert!(
        out.contains("ledger/unchecked-land") && out.contains(&merges[0]),
        "{out}"
    );
}

#[test]
fn verify_rejects_a_composed_check_whose_basis_ran_another_command() {
    // req:ledger/composed-check@af377cb9
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0"]);
    let env = env(dir.path(), &[]);
    append(dir.path(), checked_by("h", "x"), &env).unwrap();
    append(dir.path(), checked_by("b", "y"), &env).unwrap();
    append(dir.path(), composed("m", "x", "h", "b"), &env).unwrap();
    append(dir.path(), landed(&merges[0], "f", Some("m")), &env).unwrap();
    let (code, out, _) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains("ledger/unchecked-land"), "{out}");
}

#[test]
fn verify_takes_a_whole_check_beside_a_broken_composed_one() {
    // req:ledger/composed-check@af377cb9
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0"]);
    let env = env(dir.path(), &[]);
    append(dir.path(), composed("m", "x", "h", "b"), &env).unwrap();
    append(dir.path(), checked_by("m", "x"), &env).unwrap();
    append(dir.path(), landed(&merges[0], "f", Some("m")), &env).unwrap();
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn compact_keeps_the_basis_of_a_composed_check() {
    // req:ledger/composed-check@af377cb9
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0"]);
    let env = env(dir.path(), &[]);
    append(dir.path(), checked_by("h", "x"), &env).unwrap();
    append(dir.path(), checked_by("b", "x"), &env).unwrap();
    append(dir.path(), composed("m", "x", "h", "b"), &env).unwrap();
    tick();
    append(dir.path(), landed(&merges[0], "f", Some("m")), &env).unwrap();
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 0");
    let fold = clm_ledger::fold(dir.path()).unwrap();
    assert!(fold.checks.contains_key("h") && fold.checks.contains_key("b"));
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn compact_still_prunes_a_check_no_land_or_basis_names() {
    // req:ledger/compact@59996cc0
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0"]);
    let env = env(dir.path(), &[]);
    append(dir.path(), checked_by("old", "x"), &env).unwrap();
    append(dir.path(), checked_by("m", "x"), &env).unwrap();
    tick();
    append(dir.path(), landed(&merges[0], "f", Some("m")), &env).unwrap();
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 1");
    let fold = clm_ledger::fold(dir.path()).unwrap();
    assert!(!fold.checks.contains_key("old") && fold.checks.contains_key("m"));
}

#[test]
fn compact_keeps_chains_of_bases() {
    // req:ledger/composed-check@af377cb9
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0"]);
    let env = env(dir.path(), &[]);
    // `deep` and `b2` are the basis of composed `h`; `h` and `b` of composed `m`, a land's key.
    append(dir.path(), checked_by("deep", "x"), &env).unwrap();
    append(dir.path(), checked_by("b2", "x"), &env).unwrap();
    append(dir.path(), composed("h", "x", "deep", "b2"), &env).unwrap();
    append(dir.path(), checked_by("b", "x"), &env).unwrap();
    append(dir.path(), composed("m", "x", "h", "b"), &env).unwrap();
    tick();
    append(dir.path(), landed(&merges[0], "f", Some("m")), &env).unwrap();
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    let fold = clm_ledger::fold(dir.path()).unwrap();
    for key in ["deep", "b2", "h", "b", "m"] {
        assert!(fold.checks.contains_key(key), "{key} was pruned");
    }
}

#[test]
fn compact_keeps_the_basis_of_a_composed_check_newer_than_every_land() {
    // req:ledger/composed-check@af377cb9
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0"]);
    let env = env(dir.path(), &[]);
    append(dir.path(), checked_by("n1", "x"), &env).unwrap();
    append(dir.path(), checked_by("n2", "x"), &env).unwrap();
    append(dir.path(), landed(&merges[0], "f", Some("k")), &env).unwrap();
    tick();
    append(dir.path(), composed("later", "x", "n1", "n2"), &env).unwrap();
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    let fold = clm_ledger::fold(dir.path()).unwrap();
    assert!(fold.checks.contains_key("n1") && fold.checks.contains_key("n2"));
}
