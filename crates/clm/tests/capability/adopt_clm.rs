//! `adopt-clm`: an existing repository brought under CLM, with a first spec recovered from
//! its code.

use std::fs;

use crate::{Repo, printed};

/// The change `/clm:init` drafts.
const CHANGE: &str = "20261030-init";

/// The node map `/clm:init` drafts from `clm survey` and agrees with the person, for
/// `clm survey --components`: the root keeps the files `clm init` gave it, and the README.
const MAP: &str = "components:\n  - { id: clm, paths: [\".gitignore\", \"README.md\", \"clm.toml\", \"scripts/clm-check.sh\"], depends_on: [] }\n  - { id: greeter, paths: [\"src/**\"], depends_on: [clm] }\n  - { id: capability, paths: [\"tests/capability/**\"], depends_on: [clm] }\noutside: []\n";

const GREETER: &str = "---\ncomponent: greeter\ncreate: true\n---\n## Frontmatter\n\n```yaml\ntitle: Greeter\nstatus: specd\nowners: []\ndepends_on: [clm]\npaths: [\"src/**\"]\nserves: [capability/greet]\n```\n\n## Contract\n\n`greet` prints a greeting.\n\n## Design\n\nA shell function in `src/greet.sh`.\n\n## ADDED Requirements\n\n### Requirement: says-hi\n`greet` SHALL print `hi`.\n\n#### Scenario: greet\n- GIVEN the greeter sourced\n- WHEN `greet` runs\n- THEN it prints `hi`\n";

const CAPABILITY: &str = "---\ncomponent: capability\ncreate: true\n---\n## Frontmatter\n\n```yaml\ntitle: Capabilities\nstatus: specd\nowners: []\ndepends_on: [clm]\npaths: [\"tests/capability/**\"]\n```\n\n## Contract\n\nThe capabilities' black-box tests, citing `req:capability/<slug>@<version>`.\n\n## Design\n\nThe boundary is the shell: each test sources `src/greet.sh` and reads what it prints.\n";

#[test]
fn existing_codebase() {
    // req:capability/adopt-clm@f0d2dc65
    let repo = Repo::empty();
    repo.write("src/greet.sh", "greet() { echo hi; }\n");
    repo.write("README.md", "# Greeter\n");
    repo.commit("code");
    repo.git(&["push", "-q", "-u", "origin", "main"]);
    let clm = |repo: &Repo, args: &[&str]| {
        let out = repo.clm(args);
        assert!(out.status.success(), "clm {args:?}: {}", printed(&out));
        String::from_utf8_lossy(&out.stdout).into_owned()
    };

    // `/clm:init`: the tooling, in a bootstrap commit the person pushes to `main`.
    clm(&repo, &["init", "--no-detectors"]);
    repo.commit("adopt CLM");
    repo.git(&["push", "-q", "origin", "main"]);
    let survey = clm(&repo, &["survey"]);
    assert!(
        survey
            .lines()
            .any(|l| l.split_whitespace().next() == Some("src")),
        "{survey}"
    );

    // The spec branch, in a worktree of its own.
    let top = repo.root.parent().unwrap();
    let tree = repo.root.join(".claude/worktrees").join(CHANGE);
    repo.git(&[
        "worktree",
        "add",
        "-q",
        tree.to_str().unwrap(),
        "-b",
        &format!("spec/{CHANGE}"),
        "origin/main",
    ]);
    let spec = repo.at(&tree);

    // The node map the person agreed to covers all the code.
    let map = top.join("map.yaml");
    fs::write(&map, MAP).unwrap();
    let checked = spec.clm(&["survey", "--components", map.to_str().unwrap()]);
    let text = printed(&checked);
    assert!(checked.status.success(), "{text}");
    assert_eq!(text.trim(), "", "{text}");

    // The drafts: the layers' first entries, a node per part, and the root's map.
    let append = |rel: &str, entry: &str| {
        let now = fs::read_to_string(tree.join(rel)).unwrap();
        spec.write(rel, &format!("{now}\n{entry}"));
    };
    append(
        "CAPABILITIES.md",
        "### Capability: greet\nA person can be greeted.\n\n#### Why\nA greeting is the product.\n\n#### Scenario: greeting\n- GIVEN the greeter\n- WHEN a person runs `greet`\n- THEN it says hi\n",
    );
    append(
        "ARCHITECTURE.md",
        "### Constraint: shell-only\nThe greeter SHALL run in a POSIX shell alone.\n\n#### Scenario: no runtime\n- GIVEN a machine with only `sh`\n- WHEN `greet` runs\n- THEN it works\n\n#### Untested\nWhat a machine has installed is a judgement on the design.\n",
    );
    spec.write(
        &format!("changes/{CHANGE}/change.md"),
        "---\nstatus: proposed\n---\n\n## Intent\n\nThe greeter's first spec graph, recovered from its code. The nodes' builds prove each requirement with versioned markers, and the `capability` node's build proves the capabilities with tests citing `req:capability/<slug>@<version>`. The detectors stay off until the root node's build.\n",
    );
    spec.write(&format!("changes/{CHANGE}/deltas/greeter.md"), GREETER);
    spec.write(
        &format!("changes/{CHANGE}/deltas/capability.md"),
        CAPABILITY,
    );
    let base = |element: &str| spec.hash("clm", element);
    spec.write(
        &format!("changes/{CHANGE}/deltas/clm.md"),
        &format!(
            "---\ncomponent: clm\ncreate: false\nbase: {{ frontmatter: \"{}\", contract: \"{}\", design: \"{}\" }}\n---\n## MODIFIED Frontmatter\n\n```yaml\npaths: [\".gitignore\", \"README.md\", \"clm.toml\", \"scripts/clm-check.sh\"]\noutside: []\n```\n\n## MODIFIED Contract\n\nThe greeter greets.\n\n## MODIFIED Design\n\n| Node | Responsibility |\n|---|---|\n| `clm` (root) | The tooling |\n| `greeter` | The greeting |\n| `capability` | The capabilities' tests |\n",
            base("frontmatter"),
            base("contract"),
            base("design")
        ),
    );
    clm(&spec, &["check"]);
    clm(&spec, &["apply", CHANGE]);
    spec.commit("the first spec graph");

    // A node per part of the code, describing what it does today, and the check passes.
    let greeter = fs::read_to_string(tree.join("specs/greeter/SPEC.md")).unwrap();
    assert!(greeter.contains("`greet` SHALL print `hi`."), "{greeter}");
    assert!(tree.join("specs/capability/SPEC.md").exists());
    clm(&spec, &["check"]);
    let pending = clm(&spec, &["pending"]);
    assert!(
        pending.lines().any(|l| l.starts_with("greeter/says-hi ")),
        "{pending}"
    );
    // It waits for the person's approval, since it drafts the layers.
    spec.git(&["fetch", "-q", "origin"]);
    let gate = spec.clm(&["gate", "--base", "origin/main"]);
    let text = String::from_utf8_lossy(&gate.stdout).into_owned();
    assert_eq!(
        text.lines().next(),
        Some("spec review"),
        "{}",
        printed(&gate)
    );
}
