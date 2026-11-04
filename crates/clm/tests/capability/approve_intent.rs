//! `approve-intent`: a human approves a change that needs review, through a prompt only a
//! human can confirm.

use crate::{Repo, bash_event, landed, printed};

/// A repository landing on its bare `origin`, with `spec/cap`, which edits
/// `CAPABILITIES.md`, pushed.
fn capability_edit() -> Repo {
    let repo = Repo::init();
    repo.land_on_origin("true");
    let capabilities = std::fs::read_to_string(repo.root.join("CAPABILITIES.md")).unwrap();
    repo.write(
        "CAPABILITIES.md",
        &capabilities.replace("A person can c.", "A person can c, quickly."),
    );
    repo.push_branch("spec/cap", "c, quickly");
    repo
}

/// Asserts the guard answers a session's `clm approve spec/cap` with Claude Code's prompt
/// to the human (the `ask` decision).
fn guard_asks(repo: &Repo) {
    let event = bash_event("clm approve spec/cap", &repo.root);
    let out = repo.clm_stdin(&["hook", "guard"], &event);
    assert!(out.status.success(), "{}", printed(&out));
    let decision: serde_json::Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|_| panic!("{}", printed(&out)));
    assert_eq!(
        decision["hookSpecificOutput"]["permissionDecision"], "ask",
        "{decision}"
    );
}

#[test]
fn approve_a_change() {
    // req:capability/approve-intent@684e8fc4
    let repo = capability_edit();
    guard_asks(&repo);
    // The human confirms the prompt, so the command runs.
    let approve = repo.clm(&["approve", "spec/cap"]);
    assert!(approve.status.success(), "{}", printed(&approve));

    let commit = landed(&repo.clm(&["land", "spec/cap"]));
    assert_eq!(repo.origin_main(), commit);

    let ledger = repo.ledger();
    let approval = ledger
        .iter()
        .find(|r| r[0] == "approved" && r[2] == "spec/cap")
        .unwrap_or_else(|| panic!("no approval: {ledger:?}"));
    assert!(approval[3].starts_with("ann/"), "{ledger:?}");
    // The land records that it needed review.
    assert!(
        ledger
            .iter()
            .any(|r| r[0] == "landed" && r[1] == commit && r[4] == "review"),
        "{ledger:?}"
    );
}

#[test]
fn a_session_cannot_approve_alone() {
    // req:capability/approve-intent@684e8fc4
    let repo = capability_edit();
    guard_asks(&repo);
    // The human declines the prompt, so `clm approve` never runs.
    let before = repo.origin_main();

    let land = repo.clm(&["land", "spec/cap"]);
    let text = printed(&land);
    assert!(!land.status.success(), "{text}");
    assert!(text.contains("gate/approval"), "{text}");
    assert_eq!(repo.origin_main(), before, "{text}");
    let ledger = repo.ledger();
    assert!(
        !ledger
            .iter()
            .any(|r| r[0] == "approved" || r[0] == "landed"),
        "{ledger:?}"
    );
}
