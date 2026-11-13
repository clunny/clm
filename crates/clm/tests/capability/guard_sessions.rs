//! `guard-sessions`: the plugin's guard stops a session's unsafe actions.

use crate::{Repo, bash_event, landed, printed};

#[test]
fn push_to_main() {
    // req:capability/guard-sessions@b86a30d0
    let repo = Repo::init();
    let event = bash_event("git push", &repo.root);

    let out = repo.clm_stdin(&["hook", "guard"], &event);
    assert_eq!(out.status.code(), Some(2), "{}", printed(&out));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("guard-blocks-push-to-main"),
        "{}",
        printed(&out)
    );
}

#[test]
fn the_audit_finds_a_push_that_got_through() {
    // req:capability/guard-sessions@b86a30d0
    let repo = Repo::init();
    repo.land_on_origin("true");
    // A branch lands through `clm land`, which records it.
    repo.write("note.txt", "landed\n");
    repo.push_branch("fix/note", "a note");
    let land = repo.clm(&["land", "fix/note"]);
    landed(&land);
    repo.git(&["pull", "-q", "--ff-only", "origin", "main"]);
    let verify = repo.clm(&["ledger", "verify"]);
    assert!(verify.status.success(), "{}", printed(&verify));

    // A commit pushed to `main` by hand, which no land recorded.
    repo.write("sneaky.txt", "pushed\n");
    repo.commit("sneaky");
    repo.git(&["push", "-q", "origin", "main"]);
    let head = repo.git(&["rev-parse", "HEAD"]);

    let out = repo.clm(&["ledger", "verify"]);
    let text = printed(&out);
    assert!(!out.status.success(), "{text}");
    assert!(text.contains(&head.trim()[..12]), "{text}");
    assert!(text.contains("has no `landed` record"), "{text}");
}
