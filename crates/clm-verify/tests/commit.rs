//! The verification of one commit of `main` (verify/verify-commit).

mod common;

use clm_verify::Request;
use common::Fixture;

const LOCAL: &str = "mode = \"local\"\ntriggers = []";

#[test]
fn already_verified() {
    // req:verify/verify-commit@ab79a067
    let f = Fixture::new(Some(LOCAL));
    let c = f.land(&[("a.txt", "1\n")], "one");
    f.land(&[("a.txt", "2\n")], "two");
    f.verified(&c, true);
    let out = f.run(&Request::Commit(c.clone()));
    assert_eq!(out.line, format!("verify passed {} 1 lands", &c[..7]));
    assert_eq!(out.code, 0);
    assert!(f.lines("check-pwd.log").is_empty());
}

#[test]
fn a_red_record_answers_red() {
    // req:verify/verify-commit@ab79a067
    let f = Fixture::new(Some(LOCAL));
    let c = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&c, false);
    let out = f.run(&Request::Commit(c.clone()));
    assert_eq!(
        out.line,
        format!("verify failed {} culprit unknown components b", &c[..7])
    );
    assert_eq!(out.code, 1);
    assert!(f.lines("check-pwd.log").is_empty());
}

#[test]
fn verified_now() {
    // req:verify/verify-commit@ab79a067
    let f = Fixture::new(Some(LOCAL));
    let h = f.land(&[("a.txt", "1\n")], "one");
    let out = f.run(&Request::Commit(h.clone()));
    assert_eq!(out.code, 0, "{}", out.line);
    assert_eq!(f.lines("check-head.log"), std::slice::from_ref(&h));
    let records = f.records();
    assert!(matches!(
        records.as_slice(),
        [clm_ledger::Body::Verified { commit, passed: true, .. }] if commit == &h
    ));
}

#[test]
fn an_older_land_is_checked_out_with_the_last_verified_commit_before_it_as_since() {
    // req:verify/verify-commit@ab79a067
    let f = Fixture::new(Some(LOCAL));
    let g = f.land(&[("a.txt", "0\n")], "green");
    f.verified(&g, true);
    let old = f.land(&[("a.txt", "1\n")], "old");
    f.land(&[("a.txt", "2\n")], "newer");
    let out = f.run(&Request::Commit(old.clone()));
    assert_eq!(out.line, format!("verify passed {} 1 lands", &old[..7]));
    assert_eq!(f.lines("check-head.log"), std::slice::from_ref(&old));
    let Some(clm_ledger::Body::Verified { since, lands, .. }) = f.records().pop() else {
        panic!()
    };
    assert_eq!(since.as_deref(), Some(g.as_str()));
    assert_eq!(lands, [old]);
}

#[test]
fn not_on_main() {
    // req:verify/verify-commit@ab79a067
    let f = Fixture::new(Some(LOCAL));
    f.land(&[("a.txt", "1\n")], "one");
    let work = f.work();
    f.git(&work, &["checkout", "-q", "-b", "side"]);
    let side = f.commit(&[("a.txt", "side\n")], "side");
    f.git(&work, &["push", "-q", "origin", "side"]);
    f.git(&work, &["checkout", "-q", "main"]);
    let err = clm_verify::verify(&work, &Request::Commit(side), &f.env()).unwrap_err();
    let clm_verify::Error::Refused(why) = err else {
        panic!("{err}")
    };
    assert!(why.contains("first-parent chain"), "{why}");
    assert!(f.lines("check-pwd.log").is_empty());
    assert!(f.records().is_empty());
}

#[test]
fn a_merged_branch_commit_is_not_a_land() {
    // req:verify/verify-commit@ab79a067
    let f = Fixture::new(Some(LOCAL));
    let work = f.work();
    f.git(&work, &["checkout", "-q", "-b", "side"]);
    let side = f.commit(&[("b.txt", "side\n")], "side");
    f.git(&work, &["checkout", "-q", "main"]);
    f.commit(&[("a.txt", "main\n")], "main");
    f.git(
        &work,
        &["merge", "-q", "--no-ff", "-m", "merge side", "side"],
    );
    f.git(&work, &["push", "-q", "origin", "main"]);
    let err = clm_verify::verify(&work, &Request::Commit(side), &f.env()).unwrap_err();
    assert!(matches!(err, clm_verify::Error::Refused(_)), "{err}");
}
