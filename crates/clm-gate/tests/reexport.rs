//! `clm-gate` re-exports `clm-fingerprint` and defines none of it (gate/fingerprint-reexported).

use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn listing(second_blob: &str) -> Vec<u8> {
    format!(
        "100644 blob {}\tsrc/a.rs\0100644 blob {second_blob}\tsrc/b.rs\0",
        "1".repeat(40)
    )
    .into_bytes()
}

#[test]
fn same_function_through_the_gate() {
    // req:gate/fingerprint-reexported@a216e732
    let base: Vec<clm_gate::Entry> = clm_gate::parse_ls_tree(&listing(&"2".repeat(40)));
    let head = clm_gate::parse_ls_tree(&listing(&"3".repeat(40)));
    assert_eq!(base.len(), 2);
    let through_gate = clm_gate::fingerprint_of(&base, &head);
    let direct = clm_fingerprint::fingerprint_of(
        &clm_fingerprint::parse_ls_tree(&listing(&"2".repeat(40))),
        &clm_fingerprint::parse_ls_tree(&listing(&"3".repeat(40))),
    );
    assert_eq!(through_gate, direct);
    // The other re-exports name the same items: coercing each to the original's
    // function type fails to compile if one is missing or a local copy.
    let _: fn(&[clm_fingerprint::Entry], &[clm_fingerprint::Entry]) -> bool = clm_gate::adds_change;
    let _: fn(&[clm_fingerprint::Entry], &[clm_fingerprint::Entry]) -> Vec<Vec<u8>> =
        clm_gate::added_changes;
    let _: fn(&Path, &str) -> Result<String, String> = clm_gate::fingerprint;
    assert_eq!(
        clm_gate::adds_change(&base, &head),
        clm_fingerprint::adds_change(&base, &head)
    );
}

#[test]
fn no_copy_left() {
    // req:gate/fingerprint-reexported@a216e732
    let krate = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest: toml::Table = std::fs::read_to_string(krate.join("Cargo.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let deps = manifest["dependencies"].as_table().unwrap();
    assert!(deps.contains_key("clm-fingerprint"));
    assert!(!deps.contains_key("sha2"));
    let mut files = Vec::new();
    rust_files(&krate.join("src"), &mut files);
    assert!(!files.is_empty());
    let defs = [
        "struct Entry",
        "fn parse_ls_tree",
        "fn fingerprint_of",
        "fn adds_change",
        "fn added_changes",
        "fn fingerprint(",
    ];
    let found: Vec<String> = files
        .iter()
        .filter(|f| {
            let text = std::fs::read_to_string(f).unwrap();
            defs.iter().any(|d| text.contains(d))
        })
        .map(|f| f.display().to_string())
        .collect();
    assert!(found.is_empty(), "copies left: {found:?}");
}
