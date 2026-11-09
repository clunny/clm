//! `clm ledger compact`, `verify` and `show`, and `clm check`'s call of `verify`.

#[path = "../../clm-ledger/tests/common/mod.rs"]
mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use clm_ledger::{Body, LandKind};
use common::{clone_with_origin, commit, env, repo, sh};

/// `clm <args> --root <root>` with `HOME` at `home`: exit code, stdout, stderr.
fn clm(root: &Path, home: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .arg("--root")
        .arg(root)
        .env("HOME", home)
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("CLM_WRITER")
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
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
    }
}

fn records(dir: &Path, rev: &str) -> String {
    sh(dir, &["show", &format!("{rev}:records.jsonl")])
}

/// So the next record's `at` is later.
fn tick() {
    std::thread::sleep(std::time::Duration::from_millis(5));
}

#[test]
fn compact_prunes_a_landed_approval_and_older_checks() {
    // req:ledger/compact@cf9d661f
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let b = clone_with_origin(home, "b", "B");
    let mine = clm_ledger::append(&a, approved("f"), &env(home, &[])).unwrap();
    clm_ledger::append(&a, checked("k"), &env(home, &[])).unwrap();
    tick();
    let theirs = clm_ledger::append(&b, landed("c", "f", Some("m")), &env(home, &[])).unwrap();
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
    // req:ledger/compact@cf9d661f
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let mine = clm_ledger::append(&a, approved("f"), &env(home, &[])).unwrap();
    tick();
    clm_ledger::append(&a, landed("c", "f", Some("m")), &env(home, &[])).unwrap();
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
    // req:ledger/compact@cf9d661f
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let b = clone_with_origin(home, "b", "B");
    let mine = clm_ledger::append(&a, approved("g"), &env(home, &[])).unwrap();
    tick();
    clm_ledger::append(&b, landed("c", "other", Some("m")), &env(home, &[])).unwrap();
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
    // req:ledger/compact@cf9d661f
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let env = env(home, &[]);
    let mine = clm_ledger::append(&a, landed("c", "f", Some("m")), &env).unwrap();
    tick();
    clm_ledger::append(&a, checked("newer"), &env).unwrap();
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
fn compact_drops_reviewed_lines() {
    // req:ledger/compact@cf9d661f
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let env = env(home, &[]);
    let mine = clm_ledger::append(&a, approved("g"), &env).unwrap();
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
    // req:ledger/compact@cf9d661f
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
    // req:ledger/compact@cf9d661f
    let dir = tempfile::tempdir().unwrap();
    let merges = history_holding(dir.path(), &[GUIDE_PAGE], &["Merge b0", "Merge b1"]);
    let env = env(dir.path(), &[]);
    let key = legacy_key(dir.path(), &merges[0]);
    clm_ledger::append(dir.path(), checked(&key), &env).unwrap();
    // Written before `landed` records named their code key: checked under its legacy key.
    clm_ledger::append(dir.path(), landed(&merges[0], "f", None), &env).unwrap();
    tick();
    clm_ledger::append(dir.path(), landed(&merges[1], "g", Some("m")), &env).unwrap();
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
    // req:ledger/compact@cf9d661f
    // A land without `code_key` whose commit isn't in this clone: its key can't be
    // computed, so no check is dropped, though the landed approval still is.
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0"]);
    let env = env(dir.path(), &[]);
    clm_ledger::append(dir.path(), approved("f"), &env).unwrap();
    clm_ledger::append(dir.path(), checked("old"), &env).unwrap();
    tick();
    let missing = "0123456789abcdef0123456789abcdef01234567";
    clm_ledger::append(dir.path(), landed(missing, "f", None), &env).unwrap();
    clm_ledger::append(dir.path(), landed(&merges[0], "g", Some("m")), &env).unwrap();
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 1");
    assert!(err.contains("warning") && err.contains(missing), "{err}");
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
    clm_ledger::append(dir, checked(&key), &env).unwrap();
    tick();
    clm_ledger::append(dir, landed(commit, "f", Some(&key)), &env).unwrap();
}

#[test]
fn verify_clean_history() {
    // req:ledger/verify@ff314001
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0", "Merge b1"]);
    // The first land written before `landed` records named their code key: its key is
    // computed from the commit.
    let env = env(dir.path(), &[]);
    let key = clm_ledger::code_key(dir.path(), &merges[0]).unwrap();
    clm_ledger::append(dir.path(), checked(&key), &env).unwrap();
    clm_ledger::append(dir.path(), landed(&merges[0], "f", None), &env).unwrap();
    record_land(dir.path(), &merges[1]);
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn verify_reads_the_recorded_key() {
    // req:ledger/verify@ff314001
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0"]);
    let env = env(dir.path(), &[]);
    clm_ledger::append(dir.path(), checked("k"), &env).unwrap();
    clm_ledger::append(dir.path(), landed(&merges[0], "f", Some("k")), &env).unwrap();
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn verify_reads_the_legacy_key_of_a_land_recorded_without_one() {
    // req:ledger/verify@ff314001
    let dir = tempfile::tempdir().unwrap();
    let merges = history_holding(dir.path(), &[GUIDE_PAGE], &["Merge b0"]);
    let vars = env(dir.path(), &[]);
    let legacy = legacy_key(dir.path(), &merges[0]);
    clm_ledger::append(dir.path(), checked(&legacy), &vars).unwrap();
    clm_ledger::append(dir.path(), landed(&merges[0], "f", None), &vars).unwrap();
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");

    // Only today's key checked: the land is unchecked.
    let dir = tempfile::tempdir().unwrap();
    let merges = history_holding(dir.path(), &[GUIDE_PAGE], &["Merge b0"]);
    let vars = env(dir.path(), &[]);
    let live = clm_ledger::code_key(dir.path(), &merges[0]).unwrap();
    clm_ledger::append(dir.path(), checked(&live), &vars).unwrap();
    clm_ledger::append(dir.path(), landed(&merges[0], "f", None), &vars).unwrap();
    let (code, out, _) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains("ledger/unchecked-land"), "{out}");
}

#[test]
fn verify_reports_a_push_behind_clms_back() {
    // req:ledger/verify@ff314001
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
    // req:ledger/verify@ff314001
    // Merges from before the ledger existed aren't reported.
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0", "Merge b1"]);
    record_land(dir.path(), &merges[1]);
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn verify_reports_a_direct_push() {
    // req:ledger/verify@ff314001
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
    // req:ledger/verify@ff314001
    // A release merge, and a version bump that fast-forwarded onto `main`.
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0", "Release v0.9.0"]);
    record_land(dir.path(), &merges[0]);
    commit(dir.path(), &[("Cargo.toml", "v\n")], "Release v0.10.0");
    let (code, out, err) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn verify_reports_a_land_no_check_names() {
    // req:ledger/verify@ff314001
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0", "Merge b1"]);
    record_land(dir.path(), &merges[0]);
    let env = env(dir.path(), &[]);
    // The commit's computed key is checked, but the land's own key `k` isn't.
    let computed = clm_ledger::code_key(dir.path(), &merges[1]).unwrap();
    clm_ledger::append(dir.path(), checked(&computed), &env).unwrap();
    clm_ledger::append(dir.path(), landed(&merges[1], "f", Some("k")), &env).unwrap();
    let (code, out, _) = clm(dir.path(), dir.path(), &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}");
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(
        out.contains("ledger/unchecked-land") && out.contains(&merges[1]),
        "{out}"
    );
}

#[test]
fn check_runs_verify_on_main() {
    // req:ledger/verify@ff314001
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0", "Merge b1"]);
    record_land(dir.path(), &merges[0]);
    let (_, out, _) = clm(dir.path(), dir.path(), &["check", "--no-detectors"]);
    assert!(out.contains("ledger/unrecorded-land"), "{out}");
    // Off `main`, it doesn't.
    sh(dir.path(), &["switch", "-q", "-c", "other"]);
    let (_, out, _) = clm(dir.path(), dir.path(), &["check", "--no-detectors"]);
    assert!(!out.contains("ledger/"), "{out}");
}

#[test]
fn show_lists_the_fold_in_time_order() {
    // req:ledger/show@803f9c27
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), "T");
    std::fs::write(dir.path().join("clm.toml"), "host = \"none\"\n").unwrap();
    let env = env(dir.path(), &[]);
    let a = clm_ledger::append(dir.path(), approved("f"), &env).unwrap();
    tick();
    let l = clm_ledger::append(dir.path(), landed("c", "f", None), &env).unwrap();
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
    // req:ledger/show@803f9c27
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
    let l = clm_ledger::append(dir.path(), body, &env).unwrap();
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
    // req:ledger/show@803f9c27
    // Writer `b`'s record is older, though its ledger is read after `a`'s.
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let b = clone_with_origin(home, "b", "B");
    clm_ledger::append(&b, approved("old"), &env(home, &[])).unwrap();
    tick();
    clm_ledger::append(&a, approved("new"), &env(home, &[])).unwrap();
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
    let (code, out, err) = clm(dir, dir, &["fingerprint", "--base", "main"]);
    assert_eq!(code, Some(0), "{err}");
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
    clm_ledger::append(dir, signed, &env).unwrap();
}

/// A spec land of `commit` that needed review, naming `fingerprint`, its code key checked.
fn hard_land(dir: &Path, commit: &str, fingerprint: &str) -> Body {
    let key = clm_ledger::code_key(dir, commit).unwrap();
    clm_ledger::append(dir, checked(&key), &env(dir, &[])).unwrap();
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
    }
}

#[test]
fn verify_signed_and_approved_by_another() {
    // req:ledger/verify-signed-lands@09eefb14
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
    // req:ledger/verify-signed-lands@09eefb14
    // A retried land left an unsigned record beside the signed one.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signed_main(root, "Christopher Lunny");
    let land = hard_land(root, &s.merge, &s.fingerprint);
    clm_ledger::append(root, land.clone(), &env(root, &[])).unwrap();
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
    // req:ledger/verify-signed-lands@09eefb14
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
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(
        out.contains("ledger/unapproved-land") && out.contains(&s.merge),
        "{out}"
    );
}

#[test]
fn verify_author_name_changed() {
    // req:ledger/verify-signed-lands@09eefb14
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
    // req:ledger/verify-signed-lands@09eefb14
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
    // req:ledger/verify-signed-lands@09eefb14
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signed_main(root, "Christopher Lunny");
    let land = hard_land(root, &s.merge, &s.fingerprint);
    clm_ledger::append(root, land, &env(root, &[])).unwrap();
    append_signed(
        root,
        "dana",
        &s.dana,
        "clm-approve",
        approved(&s.fingerprint),
    );
    let (code, out, _) = clm(root, root, &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}");
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(
        out.contains("ledger/unsigned-land") && out.contains(&s.merge),
        "{out}"
    );
}

#[test]
fn verify_approval_for_another_diff() {
    // req:ledger/verify-signed-lands@09eefb14
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let s = signed_main(root, "Christopher Lunny");
    let other = "0".repeat(64);
    let land = hard_land(root, &s.merge, &other);
    append_signed(root, "christopher-lunny", &s.christopher, "clm-land", land);
    append_signed(root, "dana", &s.dana, "clm-approve", approved(&other));
    let (code, out, _) = clm(root, root, &["ledger", "verify"]);
    assert_eq!(code, Some(1), "{out}");
    assert!(
        out.contains("ledger/unapproved-land")
            && out.contains(&s.merge)
            && out.contains(&other)
            && out.contains(&s.fingerprint),
        "{out}"
    );
}

#[test]
fn compact_keeps_a_signed_lands_approval() {
    // req:ledger/compact@cf9d661f
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
    clm_ledger::append(root, approved("old"), &env).unwrap();
    tick();
    let old = hard_land(root, &before, "old");
    clm_ledger::append(root, old, &env).unwrap();
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
    // req:ledger/compact@cf9d661f
    // Whether its first parent is signed can't be told, so the approval stays.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    history(root, &["Merge b0"]);
    let env = env(root, &[]);
    clm_ledger::append(root, approved("f"), &env).unwrap();
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
    };
    clm_ledger::append(root, land, &env).unwrap();
    let (code, out, err) = clm(root, root, &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 0");
    assert!(err.contains("warning") && err.contains(missing), "{err}");
}

#[test]
fn verify_before_the_switch() {
    // req:ledger/verify-signed-lands@09eefb14
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
        clm_ledger::append(root, approved(fingerprint), &env).unwrap();
        clm_ledger::append(root, land, &env).unwrap();
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
    // req:ledger/subscription-records@e4cb2df2
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    repo(root, "T");
    std::fs::write(root.join("clm.toml"), "host = \"none\"\n").unwrap();
    let env = env(root, &[("CLM_WRITER", "alice")]);
    let s = clm_ledger::append(root, subscribed("https://push.example/s/2"), &env).unwrap();
    tick();
    let end = Body::Unsubscribed {
        subscription: s.record.id.clone(),
    };
    let u = clm_ledger::append(root, end, &env).unwrap();
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
    // req:ledger/subscription-records@e4cb2df2
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let env = env(home, &[]);
    let s = clm_ledger::append(&a, subscribed("https://push.example/s/1"), &env).unwrap();
    let end = Body::Unsubscribed {
        subscription: s.record.id.clone(),
    };
    clm_ledger::append(&a, end, &env).unwrap();
    clm_ledger::append(&a, approved("f"), &env).unwrap();
    tick();
    clm_ledger::append(&a, landed("c", "f", Some("m")), &env).unwrap();
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
    sh(dir, &["switch", "-q", "-c", name]);
    commit(dir, files, "work");
    let (code, out, err) = clm(dir, dir, &["fingerprint", "--base", "main"]);
    assert_eq!(code, Some(0), "{err}");
    sh(dir, &["switch", "-q", "main"]);
    let message = format!("Merge {name}");
    sh(dir, &["merge", "-q", "--no-ff", "-m", &message, name]);
    (sh(dir, &["rev-parse", "HEAD"]), out.trim().to_string())
}

/// A land of `commit` recorded as `kind` and `review`, its code key checked.
fn land_as(dir: &Path, commit: &str, fingerprint: &str, kind: LandKind, review: bool) -> Body {
    let key = clm_ledger::code_key(dir, commit).unwrap();
    clm_ledger::append(dir, checked(&key), &env(dir, &[])).unwrap();
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
    // req:ledger/verify-recomputes-review@5890d0dc
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
fn review_spec_recorded_as_code() {
    // req:ledger/verify-recomputes-review@5890d0dc
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
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(
        out.contains("ledger/understated-land")
            && out.contains(&merge)
            && out.contains("recorded code, the gate gives spec review"),
        "{out}"
    );
}

#[test]
fn review_honest_and_over_stated_lands() {
    // req:ledger/verify-recomputes-review@5890d0dc
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
    // req:ledger/verify-recomputes-review@5890d0dc
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
    // req:ledger/verify-recomputes-review@5890d0dc
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
    // req:ledger/verify-recomputes-review@5890d0dc
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let _keys = signed_main_with_specs(root, false);
    let (merge, fingerprint) = architecture_change(root);
    let land = land_as(root, &merge, &fingerprint, LandKind::Spec, false);
    clm_ledger::append(root, land, &env(root, &[])).unwrap();
    let (code, out, err) = clm(root, root, &["ledger", "verify", "--review"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
}

#[test]
fn review_ranks_code_with_spec() {
    // req:ledger/verify-recomputes-review@5890d0dc
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
    // req:ledger/verify-recomputes-review@5890d0dc
    // On a signed `main`, an unsigned record understating a diff that needs review is only
    // unsigned.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let _keys = signed_main_with_specs(root, true);
    let (merge, fingerprint) = architecture_change(root);
    let land = land_as(root, &merge, &fingerprint, LandKind::Spec, false);
    clm_ledger::append(root, land, &env(root, &[])).unwrap();
    let (code, out, err) = clm(root, root, &["ledger", "verify", "--review"]);
    assert_eq!(code, Some(1), "{out}{err}");
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(
        out.contains("ledger/unsigned-land") && out.contains(&merge),
        "{out}"
    );
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
    let (code, out, err) = clm(dir, dir, &["fingerprint", "--base", "main"]);
    assert_eq!(code, Some(0), "{err}");
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
    // req:ledger/verify-unrated-land@48a0dc74
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
    // req:ledger/verify-unrated-land@48a0dc74
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
    // req:ledger/verify-unrated-land@48a0dc74
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
    let p = clm_ledger::append(root, proposed("export-csv"), &env).unwrap();
    tick();
    let end = Body::Withdrawn {
        proposal: p.record.id.clone(),
    };
    let w = clm_ledger::append(root, end, &env).unwrap();
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
    let p = clm_ledger::append(&a, proposed("dark-mode"), &env).unwrap();
    let end = Body::Withdrawn {
        proposal: p.record.id.clone(),
    };
    clm_ledger::append(&a, end, &env).unwrap();
    clm_ledger::append(&a, approved("f"), &env).unwrap();
    tick();
    clm_ledger::append(&a, landed("c", "f", Some("m")), &env).unwrap();
    let (code, out, err) = clm(&a, home, &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 1");
    let text = records(&a, &format!("clm/ledger/{}", p.record.writer));
    assert!(text.contains(r#""kind":"proposed""#), "{text}");
    assert!(text.contains(r#""kind":"withdrawn""#), "{text}");
    assert!(!text.contains(r#""kind":"approved""#), "{text}");
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
    let r = clm_ledger::append(root, r, &env).unwrap();
    tick();
    // Each run of whitespace holding a line break is one space; others stay.
    let c = commented("f", None, "Why  this?\r\n\r\nAnd that.\u{2028}Done.");
    let c = clm_ledger::append(root, c, &env).unwrap();
    tick();
    let t = clm_ledger::append(root, commented("f", Some("n/r"), "ok"), &env).unwrap();
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
    let mine = clm_ledger::append(&a, commented("f", Some("n/r"), "Why?"), &env).unwrap();
    clm_ledger::append(&a, changes_requested("f", "Split it"), &env).unwrap();
    // Whatever its branch, feedback at a landed fingerprint goes.
    let elsewhere = Body::Commented {
        branch: "spec/y".into(),
        fingerprint: "g".into(),
        target: None,
        text: "Keep me".into(),
    };
    clm_ledger::append(&a, elsewhere, &env).unwrap();
    tick();
    let mut land = landed("c", "f", Some("m"));
    if let Body::Landed { branch, .. } = &mut land {
        *branch = "spec/renamed".into();
    }
    clm_ledger::append(&b, land, &env).unwrap();
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
    clm_ledger::append(root, note, &env).unwrap();
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
    // req:ledger/effort-records@2bee0a30
    // req:ledger/decision-records@9ca8cb93
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let env = env(root, alices(root));
    let e = clm_ledger::append(root, effort(5), &env).unwrap();
    tick();
    let d = clm_ledger::append(root, decided("capability/x"), &env).unwrap();
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
    // req:ledger/effort-records@2bee0a30
    // req:ledger/decision-records@9ca8cb93
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let env = env(home, &[]);
    let early = clm_ledger::append(&a, effort(5), &env).unwrap();
    clm_ledger::append(&a, decided("capability/x"), &env).unwrap();
    tick();
    let late = clm_ledger::append(&a, effort(50), &env).unwrap();
    clm_ledger::append(&a, decided("capability/x"), &env).unwrap();
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
    // req:ledger/effort-records@2bee0a30
    // Two of the caller's own counts with one `id` and one `at`: one goes, whichever it is.
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let env = env(home, &[]);
    let first = clm_ledger::append(&a, effort(5), &env).unwrap();
    let branch = format!("clm/ledger/{}", first.record.writer);
    let line = records(&a, &branch).replace(r#""output":5"#, r#""output":6"#);
    append_line(&a, &branch, &line);
    let (code, out, err) = clm(&a, home, &["ledger", "compact"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out.trim(), "removed 1");
    let text = records(&a, &branch);
    assert_eq!(text.matches(r#""kind":"effort""#).count(), 1, "{text}");
}
