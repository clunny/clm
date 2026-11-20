//! `says-component`: the model's messages say component, never the old words.

use clm_model::{Files, load_files};
use std::path::PathBuf;

fn load(files: &[(&str, &str)]) -> Vec<clm_model::Violation> {
    let mut map = Files::new();
    for (p, t) in files {
        map.insert(PathBuf::from(p), t.as_bytes().to_vec());
    }
    load_files(&map).1
}

fn spec(extra: &str) -> String {
    format!(
        "---\ntitle: T\nstatus: unbuilt\nowners: []\n{extra}---\n\n## Contract\nc\n\n## Design\nd\n"
    )
}

const ROOT: &str = "---\ntitle: T\nstatus: active\nowners: []\n---\n\n## Contract\nc\n";

// req:model/says-component@e37a6f5e
#[test]
fn an_unknown_component_in_an_item() {
    let item = "---\ntitle: X\nkind: bug\ncomponents: [ghost]\n---\nbody\n";
    let v = load(&[("specs/SPEC.md", ROOT), ("backlog/x.md", item)]);
    assert_eq!(v.len(), 1, "{v:?}");
    assert!(v[0].message.contains("unknown component `ghost`"), "{v:?}");
}

// req:model/says-component@e37a6f5e
#[test]
fn a_required_component_on_an_optional_one() {
    let a = spec("optional: true\n");
    let b = spec("depends_on: [a]\n");
    let v = load(&[
        ("specs/SPEC.md", ROOT),
        ("specs/a/SPEC.md", &a),
        ("specs/b/SPEC.md", &b),
    ]);
    assert_eq!(v.len(), 1, "{v:?}");
    assert!(
        v[0].message
            .contains("required component `b` depends on optional component `a`"),
        "{v:?}"
    );
}

// req:model/says-component@e37a6f5e
#[test]
fn the_shapes_it_names() {
    let needs = "---\ntitle: T\nstatus: unbuilt\nowners: []\n---\n\n## Requirements\n\n### Requirement: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n\n#### Needs\nneeds b/y\n";
    says(&[("specs/a/SPEC.md", needs)], "- <component>/<slug>`");
    let flag = "---\ntitle: T\nkind: workaround\nrequirement: nonsense\n---\n";
    says(&[("backlog/f.md", flag)], "<component>/<slug>@<version>`");
}

/// The one violation loading `files` with the root gives says `words`.
fn says(files: &[(&str, &str)], words: &str) {
    let mut all = vec![("specs/SPEC.md", ROOT)];
    all.extend_from_slice(files);
    let v = load(&all);
    assert_eq!(v.len(), 1, "{words}: {v:?}");
    assert!(v[0].message.contains(words), "{words}: {v:?}");
}

// req:model/says-component@e37a6f5e
#[test]
fn the_messages_it_names() {
    let a = spec("");
    says(
        &[("specs/assets/m.png", "x")],
        "the root component has no assets",
    );
    says(
        &[("specs/ghost/assets/m.png", "x")],
        "asset of component `ghost`",
    );
    says(
        &[("specs/a/b/SPEC.md", &a)],
        "parent component `a` has no SPEC.md",
    );
    let root = ROOT.replace("owners: []\n", "owners: []\noptional: true\n");
    let v = load(&[("specs/SPEC.md", &root)]);
    assert_eq!(v.len(), 1, "{v:?}");
    assert!(
        v[0].message
            .contains("the root component can't be `optional`"),
        "{v:?}"
    );
    says(
        &[("specs/a/SPEC.md", &spec("depends_on: [ghost]\n"))],
        "depends on unknown component `ghost`",
    );
    let change = "---\nstatus: proposed\n---\n";
    says(
        &[
            ("specs/a/SPEC.md", &a),
            ("changes/c/change.md", change),
            (
                "changes/c/deltas/a.md",
                "---\ncomponent: a\n---\n## Design\nd2\n",
            ),
        ],
        "edits an existing component's Design",
    );
    says(
        &[
            ("specs/a/SPEC.md", &a),
            ("changes/c/change.md", change),
            (
                "changes/c/deltas/a.md",
                "---\ncomponent: a\ncreate: true\nremove: true\n---\n",
            ),
        ],
        "creates its component or removes it",
    );
}
