//! `replica/markers-of-this-release`: a state another release saved has its markers counted
//! again by this release's scanner.

mod common;

use clm_replica::{Replica, Store};
use common::*;
use serde_json::{Value, json};

const CHECK: &str = "specs/b/assets/check.ts";

/// `clm_repo` with `b` holding `x` and `y`, `src/a.rs` citing `b/x` and `check.ts` citing
/// `b/y` at their current versions, `src/b.bin` binary and `README.md`, which is listed but
/// never read.
fn repo() -> FileMap {
    let mut f = clm_repo();
    f.insert(
        "specs/b/SPEC.md".into(),
        node("a", &["x", "y"], "active").into_bytes(),
    );
    let x = version_of(&f, "b/x");
    let y = version_of(&f, "b/y");
    f.insert("src/a.rs".into(), cites("b/x", &x).into_bytes());
    f.insert(
        CHECK.into(),
        format!(
            "it('y', () => {{\n  // {}b/y@{y}\n}});\n",
            concat!("req", ":")
        )
        .into_bytes(),
    );
    f.insert("src/b.bin".into(), vec![0xff, 0x00, 0xfe]);
    f.insert("README.md".into(), b"# Read me\n".to_vec());
    f
}

/// The state a store holds, read whole.
fn stored(store: &MapStore) -> Value {
    let count: usize = String::from_utf8(store.values["replica"].clone())
        .unwrap()
        .parse()
        .unwrap();
    let bytes: Vec<u8> = (0..count)
        .flat_map(|n| store.values[&format!("replica/{n}")].clone())
        .collect();
    serde_json::from_slice(&bytes).unwrap()
}

/// A store holding `state` in one value.
fn store_of(state: &Value) -> MapStore {
    let mut store = MapStore::default();
    store.put("replica", b"1".to_vec());
    store.put("replica/0", serde_json::to_vec(state).unwrap());
    store
}

/// The markers of `path` that a replica holds, as `save` writes them.
fn held_markers(r: &Replica, path: &str) -> Option<Value> {
    let mut store = MapStore::default();
    r.save(&mut store);
    stored(&store)["state"]["held"]
        .get(path)
        .map(|h| h["markers"].clone())
}

fn ids(markers: &Value) -> Vec<&str> {
    markers
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_str().unwrap())
        .collect()
}

#[test]
fn another_release_saved_it() {
    // req:replica/markers-of-this-release@8c1950f6
    let f = repo();
    let original = ready(&f, "111", 100);
    assert!(original.pending_fetch().is_empty());
    assert_eq!(view(&original, "snapshot")["coverage"]["state"], "complete");
    assert_eq!(ids(&held_markers(&original, CHECK).unwrap()), ["b/y"]);
    assert_eq!(ids(&held_markers(&original, "src/a.rs").unwrap()), ["b/x"]);

    let mut saved = MapStore::default();
    original.save(&mut saved);
    let mut state = stored(&saved);
    assert_eq!(state["release"], env!("CARGO_PKG_VERSION"));
    // The markers another release counted, which this one counts otherwise.
    state["state"]["held"][CHECK]["markers"] = json!([]);
    let mut older = state.clone();
    older["release"] = json!("0.0.0");
    let mut unnamed = state;
    unnamed.as_object_mut().unwrap().remove("release");

    for (which, store) in [("0.0.0", store_of(&older)), ("none", store_of(&unnamed))] {
        let mut loaded = Replica::load(&store);
        assert_eq!(loaded.pending_fetch(), ["src/a.rs"], "{which}");
        assert!(loaded.stale(), "{which}");
        // The last views stay as saved until a recompute.
        for name in ["snapshot", "implementation", "capabilities", "architecture"] {
            assert_eq!(view(&loaded, name), view(&original, name), "{which} {name}");
        }

        loaded.recompute(200);
        assert!(!loaded.stale(), "{which}");
        let snap = view(&loaded, "snapshot");
        assert_eq!(snap["coverage"]["state"], "indexing", "{which}");
        assert_eq!(snap["sha"], "111", "{which}");
        // `check.ts`'s text is kept, so it is scanned again; `src/a.rs`'s isn't, so it goes,
        // and the binary file stays held.
        assert_eq!(
            ids(&held_markers(&loaded, CHECK).unwrap()),
            ["b/y"],
            "{which}"
        );
        assert_eq!(held_markers(&loaded, "src/a.rs"), None, "{which}");
        assert!(held_markers(&loaded, "src/b.bin").is_some(), "{which}");

        fetch(&mut loaded, &f, "111");
        loaded.recompute(100);
        for name in ["snapshot", "implementation", "capabilities", "architecture"] {
            assert_eq!(view(&loaded, name), view(&original, name), "{which} {name}");
        }
        assert_eq!(loaded, original, "{which}");
    }
}

#[test]
fn this_release_saved_it() {
    // req:replica/markers-of-this-release@8c1950f6
    let original = ready(&repo(), "111", 100);
    let mut store = MapStore::default();
    original.save(&mut store);
    let loaded = Replica::load(&store);
    assert_eq!(loaded, original);
    assert!(loaded.pending_fetch().is_empty());
    assert!(!loaded.stale());

    // What this release saved is taken as saved, its markers included.
    let mut state = stored(&store);
    state["state"]["held"][CHECK]["markers"] = json!([]);
    let kept = Replica::load(&store_of(&state));
    assert_eq!(held_markers(&kept, CHECK), Some(json!([])));
    assert!(!kept.stale());
}
