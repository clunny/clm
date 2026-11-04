//! Which node a file belongs to, as library calls (`path-ownership`).

use std::fs;
use std::path::Path;

use tempfile::TempDir;

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn node(paths: &str) -> String {
    format!("---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\n---\n## Contract\n")
}

#[test]
fn owned_and_unowned() {
    // req:check/path-ownership@5c51292c
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "specs/SPEC.md", &node("[]"));
    write(root, "specs/a/SPEC.md", &node("[\"crates/a/**\"]"));
    write(root, "specs/a/b/SPEC.md", &node("[\"crates/b/**\"]"));
    let (graph, violations) = clm_model::load(root);
    assert!(violations.is_empty(), "{violations:?}");
    let owners: Vec<_> = ["crates/a/src/lib.rs", "crates/b/x.rs", "tools/x.sh"]
        .into_iter()
        .map(|p| clm_check::owner(&graph, Path::new(p)))
        .collect();
    assert_eq!(owners, [Some("a"), Some("a.b"), None]);
}

#[test]
fn one_glob() {
    // req:check/path-ownership@5c51292c
    assert!(clm_check::path_matches(
        "crates/*/tests/**",
        "crates/a/tests/t.rs"
    ));
    assert!(!clm_check::path_matches(
        "crates/*/tests/**",
        "crates/a/src/t.rs"
    ));
}

#[test]
fn scan_stays_public() {
    // req:check/path-ownership@5c51292c
    let dir = TempDir::new().unwrap();
    write(dir.path(), "t.rs", concat!("// req", ":a/x@00000000\n"));
    let scan: clm_check::Scan = clm_check::scan(dir.path());
    assert_eq!(scan.req.len(), 1);
    assert_eq!(
        (scan.req[0].path.as_path(), scan.req[0].line),
        (Path::new("t.rs"), 1)
    );
}
