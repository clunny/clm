//! How the culprit land checked its tree, said on stderr (verify/culprit-recheck).

mod common;

use clm_ledger::{Basis, Body, LandKind, Recheck, VerifyMode};
use clm_verify::{Outcome, Request};
use common::Fixture;

const SETTINGS: &str = "mode = \"local\"\ntriggers = []\nevery = { lands = 1 }";

/// `verify-bisect`'s breaking-land fixture: a passing record for `g`, then `l1`, `l2` and
/// `l3`, the check failing on any tree holding `l2`'s change.
fn breaking_lands() -> (Fixture, [String; 3]) {
    let f = Fixture::new(Some(SETTINGS));
    let g = f.land(&[("a.txt", "1\n")], "green");
    f.verified(&g, true);
    let l1 = f.land(&[("crates/a/x.rs", "1\n")], "l1");
    let l2 = f.land(&[("crates/b/y.rs", "2\n")], "l2");
    let l3 = f.land(&[("crates/a/z.rs", "3\n")], "l3");
    f.write("fail-path", "crates/b/y.rs");
    (f, [l1, l2, l3])
}

/// A `landed` record for `commit` with the given re-check.
fn landed(f: &Fixture, commit: &str, recheck: Option<Recheck>, files: Option<u64>) {
    let body = Body::Landed {
        commit: commit.to_string(),
        branch: "impl/x".to_string(),
        kind: LandKind::Code,
        review: false,
        fingerprint: "fp".to_string(),
        code_key: Some("codekey-of-the-land".to_string()),
        changes: vec![],
        check_seconds: None,
        steps: vec![],
        recheck,
        recheck_files: files,
    };
    clm_ledger::append(&f.work(), body, &f.env()).unwrap();
}

/// The composed `checked` record for the land's code key.
fn composed(f: &Fixture, head: &str, base: &str) {
    let body = Body::Checked {
        code_key: "codekey-of-the-land".to_string(),
        command: "check".to_string(),
        basis: Some(Basis {
            head: head.to_string(),
            base: base.to_string(),
        }),
    };
    clm_ledger::append(&f.work(), body, &f.env()).unwrap();
}

fn culprit_notes(out: &Outcome) -> Vec<&String> {
    out.notes
        .iter()
        .filter(|n| n.starts_with("culprit "))
        .collect()
}

#[test]
fn a_culprit_that_ran_no_check() {
    // req:verify/culprit-recheck@ea199b60
    let (f, [_, l2, l3]) = breaking_lands();
    landed(&f, &l2, Some(Recheck::None), None);
    composed(&f, "h12345678901234567", "b12345678901234567");
    let out = f.due();
    assert_eq!(
        out.line,
        format!(
            "verify failed {} culprit {} components b",
            &l3[..7],
            &l2[..7]
        )
    );
    assert_eq!(out.code, 1);
    assert_eq!(
        culprit_notes(&out),
        [&format!(
            "culprit {} re-checked nothing at land; its branch's check and main's were \
             composed from h12345678901 and b12345678901",
            &l2[..7]
        )]
    );
}

#[test]
fn a_culprit_that_ran_the_overlap() {
    // req:verify/culprit-recheck@ea199b60
    let (f, [_, l2, _]) = breaking_lands();
    landed(&f, &l2, Some(Recheck::Overlap), Some(4));
    composed(&f, "hh", "bb");
    let out = f.due();
    assert_eq!(
        culprit_notes(&out),
        [&format!(
            "culprit {} re-checked the overlap, 4 files at land; its branch's check and \
             main's were composed from hh and bb",
            &l2[..7]
        )]
    );
}

#[test]
fn an_overlap_that_ran_every_test() {
    // req:verify/culprit-recheck@ea199b60
    let (f, [_, l2, _]) = breaking_lands();
    landed(&f, &l2, Some(Recheck::Overlap), None);
    composed(&f, "hh", "bb");
    let out = f.due();
    assert_eq!(
        culprit_notes(&out),
        [&format!(
            "culprit {} re-checked the overlap, every test at land; its branch's check and \
             main's were composed from hh and bb",
            &l2[..7]
        )]
    );
}

#[test]
fn keys_are_unknown_without_a_composed_record() {
    // req:verify/culprit-recheck@ea199b60
    let (f, [_, l2, _]) = breaking_lands();
    landed(&f, &l2, Some(Recheck::None), None);
    let out = f.due();
    assert_eq!(
        culprit_notes(&out),
        [&format!(
            "culprit {} re-checked nothing at land; its branch's check and main's were \
             composed from unknown and unknown",
            &l2[..7]
        )]
    );
}

#[test]
fn a_culprit_that_ran_its_own_selection() {
    // req:verify/culprit-recheck@ea199b60
    let (f, [_, l2, _]) = breaking_lands();
    landed(&f, &l2, Some(Recheck::Selected), Some(2));
    composed(&f, "hh", "bb");
    assert!(culprit_notes(&f.due()).is_empty());
}

#[test]
fn a_culprit_checked_whole_or_unrecorded_says_nothing() {
    // req:verify/culprit-recheck@ea199b60
    let (f, [_, l2, _]) = breaking_lands();
    // No `landed` record at all, as for a land of an older `clm`.
    assert!(culprit_notes(&f.due()).is_empty());
    landed(&f, &l2, Some(Recheck::Checked), None);
    let out = f.run(&Request::Bisect(f.back(0)));
    assert!(culprit_notes(&out).is_empty());
    landed(&f, &l2, None, None);
    assert!(culprit_notes(&f.run(&Request::Bisect(f.back(0)))).is_empty());
}

#[test]
fn a_bisect_of_an_imported_failure_says_it_too() {
    // req:verify/culprit-recheck@ea199b60
    let f = Fixture::new(Some("mode = \"import\"\nworkflow = \"slow\""));
    let g = f.land(&[("a.txt", "1\n")], "green");
    f.verified(&g, true);
    let l1 = f.land(&[("crates/a/x.rs", "1\n")], "l1");
    let l2 = f.land(&[("crates/b/y.rs", "2\n")], "l2");
    f.write("fail-path", "crates/b/y.rs");
    f.red(&l2, &[&l1, &l2], VerifyMode::Import);
    landed(&f, &l2, Some(Recheck::Overlap), Some(1));
    composed(&f, "hh", "bb");
    let out = f.run(&Request::Bisect(l2.clone()));
    assert_eq!(
        culprit_notes(&out),
        [&format!(
            "culprit {} re-checked the overlap, 1 files at land; its branch's check and \
             main's were composed from hh and bb",
            &l2[..7]
        )]
    );
}

#[test]
fn the_record_and_stdout_are_unchanged() {
    // req:verify/culprit-recheck@ea199b60
    let (f, [_, l2, l3]) = breaking_lands();
    landed(&f, &l2, Some(Recheck::None), None);
    let out = f.due();
    assert_eq!(
        out.line,
        format!(
            "verify failed {} culprit {} components b",
            &l3[..7],
            &l2[..7]
        )
    );
    let records = f.records();
    let Some(Body::Verified {
        culprit,
        components,
        run,
        ..
    }) = records.last()
    else {
        panic!("{records:?}");
    };
    assert_eq!(culprit.as_deref(), Some(l2.as_str()));
    assert_eq!(components, &["b".to_string()]);
    assert_eq!(*run, None);
}

#[test]
fn a_whole_check_beside_the_composed_one_is_ignored() {
    // req:verify/culprit-recheck@ea199b60
    let (f, [_, l2, _]) = breaking_lands();
    landed(&f, &l2, Some(Recheck::None), None);
    composed(&f, "hh", "bb");
    let whole = Body::Checked {
        code_key: "codekey-of-the-land".to_string(),
        command: "check".to_string(),
        basis: None,
    };
    clm_ledger::append(&f.work(), whole, &f.env()).unwrap();
    let out = f.due();
    assert!(
        culprit_notes(&out)[0].ends_with("composed from hh and bb"),
        "{:?}",
        out.notes
    );
}
