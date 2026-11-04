//! `work-concurrently`: several sessions on one repository without overwriting each other.

use std::sync::Barrier;
use std::thread;

use crate::{Repo, printed};

#[test]
fn two_sessions_one_node() {
    // req:capability/work-concurrently@2bd94dde
    let repo = Repo::init();
    repo.land_on_origin("true");
    let sessions = [repo.clone_as("bob"), repo.clone_as("cat")];

    // Both claim node `a` at the same moment.
    let start = Barrier::new(sessions.len());
    let outs: Vec<_> = thread::scope(|s| {
        let claims: Vec<_> = sessions
            .iter()
            .map(|session| {
                let start = &start;
                s.spawn(move || {
                    start.wait();
                    session.clm(&["claim", "a"])
                })
            })
            .collect();
        claims.into_iter().map(|c| c.join().unwrap()).collect()
    });

    let all: Vec<_> = outs.iter().map(printed).collect();
    let codes: Vec<_> = outs.iter().map(|o| o.status.code()).collect();
    let winner = match codes[..] {
        [Some(0), Some(1)] => 0,
        [Some(1), Some(0)] => 1,
        _ => panic!("{codes:?}: {all:?}"),
    };
    // The claim that holds on `origin` is the winner's.
    repo.git(&["fetch", "-q", "origin"]);
    let author = repo.git(&["log", "-1", "--format=%an", "origin/impl/a"]);
    assert_eq!(author.trim(), ["bob", "cat"][winner], "{all:?}");
    // The other session, trying again, is still refused.
    let again = sessions[1 - winner].clm(&["claim", "a"]);
    assert_eq!(again.status.code(), Some(1), "{}", printed(&again));
}
