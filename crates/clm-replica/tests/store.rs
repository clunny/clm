//! `replica/stores-in-chunks`.

mod common;

use clm_replica::{Replica, Store};
use common::*;

#[test]
fn a_large_repository_saves_in_chunks_and_loads_back() {
    // req:replica/stores-in-chunks@0a3cc96c
    let mut f = clm_repo();
    // A decision of about 5 MiB, so the serialised state is over 5 MiB.
    let body = "Why, at length.\n".repeat(5 * 1024 * 1024 / 16);
    f.insert(
        "docs/decisions/big.md".into(),
        format!("---\ntitle: Big\ndate: 2026-10-01\nstatus: accepted\n---\n{body}").into_bytes(),
    );
    let original = ready(&f, "111", 100);

    let mut store = MapStore::default();
    original.save(&mut store);
    assert!(store.values.len() > 5, "{} values", store.values.len());
    for (key, value) in &store.values {
        assert!(value.len() <= 1 << 20, "{key} holds {} bytes", value.len());
    }
    assert_eq!(store.largest, 1 << 20);

    let loaded = Replica::load(&store);
    assert_eq!(loaded, original);
    for name in ["snapshot", "implementation", "capabilities", "architecture"] {
        assert_eq!(view(&loaded, name), view(&original, name));
    }

    // Saving again replaces what was there, and a smaller state leaves no old chunk behind.
    let small = ready(&clm_repo(), "222", 100);
    small.save(&mut store);
    assert!(store.values.len() < 5);
    let count: usize = String::from_utf8(store.values["replica"].clone())
        .unwrap()
        .parse()
        .unwrap();
    let mut keys: Vec<String> = store.values.keys().cloned().collect();
    keys.sort();
    let mut expected: Vec<String> = (0..count).map(|n| format!("replica/{n}")).collect();
    expected.push("replica".into());
    expected.sort();
    assert_eq!(keys, expected, "only the second save's values");
    assert_eq!(Replica::load(&store), small);
}

#[test]
fn an_empty_or_unreadable_store_gives_a_new_replica() {
    // req:replica/stores-in-chunks@0a3cc96c
    let mut store = MapStore::default();
    assert_eq!(Replica::load(&store), Replica::default());

    // A count of 2 over a chunk 0 that alone reads as a whole saved replica.
    let mut whole = MapStore::default();
    ready(&clm_repo(), "111", 100).save(&mut whole);
    assert_eq!(whole.values["replica"], b"1");
    store.put("replica", b"2".to_vec());
    store.put("replica/0", whole.values["replica/0"].clone());
    assert_eq!(
        Replica::load(&store),
        Replica::default(),
        "a chunk is missing"
    );

    store.delete_all();
    store.put("replica", b"1".to_vec());
    store.put("replica/0", b"{".to_vec());
    assert_eq!(
        Replica::load(&store),
        Replica::default(),
        "a chunk that doesn't read"
    );
    store.delete_all();
    assert!(store.get("replica").is_none());
}
