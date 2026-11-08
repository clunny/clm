//! `guard-sessions`: the plugin's guard stops a session's unsafe actions.

use crate::{Repo, bash_event, printed};

#[test]
fn push_to_main() {
    // req:capability/guard-sessions@a7684686
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
