mod common;

use clm_ledger::{Body, fold_records, show};
use common::{clone_with_origin, env};

fn id(c: char) -> String {
    c.to_string().repeat(40)
}

/// A `mutated` line for `commit`, written as a ledger holds it.
fn line(n: u32, commit: char, at: &str, killed: &str, unproven: &str, judged: &str) -> String {
    format!(
        r#"{{"id":"m{n}","kind":"mutated","writer":"alice/1","at":"{at}","commit":"{}","since":null,"killed":{killed},"unproven":{unproven},"not_judged":{judged}}}"#,
        id(commit),
    )
}

fn plain(n: u32, commit: char, at: &str) -> String {
    line(n, commit, at, "[]", "[]", "[]")
}

fn body(commit: char, unproven: &[&str]) -> Body {
    Body::Mutated {
        commit: id(commit),
        since: None,
        killed: vec![],
        unproven: unproven.iter().map(|s| s.to_string()).collect(),
        not_judged: vec![],
    }
}

#[test]
fn folded_and_shown() {
    // req:ledger/mutated-record@23c5c318
    let text = line(
        1,
        'c',
        "2026-01-01T00:00:00.000Z",
        r#"["a/x"]"#,
        r#"["b/y@0123abcd"]"#,
        "[]",
    );
    let fold = fold_records(&[("one", &text)]);
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    assert_eq!(fold.mutations[&id('c')].len(), 1);
    assert_eq!(
        show(&fold),
        [format!(
            "mutated {} 1 killed 1 unproven 0 not judged alice/1 2026-01-01T00:00:00.000Z",
            id('c')
        )]
    );
}

#[test]
fn shown_counts_each_list() {
    // req:ledger/mutated-record@23c5c318
    let text = line(
        1,
        'c',
        "2026-01-01T00:00:00.000Z",
        r#"["a/x","a/y"]"#,
        r#"["b/y@0123abcd"]"#,
        r#"["c/z","c/w","c/v"]"#,
    );
    assert_eq!(
        show(&fold_records(&[("one", &text)])),
        [format!(
            "mutated {} 2 killed 1 unproven 3 not judged alice/1 2026-01-01T00:00:00.000Z",
            id('c')
        )]
    );
}

#[test]
fn newest_first_by_commit() {
    // req:ledger/mutated-record@23c5c318
    let text = [
        plain(1, 'c', "2026-01-01T00:00:00.000Z"),
        plain(3, 'd', "2026-01-03T00:00:00.000Z"),
        plain(2, 'c', "2026-01-02T00:00:00.000Z"),
    ]
    .join("\n");
    for fold in [
        fold_records(&[("one", &text)]),
        fold_records(&[("one", &text.lines().rev().collect::<Vec<_>>().join("\n"))]),
    ] {
        let c: Vec<_> = fold.mutations[&id('c')].iter().map(|r| &r.id).collect();
        assert_eq!(c, ["m2", "m1"]);
        assert_eq!(fold.mutations[&id('d')].len(), 1);
    }
}

#[test]
fn bad_entry() {
    // req:ledger/mutated-record@23c5c318
    let at = "2026-01-01T00:00:00.000Z";
    let bad = [
        line(1, 'c', at, "[]", r#"["a/x"]"#, "[]"),
        line(2, 'c', at, "[]", r#"["a/x@0123ABCD"]"#, "[]"),
        line(3, 'c', at, "[]", r#"["a/x@0123abc"]"#, "[]"),
        line(4, 'c', at, "[]", r#"["x@0123abcd"]"#, "[]"),
        line(13, 'c', at, "[]", r#"["a/x@0123abcde"]"#, "[]"),
        line(5, 'c', at, r#"["a/x@0123abcd"]"#, "[]", "[]"),
        line(6, 'c', at, r#"["nope"]"#, "[]", "[]"),
        line(7, 'c', at, "[]", "[]", r#"[""]"#),
        line(8, 'c', at, "[]", "[]", r#""a/x""#),
        plain(9, 'c', at).replace(&id('c'), &"C".repeat(40)),
        plain(10, 'c', at).replace(&id('c'), "abc123"),
        plain(11, 'c', at).replace("\"since\":null", "\"since\":\"zz\""),
        plain(12, 'c', at),
    ]
    .join("\n");
    let fold = fold_records(&[("one", &bad)]);
    assert_eq!(fold.warnings.len(), 12, "{:?}", fold.warnings);
    assert!(fold.warnings[0].contains("line 1"), "{:?}", fold.warnings);
    assert_eq!(fold.mutations[&id('c')].len(), 1);
}

#[test]
fn since_may_be_a_commit() {
    // req:ledger/mutated-record@23c5c318
    let text = plain(1, 'c', "2026-01-01T00:00:00.000Z")
        .replace("\"since\":null", &format!("\"since\":\"{}\"", id('a')));
    let fold = fold_records(&[("one", &text)]);
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    assert_eq!(fold.mutations.len(), 1);
}

#[test]
fn appended_unsigned_even_in_a_signed_repository() {
    // req:ledger/mutated-record@23c5c318
    let dir = tempfile::tempdir().unwrap();
    let a = clone_with_origin(dir.path(), "a", "A");
    std::fs::create_dir_all(a.join("docs")).unwrap();
    std::fs::write(a.join("docs/allowed_signers"), "alice ssh-ed25519 AAAA\n").unwrap();
    let home = dir.path();
    for args in [&["add", "docs"][..], &["commit", "-qm", "signers"][..]] {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(&a)
            .status()
            .unwrap();
        assert!(status.success());
    }
    assert!(clm_ledger::signed(&a, "HEAD").unwrap());
    let done = clm_ledger::append(&a, body('c', &[]), &env(home, &[])).unwrap();
    let fold = clm_ledger::fold(&a).unwrap();
    assert_eq!(fold.mutations[&id('c')][0].id, done.record.id);
    assert!(!fold.lines[&done.record.id].contains_key("sig"));
}

#[test]
fn compacted() {
    // req:ledger/mutated-record@23c5c318
    let dir = tempfile::tempdir().unwrap();
    let a = clone_with_origin(dir.path(), "a", "A");
    let b = clone_with_origin(dir.path(), "b", "B");
    let home = dir.path();
    let app = |root: &std::path::Path, b: Body| {
        clm_ledger::append(root, b, &env(home, &[])).unwrap().record
    };
    let old_clean = app(&a, body('c', &[]));
    let unproven = app(&a, body('c', &["a/x@0123abcd"]));
    let newest = app(&a, body('c', &[]));
    let other_commit = app(&a, body('d', &[]));
    // Another writer's record is not the caller's to drop.
    let theirs = app(&b, body('c', &[]));
    let done = clm_ledger::compact(&a, &env(home, &[])).unwrap();
    assert_eq!(done.removed, 1, "{:?}", done.warnings);
    let fold = clm_ledger::fold(&a).unwrap();
    let mut ids: Vec<_> = fold.mutations[&id('c')].iter().map(|r| &r.id).collect();
    ids.sort();
    let mut want = [&unproven.id, &newest.id, &theirs.id];
    want.sort();
    assert_eq!(ids, want);
    assert!(!ids.contains(&&old_clean.id));
    assert_eq!(fold.mutations[&id('d')][0].id, other_commit.id);
}
