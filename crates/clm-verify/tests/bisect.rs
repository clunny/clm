//! The land that broke a red commit (verify/verify-bisect).

mod common;

use clm_ledger::{Body, VerifyMode};
use clm_verify::Request;
use common::Fixture;

/// A passing record for `g`, then `l1` changing `crates/a/x.rs`, `l2` changing
/// `crates/b/y.rs` and `l3` changing `crates/a/z.rs`; the check fails on any tree holding
/// `l2`'s change.
fn three_lands(settings: &str) -> (Fixture, [String; 4]) {
    let f = Fixture::new(Some(settings));
    let g = f.land(&[("a.txt", "1\n")], "green");
    f.verified(&g, true);
    let l1 = f.land(&[("crates/a/x.rs", "1\n")], "l1");
    let l2 = f.land(&[("crates/b/y.rs", "2\n")], "l2");
    let l3 = f.land(&[("crates/a/z.rs", "3\n")], "l3");
    f.write("fail-path", "crates/b/y.rs");
    (f, [g, l1, l2, l3])
}

fn last_red(f: &Fixture) -> (Option<String>, Vec<String>) {
    let records = f.records();
    let Some(Body::Verified {
        passed: false,
        culprit,
        components,
        ..
    }) = records.last()
    else {
        panic!("{records:?}");
    };
    (culprit.clone(), components.clone())
}

#[test]
fn the_breaking_land_found() {
    // req:verify/verify-bisect@b2330a00
    let (f, [_, l1, l2, l3]) =
        three_lands("mode = \"local\"\ntriggers = []\nevery = { lands = 1 }");
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
    assert_eq!(last_red(&f), (Some(l2.clone()), vec!["b".to_string()]));
    // The head, then a binary search: the middle land, then the one before it.
    assert_eq!(f.lines("check-head.log"), [l3, l2, l1]);
}

#[test]
fn the_search_starts_after_the_last_green_commit() {
    // req:verify/verify-bisect@b2330a00
    // An earlier red run left lands that no record covers as passing: they are searched.
    let (f, [_, l1, l2, l3]) =
        three_lands("mode = \"local\"\ntriggers = []\nevery = { lands = 1 }");
    f.red(&l1, &[&l1], VerifyMode::Local);
    let out = f.due();
    assert_eq!(
        out.line,
        format!(
            "verify failed {} culprit {} components b",
            &l3[..7],
            &l2[..7]
        )
    );
    // l1 is red-recorded, so the unverified lands are l2 and l3; l1 is still searched,
    // since the last green commit is before it.
    assert_eq!(f.lines("check-head.log").len(), 3);
}

#[test]
fn one_land_is_the_culprit_without_a_search() {
    // req:verify/verify-bisect@b2330a00
    let f = Fixture::new(Some(
        "mode = \"local\"\ntriggers = []\nevery = { lands = 1 }",
    ));
    let g = f.land(&[("a.txt", "1\n")], "green");
    f.verified(&g, true);
    let l = f.land(&[("crates/a/x.rs", "1\n")], "l");
    f.write("check-fails", "");
    let out = f.due();
    assert_eq!(
        out.line,
        format!("verify failed {0} culprit {0} components a", &l[..7])
    );
    assert_eq!(f.lines("check-head.log").len(), 1);
}

#[test]
fn a_land_that_changed_only_docs_names_the_root_component() {
    // req:verify/verify-bisect@b2330a00
    let f = Fixture::new(Some(
        "mode = \"local\"\ntriggers = []\nevery = { lands = 1 }",
    ));
    let g = f.land(&[("a.txt", "1\n")], "green");
    f.verified(&g, true);
    let l = f.land(&[("notes/readme.md", "docs\n")], "docs");
    f.write("check-fails", "");
    let out = f.due();
    assert_eq!(
        out.line,
        format!("verify failed {0} culprit {0} components clm", &l[..7])
    );
}

#[test]
fn bisect_an_imported_failure() {
    // req:verify/verify-bisect@b2330a00
    let (f, [_, l1, l2, l3]) = three_lands("mode = \"import\"\nworkflow = \"slow\"");
    f.red(&l3, &[&l1, &l2, &l3], VerifyMode::Import);
    let before = f.records().len();
    let out = f.run(&Request::Bisect(l3.clone()));
    assert_eq!(
        out.line,
        format!(
            "verify failed {} culprit {} components b",
            &l3[..7],
            &l2[..7]
        )
    );
    assert_eq!(out.code, 1);
    assert_eq!(f.records().len(), before + 1);
    let records = f.records();
    let Some(Body::Verified {
        commit,
        lands,
        mode,
        run,
        culprit,
        components,
        passed,
        ..
    }) = records.last()
    else {
        panic!("{records:?}");
    };
    assert_eq!(commit, &l3);
    assert!(!passed);
    assert_eq!(lands, &[l1, l2.clone(), l3]);
    assert_eq!(*mode, VerifyMode::Import);
    assert_eq!(run.as_deref(), Some("https://host/run/1"));
    assert_eq!(culprit.as_deref(), Some(l2.as_str()));
    assert_eq!(components, &["b".to_string()]);
}

#[test]
fn only_a_red_commit_can_be_bisected() {
    // req:verify/verify-bisect@b2330a00
    let (f, [g, ..]) = three_lands("mode = \"local\"\ntriggers = []");
    for sha in [g, f.back(0)] {
        let err = clm_verify::verify(&f.work(), &Request::Bisect(sha), &f.env()).unwrap_err();
        assert!(matches!(err, clm_verify::Error::Refused(_)), "{err}");
    }
    assert!(f.lines("check-head.log").is_empty());
}

const LOCAL: &str = "mode = \"local\"\ntriggers = []\nevery = { lands = 1 }";

fn unknown_notes(out: &clm_verify::Outcome) -> Vec<&String> {
    out.notes
        .iter()
        .filter(|n| n.starts_with("culprit unknown"))
        .collect()
}

#[test]
fn an_untestable_land_is_skipped() {
    // req:verify/verify-bisect@b2330a00
    let (f, [_, _, l2, l3]) = three_lands(LOCAL);
    let l4 = f.land(&[("crates/a/p.rs", "4\n")], "l4");
    let l5 = f.land(&[("crates/a/q.rs", "5\n")], "l5");
    f.write("exit-at", &format!("{l3} 127\n"));
    let out = f.due();
    assert_eq!(
        out.line,
        format!(
            "verify failed {} culprit {} components b",
            &l5[..7],
            &l2[..7]
        )
    );
    assert!(unknown_notes(&out).is_empty(), "{:?}", out.notes);
    // The head, the middle land l3 that could not run, then the nearer later land l4.
    assert_eq!(f.lines("check-head.log")[..3], [l5, l3, l4]);
    assert_eq!(last_red(&f).0, Some(l2));
}

#[test]
fn no_land_before_the_red_one_could_be_tested() {
    // req:verify/verify-bisect@b2330a00
    let f = Fixture::new(Some(LOCAL));
    let r = f.back(0);
    let l1 = f.land(&[("crates/a/x.rs", "1\n")], "l1");
    let l2 = f.land(&[("crates/b/y.rs", "2\n")], "l2");
    let l3 = f.land(&[("crates/a/z.rs", "3\n")], "l3");
    f.write("exit-at", &format!("{r} 127\n{l1} 127\n{l2} 127\n{l3} 1\n"));
    let out = f.due();
    assert_eq!(
        out.line,
        format!("verify failed {} culprit unknown components a b", &l3[..7])
    );
    assert_eq!(out.code, 1);
    assert_eq!(
        unknown_notes(&out),
        [&format!(
            "culprit unknown: [verify] check could not run on 3 lands, {} to {}, before the \
             first red one, {}",
            &r[..7],
            &l2[..7],
            &l3[..7]
        )]
    );
    let (culprit, components) = last_red(&f);
    assert_eq!(culprit, None);
    assert_eq!(components, ["a".to_string(), "b".to_string()]);
}

#[test]
fn a_script_says_it_cannot_test() {
    // req:verify/verify-bisect@b2330a00
    for code in [125, 126, 127] {
        let (f, [_, l1, l2, l3]) = three_lands(LOCAL);
        f.write("exit-at", &format!("{l2} {code}\n{l1} 0\n{l3} 1\n"));
        let out = f.due();
        assert_eq!(
            out.line,
            format!("verify failed {} culprit unknown components a b", &l3[..7]),
            "exit {code}"
        );
        assert_eq!(
            unknown_notes(&out),
            [&format!(
                "culprit unknown: [verify] check could not run on 1 lands, {0} to {0}, before \
                 the first red one, {1}",
                &l2[..7],
                &l3[..7]
            )]
        );
    }
}

#[test]
fn bisecting_an_imported_failure_can_leave_the_culprit_unknown() {
    // req:verify/verify-bisect@b2330a00
    let (f, [_, l1, l2, l3]) = three_lands("mode = \"import\"\nworkflow = \"slow\"");
    f.red(&l3, &[&l1, &l2, &l3], VerifyMode::Import);
    f.write("exit-at", &format!("{l2} 125\n{l1} 0\n{l3} 1\n"));
    let out = f.run(&Request::Bisect(l3.clone()));
    assert_eq!(
        out.line,
        format!("verify failed {} culprit unknown components a b", &l3[..7])
    );
    assert_eq!(unknown_notes(&out).len(), 1, "{:?}", out.notes);
    assert_eq!(last_red(&f), (None, vec!["a".to_string(), "b".to_string()]));
}

#[test]
fn a_commit_answered_from_the_record_prints_no_search_note() {
    // req:verify/verify-bisect@b2330a00
    let (f, [_, l1, l2, l3]) = three_lands(LOCAL);
    f.write("exit-at", &format!("{l2} 125\n{l1} 0\n{l3} 1\n"));
    f.due();
    let out = f.run(&Request::Commit(l3));
    assert!(unknown_notes(&out).is_empty(), "{:?}", out.notes);
    assert!(out.line.contains("culprit unknown"), "{}", out.line);
}
