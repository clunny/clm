mod common;

use clm_ledger::{Body, LandKind, Recheck, fold_records};
use common::{clone_with_origin, env, sh};

fn landed(commit: &str, recheck: Option<Recheck>, recheck_files: Option<u64>) -> Body {
    Body::Landed {
        commit: commit.into(),
        branch: "impl/x".into(),
        kind: LandKind::Code,
        review: false,
        fingerprint: "f".into(),
        code_key: None,
        changes: vec![],
        check_seconds: Some(4),
        steps: vec![],
        recheck,
        recheck_files,
    }
}

#[test]
fn a_land_that_ran_nothing() {
    // req:ledger/landed-recheck@f461db3d
    let dir = tempfile::tempdir().unwrap();
    let a = clone_with_origin(dir.path(), "a", "A");
    let body = landed("c", Some(Recheck::None), None);
    let appended = clm_ledger::append(&a, body, &env(dir.path(), &[])).unwrap();
    let branch = format!("clm/ledger/{}", appended.record.writer);
    let text = sh(&a, &["show", &format!("{branch}:records.jsonl")]);
    assert!(text.contains(r#""recheck":"none""#), "{text}");
    assert!(!text.contains("recheck_files"), "{text}");
    let fold = clm_ledger::fold(&a).unwrap();
    assert!(matches!(
        &fold.lands["c"][0].body,
        Body::Landed {
            recheck: Some(Recheck::None),
            recheck_files: None,
            ..
        }
    ));
    let timed = fold.timed_lands(1);
    assert_eq!(timed[0].recheck, Some(Recheck::None));
    assert_eq!(timed[0].recheck_files, None);
}

#[test]
fn every_recheck_word_and_a_file_count_round_trip() {
    // req:ledger/landed-recheck@f461db3d
    let dir = tempfile::tempdir().unwrap();
    let a = clone_with_origin(dir.path(), "a", "A");
    for (commit, recheck, files, word) in [
        ("c1", Recheck::Checked, None, "checked"),
        ("c2", Recheck::Overlap, Some(7), "overlap"),
        ("c3", Recheck::Selected, Some(3), "selected"),
    ] {
        let appended = clm_ledger::append(
            &a,
            landed(commit, Some(recheck), files),
            &env(dir.path(), &[]),
        )
        .unwrap();
        let branch = format!("clm/ledger/{}", appended.record.writer);
        let text = sh(&a, &["show", &format!("{branch}:records.jsonl")]);
        assert!(text.contains(&format!(r#""recheck":"{word}""#)), "{text}");
    }
    let fold = clm_ledger::fold(&a).unwrap();
    let got: Vec<_> = fold
        .timed_lands(3)
        .iter()
        .map(|l| (l.commit.clone(), l.recheck, l.recheck_files))
        .collect();
    assert_eq!(
        got,
        [
            ("c1".into(), Some(Recheck::Checked), None),
            ("c2".into(), Some(Recheck::Overlap), Some(7)),
            ("c3".into(), Some(Recheck::Selected), Some(3)),
        ]
    );
}

#[test]
fn an_older_record() {
    // req:ledger/landed-recheck@f461db3d
    let line = r#"{"id":"1","writer":"a/1","at":"2026-01-01T00:00:00.000Z","kind":"landed","commit":"c","branch":"impl/x","land_kind":"code","review":false,"fingerprint":"f","changes":[],"check_seconds":3}"#;
    let fold = fold_records(&[("one", line)]);
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    assert!(matches!(
        &fold.lands["c"][0].body,
        Body::Landed {
            recheck: None,
            recheck_files: None,
            ..
        }
    ));
    let timed = fold.timed_lands(1);
    assert_eq!((timed[0].recheck, timed[0].recheck_files), (None, None));
}
