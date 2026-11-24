//! `replica/files-a-source-gives`: what a replica holds for each kind of `File` a source
//! builds.

mod common;

use clm_replica::{Change, File, Replica};
use common::*;
use serde_json::Value;

/// The markers the snapshot lists for requirement `id`.
fn markers(snapshot: &Value, id: &str) -> Vec<String> {
    snapshot["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|n| n["requirements"].as_array().unwrap())
        .find(|r| r["id"] == id)
        .unwrap()["markers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn each_kind_of_file() {
    // req:replica/files-a-source-gives@730886a9
    let mut f = clm_repo();
    let current = version_of(&f, "a/p");
    let test = format!(
        "#[test]\nfn t() {{\n    // {}@{current}\n}}\n",
        concat!("req", ":a/p")
    );
    let mut nul = "0123456789".to_string();
    nul.push('\0');
    nul.push_str(&format!("\n{test}"));
    f.extend(files(&[
        ("src/a.rs", &test),
        ("src/nul.rs", &nul),
        ("src/c.rs", "fn c() {}\n"),
    ]));
    f.insert("src/b.bin".into(), vec![0xff, 0x00, 0xfe]);

    let mut r = Replica::default();
    r.apply(tree("111", &f));
    let given = |p: &str| String::from_utf8(f[p].clone()).unwrap();
    let mut given_files: Vec<File> = f
        .keys()
        .filter(|p| p.ends_with(".md") || p.as_str() == "src/a.rs")
        .map(|p| File::text(entry(p, &f[p]), given(p)))
        .collect();
    given_files.push(File::binary(entry("src/b.bin", &f["src/b.bin"])));
    given_files.push(File::text(
        entry("src/nul.rs", &f["src/nul.rs"]),
        given("src/nul.rs"),
    ));
    given_files.push(File::unfetched(entry("src/c.rs", &f["src/c.rs"])));
    r.apply(Change::Tip {
        sha: "111".into(),
        files: given_files,
        removed: Vec::new(),
    });

    assert_eq!(r.pending_fetch(), ["src/c.rs"]);
    // The snapshot lists markers once every scannable file is held, so give the last one.
    r.apply(Change::Tip {
        sha: "111".into(),
        files: vec![File::text(
            entry("src/c.rs", &f["src/c.rs"]),
            given("src/c.rs"),
        )],
        removed: Vec::new(),
    });
    assert!(r.pending_fetch().is_empty());
    r.recompute(1_000);
    let snapshot = view(&r, "snapshot");
    let found = markers(&snapshot, "a/p");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("src/a.rs"), "{found:?}");
    // The spec file's text is held: its requirement is in the snapshot.
    assert!(
        snapshot["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["id"] == "a")
    );
}
