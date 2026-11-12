mod common;

use common::{commit, repo, sh};

#[test]
fn doc_only_difference() {
    // req:ledger/code-key@9217c035
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    repo(root, "T");
    let base = [
        ("crates/a/src/lib.rs", "fn a() {}\n"),
        ("docs/guide/intro.md", "intro\n"),
        ("docs/spec-format.md", "format\n"),
        ("specs/SPEC.md", "spec\n"),
        ("ARCHITECTURE.md", "arch\n"),
        ("CLAUDE.md", "claude\n"),
        ("AGENTS.md", "agents\n"),
    ];
    let a = commit(root, &base, "a");
    let b = commit(root, &[("docs/x.md", "x\n"), ("backlog/y.md", "y\n")], "b");
    // More doc-only paths, still the same code.
    let b2 = commit(
        root,
        &[
            ("specs/SPEC.md", "spec 2\n"),
            ("changes/c/change.md", "c\n"),
            ("ARCHITECTURE.md", "arch 2\n"),
            ("CAPABILITIES.md", "caps 2\n"),
            ("README.md", "readme 2\n"),
            ("CLAUDE.md", "claude 2\n"),
            ("AGENTS.md", "agents 2\n"),
        ],
        "b2",
    );
    let c = commit(root, &[("crates/a/src/lib.rs", "fn a() { 1; }\n")], "c");
    sh(root, &["reset", "-q", "--hard", &b]);
    let d = commit(root, &[("tool.toml", "x = 1\n")], "d");
    sh(root, &["reset", "-q", "--hard", &b]);
    let e = commit(root, &[("docs/guide/intro.md", "intro 2\n")], "e");
    sh(root, &["reset", "-q", "--hard", &b]);
    // A file named like a doc-only one, but not at the root, is code.
    let named = commit(root, &[("crates/a/README.md", "readme\n")], "named");
    sh(root, &["reset", "-q", "--hard", &b]);
    // The guide's pages and the spec format are doc-only too.
    let f = commit(root, &[("docs/spec-format.md", "format 2\n")], "f");

    let key = |rev: &str| clm_ledger::code_key(root, rev).unwrap();
    let ka = key(&a);
    assert_eq!(ka.len(), 64, "{ka}");
    assert!(ka.chars().all(|ch| ch.is_ascii_hexdigit()), "{ka}");
    assert_eq!(key(&b), ka);
    assert_eq!(key(&b2), ka);
    assert_eq!(key(&e), ka);
    assert_eq!(key(&f), ka);
    let others = [key(&c), key(&d), key(&named)];
    for (i, k) in others.iter().enumerate() {
        assert_ne!(*k, ka, "commit {i} has a's key");
        for other in &others[i + 1..] {
            assert_ne!(k, other);
        }
    }
}
