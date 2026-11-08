//! `claim_local`: the local branch exists before the first push (`claim-local`).

mod common;

use clm_orchestrator::{Error, claim, claim_local};
use common::*;

fn local_tip(r: &Repo, branch: &str) -> Option<String> {
    let out = output(
        std::process::Command::new("git")
            .args([
                "rev-parse",
                "--verify",
                "-q",
                &format!("refs/heads/{branch}"),
            ])
            .current_dir(&r.work),
    );
    out.status
        .success()
        .then(|| String::from_utf8(out.stdout).unwrap().trim().to_string())
}

fn hook(r: &Repo, name: &str, body: &str) {
    write_exec(
        &r.work.join(format!(".git/hooks/{name}")),
        &format!("#!/bin/sh\n{body}"),
    );
}

#[test]
fn scenario_branch_before_the_push() {
    // req:orchestrator/claim-local@bf90a932
    let r = repo();
    hook(
        &r,
        "pre-push",
        "git rev-parse refs/heads/impl/b > \"$(git rev-parse --absolute-git-dir)/seen\" || exit 1\n",
    );
    assert_eq!(claim_local(&r.work, "b", 4).unwrap(), "impl/b");
    let tip = remote_tip(&r, "impl/b").unwrap();
    let seen = std::fs::read_to_string(r.work.join(".git/seen")).unwrap();
    assert_eq!(seen.trim(), tip);
    assert_eq!(local_tip(&r, "impl/b").as_deref(), Some(tip.as_str()));
    let cfg = |k: &str| git(&r.work, &["config", &format!("branch.impl/b.{k}")]);
    assert_eq!(cfg("remote").trim(), "origin");
    assert_eq!(cfg("merge").trim(), "refs/heads/impl/b");
    let up = git(&r.work, &["rev-parse", "impl/b@{upstream}"]);
    assert_eq!(up.trim(), tip);
}

#[test]
fn scenario_lost_race() {
    // req:orchestrator/claim-local@bf90a932
    let r = repo();
    hook(
        &r,
        "pre-push",
        "[ -f \"$(git rev-parse --absolute-git-dir)/raced\" ] && exit 0\ntouch \"$(git rev-parse --absolute-git-dir)/raced\"\n\
         c=$(git commit-tree 'main^{tree}' -p main -m other) || exit 1\n\
         git push -q --no-verify origin \"$c:refs/heads/impl/b\" || exit 1\n",
    );
    let why = match claim_local(&r.work, "b", 4) {
        Err(Error::Refused(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    };
    assert!(why.contains("claimed by someone else"), "{why}");
    let tip = remote_tip(&r, "impl/b").unwrap();
    assert_eq!(
        git(&r.origin, &["log", "-1", "--format=%s", &tip]).trim(),
        "other"
    );
    assert_eq!(local_tip(&r, "impl/b"), None);
}

#[test]
fn scenario_rejected_for_another_reason() {
    // req:orchestrator/claim-local@bf90a932
    let r = repo();
    pre_receive(&r, "echo declined by test >&2\nexit 1\n");
    let why = match claim_local(&r.work, "b", 4) {
        Err(Error::Git(why)) => why,
        other => panic!("expected a git error, got {other:?}"),
    };
    assert!(why.contains("may be transient"), "{why}");
    assert_eq!(remote_tip(&r, "impl/b"), None);
    assert_eq!(local_tip(&r, "impl/b"), None);
}

#[test]
fn scenario_tip_unreadable() {
    // req:orchestrator/claim-local@bf90a932
    let r = repo();
    hook(
        &r,
        "pre-push",
        "git config remote.origin.url /nonexistent/origin\nexit 1\n",
    );
    assert!(matches!(claim_local(&r.work, "b", 4), Err(Error::Git(_))));
    let claim_commit = local_tip(&r, "impl/b").expect("the branch stays");
    let main = git(&r.work, &["rev-parse", "origin/main"]);
    assert_eq!(
        git(&r.work, &["rev-parse", &format!("{claim_commit}^")]),
        main
    );
}

#[test]
fn scenario_existing_local_branch() {
    // req:orchestrator/claim-local@bf90a932
    let r = repo();
    let main = git(&r.work, &["rev-parse", "origin/main"]);
    git(&r.work, &["branch", "impl/b", main.trim()]);
    assert!(matches!(claim_local(&r.work, "b", 4), Err(Error::Git(_))));
    assert_eq!(remote_tip(&r, "impl/b"), None);
    assert_eq!(local_tip(&r, "impl/b").as_deref(), Some(main.trim()));
}

#[test]
fn scenario_plain_claim() {
    // req:orchestrator/claim-local@bf90a932
    let r = repo();
    assert_eq!(claim(&r.work, "b", 4).unwrap(), "impl/b");
    assert_eq!(local_tip(&r, "impl/b"), None);
}

#[test]
fn refusals_leave_no_local_branch() {
    // req:orchestrator/claim-local@bf90a932
    let r = repo();
    push_claim(&r.work, "b", 1);
    assert!(matches!(
        claim_local(&r.work, "b", 4),
        Err(Error::Refused(_))
    ));
    assert_eq!(local_tip(&r, "impl/b"), None);
    assert!(matches!(
        claim_local(&r.work, "zz", 4),
        Err(Error::Refused(_))
    ));
}

#[test]
fn takeover_of_a_lapsed_claim_records_the_branch() {
    // req:orchestrator/claim-local@bf90a932
    let r = repo();
    let old = push_claim(&r.work, "b", 5);
    assert_eq!(claim_local(&r.work, "b", 4).unwrap(), "impl/b");
    let tip = remote_tip(&r, "impl/b").unwrap();
    assert_ne!(tip, old);
    assert_eq!(local_tip(&r, "impl/b").as_deref(), Some(tip.as_str()));
}

#[test]
fn scenario_no_other_public_claim_call() {
    // req:orchestrator/claim-local@bf90a932
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let repo = std::fs::read_to_string(src.join("repo.rs")).unwrap();
    let lib = std::fs::read_to_string(src.join("lib.rs")).unwrap();
    assert!(!repo.contains("pub fn claim_node"), "claim_node is public");
    assert!(!lib.contains("claim_node"), "claim_node is re-exported");
}
