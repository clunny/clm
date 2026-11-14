//! `test_depends_on` is a key the model doesn't know.

use std::fs;
use std::path::Path;

use clm_model::load;
use tempfile::TempDir;

const FM: &str = "title: T\nstatus: unbuilt\nowners: [\"@a\"]";

#[test]
fn test_dependencies_retired() {
    // req:model/test-dependencies-retired@16bae7f6
    let dir = TempDir::new().unwrap();
    for (id, extra) in [("", ""), ("a/", "\ntest_depends_on: [b]")] {
        let path = dir.path().join(format!("specs/{id}SPEC.md"));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, format!("---\n{FM}{extra}\n---\n")).unwrap();
    }
    let (_, violations) = load(dir.path());
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(violations[0].rule, "model/valid-frontmatter");
    assert_eq!(violations[0].path, Path::new("specs/a/SPEC.md"));
    assert!(violations[0].message.contains("test_depends_on"));
}
