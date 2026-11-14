//! `right-model-per-task`: every agent CLM launches names its model.

use std::fs;

use crate::{Repo, printed};

#[test]
fn a_build_names_its_model() {
    // req:capability/right-model-per-task@e5783c37
    let repo = Repo::init();
    repo.land_on_origin("true");

    // `clm agent-harness` names the model builds run on.
    let harness = repo.clm_env(&["agent-harness"], "", &[("CLAUDECODE", "1")]);
    assert!(harness.status.success(), "{}", printed(&harness));
    let report = String::from_utf8_lossy(&harness.stdout);
    assert!(
        report.lines().any(|l| l.trim() == "build-model sonnet"),
        "{report}"
    );

    // `/clm:build` prepares node `a`'s build with the command it runs, passing its own model.
    // The skill's own `model` on the Agent tool, which launches the build, lies beyond the
    // binary's boundary; `clm run`'s prompt names the model of every agent the build launches
    // in turn. The proof rests on the values the program puts into the prompt, not on its
    // wording (`prompt-text-is-source`): the build prepared for two session models names each
    // in its prompt, and never the other.
    let prompt_for = |model: &str| {
        let out = repo.clm(&["run", "a", "--review-model", model]);
        assert!(out.status.success(), "{}", printed(&out));
        fs::read_to_string(repo.root.join(".clm/prompts/a.md")).unwrap()
    };
    let opus = prompt_for("opus");
    let other = prompt_for("session-model-x");

    // The final review runs on the session's model, whatever it is.
    assert!(opus.contains("model: \"opus\""), "{opus}");
    assert!(!opus.contains("session-model-x"), "{opus}");
    assert!(other.contains("model: \"session-model-x\""), "{other}");
    assert!(!other.contains("opus"), "{other}");

    // Every other agent runs on the model `clm agent-harness` names for builds.
    assert!(opus.contains("model: \"sonnet\""), "{opus}");
    assert!(other.contains("model: \"sonnet\""), "{other}");
}
