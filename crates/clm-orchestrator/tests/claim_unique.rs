//! Its own test binary, because it freezes git's clock for the whole process.

mod common;

use common::*;

#[test]
fn claims_in_the_same_second_are_distinct_commits() {
    // req:orchestrator/claim@026fab7a
    // Two sessions with the same identity claiming within one second must not build the
    // same commit: pushing a commit the branch already holds succeeds without the lease
    // check, so both would think they had won.
    let date = hours_ago(0);
    // SAFETY: this binary has a single test, so nothing else reads the environment.
    unsafe {
        std::env::set_var("GIT_COMMITTER_DATE", &date);
        std::env::set_var("GIT_AUTHOR_DATE", &date);
    }
    let r = repo();
    clm_orchestrator::claim(&r.work, "b", 4).unwrap();
    let first = remote_tip(&r, "impl/b").unwrap();
    git(&r.origin, &["update-ref", "-d", "refs/heads/impl/b"]);
    clm_orchestrator::claim(&r.work, "b", 4).unwrap();
    let second = remote_tip(&r, "impl/b").unwrap();
    assert_ne!(first, second);
}
