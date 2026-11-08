mod common;

use clm_ledger::{Body, LandKind, TimedStep, fold_records};
use common::{clone_with_origin, env, sh};

fn landed(commit: &str, check_seconds: Option<u64>, steps: Vec<TimedStep>) -> Body {
    Body::Landed {
        commit: commit.into(),
        branch: "impl/x".into(),
        kind: LandKind::Code,
        review: false,
        fingerprint: "f".into(),
        code_key: None,
        changes: vec![],
        check_seconds,
        steps,
    }
}

fn line(id: &str, at: &str, commit: &str, times: &str) -> String {
    format!(
        r#"{{"id":"{id}","writer":"a/1","at":"{at}","kind":"landed","commit":"{commit}","branch":"impl/x","land_kind":"code","review":false,"fingerprint":"f","changes":[]{times}}}"#
    )
}

#[test]
fn times_round_trip() {
    // req:ledger/landed-times@e78277f9
    let dir = tempfile::tempdir().unwrap();
    let a = clone_with_origin(dir.path(), "a", "A");
    let step = TimedStep {
        name: "web".into(),
        seconds: 25,
        selected_by: vec!["web/src/main.ts".into()],
    };
    let body = landed("c", Some(30), vec![step.clone()]);
    let appended = clm_ledger::append(&a, body, &env(dir.path(), &[])).unwrap();
    let branch = format!("clm/ledger/{}", appended.record.writer);
    let text = sh(&a, &["show", &format!("{branch}:records.jsonl")]);
    assert!(text.contains(r#""check_seconds":30"#), "{text}");
    let fold = clm_ledger::fold(&a).unwrap();
    let Body::Landed {
        check_seconds,
        steps,
        ..
    } = &fold.lands["c"][0].body
    else {
        panic!("not landed");
    };
    assert_eq!(*check_seconds, Some(30));
    assert_eq!(steps, &[step]);
}

#[test]
fn untimed_records() {
    // req:ledger/landed-times@e78277f9
    let dir = tempfile::tempdir().unwrap();
    let a = clone_with_origin(dir.path(), "a", "A");
    let body = landed("c", None, vec![]);
    let appended = clm_ledger::append(&a, body, &env(dir.path(), &[])).unwrap();
    let branch = format!("clm/ledger/{}", appended.record.writer);
    let text = sh(&a, &["show", &format!("{branch}:records.jsonl")]);
    assert!(!text.contains("check_seconds"), "{text}");
    assert!(!text.contains("steps"), "{text}");
    let fold = clm_ledger::fold(&a).unwrap();
    assert!(matches!(
        &fold.lands["c"][0].body,
        Body::Landed { check_seconds: None, steps, .. } if steps.is_empty()
    ));
    assert!(fold.timed_lands(5).is_empty());
}

#[test]
fn a_step_without_selected_by() {
    // req:ledger/landed-times@e78277f9
    let times = r#","check_seconds":9,"steps":[{"name":"fmt","seconds":2}]"#;
    let fold = fold_records(&[("one", &line("1", "2026-01-01T00:00:00.000Z", "c", times))]);
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    let timed = fold.timed_lands(1);
    assert_eq!(timed[0].steps[0].name, "fmt");
    assert!(timed[0].steps[0].selected_by.is_empty());
}

#[test]
fn the_last_timed_lands() {
    // req:ledger/landed-times@e78277f9
    let t = |n: u64| format!(r#","check_seconds":{n},"steps":[{{"name":"s{n}","seconds":{n}}}]"#);
    // Read out of order: `at` decides, and `b` has no times.
    let text = [
        line("3", "2026-01-03T00:00:00.000Z", "c", &t(3)),
        line("1", "2026-01-01T00:00:00.000Z", "a", &t(1)),
        line("2", "2026-01-02T00:00:00.000Z", "b", ""),
    ]
    .join("\n");
    let fold = fold_records(&[("one", &text)]);
    let timed = fold.timed_lands(2);
    let got: Vec<_> = timed
        .iter()
        .map(|l| {
            (
                l.commit.as_str(),
                l.at.as_str(),
                l.check_seconds,
                l.steps[0].seconds,
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            ("a", "2026-01-01T00:00:00.000Z", 1, 1),
            ("c", "2026-01-03T00:00:00.000Z", 3, 3)
        ]
    );
    assert_eq!(fold.timed_lands(1)[0].commit, "c");
}
