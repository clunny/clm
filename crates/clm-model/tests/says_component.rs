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
    assert!(v[0].message.contains("required component `b`"), "{v:?}");
    assert!(v[0].message.contains("optional component `a`"), "{v:?}");
}
