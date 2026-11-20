//! The messages of `clm apply` say component, never the old words (`component-words`).

use std::fs;
use std::path::Path;

use clm_apply::{apply, validate};
use clm_model::load;
use tempfile::TempDir;

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nroot\n\n## Design\nroot\n\n## Requirements\n";
const A: &str = "---\ntitle: A\nstatus: unbuilt\nowners: []\ndepends_on: []\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn creating_a_component_that_exists() {
    // req:apply/says-component@e89bf27f
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", A);
    write(d, "changes/c/change.md", "---\nstatus: proposed\n---\n");
    write(
        d,
        "changes/c/deltas/a.md",
        "---\ncomponent: a\ncreate: true\n---\n## Frontmatter\n```yaml\ntitle: A\nstatus: unbuilt\nowners: []\n```\n\n## Contract\nc\n\n## Design\nd\n",
    );
    let (graph, _) = load(d);
    let violations = validate(&graph, &graph.changes["c"]);
    let before = fs::read_to_string(d.join("specs/a/SPEC.md")).unwrap();
    let refused = apply(d, "c").unwrap_err();
    assert!(
        refused
            .iter()
            .any(|v| v.message.contains("component `a` already exists")),
        "{refused:?}"
    );
    assert_eq!(
        fs::read_to_string(d.join("specs/a/SPEC.md")).unwrap(),
        before
    );
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("component `a` already exists")),
        "{violations:?}"
    );
}

#[test]
fn the_other_messages_say_component() {
    // req:apply/says-component@e89bf27f
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "changes/c/change.md", "---\nstatus: proposed\n---\n");
    write(
        d,
        "changes/c/deltas/gone.md",
        "---\ncomponent: gone\nbase: { design: \"0000000000000000\" }\n---\n## MODIFIED Design\nd2\n",
    );
    write(
        d,
        "changes/c/deltas/clm.md",
        "---\ncomponent: clm\nremove: true\n---\n## REMOVED Requirements\n",
    );
    write(d, "changes/c/assets/clm/x.png", "png");
    let messages: Vec<String> = apply(d, "c")
        .unwrap_err()
        .into_iter()
        .map(|v| v.message)
        .collect();
    for expected in [
        "component `gone` no longer exists",
        "the root component `clm` can't be removed",
        "the root component has no assets",
    ] {
        assert!(
            messages.iter().any(|m| m.contains(expected)),
            "{expected}: {messages:?}"
        );
    }
}
