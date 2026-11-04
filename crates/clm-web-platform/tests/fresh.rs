//! A fresh read replaces the plain entry (`fresh-replaces-plain`): what `live::cached`
//! fetches for a notice's delivery is also what the next plain read answers from.

use clm_web_platform::access::{self, Access};
use clm_web_platform::fake::*;
use clm_web_platform::platform::Cache;
use clm_web_platform::{ledger, live};
use serde_json::json;

fn setup() -> (Fake, Access) {
    let fake = Fake::default();
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    fake.gh.grant("u", "o/a");
    let access = block_on(access::check(&fake, "u", "o", "a")).unwrap();
    (fake, access)
}

fn record(id: &str) -> serde_json::Value {
    json!({"id": id, "kind": "subscribed", "writer": "alice/web", "at": iso(NOW),
        "endpoint": "https://fcm.googleapis.com/x", "p256dh": "k", "auth": "a"})
}

fn reads(fake: &Fake) -> usize {
    fake.gh.calls.borrow().len()
}

#[test]
fn the_next_plain_read_sees_it() {
    // req:web.worker.platform/fresh-replaces-plain@2297901a
    let (fake, access) = setup();
    // The ledgers as the minute-old live state holds them: without alice's branch.
    let before = block_on(ledger::fold(&fake, &access)).unwrap();
    assert!(before.subscriptions.is_empty());
    fake.now.set(NOW + 10);
    ledger_branch(&fake);
    let fresh = access.clone().fresh(Some("d1"));
    let fold = block_on(ledger::fold(&fake, &fresh)).unwrap();
    assert_eq!(fold.subscriptions.len(), 1, "the fresh read");
    let calls = reads(&fake);
    let plain = block_on(ledger::fold(&fake, &access)).unwrap();
    assert_eq!(plain.subscriptions.len(), 1, "the plain read after it");
    assert_eq!(reads(&fake), calls, "no request to GitHub");
}

fn ledger_branch(fake: &Fake) {
    let mut repos = fake.gh.repos.borrow_mut();
    let repo = repos.get_mut("o/a").unwrap();
    clm_web_platform::fake::ledger(repo, "alice/web", "l1", &[record("r1")]);
}

#[test]
fn the_plain_copy_lasts_a_minute() {
    // req:web.worker.platform/fresh-replaces-plain@2297901a
    let (fake, access) = setup();
    let fresh = access.fresh(Some("d1"));
    let fetch = async || Some(1u32);
    block_on(live::cached(&fake, &fresh, "x", fetch));
    fake.now.set(NOW + 59);
    assert!(block_on(fake.cache_get("live:o/a:x")).is_some());
    fake.now.set(NOW + 60);
    assert!(block_on(fake.cache_get("live:o/a:x")).is_none());
}

#[test]
fn a_cached_fresh_read() {
    // req:web.worker.platform/fresh-replaces-plain@2297901a
    let (fake, access) = setup();
    let fresh = access.fresh(Some("d1"));
    let fetch = async || Some(1u32);
    assert_eq!(block_on(live::cached(&fake, &fresh, "x", fetch)), Some(1));
    // The plain key may have moved on since; the delivery's key is not copied over it.
    block_on(fake.cache_put("live:o/a:x", "2", 60));
    let stored = fake.stored.borrow().len();
    let calls = reads(&fake);
    let again = async || -> Option<u32> { panic!("fetched again") };
    assert_eq!(block_on(live::cached(&fake, &fresh, "x", again)), Some(1));
    assert_eq!(fake.stored.borrow().len(), stored, "stores nothing");
    assert_eq!(reads(&fake), calls, "no request to GitHub");
    assert_eq!(
        block_on(fake.cache_get("live:o/a:x")).as_deref(),
        Some("2"),
        "the plain entry is as it was"
    );
}
