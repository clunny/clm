//! `clm next`, `clm claim` and `clm status`'s claims against a bare `origin` and a clone.

mod common;

use std::fs;

use clm_orchestrator::{
    Error, PlaceKind, Placed, State, claim, claim_fix, claim_refs, next, next_wait,
    queue_from_refs, status_claims, take_place,
};
use common::*;

/// `clm next` with room for every node, so the places never cut the list short.
fn ready(r: &Repo) -> Vec<String> {
    next(&r.work, 4, 9).unwrap()
}

fn refused(res: Result<String, Error>) -> String {
    match res {
        Err(Error::Refused(m)) => m,
        other => panic!("{other:?}"),
    }
}

#[test]
fn scenario_claiming_an_unknown_component() {
    // req:orchestrator/says-component@4aae7d81
    let r = repo();
    let why = refused(claim(&r.work, "zz", 4));
    assert!(why.contains("no component `zz`"), "{why}");
    let why = refused(claim(&r.work, "a/b", 4));
    assert!(why.contains("isn't a component ID"), "{why}");
}

#[test]
fn scenario_one_remote_name() {
    // req:orchestrator/shared-plumbing@aeadf0f1
    assert_eq!(clm_orchestrator::REMOTE, clm_git::REMOTE);
}

#[test]
fn scenario_dependency_doesnt_order_then_converged() {
    // req:orchestrator/next@1f34c883
    let r = repo();
    // `a` depends on `b`; `c` is on its own. Nothing is cited yet.
    assert_eq!(ready(&r), ["a", "b", "c"]);
    cite(&r.work, "b.rs", &["b/r"]);
    push_main(&r.work, "build b");
    assert_eq!(ready(&r), ["a", "c"]);
}

#[test]
fn next_reads_origin_main_not_the_checkout() {
    // req:orchestrator/next@1f34c883
    let r = repo();
    cite(&r.work, "c.rs", &["c/s"]);
    write(&r.work, "specs/a/SPEC.md", "garbage");
    assert_eq!(
        ready(&r),
        ["a", "b", "c"],
        "the checkout's edits don't count"
    );
}

#[test]
fn scenario_live_and_lapsed_claims() {
    // req:orchestrator/next@1f34c883
    let r = repo();
    push_claim(&r.work, "b", 1);
    push_claim(&r.work, "c", 5);
    assert_eq!(ready(&r), ["a", "c"]);
    assert_eq!(next(&r.work, 6, 3).unwrap(), ["a"]);
}

#[test]
fn scenario_merged_claim() {
    // req:orchestrator/next@1f34c883
    let r = repo();
    push_claim(&r.work, "b", 1);
    merge_claim(&r.work, "b");
    assert_eq!(ready(&r), ["a", "b", "c"]);
}

/// Takes build place `node` on the clone at `root`, belonging to `impl/<node>`.
fn hold(root: &std::path::Path, node: &str) {
    let placed = take_place(
        root,
        node,
        PlaceKind::Build,
        &[format!("impl/{node}")],
        9,
        4,
    );
    assert_eq!(placed.unwrap(), Placed::New);
}

#[test]
fn scenario_capped_by_places() {
    // req:orchestrator/next@1f34c883
    let r = repo();
    write(&r.work, "specs/d/SPEC.md", &node("", &["t"]));
    write(&r.work, "specs/e/SPEC.md", &node("", &["u"]));
    push_main(&r.work, "d and e");
    push_claim(&r.work, "d", 1);
    hold(&r.work, "d");
    // `a`, `b`, `c` and `e` are ready and unclaimed; `d`'s place counts on its live claim.
    assert_eq!(next(&r.work, 4, 9).unwrap(), ["a", "b", "c", "e"]);
    assert_eq!(next(&r.work, 4, 3).unwrap(), ["a", "b"]);
}

#[test]
fn scenario_no_place_free() {
    // req:orchestrator/next@1f34c883 req:orchestrator/claim@026fab7a
    let r = repo();
    hold(&r.work, "z");
    assert_eq!(next(&r.work, 4, 1).unwrap(), Vec::<String>::new());
    assert_eq!(next(&r.work, 4, 0).unwrap(), Vec::<String>::new());
    assert_eq!(
        claim(&r.work, "a", 4).unwrap(),
        "impl/a",
        "places don't gate a claim"
    );
}

#[test]
fn scenario_same_as_next_on_a_repo() {
    // req:orchestrator/queue-matches-next@b56edb27 req:orchestrator/next@1f34c883
    // The tree of `queue.rs`'s *same as next*: `a` depends on `b`, `c` on `d`, `b` is
    // converged, `e` claimed an hour ago, and `backlog/h.md` blocks `f/x`.
    let r = repo();
    fs::remove_dir_all(r.work.join("changes")).unwrap();
    for (id, deps) in [
        ("a", "b"),
        ("b", ""),
        ("c", "d"),
        ("d", ""),
        ("e", ""),
        ("f", ""),
    ] {
        write(&r.work, &format!("specs/{id}/SPEC.md"), &node(deps, &["x"]));
    }
    write(
        &r.work,
        "backlog/h.md",
        "---\ntitle: H\nkind: human-task\nblocks: [f/x]\n---\n",
    );
    cite(&r.work, "b.rs", &["b/x"]);
    push_main(&r.work, "same as next");
    push_claim(&r.work, "e", 1);
    assert_eq!(ready(&r), ["a", "c", "d"]);

    // `queue_from_refs` on the same tree, branches and time reports the same nodes ready,
    // and `e` alone claimed.
    git(&r.work, &["fetch", "-q", "origin"]);
    let graph = load(&r.work);
    let markers = clm_check::scan(&r.work).req;
    let queue = queue_from_refs(&graph, &markers, &claim_refs(&r.work), now(), 4);
    let in_state = |f: fn(&State) -> bool| -> Vec<&str> {
        queue
            .nodes
            .iter()
            .filter(|n| f(&n.state))
            .map(|n| n.node.as_str())
            .collect()
    };
    assert_eq!(
        in_state(|s| matches!(s, State::Ready { .. })),
        ["a", "c", "d"]
    );
    assert_eq!(in_state(|s| matches!(s, State::Claimed { .. })), ["e"]);
}

#[test]
fn scenario_places_shared_by_worktrees() {
    // req:orchestrator/next@1f34c883
    let r = repo();
    hold(&r.work, "z");
    let tmp = tempfile::TempDir::new().unwrap();
    let linked = tmp.path().join("linked");
    git(
        &r.work,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            linked.to_str().unwrap(),
        ],
    );
    assert_eq!(next(&linked, 4, 2).unwrap(), ["a"]);
}

#[test]
fn nested_branches_claim_no_node() {
    // req:orchestrator/next@1f34c883 req:orchestrator/status-claims@726cb7c5
    // req:orchestrator/nested-branches-not-claims@a7ec57b2
    let r = repo();
    push_claim(&r.work, "b/x", 1);
    assert_eq!(ready(&r), ["a", "b", "c"]);
    assert_eq!(status_claims(&r.work), [["none"]]);
    assert!(clm_orchestrator::claim_refs(&r.work).is_empty());
}

#[test]
fn scenario_archive_branch_beside_a_claim() {
    // req:orchestrator/nested-branches-not-claims@a7ec57b2
    let r = repo();
    push_claim(&r.work, "b", 1);
    push_claim(&r.work, "archive/20261026-x", 1);
    assert_eq!(status_claims(&r.work), [["b", "live"]]);
    assert_eq!(ready(&r), ["a", "c"]);
}

#[test]
fn a_node_that_writes_a_protected_path_is_listed_and_claimed() {
    // req:orchestrator/next@1f34c883 req:orchestrator/claim@026fab7a
    let r = repo();
    write(
        &r.work,
        "specs/r/SPEC.md",
        &node_with(
            "",
            "paths: [\".claude/settings.json\"]\n",
            &[("x", "It SHALL write `.claude/settings.json`.")],
        ),
    );
    push_main(&r.work, "r");
    assert_eq!(ready(&r), ["a", "b", "c", "r"]);
    assert_eq!(claim(&r.work, "r", 4).unwrap(), "impl/r");
    assert!(remote_tip(&r, "impl/r").is_some());
    assert_eq!(ready(&r), ["a", "b", "c"]);
}

#[test]
fn claim_pushes_a_fresh_commit_on_main() {
    // req:orchestrator/claim@026fab7a
    let r = repo();
    assert_eq!(claim(&r.work, "b", 4).unwrap(), "impl/b");
    let tip = remote_tip(&r, "impl/b").unwrap();
    let main = git(&r.origin, &["rev-parse", "main"]);
    let parent = git(&r.origin, &["rev-parse", &format!("{tip}^")]);
    assert_eq!(parent, main);
    let msg = git(&r.origin, &["log", "-1", "--format=%s", &tip]);
    assert_eq!(msg.trim(), "Claim b");
    assert_eq!(ready(&r), ["a", "c"]);
}

#[test]
fn scenario_already_claimed() {
    // req:orchestrator/claim@026fab7a
    let r = repo();
    let sha = push_claim(&r.work, "b", 1);
    assert!(refused(claim(&r.work, "b", 4)).contains("claimed"));
    assert_eq!(remote_tip(&r, "impl/b").as_deref(), Some(sha.as_str()));
}

#[test]
fn scenario_why_it_isnt_ready() {
    // req:orchestrator/claim@026fab7a
    let r = repo();
    write(
        &r.work,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [c/s]\n---\n",
    );
    write(&r.work, "specs/d/SPEC.md", &node("", &["t"]));
    cite(&r.work, "d.rs", &["d/t"]);
    push_main(&r.work, "q, d");
    let refs = git(&r.origin, &["for-each-ref"]);
    assert!(refused(claim(&r.work, "c", 4)).contains("backlog/q"));
    let converged = refused(claim(&r.work, "d", 4));
    assert!(converged.contains("converged"), "{converged}");
    assert!(converged.contains("clm land"), "{converged}");
    assert!(refused(claim(&r.work, "ghost", 4)).contains("no component"));
    assert!(refused(claim(&r.work, "x/b", 4)).contains("isn't a component"));
    assert_eq!(git(&r.origin, &["for-each-ref"]), refs, "nothing pushed");
}

#[test]
fn scenario_pending_dependency() {
    // req:orchestrator/claim@026fab7a
    let r = repo();
    // `a` depends on `b`, and both have uncited requirements.
    assert_eq!(claim(&r.work, "a", 4).unwrap(), "impl/a");
    assert!(remote_tip(&r, "impl/a").is_some());
}

#[test]
fn scenario_blocked_by_a_need() {
    // req:orchestrator/claim@026fab7a
    let r = repo();
    // `c/s` needs `b/r`, which nothing cites.
    write(
        &r.work,
        "specs/c/SPEC.md",
        &node_with("", "", &[("s", "It SHALL.\n\n#### Needs\n- b/r")]),
    );
    push_main(&r.work, "c needs b/r");
    let refs = git(&r.origin, &["for-each-ref"]);
    let why = refused(claim(&r.work, "c", 4));
    assert!(why.contains("blocked by `b/r`"), "{why}");
    assert_eq!(git(&r.origin, &["for-each-ref"]), refs, "nothing pushed");
}

#[test]
fn scenario_takeover() {
    // req:orchestrator/claim@026fab7a
    let r = repo();
    let old = push_claim(&r.work, "b", 5);
    assert_eq!(claim(&r.work, "b", 4).unwrap(), "impl/b");
    let tip = remote_tip(&r, "impl/b").unwrap();
    assert_ne!(tip, old);
    assert_eq!(
        git(&r.origin, &["log", "-1", "--format=%s", &tip]).trim(),
        "Claim b"
    );
}

#[test]
fn scenario_merged_takeover() {
    // req:orchestrator/claim@026fab7a
    let r = repo();
    push_claim(&r.work, "b", 1);
    merge_claim(&r.work, "b");
    assert_eq!(claim(&r.work, "b", 4).unwrap(), "impl/b");
    let tip = remote_tip(&r, "impl/b").unwrap();
    let main = git(&r.origin, &["rev-parse", "main"]);
    assert_eq!(git(&r.origin, &["rev-parse", &format!("{tip}^")]), main);
}

#[test]
fn claim_that_lands_though_the_push_is_rejected_succeeds_and_tracks_the_claim() {
    // req:orchestrator/claim@026fab7a
    let r = repo();
    // The hook lets the update through and then fails, as a flaky remote would.
    pre_receive(&r, "exit 1\n");
    // `pre-receive` failing rejects the push, so make the hook accept and `update` reject
    // after the ref moved: emulate with a post-receive that reports failure instead.
    fs::remove_file(r.origin.join("hooks/pre-receive")).unwrap();
    write_exec(
        &r.origin.join("hooks/post-receive"),
        "#!/bin/sh\necho 'post-receive broke' >&2\nexit 1\n",
    );
    assert_eq!(claim(&r.work, "b", 4).unwrap(), "impl/b");
    let tip = remote_tip(&r, "impl/b").unwrap();
    assert_eq!(
        git(&r.work, &["rev-parse", "refs/remotes/origin/impl/b"]).trim(),
        tip
    );
}

#[test]
fn scenario_rejected_for_another_reason() {
    // req:orchestrator/claim@026fab7a
    let r = repo();
    pre_receive(&r, "echo 'declined by test' >&2\nexit 1\n");
    // Each hook run is seen on this process's monotonic clock: a hook's wall clock can jump.
    let work = r.work.clone();
    let claiming = std::thread::spawn(move || claim(&work, "b", 4));
    let mut seen = vec![];
    while !claiming.is_finished() {
        if hook_runs(&r) > seen.len() {
            seen.push(std::time::Instant::now());
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    match claiming.join().unwrap() {
        Err(Error::Git(m)) => {
            assert!(m.contains("declined by test"), "{m}");
            assert!(m.contains("may be transient"), "{m}");
            assert!(m.contains("run the same command again"), "{m}");
            assert!(!m.contains("claimed by someone else"), "{m}");
        }
        other => panic!("{other:?}"),
    }
    assert!(remote_tip(&r, "impl/b").is_none());
    assert_eq!(hook_runs(&r), 3, "two retries");
    // 1 second before the first retry, 2 before the second.
    assert_eq!(seen.len(), 3, "each run seen once");
    let gaps: Vec<u128> = seen.windows(2).map(|w| (w[1] - w[0]).as_millis()).collect();
    assert!(gaps[0] >= 990 && gaps[1] >= 1990, "{gaps:?} ms");
}

#[test]
fn scenario_rejected_twice() {
    // req:orchestrator/claim@026fab7a
    let r = repo();
    pre_receive(
        &r,
        "read old new ref; echo \"$new\" >> \"$GIT_DIR/hooks/pushed\"\n\
         [ \"$(wc -l < \"$GIT_DIR/hooks/runs\")\" -gt 2 ] || \
         { echo 'remote: fatal error in commit_refs' >&2; exit 1; }\n",
    );
    assert_eq!(claim(&r.work, "b", 4).unwrap(), "impl/b");
    let tip = remote_tip(&r, "impl/b").expect("claimed");
    assert!(
        git(&r.work, &["log", "-1", "--format=%B", &tip]).starts_with("Claim b"),
        "the tip is the claim commit"
    );
    let pushed = fs::read_to_string(r.origin.join("hooks/pushed")).unwrap();
    assert_eq!(
        pushed.lines().collect::<Vec<_>>(),
        [tip.as_str(); 3],
        "every push sends the same commit"
    );
}

#[test]
fn claim_rejected_while_the_tip_moved_is_lost_without_retrying() {
    // req:orchestrator/claim@026fab7a
    let r = repo();
    // The rejection coincides with another session's claim landing. Hooks may not move refs
    // from git's quarantine, so this one steps outside it.
    pre_receive(
        &r,
        "env -u GIT_QUARANTINE_PATH git update-ref refs/heads/impl/b \"$(git rev-parse main)\"\n\
         exit 1\n",
    );
    assert!(refused(claim(&r.work, "b", 4)).contains("someone else"));
    assert_eq!(hook_runs(&r), 1, "no retry once the tip moved");
}

#[test]
fn scenario_race() {
    // req:orchestrator/claim@026fab7a
    let r = repo();
    let other = r.work.with_file_name("other");
    git(
        &r.work,
        &[
            "clone",
            "-q",
            r.origin.to_str().unwrap(),
            other.to_str().unwrap(),
        ],
    );
    identify(&other);
    for round in 0..3 {
        let node = if round == 0 { "b" } else { "c" };
        if round == 2 {
            // Both see `c` as lapsed and race to take it over.
            push_claim(&r.work, node, 5);
        }
        let (w1, w2) = (r.work.clone(), other.clone());
        let (n1, n2) = (node.to_string(), node.to_string());
        let t1 = std::thread::spawn(move || claim(&w1, &n1, 4));
        let t2 = std::thread::spawn(move || claim(&w2, &n2, 4));
        let results = [t1.join().unwrap(), t2.join().unwrap()];
        let wins = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(wins, 1, "round {round}: {results:?}");
        for res in &results {
            assert!(matches!(res, Ok(_) | Err(Error::Refused(_))), "{res:?}");
        }
        if round == 1 {
            git(&r.origin, &["update-ref", "-d", "refs/heads/impl/c"]);
        }
    }
}

#[test]
fn scenario_status_claims_live_lapsed_and_merged() {
    // req:orchestrator/status-claims@726cb7c5
    let r = repo();
    push_claim(&r.work, "b", 1);
    push_claim(&r.work, "c", 5);
    let before = git(&r.origin, &["for-each-ref"]);
    git(&r.work, &["fetch", "-q", "origin"]);
    assert_eq!(status_claims(&r.work), [["b", "live"], ["c", "lapsed"]]);
    merge_claim(&r.work, "b");
    assert_eq!(status_claims(&r.work), [["b", "merged"], ["c", "lapsed"]]);
    // It never fetches: a claim on `origin` the last fetch didn't see doesn't show.
    push_claim(&r.work, "a", 0);
    git(&r.work, &["update-ref", "-d", "refs/remotes/origin/impl/a"]);
    assert_eq!(status_claims(&r.work).len(), 2);
    assert_ne!(git(&r.origin, &["for-each-ref"]), before);
}

#[test]
fn scenario_backlog_claims_after_node_claims() {
    // req:orchestrator/status-claims@726cb7c5
    let r = repo();
    push_claim(&r.work, "b", 1);
    push_propose(&r.work, "q", 1);
    git(&r.work, &["fetch", "-q", "origin"]);
    assert_eq!(
        status_claims(&r.work),
        [["b", "live"], ["backlog/q", "live"]]
    );
}

#[test]
fn scenario_no_claims() {
    // req:orchestrator/status-claims@726cb7c5
    let r = repo();
    assert_eq!(status_claims(&r.work), [["none"]]);
    let dir = tempfile::TempDir::new().unwrap();
    assert_eq!(status_claims(dir.path()), [["none"]]);
}

/// A backlog item of `kind` naming `nodes` (comma-separated).
fn kind_item(kind: &str, nodes: &str) -> String {
    format!("---\ntitle: I\nkind: {kind}\ncomponents: [{nodes}]\n---\n")
}

/// The branch the checkout at `work` is on.
fn on(work: &std::path::Path) -> String {
    git(work, &["rev-parse", "--abbrev-ref", "HEAD"])
        .trim()
        .to_string()
}

/// The repo with `b` converged and bug `x` on `b` on main, and the checkout on a clean
/// `spec/s`.
fn fixable() -> Repo {
    let r = repo();
    cite(&r.work, "b.rs", &["b/r"]);
    write(&r.work, "backlog/x.md", &kind_item("bug", "b"));
    push_main(&r.work, "b converged, bug x");
    git(&r.work, &["switch", "-q", "-c", "spec/s"]);
    r
}

/// Asserts the checkout at `r.work` is on `impl/b` at `origin/impl/b`'s tip, tracking it.
fn on_the_claim(r: &Repo) {
    let tip = remote_tip(r, "impl/b").unwrap();
    assert_eq!(on(&r.work), "impl/b");
    assert_eq!(git(&r.work, &["rev-parse", "HEAD"]).trim(), tip);
    assert_eq!(
        git(&r.work, &["rev-parse", "--abbrev-ref", "impl/b@{upstream}"]).trim(),
        "origin/impl/b"
    );
    assert_eq!(git(&r.work, &["rev-parse", "origin/impl/b"]).trim(), tip);
}

#[test]
fn scenario_fix_on_a_converged_node() {
    // req:orchestrator/claim-fix@5a3c119e
    let r = fixable();
    assert!(refused(claim(&r.work, "b", 4)).contains("converged"));
    assert_eq!(on(&r.work), "spec/s");
    assert_eq!(claim_fix(&r.work, "b", "backlog/x", 4).unwrap(), "impl/b");
    let tip = remote_tip(&r, "impl/b").unwrap();
    let main = git(&r.origin, &["rev-parse", "main"]);
    assert_eq!(git(&r.origin, &["rev-parse", &format!("{tip}^")]), main);
    on_the_claim(&r);
    // The fix claim is an ordinary claim: the node is claimed now, which is said first
    // even from the local `impl/b` the earlier fix claim left checked out.
    let why = refused(claim_fix(&r.work, "b", "backlog/x", 4));
    assert!(why.contains("`b` is claimed"), "{why}");
    assert_eq!(remote_tip(&r, "impl/b").unwrap(), tip);
}

#[test]
fn scenario_fix_for_a_deferred_item() {
    // req:orchestrator/claim-fix@5a3c119e
    let r = repo();
    cite(&r.work, "b.rs", &["b/r"]);
    write(&r.work, "backlog/k.md", &kind_item("deferred", "b"));
    push_main(&r.work, "b converged, deferred k");
    git(&r.work, &["switch", "-q", "-c", "spec/s"]);
    assert_eq!(claim_fix(&r.work, "b", "backlog/k", 4).unwrap(), "impl/b");
    on_the_claim(&r);
}

#[test]
fn scenario_fix_on_a_claimed_node() {
    // req:orchestrator/claim-fix@5a3c119e
    let r = repo();
    write(&r.work, "backlog/x.md", &kind_item("bug", "b"));
    push_main(&r.work, "bug x");
    let sha = push_claim(&r.work, "b", 1);
    let why = refused(claim_fix(&r.work, "b", "backlog/x", 4));
    assert!(why.contains("`b` is claimed"), "{why}");
    assert_eq!(remote_tip(&r, "impl/b").as_deref(), Some(sha.as_str()));
}

#[test]
fn scenario_stale_local_branch_reset() {
    // req:orchestrator/claim-fix@5a3c119e
    let r = fixable();
    git(&r.work, &["branch", "impl/b", "origin/main~1"]);
    assert_eq!(claim_fix(&r.work, "b", "backlog/x", 4).unwrap(), "impl/b");
    on_the_claim(&r);
}

#[test]
fn fix_claim_from_a_merged_fix_branch_checked_out_resets_it() {
    // req:orchestrator/claim-fix@5a3c119e
    let r = fixable();
    assert_eq!(claim_fix(&r.work, "b", "backlog/x", 4).unwrap(), "impl/b");
    merge_claim(&r.work, "b");
    // The checkout is still on the merged `impl/b`, which `git branch -f` can't move.
    let old = git(&r.work, &["rev-parse", "HEAD"]);
    assert_eq!(claim_fix(&r.work, "b", "backlog/x", 4).unwrap(), "impl/b");
    on_the_claim(&r);
    assert_ne!(git(&r.work, &["rev-parse", "HEAD"]), old);
}

#[test]
fn scenario_branch_held_by_another_worktree() {
    // req:orchestrator/claim-fix@5a3c119e
    let r = fixable();
    let dir = tempfile::TempDir::new().unwrap();
    let w = dir.path().join("w");
    git(
        &r.work,
        &["worktree", "add", "-q", w.to_str().unwrap(), "-b", "impl/b"],
    );
    let why = refused(claim_fix(&r.work, "b", "backlog/x", 4));
    assert!(why.contains("`impl/b`"), "{why}");
    let w = fs::canonicalize(&w).unwrap();
    assert!(why.contains(w.to_str().unwrap()), "{why}");
    assert_eq!(remote_tip(&r, "impl/b"), None);
    assert_eq!(on(&r.work), "spec/s");
}

#[test]
fn scenario_unmerged_local_commits() {
    // req:orchestrator/claim-fix@5a3c119e
    let r = fixable();
    let tree = git(&r.work, &["rev-parse", "origin/main^{tree}"]);
    let main = git(&r.work, &["rev-parse", "origin/main"]);
    let work = git(
        &r.work,
        &["commit-tree", tree.trim(), "-p", main.trim(), "-m", "work"],
    );
    git(&r.work, &["branch", "impl/b", work.trim()]);
    let why = refused(claim_fix(&r.work, "b", "backlog/x", 4));
    assert!(why.contains("`impl/b`"), "{why}");
    assert_eq!(remote_tip(&r, "impl/b"), None);
    assert_eq!(git(&r.work, &["rev-parse", "impl/b"]), work);
    assert_eq!(on(&r.work), "spec/s");
}

#[test]
fn scenario_uncommitted_changes() {
    // req:orchestrator/claim-fix@5a3c119e
    let r = fixable();
    write(&r.work, "ARCHITECTURE.md", "# Edited\n");
    let why = refused(claim_fix(&r.work, "b", "backlog/x", 4));
    assert!(why.contains("uncommitted changes"), "{why}");
    assert_eq!(remote_tip(&r, "impl/b"), None);
    assert_eq!(
        fs::read_to_string(r.work.join("ARCHITECTURE.md")).unwrap(),
        "# Edited\n"
    );
    // Untracked files ride along, as `git switch` carries them.
    write(&r.work, "ARCHITECTURE.md", "# Arch\n");
    write(&r.work, "notes.txt", "n\n");
    assert_eq!(claim_fix(&r.work, "b", "backlog/x", 4).unwrap(), "impl/b");
    on_the_claim(&r);
    assert!(r.work.join("notes.txt").exists());
}

#[test]
fn scenario_switch_fails_after_the_push() {
    // req:orchestrator/claim-fix@5a3c119e
    let r = fixable();
    // `spec/s`'s tree differs from the claim's, so a switch that moved the files before the
    // ref would leave them changed.
    write(&r.work, "notes.md", "spec work\n");
    git(&r.work, &["add", "notes.md"]);
    git(&r.work, &["commit", "-qm", "spec work"]);
    let hooks = r.work.join(".git/test-hooks");
    fs::create_dir_all(&hooks).unwrap();
    write_exec(
        &hooks.join("reference-transaction"),
        "#!/bin/sh\n[ \"$1\" = prepared ] || exit 0\n\
         while read -r old new ref; do\n\
         [ \"$ref\" = refs/heads/impl/b ] && { echo 'impl/b is frozen' >&2; exit 1; }\n\
         done\nexit 0\n",
    );
    git(
        &r.work,
        &["config", "core.hooksPath", hooks.to_str().unwrap()],
    );
    let why = match claim_fix(&r.work, "b", "backlog/x", 4) {
        Err(Error::Git(m)) => m,
        other => panic!("{other:?}"),
    };
    assert!(
        why.contains("`impl/b` is claimed on `origin`") && why.contains("couldn't switch"),
        "{why}"
    );
    assert!(why.contains("git switch -C impl/b origin/impl/b"), "{why}");
    // Git's reason, which carries the hook's.
    assert!(why.contains("impl/b is frozen"), "{why}");
    let tip = remote_tip(&r, "impl/b").unwrap();
    assert_eq!(
        git(&r.origin, &["log", "-1", "--format=%s", &tip]).trim(),
        "Claim b"
    );
    assert_eq!(on(&r.work), "spec/s");
    assert_eq!(
        git(&r.work, &["status", "--porcelain", "--untracked-files=no"]),
        ""
    );
    assert!(r.work.join("notes.md").exists());
}

#[test]
fn scenario_item_not_a_bug_on_the_node() {
    // req:orchestrator/claim-fix@5a3c119e
    let r = repo();
    write(&r.work, "backlog/y.md", &kind_item("idea", "b"));
    write(&r.work, "backlog/z.md", &kind_item("bug", "c"));
    write(&r.work, "backlog/q.md", &kind_item("question", "b"));
    push_main(&r.work, "y, z, q");
    let refs = git(&r.origin, &["for-each-ref"]);
    let why = refused(claim_fix(&r.work, "b", "backlog/y", 4));
    assert!(
        why.contains("`backlog/y` is neither a bug nor deferred work") && why.contains("`idea`"),
        "{why}"
    );
    let why = refused(claim_fix(&r.work, "b", "backlog/q", 4));
    assert!(
        why.contains("`backlog/q` is neither a bug nor deferred work")
            && why.contains("`question`"),
        "{why}"
    );
    let why = refused(claim_fix(&r.work, "b", "backlog/z", 4));
    assert!(why.contains("`backlog/z` doesn't name `b`"), "{why}");
    let why = refused(claim_fix(&r.work, "b", "backlog/nope", 4));
    assert!(why.contains("isn't in `origin/main`"), "{why}");
    for bad in ["z", "backlog/", "backlog/a/z"] {
        let why = refused(claim_fix(&r.work, "b", bad, 4));
        assert!(why.contains("isn't a backlog item"), "{why}");
    }
    assert!(refused(claim_fix(&r.work, "ghost", "backlog/z", 4)).contains("no component"));
    assert!(refused(claim_fix(&r.work, "x/b", "backlog/z", 4)).contains("isn't a component"));
    assert_eq!(git(&r.origin, &["for-each-ref"]), refs, "nothing pushed");
}

#[test]
fn fix_takes_over_a_released_claim() {
    // req:orchestrator/claim-fix@5a3c119e
    let r = repo();
    // `a`'s old claim lapsed, and `c`'s merged.
    write(&r.work, "backlog/x.md", &kind_item("bug", "a, c"));
    push_main(&r.work, "bug x");
    let old = push_claim(&r.work, "a", 5);
    assert_eq!(claim_fix(&r.work, "a", "backlog/x", 4).unwrap(), "impl/a");
    assert_ne!(remote_tip(&r, "impl/a").unwrap(), old);
    push_claim(&r.work, "c", 1);
    merge_claim(&r.work, "c");
    assert_eq!(claim_fix(&r.work, "c", "backlog/x", 4).unwrap(), "impl/c");
}

#[test]
fn fix_claims_a_blocked_node_and_reads_items_from_origin_main() {
    // req:orchestrator/claim-fix@5a3c119e
    let r = repo();
    // `c`'s only pending requirement is blocked by `q`.
    write(
        &r.work,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [c/s]\n---\n",
    );
    write(&r.work, "backlog/x.md", &kind_item("bug", "c"));
    push_main(&r.work, "q, x");
    assert!(refused(claim(&r.work, "c", 4)).contains("backlog/q"));
    // An item only in the checkout doesn't count.
    write(&r.work, "backlog/w.md", &kind_item("bug", "c"));
    let why = refused(claim_fix(&r.work, "c", "backlog/w", 4));
    assert!(why.contains("isn't in `origin/main`"), "{why}");
    assert_eq!(claim_fix(&r.work, "c", "backlog/x", 4).unwrap(), "impl/c");
}

// `clm next --wait` (`next-wait`): the library call behind it, with intervals of a few
// hundred milliseconds.

/// A second clone of `r`'s `origin`, for pushing while `next_wait` reads the first.
fn second_clone(r: &Repo) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::TempDir::new().unwrap();
    let work = dir.path().join("other");
    git(
        dir.path(),
        &[
            "clone",
            "-q",
            r.origin.to_str().unwrap(),
            work.to_str().unwrap(),
        ],
    );
    identify(&work);
    (dir, work)
}

/// `r` with every requirement cited, so no node is ready.
fn converged() -> Repo {
    let r = repo();
    cite(&r.work, "all.rs", &["a/p", "a/q", "b/r", "c/s"]);
    push_main(&r.work, "cite all");
    assert!(ready(&r).is_empty());
    r
}

fn wait(r: &Repo, interval: &str, notes: &mut Vec<String>) -> Result<Vec<String>, Error> {
    next_wait(&r.work, 4, 9, interval, &mut |n| notes.push(n.to_string()))
}

#[test]
fn scenario_next_wait_work_arrives() {
    // req:orchestrator/next-wait@49b8dc88
    let r = converged();
    let (_keep, other) = second_clone(&r);
    let pusher = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        write(&other, "specs/a/SPEC.md", &node("b", &["p", "q", "new"]));
        push_main(&other, "a gains a requirement");
    });
    let mut notes = vec![];
    let started = std::time::Instant::now();
    let got = wait(&r, "0.01", &mut notes).unwrap();
    pusher.join().unwrap();
    assert_eq!(got, ["a"]);
    assert!(notes.is_empty(), "{notes:?}");
    // The interval is in minutes: 0.01 of one is 0.6 s, so the second look is not sooner.
    assert!(started.elapsed() >= std::time::Duration::from_millis(550));
}

#[test]
fn scenario_next_wait_work_already_there() {
    // req:orchestrator/next-wait@49b8dc88
    let r = repo();
    let mut notes = vec![];
    assert_eq!(wait(&r, "60", &mut notes).unwrap(), ready(&r));
    assert_eq!(ready(&r), ["a", "b", "c"]);
}

#[test]
fn scenario_next_wait_a_place_frees() {
    // req:orchestrator/next-wait@49b8dc88
    let r = repo();
    push_claim(&r.work, "b", 1);
    hold(&r.work, "b");
    assert!(next(&r.work, 4, 1).unwrap().is_empty());
    let (_keep, other) = second_clone(&r);
    let lander = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(700));
        merge_claim(&other, "b");
    });
    let mut notes = vec![];
    let got = next_wait(&r.work, 4, 1, "0.005", &mut |n| notes.push(n.to_string())).unwrap();
    lander.join().unwrap();
    assert_eq!(got, ["a"]);
}

#[test]
fn scenario_next_wait_a_later_look_fails() {
    // req:orchestrator/next-wait@49b8dc88
    let r = converged();
    let (_keep, other) = second_clone(&r);
    let gone = r.origin.with_extension("gone");
    let origin = r.origin.clone();
    let flaky = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        std::fs::rename(&origin, &gone).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(900));
        std::fs::rename(&gone, &origin).unwrap();
        write(&other, "specs/a/SPEC.md", &node("b", &["p", "q", "new"]));
        push_main(&other, "a gains a requirement");
    });
    let mut notes = vec![];
    let got = wait(&r, "0.005", &mut notes).unwrap();
    flaky.join().unwrap();
    assert_eq!(got, ["a"]);
    assert!(!notes.is_empty());
    assert!(
        notes
            .iter()
            .all(|n| n.starts_with("clm next failed: ") && n.contains("git fetch")),
        "{notes:?}"
    );
}

#[test]
fn scenario_next_wait_first_look_fails() {
    // req:orchestrator/next-wait@49b8dc88
    let r = repo();
    std::fs::rename(&r.origin, r.origin.with_extension("gone")).unwrap();
    let mut notes = vec![];
    assert!(matches!(wait(&r, "0.005", &mut notes), Err(Error::Git(_))));
    assert!(notes.is_empty());
}

#[test]
fn scenario_next_wait_bad_interval() {
    // req:orchestrator/next-wait@49b8dc88
    // No repository at all: a look would fail with a git error, so a refusal proves the
    // interval was judged before any look.
    let dir = tempfile::TempDir::new().unwrap();
    for bad in ["0", "-1", "abc", "NaN", "inf", "", "1e-300"] {
        let mut notes = vec![];
        match next_wait(dir.path(), 4, 9, bad, &mut |n| notes.push(n.to_string())) {
            Err(Error::Refused(m)) => assert!(m.contains("positive number"), "{bad}: {m}"),
            other => panic!("{bad}: {other:?}"),
        }
    }
}
