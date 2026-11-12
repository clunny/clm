//! `work-from-any-agent-harness`: one project worked from Claude Code and from Codex, with
//! the same rules, state and approvals.

use std::fs;

use crate::{Repo, bash_event, landed, printed};

const CLAUDE_CODE: [(&str, &str); 1] = [("CLAUDECODE", "1")];
const CODEX: [(&str, &str); 1] = [(clm_agent_harness::CODEX_MARKER, "thread-1")];

/// `CAPABILITIES.md` edited on `spec/cap`, which needs review, pushed; the work tree is on `main`.
fn capability_edit(repo: &Repo) {
    let capabilities = fs::read_to_string(repo.root.join("CAPABILITIES.md")).unwrap();
    repo.write(
        "CAPABILITIES.md",
        &capabilities.replace("A person can c.", "A person can c, quickly."),
    );
    repo.push_branch("spec/cap", "c, quickly");
}

#[test]
fn switch_tools_mid_change() {
    // req:capability/work-from-any-agent-harness@2db7181d
    let repo = Repo::init();
    // The instructions both tools read: `AGENTS.md`, which `CLAUDE.md` imports.
    let init = repo.clm(&["init"]);
    assert!(init.status.success(), "{}", printed(&init));
    let agents = fs::read_to_string(repo.root.join("AGENTS.md")).unwrap();
    assert!(agents.contains("clm"), "{agents}");
    let claude = fs::read_to_string(repo.root.join("CLAUDE.md")).unwrap();
    assert!(claude.lines().any(|l| l == "@AGENTS.md"), "{claude}");
    repo.commit("init");
    repo.land_on_origin("true");

    // From Claude Code: a spec branch that needs review, approved by the human at the
    // guard's prompt.
    capability_edit(&repo);
    let event = bash_event("clm approve spec/cap", &repo.root);
    let guard = repo.clm_env(&["hook", "guard"], &event, &CLAUDE_CODE);
    assert!(guard.status.success(), "{}", printed(&guard));
    let decision: serde_json::Value = serde_json::from_slice(&guard.stdout).unwrap();
    assert_eq!(decision["hookSpecificOutput"]["permissionDecision"], "ask");
    let approve = repo.clm_env(&["approve", "spec/cap"], "", &CLAUDE_CODE);
    assert!(approve.status.success(), "{}", printed(&approve));

    // From Codex: the same approval counts, and the build claims, builds and lands.
    let land = repo.clm_env(&["land", "spec/cap"], "", &CODEX);
    assert_eq!(repo.origin_main(), landed(&land));
    let claim = repo.clm_env(&["claim", "a"], "", &CODEX);
    assert!(claim.status.success(), "{}", printed(&claim));
    // A Claude Code session sees the claim.
    repo.git(&["fetch", "-q", "origin"]);
    let next = repo.clm_env(&["next"], "", &CLAUDE_CODE);
    assert!(next.status.success(), "{}", printed(&next));
    assert!(
        !String::from_utf8_lossy(&next.stdout)
            .lines()
            .any(|l| l.starts_with("a ")),
        "{}",
        printed(&next)
    );

    let dir = repo.root.join(".claude/worktrees/codex-build");
    repo.git(&[
        "worktree",
        "add",
        "-q",
        "--detach",
        dir.to_str().unwrap(),
        "origin/main",
    ]);
    let session = repo.at(&dir);
    session.git(&["switch", "-q", "impl/a"]);
    for slug in ["x", "y"] {
        let version = repo.hash("a", &format!("requirement:{slug}"));
        session.write(
            &format!("tests/t_{slug}.sh"),
            &format!(
                "# {}a/{slug}@{}\ntrue\n",
                concat!("req", ":"),
                &version[..8]
            ),
        );
    }
    session.commit("build a");
    session.git(&["push", "-q", "origin", "impl/a"]);
    let build = session.clm_env(&["land", "impl/a"], "", &CODEX);
    let commit = landed(&build);
    assert_eq!(repo.origin_main(), commit);

    // The ledger holds both lands in the one form, whichever tool landed.
    let ledger = repo.ledger();
    let lands: Vec<_> = ledger.iter().filter(|r| r[0] == "landed").collect();
    assert!(lands.len() >= 2, "{ledger:?}");
    assert!(
        lands.iter().all(|r| r.len() == lands[0].len()),
        "{ledger:?}"
    );
    assert!(
        ledger
            .iter()
            .any(|r| r[0] == "approved" && r[2] == "spec/cap"),
        "{ledger:?}"
    );
}

#[test]
fn approve_under_a_harness_that_cannot_ask() {
    // req:capability/work-from-any-agent-harness@2db7181d
    let repo = Repo::init();
    repo.land_on_origin("true");
    capability_edit(&repo);

    let out = repo.clm_env(&["approve", "spec/cap"], "", &CODEX);
    let text = printed(&out);
    assert!(!out.status.success(), "{text}");
    assert!(text.contains("own terminal"), "{text}");
    assert!(
        !repo.ledger().iter().any(|r| r[0] == "approved"),
        "nothing is approved"
    );
    let land = repo.clm_env(&["land", "spec/cap"], "", &CODEX);
    assert!(!land.status.success(), "{}", printed(&land));
}
