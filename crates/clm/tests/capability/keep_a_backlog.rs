//! `keep-a-backlog`: backlog items, each linked to the work it blocks.

use crate::{Repo, printed};

#[test]
fn a_question_blocks_work() {
    // req:capability/keep-a-backlog@1ee1e52a
    let repo = Repo::init();
    repo.write(
        "backlog/q.md",
        "---\ntitle: \"Which clock?\"\nkind: question\ncomponents: [a]\nblocks: [a/x]\n---\n\nWhich clock does x read?\n",
    );
    repo.commit("a question");

    let out = repo.clm(&["pending", "a"]);
    assert!(out.status.success(), "{}", printed(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let line = |id: &str| {
        stdout
            .lines()
            .find(|l| l.starts_with(&format!("{id} ")))
            .unwrap_or_else(|| panic!("no line for {id}: {stdout}"))
            .to_string()
    };
    assert!(
        line("a/x").ends_with(" unbuilt blocked backlog/q"),
        "{stdout}"
    );
    assert!(line("a/y").ends_with(" unbuilt"), "{stdout}");
}
