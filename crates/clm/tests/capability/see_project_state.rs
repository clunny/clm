//! `see-project-state`: the project's nodes, the work in flight and what holds it up.

use crate::{Repo, printed};

/// Node `b`, owning `lib/**`, with requirement `z`.
const NODE_B: &str = "---\ntitle: B\nstatus: active\nowners: []\npaths: [\"lib/**\"]\nserves: [capability/c]\n---\n\n## Contract\nB does z.\n\n## Requirements\n\n### Requirement: z\nB SHALL z.\n\n#### Scenario: z\n- GIVEN g\n- WHEN w\n- THEN t\n";

#[test]
fn status() {
    // req:capability/see-project-state@de0c9b79
    let repo = Repo::init();
    repo.write("specs/b/SPEC.md", NODE_B);
    repo.write(
        "backlog/q.md",
        "---\ntitle: \"Which clock?\"\nkind: question\ncomponents: [a]\nblocks: [a/x, a/y]\n---\n\nWhich clock does x read?\n",
    );
    repo.commit("node b, and a question blocking node a");
    repo.land_on_origin("true");
    let claim = repo.clm(&["claim", "b"]);
    assert!(claim.status.success(), "{}", printed(&claim));

    let out = repo.clm(&["status"]);
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(out.status.success(), "{}", printed(&out));
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let has = |line: &str| lines.contains(&line);

    // Pending, claimed and blocked: `b` is claimed, `a` blocked by the question, and the
    // root (`clm` in its node list) holds capability `c`, which waits on the rules that
    // serve it: `a/x`, `a/y` and `b/z`.
    assert!(has("pending 3 components: 1 claimed, 2 blocked"), "{text}");
    assert!(has("waiting on you 1: backlog/q"), "{text}");
    assert!(
        lines.iter().any(|l| l.split_whitespace().eq(["b", "live"])),
        "{text}"
    );
    // The commit it describes.
    let commit = lines
        .iter()
        .find_map(|l| l.strip_prefix("commit "))
        .unwrap_or_else(|| panic!("no commit: {text}"));
    let head = repo.git(&["rev-parse", "HEAD"]);
    assert!(commit.len() >= 7 && head.starts_with(commit), "{text}");
}
