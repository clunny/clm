use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn clm_context(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .arg("context")
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
        .unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// A node spec whose contract and design are `<NAME>-CONTRACT` and `<NAME>-DESIGN`.
fn node(root: &Path, dir: &str, name: &str, deps: &str) {
    let rel = if dir.is_empty() {
        "specs/SPEC.md".to_string()
    } else {
        format!("specs/{dir}/SPEC.md")
    };
    write(
        root,
        &rel,
        &format!(
            "---\ntitle: {name}\nstatus: specd\nowners: []\ndepends_on: [{deps}]\n---\n\n\
             ## Contract\n{name}-CONTRACT\n\n## Design\n{name}-DESIGN\n\n## Requirements\n\n\
             ### Requirement: {lower}-req\n{name}-REQ\n\n#### Scenario: s\n- GIVEN x\n",
            lower = name.to_lowercase()
        ),
    );
}

fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "ARCHITECTURE.md", "ARCH-TEXT\n");
    node(dir.path(), "", "ROOT", "");
    node(dir.path(), "a", "A", "c, b");
    node(dir.path(), "a/n", "N", "b");
    node(dir.path(), "b", "B", "");
    node(dir.path(), "c", "C", "");
    write(
        dir.path(),
        "backlog/x.md",
        "---\ntitle: X\nkind: idea\ncomponents: [a]\n---\nBACKLOG-TEXT\n",
    );
    write(
        dir.path(),
        "docs/decisions/d.md",
        "---\ntitle: D\ndate: 2026-10-21\nstatus: accepted\n---\nDECISION-TEXT\n",
    );
    write(
        dir.path(),
        "changes/20261021-x/change.md",
        "---\nstatus: proposed\n---\n\n## Intent\nCHANGE-TEXT\n",
    );
    dir
}

#[test]
fn dependency_contract_only() {
    // req:context/bundle-contents@88b3e0e8
    let dir = repo();
    write(dir.path(), "specs/b/assets/bm.html", "BM-CONTENT\n");
    let out = clm_context(dir.path(), &["a"]);
    assert_eq!(out.status.code(), Some(0));
    let text = stdout(&out);
    for present in [
        "ARCH-TEXT",
        "ROOT-CONTRACT",
        "ROOT-DESIGN",
        "A-CONTRACT",
        "A-DESIGN",
        "A-REQ",
        "depends_on: [c, b]",
        "B-CONTRACT",
        "C-CONTRACT",
    ] {
        assert!(text.contains(present), "missing {present}:\n{text}");
    }
    for absent in [
        "B-DESIGN",
        "B-REQ",
        "b-req",
        "ROOT-REQ",
        "N-CONTRACT",
        "BACKLOG-TEXT",
        "DECISION-TEXT",
        "CHANGE-TEXT",
        "bm.html",
        "BM-CONTENT",
        "### Assets",
    ] {
        assert!(!text.contains(absent), "leaked {absent}:\n{text}");
    }
}

#[test]
fn assets_listed_not_included() {
    // req:context/bundle-contents@88b3e0e8
    let dir = repo();
    write(dir.path(), "specs/a/assets/mockups/m.html", "M-CONTENT\n");
    write(dir.path(), "specs/a/assets/b.png", "B-CONTENT\n");
    let text = stdout(&clm_context(dir.path(), &["a"]));
    let expected = format!(
        "### Assets\n\n- specs/a/assets/b.png {}\n- specs/a/assets/mockups/m.html {}\n",
        clm_model::asset_hash(b"B-CONTENT\n"),
        clm_model::asset_hash(b"M-CONTENT\n"),
    );
    assert!(text.contains(&expected), "{text}");
    assert!(text.find("A-REQ").unwrap() < text.find("### Assets").unwrap());
    assert!(text.find("### Assets").unwrap() < text.find("# Dependencies").unwrap());
    assert!(!text.contains("M-CONTENT"), "{text}");

    let hash = || stdout(&clm_context(dir.path(), &["a", "--hash"]));
    let before = hash();
    write(
        dir.path(),
        "specs/a/assets/mockups/m.html",
        "M-CONTENT-EDITED\n",
    );
    assert_ne!(hash(), before, "an asset's bytes are in the bundle hash");
}

#[test]
fn spine_assets_stay_out() {
    // req:context/bundle-contents@88b3e0e8
    let dir = repo();
    write(dir.path(), "specs/a/assets/m.html", "M\n");
    write(dir.path(), "specs/a/n/assets/x.bin", "X\n");
    let text = stdout(&clm_context(dir.path(), &["a.n"]));
    assert!(!text.contains("m.html"), "{text}");
    assert!(
        text.contains(&format!(
            "- specs/a/n/assets/x.bin {}",
            clm_model::asset_hash(b"X\n")
        )),
        "the node path, not the ID:\n{text}"
    );
}

#[test]
fn bundle_from_equals_checkout() {
    // req:context/bundle-from-text@418dfef8
    let dir = repo();
    write(dir.path(), "specs/a/assets/m.html", "M\n");
    let (graph, violations) = clm_model::load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    let arch = fs::read_to_string(dir.path().join("ARCHITECTURE.md")).unwrap();
    assert_eq!(
        clm_context::bundle_from(&graph, Some(&arch), "a").unwrap(),
        stdout(&clm_context(dir.path(), &["a"])),
    );

    fs::remove_file(dir.path().join("ARCHITECTURE.md")).unwrap();
    let without = stdout(&clm_context(dir.path(), &["a"]));
    assert!(!without.contains("ARCH-TEXT"), "{without}");
    assert_eq!(
        clm_context::bundle_from(&graph, None, "a").unwrap(),
        without
    );
    assert_eq!(clm_context::bundle_from(&graph, None, "zz"), None);
}

#[test]
fn bundle_from_equals_checkout_with_capabilities() {
    // req:context/bundle-from-text@418dfef8
    // req:context/bundle-contents@88b3e0e8
    let dir = repo();
    write(
        dir.path(),
        "CAPABILITIES.md",
        "## Capabilities\n\n### Capability: k\nCAP-K\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n\n\
         ### Capability: u\nCAP-U\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n",
    );
    let spec = dir.path().join("specs/a/SPEC.md");
    let text = fs::read_to_string(&spec).unwrap();
    fs::write(
        &spec,
        text.replace("owners: []\n", "owners: []\nserves: [capability/k]\n"),
    )
    .unwrap();
    let (graph, violations) = clm_model::load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");

    let out = stdout(&clm_context(dir.path(), &["a"]));
    assert!(
        out.contains("# Capabilities served\n\n### Capability: k\nCAP-K"),
        "{out}"
    );
    assert!(!out.contains("CAP-U"), "{out}");
    let arch = fs::read_to_string(dir.path().join("ARCHITECTURE.md")).unwrap();
    assert_eq!(
        clm_context::bundle_from(&graph, Some(&arch), "a").unwrap(),
        out
    );
}

#[test]
fn nested_node_sees_its_spine_in_order() {
    // req:context/bundle-contents@88b3e0e8
    let dir = repo();
    let text = stdout(&clm_context(dir.path(), &["a.n"]));
    let pos = |s: &str| {
        text.find(s)
            .unwrap_or_else(|| panic!("missing {s}:\n{text}"))
    };
    assert!(pos("ROOT-CONTRACT") < pos("ROOT-DESIGN"));
    assert!(pos("ROOT-DESIGN") < pos("A-CONTRACT"));
    assert!(pos("A-DESIGN") < pos("N-CONTRACT"));
    assert!(pos("N-DESIGN") < pos("N-REQ"));
    assert!(!text.contains("A-REQ"), "{text}");
}

#[test]
fn identical_runs_and_sorted_dependencies() {
    // req:context/bundle-deterministic@1efac81a
    let dir = repo();
    let first = stdout(&clm_context(dir.path(), &["a"]));
    let second = stdout(&clm_context(dir.path(), &["a"]));
    assert_eq!(first, second);
    assert!(first.find("B-CONTRACT").unwrap() < first.find("C-CONTRACT").unwrap());
    assert!(first.ends_with("\n") && !first.ends_with("\n\n"));
}

#[test]
fn hash_follows_the_bundle_only() {
    // req:context/bundle-hash@7010972d
    let dir = repo();
    let hash = || stdout(&clm_context(dir.path(), &["a", "--hash"]));
    let before = hash();
    let bundle = stdout(&clm_context(dir.path(), &["a"]));
    assert_eq!(before.trim().len(), 16, "{before}");
    assert_eq!(before.trim(), clm_model::content_hash(&bundle));

    let b = dir.path().join("specs/b/SPEC.md");
    let source = fs::read_to_string(&b).unwrap();
    fs::write(&b, source.replace("B-DESIGN", "B-DESIGN-EDITED")).unwrap();
    assert_eq!(hash(), before, "dependency design is outside the bundle");

    let source = fs::read_to_string(&b).unwrap();
    fs::write(&b, source.replace("B-CONTRACT", "B-CONTRACT-EDITED")).unwrap();
    assert_ne!(hash(), before, "dependency contract is inside the bundle");
}

#[test]
fn unknown_node_exits_2() {
    // req:context/bundle-errors@fa9b5998
    let dir = repo();
    let out = clm_context(dir.path(), &["zz"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8(out.stderr).unwrap().contains("`zz`"));
    assert!(out.stdout.is_empty());
}

#[test]
fn invalid_graph_prints_violations_not_a_bundle() {
    // req:context/bundle-errors@fa9b5998
    let dir = repo();
    node(dir.path(), "b", "B", "missing");
    let out = clm_context(dir.path(), &["a"]);
    let text = stdout(&out);
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(text.contains("model/dependency-dag"), "{text}");
    assert!(!text.contains("A-CONTRACT"), "{text}");
}
