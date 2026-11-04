//! `right-model-per-task`: every agent CLM launches names its model.

use std::fs;

use crate::{Repo, printed};

#[test]
fn a_build_names_its_model() {
    // req:capability/right-model-per-task@06c2f40a
    let repo = Repo::init();
    repo.land_on_origin("true");

    // `/clm:build`, running on `opus`, prepares node `a`'s build with the command it
    // runs, passing its own model. The skill's own `model` on the Agent tool, which launches
    // the build, lies beyond the binary's boundary; `clm run`'s prompt names the model of
    // every agent the build launches in turn.
    let out = repo.clm(&["run", "a", "--review-model", "opus"]);
    assert!(out.status.success(), "{}", printed(&out));
    let prompt = fs::read_to_string(repo.root.join(".clm/prompts/a.md")).unwrap();
    let paragraph = |words: &str| {
        prompt
            .split("\n\n")
            .find(|p| p.contains(words))
            .unwrap_or_else(|| panic!("no paragraph with {words}: {prompt}"))
    };

    // The final review runs on the main session's model.
    let review = paragraph("launch the `clm:spec-auditor` subagent");
    assert!(review.contains("model: \"opus\""), "{review}");
    // Every other agent runs on the cheaper model, named.
    let others = paragraph("Name the model of every subagent you launch");
    assert!(others.contains("`model: \"sonnet\"` for each"), "{others}");
    assert!(
        others.contains("never launch a subagent without naming its model"),
        "{others}"
    );
    assert!(!others.contains("opus"), "{others}");
}
