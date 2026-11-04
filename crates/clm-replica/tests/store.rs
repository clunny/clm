//! `replica/stores-in-chunks`.

mod common;

use clm_replica::{Replica, Store};
use common::*;

#[test]
fn a_large_repository_saves_in_chunks_and_loads_back() {
    // req:replica/stores-in-chunks@05cffd04
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
    assert_eq!(Replica::load(&store), small);
}

#[test]
fn an_empty_or_unreadable_store_gives_a_new_replica() {
    // req:replica/stores-in-chunks@05cffd04
    let mut store = MapStore::default();
    assert_eq!(Replica::load(&store), Replica::default());
    store.put("replica", b"2".to_vec());
    store.put("replica/0", b"{".to_vec());
    assert_eq!(
        Replica::load(&store),
        Replica::default(),
        "a chunk is missing"
    );
    store.delete_all();
    assert!(store.get("replica").is_none());
}
