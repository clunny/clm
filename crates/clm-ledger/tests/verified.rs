mod common;

use clm_ledger::{Body, VerifyMode, fold_records, show};
use common::{clone_with_origin, env};

fn id(c: char) -> String {
    c.to_string().repeat(40)
}

/// A `verified` line for `commit`, written as a ledger holds it.
fn line(n: u32, commit: char, passed: &str, at: &str, culprit: Option<char>) -> String {
    let culprit = culprit.map_or("null".to_string(), |c| format!("\"{}\"", id(c)));
    format!(
        r#"{{"id":"v{n}","kind":"verified","writer":"alice/1","at":"{at}","commit":"{}","passed":{passed},"since":null,"lands":["{}"],"culprit":{culprit},"components":[],"mode":"local","run":null}}"#,
        id(commit),
        id(commit),
    )
}

fn body(commit: char, passed: bool) -> Body {
    Body::Verified {
        commit: id(commit),
        passed,
        since: None,
        lands: vec![id(commit)],
        culprit: None,
        components: vec![],
        mode: VerifyMode::Local,
        run: None,
    }
}

#[test]
fn by_commit() {
    // req:ledger/verified-record@e728354d
    let text = [
        // Read newest first, so the order is by `at`, not by line.
        line(3, 'd', "true", "2026-01-03T00:00:00.000Z", None),
        line(2, 'c', "false", "2026-01-02T00:00:00.000Z", Some('a')),
        line(1, 'c', "true", "2026-01-01T00:00:00.000Z", None),
    ]
    .join("\n");
    let fold = fold_records(&[("one", &text)]);
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    let c: Vec<_> = fold.verifications[&id('c')].iter().map(|r| &r.id).collect();
    assert_eq!(c, ["v2", "v1"]);
    let Body::Verified {
        passed, culprit, ..
    } = &fold.verifications[&id('c')][0].body
    else {
        panic!("not verified");
    };
    assert!(!passed);
    assert_eq!(culprit.as_deref(), Some(id('a').as_str()));
    assert_eq!(fold.verifications[&id('d')].len(), 1);
}

#[test]
fn uppercase_commit() {
    // req:ledger/verified-record@e728354d
    let upper =
        line(1, 'a', "true", "2026-01-01T00:00:00.000Z", None).replace(&id('a'), &"A".repeat(40));
    let fold = fold_records(&[("one", &upper)]);
    assert!(fold.verifications.is_empty());
    assert_eq!(fold.warnings.len(), 1, "{:?}", fold.warnings);
}

#[test]
fn bad_result() {
    // req:ledger/verified-record@e728354d
    let ok = id('c');
    let bad = [
        line(1, 'c', "\"yes\"", "2026-01-01T00:00:00.000Z", None),
        line(2, 'c', "true", "2026-01-01T00:00:00.000Z", None).replace("local", "remote"),
        line(3, 'c', "true", "2026-01-01T00:00:00.000Z", None).replacen(&ok, "abc123", 1),
        line(8, 'c', "true", "2026-01-01T00:00:00.000Z", None)
            .replace("\"lands\":[\"", "\"lands\":[\"x"),
        line(4, 'c', "true", "2026-01-01T00:00:00.000Z", None)
            .replace("\"culprit\":null", "\"culprit\":\"zz\""),
        line(5, 'c', "true", "2026-01-01T00:00:00.000Z", None).replace(
            "\"since\":null",
            &format!("\"since\":\"{}\"", id('A').replace('A', "g")),
        ),
        // Git writes IDs in lowercase, and records are looked up by exact string.
        line(7, 'c', "true", "2026-01-01T00:00:00.000Z", None).replace(&ok, &"C".repeat(40)),
        line(6, 'c', "true", "2026-01-01T00:00:00.000Z", None),
    ]
    .join("\n");
    let fold = fold_records(&[("one", &bad)]);
    assert_eq!(fold.warnings.len(), 7, "{:?}", fold.warnings);
    assert!(fold.warnings[0].contains("line 1"), "{:?}", fold.warnings);
    assert_eq!(fold.verifications[&id('c')].len(), 1);
}

#[test]
fn shown() {
    // req:ledger/verified-record@e728354d
    let text = [
        line(1, 'c', "true", "2026-01-01T00:00:00.000Z", None),
        line(2, 'd', "false", "2026-01-02T00:00:00.000Z", None),
    ]
    .join("\n");
    let lines = show(&fold_records(&[("one", &text)]));
    assert_eq!(
        lines,
        [
            format!(
                "verified {} passed alice/1 2026-01-01T00:00:00.000Z",
                id('c')
            ),
            format!(
                "verified {} failed alice/1 2026-01-02T00:00:00.000Z",
                id('d')
            ),
        ]
    );
}

#[test]
fn unresolved_reds() {
    // req:ledger/verified-record@e728354d
    let text = [
        line(1, 'a', "true", "2026-01-01T00:00:00.000Z", None),
        line(2, 'b', "false", "2026-01-02T00:00:00.000Z", None),
        line(3, 'c', "false", "2026-01-03T00:00:00.000Z", None),
        line(4, 'c', "true", "2026-01-04T00:00:00.000Z", None),
        line(5, 'd', "false", "2026-01-05T00:00:00.000Z", None),
    ]
    .join("\n");
    let fold = fold_records(&[("one", &text)]);
    let chain: Vec<String> = "abcd".chars().map(id).collect();
    let reds = fold.unresolved_reds(&chain);
    assert_eq!(reds.iter().map(|r| &r.id).collect::<Vec<_>>(), ["v5"]);
    // A red after the last pass stays unresolved.
    let reds = fold.unresolved_reds(&chain[..2]);
    assert_eq!(reds.iter().map(|r| &r.id).collect::<Vec<_>>(), ["v2"]);
    // With no passing result at all, every red commit counts.
    let none = fold_records(&[(
        "one",
        &line(2, 'b', "false", "2026-01-02T00:00:00.000Z", None),
    )]);
    let reds = none.unresolved_reds(&chain);
    assert_eq!(reds.iter().map(|r| &r.id).collect::<Vec<_>>(), ["v2"]);
    // Several reds come back in chain order, whatever order they were recorded in.
    let text = [
        line(5, 'd', "false", "2026-01-01T00:00:00.000Z", None),
        line(2, 'b', "false", "2026-01-02T00:00:00.000Z", None),
    ]
    .join("\n");
    let reds = fold_records(&[("one", &text)]).unresolved_reds(&chain);
    assert_eq!(reds.iter().map(|r| &r.id).collect::<Vec<_>>(), ["v2", "v5"]);
}

#[test]
fn compacted() {
    // req:ledger/verified-record@e728354d
    let dir = tempfile::tempdir().unwrap();
    let a = clone_with_origin(dir.path(), "a", "A");
    let b = clone_with_origin(dir.path(), "b", "B");
    let home = dir.path();
    clm_ledger::append(&a, body('c', false), &env(home, &[])).unwrap();
    clm_ledger::append(&a, body('d', true), &env(home, &[])).unwrap();
    let newer = clm_ledger::append(&a, body('c', true), &env(home, &[])).unwrap();
    // Another writer's later result for `c` is not the caller's newest, and stays.
    let other = clm_ledger::append(&b, body('c', false), &env(home, &[])).unwrap();
    let done = clm_ledger::compact(&a, &env(home, &[])).unwrap();
    assert_eq!(done.removed, 1, "{:?}", done.warnings);
    let fold = clm_ledger::fold(&a).unwrap();
    let mut ids: Vec<_> = fold.verifications[&id('c')].iter().map(|r| &r.id).collect();
    ids.sort();
    let mut want = [&newer.record.id, &other.record.id];
    want.sort();
    assert_eq!(ids, want);
    assert_eq!(fold.verifications[&id('d')].len(), 1);
}
