//! `doc_only` (ledger/doc-only) and `legacy_doc_only` (ledger/legacy-doc-only). Built with the default features off as well: `tests/wasm.rs`
//! runs this file so.

#[test]
fn paths() {
    // req:ledger/doc-only@6986bbf0
    for (path, want) in [
        ("docs/x.md", true),
        ("docs/guide/intro.md", true),
        ("docs/spec-format.md", true),
        ("backlog/y.md", true),
        ("CLAUDE.md", true),
        ("AGENTS.md", true),
        ("crates/a/src/lib.rs", false),
        ("docsy/z.md", false),
        // The rest of the list.
        ("specs/SPEC.md", true),
        ("changes/x/change.md", true),
        ("ARCHITECTURE.md", true),
        ("CAPABILITIES.md", true),
        ("crates/a/CAPABILITIES.md", false),
        ("README.md", true),
        ("crates/a/README.md", false),
        ("crates/a/AGENTS.md", false),
        ("docs", false),
    ] {
        assert_eq!(clm_ledger::doc_only(path), want, "{path}");
    }
}

#[test]
fn legacy_paths() {
    // req:ledger/legacy-doc-only@ddac0ce8
    for (path, want) in [
        ("docs/spec-format.md", true),
        ("docs/x.md", true),
        ("docs/guide/intro.md", false),
        ("CLAUDE.md", true),
        ("crates/a/src/lib.rs", false),
        // The rest of the list as it was.
        ("specs/SPEC.md", true),
        ("changes/x/change.md", true),
        ("backlog/y.md", true),
        ("ARCHITECTURE.md", true),
        // Frozen: the file didn't exist when the keyless records were written.
        ("CAPABILITIES.md", false),
        ("README.md", true),
        ("crates/a/README.md", false),
        // Frozen: AGENTS.md was code then.
        ("AGENTS.md", false),
        ("docsy/z.md", false),
    ] {
        assert_eq!(clm_ledger::legacy_doc_only(path), want, "{path}");
    }
}

#[test]
fn legacy_list_is_frozen() {
    // req:ledger/legacy-doc-only@ddac0ce8
    let src =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/doc_only.rs")).unwrap();
    let body = &src[src.find("pub fn legacy_doc_only").expect("legacy_doc_only")..];
    // After the signature, which names the function itself.
    let body = &body[body.find('{').unwrap()..];
    let body = &body[..body.find("\n}").expect("its body's end")];
    assert!(!body.contains("doc_only("), "calls doc_only:\n{body}");
    assert!(body.contains("LEGACY_DOC_ONLY"), "{body}");
    let list = &src[src
        .find("const LEGACY_DOC_ONLY")
        .expect("a constant of its own")..];
    let list = &list[..list.find("];").unwrap()];
    for text in [body, list] {
        assert!(!text.contains("spec-format"), "{text}");
    }
}
