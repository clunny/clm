//! `replica/asset-hashes`: a binary asset given with its bytes is held by its hash alone.

mod common;

use clm_replica::{Change, File, Replica, Store, keeps_hash};
use common::*;
use serde_json::Value;

const LOGO: &str = "specs/b/assets/logo.png";
/// Text in the asset's bytes that a saved store would show if it kept them.
const MARK: &[u8] = b"LOGO-BYTES-THAT-NO-STORE-KEEPS";

/// 3,000 bytes holding a NUL, `MARK` and a byte that isn't UTF-8.
fn logo() -> Vec<u8> {
    let mut bytes = MARK.to_vec();
    bytes.extend([0x00, 0xff, 0xfe]);
    bytes.resize(3000, 0x01);
    bytes
}

/// The `clm_repo` fixture with a text asset of `b`.
fn repo() -> FileMap {
    let mut f = clm_repo();
    f.insert("specs/b/assets/note.txt".into(), b"a note\n".to_vec());
    f
}

/// A ready replica of `f` whose binary file `path` is given as `given` makes it.
fn given_as(f: &FileMap, path: &str, given: fn(clm_replica::Entry, &[u8]) -> File) -> Replica {
    let mut r = Replica::default();
    r.apply(tree("111", f));
    let files = r
        .pending_fetch()
        .iter()
        .map(|p| match p.as_str() {
            p if p == path => given(entry(p, &f[p]), &f[p]),
            p => file(p, &f[p]),
        })
        .collect();
    r.apply(Change::Tip {
        sha: "111".into(),
        files,
        removed: Vec::new(),
    });
    r.recompute(100);
    r
}

fn as_asset(e: clm_replica::Entry, bytes: &[u8]) -> File {
    File::asset(e, bytes.to_vec())
}

fn as_binary(e: clm_replica::Entry, _: &[u8]) -> File {
    File::binary(e)
}

fn assets_of(snapshot: &Value, node: &str) -> Value {
    snapshot["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == node)
        .unwrap()["assets"]
        .clone()
}

/// The store's values, one after another.
fn stored(store: &MapStore) -> Vec<u8> {
    let mut keys: Vec<_> = store.values.keys().collect();
    keys.sort();
    keys.into_iter()
        .flat_map(|k| store.values[k].clone())
        .collect()
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn a_binary_asset() {
    // req:replica/asset-hashes@f1d76b71
    for (path, kept) in [
        (LOGO, true),
        ("specs/b/x/assets/y.bin", true),
        ("changes/c/assets/b/logo.png", false),
        ("src/assets/z.bin", false),
        ("specs/b/SPEC.md", false),
    ] {
        assert_eq!(keeps_hash(path), kept, "{path}");
    }

    let mut f = repo();
    f.insert(LOGO.into(), logo());
    let dir = tempfile::tempdir().unwrap();
    write_checkout(dir.path(), &f);
    let (graph, _) = clm_model::load(dir.path());
    let checkout = graph.nodes["b"].assets["logo.png"].clone();

    // Given by `File::asset`: listed with the checkout's hash, and nothing left to fetch.
    let first = given_as(&f, LOGO, as_asset);
    assert!(
        first.pending_fetch().is_empty(),
        "{:?}",
        first.pending_fetch()
    );
    let snap = view(&first, "snapshot");
    assert_eq!(assets_of(&snap, "b")["logo.png"], checkout.as_str());
    assert_eq!(
        assets_of(&snap, "b")["note.txt"],
        graph.nodes["b"].assets["note.txt"].as_str()
    );
    assert_eq!(snap["violations"], serde_json::json!([]));

    // Saved and loaded again: the hash stays, and the store holds none of the bytes.
    let mut store = MapStore::default();
    first.save(&mut store);
    let saved = stored(&store);
    let decimals = MARK.iter().map(u8::to_string).collect::<Vec<_>>().join(",");
    assert!(!contains(&saved, MARK));
    assert!(!contains(&saved, decimals.as_bytes()));
    let mut loaded = Replica::load(&store);
    assert_eq!(loaded, first);
    loaded.recompute(200);
    assert_eq!(
        assets_of(&view(&loaded, "snapshot"), "b")["logo.png"],
        checkout.as_str()
    );

    // Given by `File::binary`: held binary, in no `assets`, and not to fetch.
    let second = given_as(&f, LOGO, as_binary);
    assert!(
        second.pending_fetch().is_empty(),
        "{:?}",
        second.pending_fetch()
    );
    let assets = assets_of(&view(&second, "snapshot"), "b");
    assert!(assets.get("logo.png").is_none(), "{assets}");
    assert!(assets.get("note.txt").is_some(), "{assets}");

    // An asset of the root component: the line `clm check` prints for it on a checkout.
    let mut root = repo();
    root.insert("specs/assets/x.png".into(), logo());
    let dir = tempfile::tempdir().unwrap();
    write_checkout(dir.path(), &root);
    let (_, violations) = clm_model::load(dir.path());
    let lines: Vec<String> = violations.iter().map(clm_check::format).collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("specs/assets/x.png: model/node-assets: "));
    let r = given_as(&root, "specs/assets/x.png", as_asset);
    assert_eq!(view(&r, "snapshot")["violations"], serde_json::json!(lines));
}

#[test]
fn a_store_saved_before_hashes_loads_as_it_did() {
    // req:replica/asset-hashes@f1d76b71
    // Saved by the replica before it held hashes: `repo()` ready at `111`, at time 100.
    let old = include_bytes!("fixtures/old-store.json");
    assert!(!contains(old, b"\"hash\""));
    let mut store = MapStore::default();
    store.put("replica", b"1".to_vec());
    store.put("replica/0", old.to_vec());
    let mut loaded = Replica::load(&store);
    assert_ne!(loaded, Replica::default());
    assert!(loaded.pending_fetch().is_empty());
    let fresh = ready(&repo(), "111", 200);
    loaded.recompute(200);
    for name in ["snapshot", "implementation", "capabilities", "architecture"] {
        assert_eq!(view(&loaded, name), view(&fresh, name), "{name}");
    }
}
