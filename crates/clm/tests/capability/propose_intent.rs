//! `propose-intent`: an idea becomes a checked spec change, which lands on checks or waits
//! for the person's approval.

use crate::{Repo, landed, printed};

#[test]
fn an_idea_becomes_a_spec_branch() {
    // req:capability/propose-intent@a640e8f5
    let repo = Repo::init();
    repo.land_on_origin("true");

    // `/clm:propose`'s session writes the change on a spec branch, its base from `clm hashes`.
    repo.git(&["switch", "-q", "-c", "spec/20261030-x-twice"]);
    let base = repo.hash("a", "requirement:x");
    repo.write(
        "changes/20261030-x-twice/change.md",
        "---\nstatus: proposed\n---\n\n## Intent\n\nA does x twice.\n",
    );
    repo.write(
        "changes/20261030-x-twice/deltas/a.md",
        &format!(
            "---\ncomponent: a\ncreate: false\n---\n## MODIFIED Requirements\n\n### Requirement: x <!-- base: {base} -->\nA SHALL x twice.\n\n#### Scenario: x\n- GIVEN g\n- WHEN w\n- THEN t, twice\n"
        ),
    );
    let check = repo.clm(&["check"]);
    assert!(check.status.success(), "{}", printed(&check));
    let apply = repo.clm(&["apply", "20261030-x-twice"]);
    assert!(apply.status.success(), "{}", printed(&apply));
    repo.commit("a does x twice");
    let check = repo.clm(&["check"]);
    assert!(check.status.success(), "{}", printed(&check));

    // It declares no burden and edits only a node's rule, so it lands on checks.
    repo.git(&["fetch", "-q", "origin"]);
    let gate = repo.clm(&["gate", "--base", "origin/main"]);
    let text = String::from_utf8_lossy(&gate.stdout).into_owned();
    assert!(gate.status.success(), "{}", printed(&gate));
    assert_eq!(text.lines().next(), Some("spec"), "{text}");
    repo.git(&["push", "-q", "-u", "origin", "spec/20261030-x-twice"]);

    let commit = landed(&repo.clm(&["land", "spec/20261030-x-twice"]));
    assert_eq!(repo.origin_main(), commit);
    let spec = repo.git(&["show", &format!("{commit}:specs/a/SPEC.md")]);
    assert!(spec.contains("A SHALL x twice."), "{spec}");
}

#[test]
fn an_idea_that_changes_a_capability_waits_for_approval() {
    // req:capability/propose-intent@a640e8f5
    let repo = Repo::init();
    repo.land_on_origin("true");

    // The idea changes what the product is for, so the spec branch edits `CAPABILITIES.md`.
    repo.git(&["switch", "-q", "-c", "spec/20261030-c-quickly"]);
    let capabilities = std::fs::read_to_string(repo.root.join("CAPABILITIES.md")).unwrap();
    let edited = capabilities.replace("A person can c.", "A person can c, quickly.");
    assert_ne!(edited, capabilities, "the fixture's capability text moved");
    repo.write("CAPABILITIES.md", &edited);
    repo.write(
        "changes/20261030-c-quickly/change.md",
        "---\nstatus: applied\n---\n\n## Intent\n\nc, quickly.\n",
    );
    repo.commit("c, quickly");
    let check = repo.clm(&["check"]);
    assert!(check.status.success(), "{}", printed(&check));

    // It waits for review: the gate says so and `clm land` refuses it, leaving `main` as it was.
    repo.git(&["fetch", "-q", "origin"]);
    let gate = repo.clm(&["gate", "--base", "origin/main"]);
    let text = String::from_utf8_lossy(&gate.stdout).into_owned();
    assert_eq!(text.lines().next(), Some("spec review"), "{text}");
    repo.git(&["push", "-q", "-u", "origin", "spec/20261030-c-quickly"]);
    let before = repo.origin_main();
    let land = repo.clm(&["land", "spec/20261030-c-quickly"]);
    let text = printed(&land);
    assert!(!land.status.success(), "{text}");
    assert!(text.contains("gate/approval"), "{text}");
    assert_eq!(repo.origin_main(), before, "{text}");

    // Once the human approves it, it lands.
    let approve = repo.clm(&["approve", "spec/20261030-c-quickly"]);
    assert!(approve.status.success(), "{}", printed(&approve));
    let commit = landed(&repo.clm(&["land", "spec/20261030-c-quickly"]));
    assert_eq!(repo.origin_main(), commit);
}
