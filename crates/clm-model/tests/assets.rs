use std::fs;
use std::path::{Path, PathBuf};

use clm_model::{Files, Graph, Violation, asset_hash, content_hash, load, load_files};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const NODE: &str = "---\ntitle: T\nstatus: active\nowners: []\n---\n\n## Requirements\n";

fn files(entries: &[(&str, &[u8])]) -> Files {
    entries
        .iter()
        .map(|(p, b)| (PathBuf::from(p), b.to_vec()))
        .collect()
}

fn on_disk(files: &Files) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (rel, bytes) in files {
        let path = dir.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    dir
}

fn rules(v: &[Violation]) -> Vec<&str> {
    v.iter().map(|v| v.rule).collect()
}

fn clean(files: &Files) -> Graph {
    let (graph, violations) = load_files(files);
    assert!(violations.is_empty(), "{violations:?}");
    graph
}

#[test]
fn same_as_disk() {
    // req:model/load-from-files@5011a677
    let map = files(&[
        ("specs/SPEC.md", NODE.as_bytes()),
        ("specs/a/SPEC.md", NODE.as_bytes()),
        ("specs/a/assets/m.png", &[0xff, 0, 1]),
        (
            "specs/b/SPEC.md",
            b"---\ntitle: B\nstatus: done\nowners: []\n---\n",
        ),
        ("specs/c/SPEC.md", &[0xff, 0xfe]),
        ("changes/x/change.md", b"---\nstatus: proposed\n---\n"),
        (
            "changes/x/deltas/a.md",
            b"---\ncomponent: a\n---\n## MODIFIED Contract\nnew\n",
        ),
        ("changes/archive/y/change.md", b"---\nstatus: done\n---\n"),
        (
            "backlog/i.md",
            b"---\ntitle: I\nkind: idea\ncomponents: [a]\n---\n",
        ),
        (
            "docs/decisions/d.md",
            b"---\ntitle: D\ndate: 2026-10-22\nstatus: accepted\n---\n",
        ),
    ]);
    let dir = on_disk(&map);
    let from_disk = load(dir.path());
    let from_map = load_files(&map);
    assert_eq!(from_disk.0, from_map.0);
    assert_eq!(from_disk.1, from_map.1);
    assert_eq!(
        rules(&from_map.1),
        ["model/valid-frontmatter", "model/parse-node-specs"],
        "{:?}",
        from_map.1
    );
    let g = &from_map.0;
    assert!(g.nodes.contains_key("a"));
    assert!(g.changes["y"].archived);
    assert!(g.backlog.contains_key("i"));
    assert!(g.decisions.contains_key("d"));
    assert_eq!(from_map.1[1].path, Path::new("specs/c/SPEC.md"));
    assert_eq!(
        g.nodes["a"].assets["m.png"],
        asset_hash(&[0xff, 0, 1]),
        "assets load from both"
    );
}

#[test]
fn asset_hash_of_text() {
    // req:model/node-assets@e54a1acb
    let graph = clean(&files(&[
        ("specs/SPEC.md", NODE.as_bytes()),
        ("specs/a/SPEC.md", NODE.as_bytes()),
        ("specs/a/assets/mockups/m.html", b"<p>x</p>"),
    ]));
    let assets = &graph.nodes["a"].assets;
    assert_eq!(assets.len(), 1, "{assets:?}");
    assert_eq!(assets["mockups/m.html"], content_hash("<p>x</p>"));
    assert!(graph.nodes["clm"].assets.is_empty());
}

#[test]
fn asset_hash_of_bytes() {
    // req:model/node-assets@e54a1acb
    let bytes = [0xff, 0xfe, 0, 7];
    let digest = Sha256::digest(bytes);
    let expected: String = digest[..8].iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(asset_hash(&bytes), expected);
    assert_ne!(
        asset_hash(&bytes),
        content_hash(&String::from_utf8_lossy(&bytes))
    );
    assert_eq!(asset_hash(b"<p>x</p>\r\n"), content_hash("<p>x</p>"));
}

#[test]
fn spec_inside_assets() {
    // req:model/parse-node-specs@f87bf5ee
    let graph = clean(&files(&[
        ("specs/SPEC.md", NODE.as_bytes()),
        ("specs/a/SPEC.md", NODE.as_bytes()),
        ("specs/a/assets/SPEC.md", NODE.as_bytes()),
        ("specs/a/assets/b/SPEC.md", b"not a spec"),
    ]));
    assert_eq!(graph.nodes.keys().collect::<Vec<_>>(), ["a", "clm"]);
    let assets: Vec<_> = graph.nodes["a"].assets.keys().collect();
    assert_eq!(assets, ["SPEC.md", "b/SPEC.md"]);
}

#[test]
fn root_has_no_assets() {
    // req:model/node-assets@e54a1acb
    let (graph, violations) = load_files(&files(&[
        ("specs/SPEC.md", NODE.as_bytes()),
        ("specs/assets/m.png", &[1]),
    ]));
    assert_eq!(rules(&violations), ["model/node-assets"]);
    assert_eq!(violations[0].path, Path::new("specs/assets/m.png"));
    assert!(graph.nodes["clm"].assets.is_empty());
}

#[test]
fn assets_of_a_missing_node() {
    // req:model/node-assets@e54a1acb
    let (_, violations) = load_files(&files(&[
        ("specs/SPEC.md", NODE.as_bytes()),
        ("specs/ghost/assets/m.png", &[1]),
    ]));
    assert_eq!(rules(&violations), ["model/node-assets"]);
    assert!(violations[0].message.contains("`ghost`"), "{violations:?}");
}

#[cfg(unix)]
#[test]
fn links_are_not_followed_into_directories() {
    // req:model/load-from-files@5011a677
    use std::os::unix::fs::symlink;
    let dir = on_disk(&files(&[
        ("specs/SPEC.md", NODE.as_bytes()),
        ("changes/x/change.md", b"---\nstatus: proposed\n---\n"),
    ]));
    symlink("..", dir.path().join("changes/x/up")).unwrap();
    symlink("gone.md", dir.path().join("changes/x/dangling.md")).unwrap();
    let (graph, violations) = load(dir.path());
    assert!(graph.changes.contains_key("x"));
    assert_eq!(
        rules(&violations),
        ["model/load-from-files", "model/load-from-files"]
    );
    assert!(
        violations
            .iter()
            .all(|v| v.message.starts_with("unreadable"))
    );
}

#[test]
fn files_outside_the_loaded_dirs_are_ignored() {
    // req:model/load-from-files@5011a677
    let mut map = files(&[("specs/SPEC.md", NODE.as_bytes())]);
    let graph = clean(&map);
    map.insert("src/SPEC.md".into(), b"junk".to_vec());
    assert_eq!(clean(&map), graph);
}
