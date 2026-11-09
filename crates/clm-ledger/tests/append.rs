mod common;

use clm_ledger::Body;
use common::{clone_with_origin, env, sh};

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
        command: "ci-check.sh".into(),
        basis: None,
    }
}

fn lines(text: &str) -> Vec<serde_json::Value> {
    text.lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[test]
fn first_record() {
    // req:ledger/append@d894aff0
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_with_origin(dir.path(), "a", "Ann");
    let head = sh(&clone, &["rev-parse", "HEAD"]);
    let out = clm_ledger::append(&clone, approved("f"), &env(dir.path(), &[])).unwrap();
    assert!(out.pushed);
    let branch = format!("clm/ledger/{}", out.record.writer);
    let origin = dir.path().join("origin.git");
    let records = lines(&sh(&origin, &["show", &format!("{branch}:records.jsonl")]));
    assert_eq!(records.len(), 1);
    let r = &records[0];
    assert_eq!(r["kind"], "approved");
    assert_eq!(r["fingerprint"], "f");
    assert_eq!(r["change"], "x");
    assert_eq!(r["writer"], out.record.writer.as_str());
    assert!(r["writer"].as_str().unwrap().starts_with("ann/"));
    let at = r["at"].as_str().unwrap();
    assert!(
        at.len() == 24 && at.ends_with('Z') && at.as_bytes()[10] == b'T',
        "{at}"
    );
    let id = r["id"].as_str().unwrap();
    assert!(
        id.len() == 32 && id.chars().all(|c| c.is_ascii_hexdigit()),
        "{id}"
    );
    // An orphan branch holding only the one file.
    assert_eq!(sh(&origin, &["rev-list", "--count", &branch]), "1");
    assert_eq!(
        sh(&origin, &["ls-tree", "--name-only", &branch]),
        "records.jsonl"
    );
    // The working tree, the index and HEAD are untouched.
    assert_eq!(sh(&clone, &["status", "--porcelain"]), "");
    assert_eq!(sh(&clone, &["rev-parse", "HEAD"]), head);
    assert_eq!(sh(&clone, &["branch", "--show-current"]), "main");
}

#[test]
fn second_record() {
    // req:ledger/append@d894aff0
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_with_origin(dir.path(), "a", "Ann");
    let env = env(dir.path(), &[]);
    let first = clm_ledger::append(&clone, approved("f"), &env).unwrap();
    let second = clm_ledger::append(&clone, checked("k"), &env).unwrap();
    assert!(second.pushed);
    assert_ne!(first.record.id, second.record.id);
    let branch = format!("clm/ledger/{}", first.record.writer);
    let origin = dir.path().join("origin.git");
    let records = lines(&sh(&origin, &["show", &format!("{branch}:records.jsonl")]));
    let kinds: Vec<_> = records.iter().map(|r| r["kind"].clone()).collect();
    assert_eq!(kinds, ["approved", "checked"]);
    assert_eq!(records[1]["code_key"], "k");
    assert_eq!(sh(&origin, &["rev-list", "--count", &branch]), "2");
}

#[test]
fn offline() {
    // req:ledger/append@d894aff0
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_with_origin(dir.path(), "a", "Ann");
    let nowhere = dir.path().join("nowhere.git");
    sh(
        &clone,
        &["remote", "set-url", "origin", nowhere.to_str().unwrap()],
    );
    let out = clm_ledger::append(&clone, approved("f"), &env(dir.path(), &[])).unwrap();
    assert!(!out.pushed);
    let branch = format!("clm/ledger/{}", out.record.writer);
    let records = lines(&sh(&clone, &["show", &format!("{branch}:records.jsonl")]));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["id"], out.record.id.as_str());
}

#[cfg(unix)]
#[test]
fn push_refused() {
    // req:ledger/append-refused-push@8cdae20f
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_with_origin(dir.path(), "a", "Ann");
    let origin = dir.path().join("origin.git");
    let hook = origin.join("hooks/pre-receive");
    std::fs::create_dir_all(hook.parent().unwrap()).unwrap();
    std::fs::write(&hook, "#!/bin/sh\necho declined by test >&2\nexit 1\n").unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let out = clm_ledger::append(&clone, approved("f"), &env(dir.path(), &[])).unwrap();
    assert!(!out.pushed);
    let branch = format!("clm/ledger/{}", out.record.writer);
    let records = lines(&sh(&clone, &["show", &format!("{branch}:records.jsonl")]));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["id"], out.record.id.as_str());
    assert_eq!(sh(&origin, &["branch", "--list", "clm/*"]), "");
}

#[test]
fn no_host_pushes_to_its_own_remote() {
    // req:ledger/append@d894aff0
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_with_origin(dir.path(), "a", "Ann");
    std::fs::write(clone.join("clm.toml"), "host = \"none\"\n").unwrap();
    let out = clm_ledger::append(&clone, approved("f"), &env(dir.path(), &[])).unwrap();
    assert!(out.pushed);
    let origin = dir.path().join("origin.git");
    let branch = format!("clm/ledger/{}", out.record.writer);
    assert_eq!(sh(&origin, &["branch", "--list", "clm/*"]), branch.as_str());
    let records = lines(&sh(&origin, &["show", &format!("{branch}:records.jsonl")]));
    assert_eq!(records[0]["id"], out.record.id.as_str());
}

#[test]
fn concurrent_appends_keep_every_record() {
    // req:ledger/append@d894aff0
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_with_origin(dir.path(), "a", "Ann");
    std::fs::write(clone.join("clm.toml"), "host = \"none\"\n").unwrap();
    let home = dir.path();
    let ids: Vec<String> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let clone = &clone;
                s.spawn(move || {
                    clm_ledger::append(clone, checked(&format!("k{i}")), &env(home, &[]))
                        .unwrap()
                        .record
                        .id
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let writer = clm_ledger::writer(&clone, &env(home, &[])).unwrap();
    let records = lines(&sh(
        &clone,
        &["show", &format!("clm/ledger/{writer}:records.jsonl")],
    ));
    let mut got: Vec<_> = records
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_string())
        .collect();
    let mut want = ids;
    got.sort();
    want.sort();
    assert_eq!(got, want);
}

fn effort(session: &str, branch: &str) -> Body {
    Body::Effort {
        branch: branch.into(),
        session: session.into(),
        models: [("claude-opus-5-5".to_string(), clm_ledger::Usage::default())].into(),
        active_seconds: 0,
    }
}

#[test]
fn append_all_one_commit() {
    // req:ledger/append-all@86b61f21
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_with_origin(dir.path(), "a", "Ann");
    let env = env(dir.path(), &[]);
    let first = clm_ledger::append(&clone, approved("f"), &env).unwrap();
    let branch = format!("clm/ledger/{}", first.record.writer);
    let tip = sh(&clone, &["rev-parse", &branch]);
    let out = clm_ledger::append_all(
        &clone,
        vec![
            approved("g").into(),
            checked("k").into(),
            effort("s1", "impl/a").into(),
        ],
        &env,
    )
    .unwrap();
    assert_eq!(out.len(), 3);
    assert!(out.iter().all(|a| a.pushed), "{out:?}");
    let new_tip = sh(&clone, &["rev-parse", &branch]);
    assert_eq!(sh(&clone, &["rev-parse", &format!("{new_tip}^")]), tip);
    let origin = dir.path().join("origin.git");
    assert_eq!(sh(&origin, &["rev-parse", &branch]), new_tip);
    let records = lines(&sh(&origin, &["show", &format!("{branch}:records.jsonl")]));
    let kinds: Vec<&str> = records
        .iter()
        .map(|r| r["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["approved", "approved", "checked", "effort"]);
    // No entries, no commit.
    assert!(
        clm_ledger::append_all(&clone, vec![], &env)
            .unwrap()
            .is_empty()
    );
    assert_eq!(sh(&clone, &["rev-parse", &branch]), new_tip);
}

#[test]
fn effort_id_is_stable() {
    // req:ledger/append-all@86b61f21
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_with_origin(dir.path(), "a", "Ann");
    let env = env(dir.path(), &[]);
    let id = clm_ledger::effort_id("s1", "impl/a");
    assert!(
        id.len() == 32 && id.chars().all(|c| c.is_ascii_hexdigit()),
        "{id}"
    );
    assert_ne!(id, clm_ledger::effort_id("s1", "impl/b"));
    // SHA-256 over `s1\nimpl/a`, its first 32 hex digits (`printf 's1\nimpl/a' | sha256sum`).
    assert_eq!(id, "ea99843f057d25185db31ddfb21449c8");
    let one = clm_ledger::append(&clone, effort("s1", "impl/a"), &env).unwrap();
    let two = clm_ledger::append(&clone, effort("s1", "impl/a"), &env).unwrap();
    assert_eq!(
        (one.record.id.as_str(), two.record.id.as_str()),
        (id.as_str(), id.as_str())
    );
}
