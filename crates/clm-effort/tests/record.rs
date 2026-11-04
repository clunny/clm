//! `effort/record-command`: `clm effort record`'s library call.

#[path = "../../clm-ledger/tests/common/mod.rs"]
mod common;

use std::path::{Path, PathBuf};

use clm_ledger::Body;
use common::{clone_with_origin, env, sh};

fn encoded(dir: &Path) -> String {
    dir.to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// One entry of session `sid` on `branch` run in `cwd`, `secs` seconds into the day.
fn entry(sid: &str, branch: &str, cwd: &Path, secs: u64, id: &str) -> String {
    serde_json::json!({
        "sessionId": sid,
        "gitBranch": branch,
        "cwd": cwd,
        "timestamp": format!("2026-10-30T00:{:02}:{:02}.000Z", secs / 60, secs % 60),
        "message": {
            "id": id,
            "model": "claude-opus-5-5",
            "usage": {"input_tokens": 10, "output_tokens": 5, "cache_creation_input_tokens": 20},
        },
    })
    .to_string()
}

/// Appends `lines` to `<home>/.claude/projects/<encoded cwd>/<sid>.jsonl`.
fn transcript(home: &Path, cwd: &Path, sid: &str, lines: &[String]) {
    let path = home
        .join(".claude/projects")
        .join(encoded(cwd))
        .join(format!("{sid}.jsonl"));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut text = std::fs::read_to_string(&path).unwrap_or_default();
    text.push_str(&(lines.join("\n") + "\n"));
    std::fs::write(path, text).unwrap();
}

/// A CLM repository cloned from a bare `origin`, its `HOME` the temporary directory.
fn repository(dir: &Path) -> PathBuf {
    let clone = clone_with_origin(dir, "a", "Ann");
    std::fs::create_dir_all(clone.join("specs")).unwrap();
    std::fs::write(clone.join("specs/SPEC.md"), "---\ntitle: T\n---\n").unwrap();
    clone
}

fn ledger_tip(clone: &Path) -> String {
    sh(
        clone,
        &[
            "for-each-ref",
            "--format=%(objectname)",
            "refs/heads/clm/ledger/",
        ],
    )
}

#[test]
fn record_appends_one_commit() {
    // req:effort/record-command@2365b153
    let dir = tempfile::tempdir().unwrap();
    let clone = repository(dir.path());
    transcript(
        dir.path(),
        &clone,
        "s1",
        &[
            entry("s1", "impl/a", &clone, 0, "m1"),
            entry("s1", "impl/a", &clone, 60, "m2"),
        ],
    );
    transcript(
        dir.path(),
        &clone,
        "s2",
        &[entry("s2", "impl/a", &clone, 0, "m3")],
    );
    let env = env(dir.path(), &[]);
    let out = clm_effort::record(&clone, &clm_effort::Options::default(), &env).unwrap();
    assert_eq!(
        out.lines,
        [
            "effort impl/a s1 70 tokens 60s",
            "effort impl/a s2 35 tokens 0s",
            "recorded 2 sessions on 1 branches",
        ],
        "{:?}",
        out.warnings
    );
    let branch = sh(
        &clone,
        &[
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/heads/clm/ledger/",
        ],
    );
    assert_eq!(sh(&clone, &["rev-list", "--count", &branch]), "1");
    let fold = clm_ledger::fold(&clone).unwrap();
    for sid in ["s1", "s2"] {
        let record = &fold.efforts[&clm_ledger::effort_id(sid, "impl/a")];
        assert!(matches!(&record.body, Body::Effort { session, .. } if session == sid));
    }
    // Pushed.
    sh(&clone, &["fetch", "-q", "origin"]);
    assert_eq!(
        sh(&clone, &["rev-parse", &format!("origin/{branch}")]),
        ledger_tip(&clone)
    );
}

#[test]
fn rerun_appends_nothing() {
    // req:effort/record-command@2365b153
    let dir = tempfile::tempdir().unwrap();
    let clone = repository(dir.path());
    transcript(
        dir.path(),
        &clone,
        "s1",
        &[entry("s1", "impl/a", &clone, 0, "m1")],
    );
    let env = env(dir.path(), &[]);
    clm_effort::record(&clone, &clm_effort::Options::default(), &env).unwrap();
    let tip = ledger_tip(&clone);
    let out = clm_effort::record(&clone, &clm_effort::Options::default(), &env).unwrap();
    assert_eq!(out.lines, ["nothing new to record"]);
    assert_eq!(ledger_tip(&clone), tip);
}

#[test]
fn more_work_appends_again() {
    // req:effort/record-command@2365b153
    let dir = tempfile::tempdir().unwrap();
    let clone = repository(dir.path());
    transcript(
        dir.path(),
        &clone,
        "s1",
        &[entry("s1", "impl/a", &clone, 0, "m1")],
    );
    let env = env(dir.path(), &[]);
    clm_effort::record(&clone, &clm_effort::Options::default(), &env).unwrap();
    transcript(
        dir.path(),
        &clone,
        "s1",
        &[entry("s1", "impl/a", &clone, 30, "m2")],
    );
    let out = clm_effort::record(&clone, &clm_effort::Options::default(), &env).unwrap();
    assert_eq!(
        out.lines,
        [
            "effort impl/a s1 70 tokens 30s",
            "recorded 1 sessions on 1 branches"
        ]
    );
    let fold = clm_ledger::fold(&clone).unwrap();
    let Body::Effort {
        active_seconds,
        models,
        ..
    } = &fold.efforts[&clm_ledger::effort_id("s1", "impl/a")].body
    else {
        panic!()
    };
    assert_eq!((*active_seconds, models["claude-opus-5-5"].input), (30, 20));
}

#[test]
fn branch_filter() {
    // req:effort/record-command@2365b153
    let dir = tempfile::tempdir().unwrap();
    let clone = repository(dir.path());
    transcript(
        dir.path(),
        &clone,
        "s1",
        &[
            entry("s1", "impl/a", &clone, 0, "m1"),
            entry("s1", "impl/b", &clone, 10, "m2"),
        ],
    );
    let env = env(dir.path(), &[]);
    let options = clm_effort::Options {
        branch: Some("impl/a".into()),
        ..Default::default()
    };
    let out = clm_effort::record(&clone, &options, &env).unwrap();
    assert_eq!(
        out.lines,
        [
            "effort impl/a s1 35 tokens 0s",
            "recorded 1 sessions on 1 branches"
        ]
    );
    let fold = clm_ledger::fold(&clone).unwrap();
    assert!(
        !fold
            .efforts
            .contains_key(&clm_ledger::effort_id("s1", "impl/b"))
    );
}

#[test]
fn linked_worktree_counted() {
    // req:effort/record-command@2365b153
    let dir = tempfile::tempdir().unwrap();
    let clone = repository(dir.path());
    let linked = dir.path().join("linked");
    sh(
        &clone,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "impl/a",
            linked.to_str().unwrap(),
        ],
    );
    transcript(
        dir.path(),
        &linked,
        "s1",
        &[entry("s1", "impl/a", &linked, 0, "m1")],
    );
    // Elsewhere, and not given: not counted.
    let elsewhere = dir.path().join("elsewhere");
    transcript(
        dir.path(),
        &elsewhere,
        "s2",
        &[entry("s2", "impl/a", &elsewhere, 0, "m2")],
    );
    let env = env(dir.path(), &[]);
    let out = clm_effort::record(&clone, &clm_effort::Options::default(), &env).unwrap();
    assert_eq!(
        out.lines,
        [
            "effort impl/a s1 35 tokens 0s",
            "recorded 1 sessions on 1 branches"
        ]
    );
    // Given with `--cwd`, it is.
    let options = clm_effort::Options {
        cwds: vec![elsewhere],
        ..Default::default()
    };
    let out = clm_effort::record(&clone, &options, &env).unwrap();
    assert_eq!(
        out.lines,
        [
            "effort impl/a s2 35 tokens 0s",
            "recorded 1 sessions on 1 branches"
        ]
    );
}

#[test]
fn outside_a_clm_repository_refused() {
    // req:effort/record-command@2365b153
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_with_origin(dir.path(), "a", "Ann");
    let env = env(dir.path(), &[]);
    let e = clm_effort::record(&clone, &clm_effort::Options::default(), &env).unwrap_err();
    assert_eq!(e.exit_code(), 1, "{e}");
}

#[test]
fn less_seen_never_shrinks() {
    // req:effort/record-command@2365b153
    let dir = tempfile::tempdir().unwrap();
    let clone = repository(dir.path());
    let elsewhere = dir.path().join("elsewhere");
    transcript(
        dir.path(),
        &clone,
        "s1",
        &[entry("s1", "impl/a", &clone, 0, "m1")],
    );
    transcript(
        dir.path(),
        &elsewhere,
        "s1",
        &[entry("s1", "impl/a", &elsewhere, 30, "m2")],
    );
    let env = env(dir.path(), &[]);
    let options = clm_effort::Options {
        cwds: vec![elsewhere],
        ..Default::default()
    };
    let out = clm_effort::record(&clone, &options, &env).unwrap();
    assert_eq!(
        out.lines,
        [
            "effort impl/a s1 70 tokens 30s",
            "recorded 1 sessions on 1 branches"
        ]
    );
    let tip = ledger_tip(&clone);
    let out = clm_effort::record(&clone, &clm_effort::Options::default(), &env).unwrap();
    assert_eq!(out.lines, ["nothing new to record"]);
    assert_eq!(ledger_tip(&clone), tip);
}

#[test]
fn claude_config_dir_is_read_first() {
    // req:effort/record-command@2365b153
    let dir = tempfile::tempdir().unwrap();
    let clone = repository(dir.path());
    let config = dir.path().join("config");
    // Under `HOME`, a session that isn't read when `CLAUDE_CONFIG_DIR` is set.
    transcript(
        dir.path(),
        &clone,
        "s1",
        &[entry("s1", "impl/a", &clone, 0, "m1")],
    );
    let path = config
        .join("projects")
        .join(encoded(&clone))
        .join("s2.jsonl");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, entry("s2", "impl/a", &clone, 0, "m2") + "\n").unwrap();
    let vars = [("CLAUDE_CONFIG_DIR", config.to_str().unwrap())];
    let env = env(dir.path(), &vars);
    let out = clm_effort::record(&clone, &clm_effort::Options::default(), &env).unwrap();
    assert_eq!(
        out.lines,
        [
            "effort impl/a s2 35 tokens 0s",
            "recorded 1 sessions on 1 branches"
        ]
    );
}
