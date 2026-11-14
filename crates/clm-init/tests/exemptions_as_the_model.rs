//! `clm survey --components` refuses exactly the `exemptions` entries the model refuses.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use tempfile::TempDir;

/// A git repository where `tests/feat/test_x.py`, a file of `feat`'s, imports `app`'s.
fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    clm_git::git(dir.path(), &["init", "-q"]).unwrap();
    for (rel, content) in [
        ("app/mod.py", "x = 1\n"),
        ("feat/lib.py", "y = 1\n"),
        ("tests/feat/test_x.py", "import app.mod\n"),
    ] {
        let path = dir.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    clm_git::git(dir.path(), &["add", "-A"]).unwrap();
    dir
}

/// What the model says is wrong with `exemptions` in a component's frontmatter: none when
/// it accepts them.
fn model_refusal(exemptions: &str) -> Vec<String> {
    let fm = format!("title: Feat\nstatus: unbuilt\nowners: []\nexemptions: {exemptions}\n");
    match clm_model::check_node_frontmatter(
        "feat",
        Path::new("specs/feat/SPEC.md"),
        &fm,
        &BTreeSet::new(),
    ) {
        Ok(_) => Vec::new(),
        Err(violations) => violations.into_iter().map(|v| v.message).collect(),
    }
}

#[test]
fn survey_refuses_exactly_the_exemptions_the_model_refuses() {
    // req:init/survey-components@368a5dcd
    let repo = repo();
    let file = Path::new("map.yaml");
    let mut refused = 0;
    let mut accepted = 0;
    for exemptions in [
        "[{ rule: undeclared-imports, files: [\"tests/feat/**\"], until: backlog/x }]",
        "[{ rule: skipped-tests, files: [\"a\"], until: backlog/x, why: \"y\" }]",
        "[]",
        "3",
        "null",
        "{}",
        "[3]",
        "[{ files: [\"a\"], until: backlog/x }]",
        "[{ rule: undeclared-imports, until: backlog/x }]",
        "[{ rule: undeclared-imports, files: [\"a\"] }]",
        "[{ rule: paths-disjoint, files: [\"a\"], until: backlog/x }]",
        "[{ rule: 3, files: [\"a\"], until: backlog/x }]",
        "[{ rule: undeclared-imports, files: [], until: backlog/x }]",
        "[{ rule: undeclared-imports, files: [\"\"], until: backlog/x }]",
        "[{ rule: undeclared-imports, files: \"a\", until: backlog/x }]",
        "[{ rule: undeclared-imports, files: [\"a\"], until: x }]",
        "[{ rule: undeclared-imports, files: [\"a\"], until: 5 }]",
        "[{ rule: undeclared-imports, files: [\"a\"], until: backlog/ }]",
        // The model's rule for a slug, which a second copy of the rules once let through.
        "[{ rule: undeclared-imports, files: [\"a\"], until: backlog/x/y }]",
        "[{ rule: undeclared-imports, files: [\"a\"], until: \"backlog/x y\" }]",
        "[{ rule: undeclared-imports, files: [\"a\"], until: backlog/x, extra: 1 }]",
        "[{ rule: undeclared-imports, files: [\"a\"], until: backlog/x, why: [1] }]",
        // Two things wrong in one entry, and a bad entry after a good one.
        "[{ rule: paths-disjoint, files: [], until: x }]",
        "[{ rule: skipped-tests, files: [\"a\"], until: backlog/x }, { rule: skipped-tests }]",
    ] {
        let map = format!(
            "components:\n  - {{ id: app, paths: [\"app/**\"] }}\n  - {{ id: feat, paths: [\"feat/**\", \"tests/**\"], exemptions: {exemptions} }}\n"
        );
        let model = model_refusal(exemptions);
        match clm_init::check_nodes(repo.path(), file, &map) {
            Ok(_) => {
                assert_eq!(model, Vec::<String>::new(), "{exemptions}: survey accepts");
                accepted += 1;
            }
            Err(error) => {
                assert!(
                    !model.is_empty(),
                    "{exemptions}: only survey refuses: {error}"
                );
                refused += 1;
            }
        }
    }
    // The list holds both kinds, so neither arm passes by never running.
    assert!(accepted >= 3 && refused >= 15, "{accepted} {refused}");
}

#[test]
fn an_accepted_exemption_from_undeclared_imports_alone_hides_its_edge() {
    // req:init/survey-components@368a5dcd
    let repo = repo();
    let edges = |exemptions: &str| {
        let map = format!(
            "components:\n  - {{ id: app, paths: [\"app/**\"] }}\n  - {{ id: feat, paths: [\"feat/**\", \"tests/**\"], exemptions: {exemptions} }}\n"
        );
        clm_init::check_nodes(repo.path(), Path::new("map.yaml"), &map)
            .unwrap()
            .into_iter()
            .filter(|v| v.rule == "survey/undeclared-edge")
            .count()
    };
    assert_eq!(edges("[]"), 1);
    assert_eq!(
        edges("[{ rule: undeclared-imports, files: [\"tests/feat/**\"], until: backlog/x }]"),
        0
    );
    assert_eq!(
        edges("[{ rule: skipped-tests, files: [\"tests/feat/**\"], until: backlog/x }]"),
        1
    );
}
