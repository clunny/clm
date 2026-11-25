//! A minute of live state (`live-state-cache`), and a repo's open pull requests read in
//! pages (`open-pulls`).

use std::cell::Cell;

use clm_web_platform::access::{self, Access};
use clm_web_platform::fake::*;
use clm_web_platform::live;
use clm_web_platform::platform::{Auth, Cache};

fn setup(fake: &Fake) -> Access {
    fake.gh.grant("u", "o/a");
    block_on(access::check(fake, "u", "o", "a")).unwrap()
}

#[test]
fn a_minute_of_live_state() {
    // req:web.worker.platform/live-state-cache@9b0374fa
    let fake = Fake::default();
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    let acc = setup(&fake);
    let fetches = Cell::new(0);
    let read = |value: Option<u32>, what: &str| {
        block_on(live::cached(&fake, &acc, what, async || {
            fetches.set(fetches.get() + 1);
            value
        }))
    };
    assert_eq!(read(Some(1), "n"), Some(1));
    assert_eq!(fetches.get(), 1);
    fake.now.set(NOW + 59);
    assert_eq!(read(Some(2), "n"), Some(1), "kept for 59 seconds");
    assert_eq!(fetches.get(), 1);
    fake.now.set(NOW + 60);
    assert_eq!(read(Some(2), "n"), Some(2), "fetched at 60");
    assert_eq!(fetches.get(), 2);

    let stored = fake.stored.borrow().len();
    assert_eq!(read(None, "gone"), None);
    assert_eq!(read(None, "gone"), None);
    assert_eq!(fetches.get(), 4, "the failing item is fetched both times");
    assert_eq!(fake.stored.borrow().len(), stored, "nothing stored for it");
    assert!(block_on(fake.cache_get("live:o/a:gone")).is_none());

    block_on(fake.cache_put("live:o/a:odd", "\"x\"", 60));
    assert_eq!(read(Some(3), "odd"), Some(3), "fetched anew");
    assert_eq!(fetches.get(), 5);
    assert_eq!(
        block_on(fake.cache_get("live:o/a:odd")).as_deref(),
        Some("3")
    );
}

fn pulls(n: u64) -> Vec<FakePull> {
    (1..=n)
        .map(|i| FakePull {
            number: i,
            state: "open".into(),
            title: format!("t{i}"),
            base_ref: "main".into(),
            head_ref: format!("spec/p{i}"),
            head_sha: format!("h{i}"),
            fork: i == 2,
            ..FakePull::default()
        })
        .collect()
}

fn graphql_queries(fake: &Fake) -> usize {
    let calls = fake.gh.calls.borrow();
    calls.iter().filter(|c| c.url.ends_with("/graphql")).count()
}

#[test]
fn many_open_pull_requests() {
    // req:web.worker.platform/open-pulls@8c7315f7
    let auth = Auth::User("u".into());

    let fake = Fake::default();
    let mut repo = FakeRepo::new("111", clm_repo());
    repo.pulls = pulls(150);
    repo.pulls.insert(
        40,
        FakePull {
            number: 999,
            state: "closed".into(),
            head_ref: "spec/closed".into(),
            ..FakePull::default()
        },
    );
    fake.gh.add_repo("o/a", repo);
    let acc = setup(&fake);
    let before = graphql_queries(&fake);
    let (got, whole) = block_on(live::pulls_direct(&fake, &auth, "o/a")).unwrap();
    assert!(whole, "read to the end");
    assert_eq!(graphql_queries(&fake) - before, 2);
    assert_eq!(
        got.iter().map(|p| p.number).collect::<Vec<_>>(),
        (1..=150).collect::<Vec<_>>()
    );
    let p2 = &got[1];
    assert_eq!(
        (
            p2.title.as_str(),
            p2.head.as_str(),
            p2.base.as_str(),
            p2.head_sha.as_str(),
            p2.fork
        ),
        ("t2", "spec/p2", "main", "h2", true)
    );
    assert_eq!(p2.url, "https://github.com/o/a/pull/2");
    assert!(!got[0].fork);

    let first = block_on(live::pulls(&fake, &acc)).unwrap();
    assert_eq!(first, got);
    let calls = fake.gh.calls.borrow().len();
    assert_eq!(block_on(live::pulls(&fake, &acc)).unwrap(), got);
    assert_eq!(
        fake.gh.calls.borrow().len(),
        calls,
        "the second read asks nothing"
    );

    let fake = Fake::default();
    let mut repo = FakeRepo::new("111", clm_repo());
    repo.pulls = pulls(301);
    fake.gh.add_repo("o/a", repo);
    fake.gh.grant("u", "o/a");
    let (got, whole) = block_on(live::pulls_direct(&fake, &auth, "o/a")).unwrap();
    assert!(!whole, "not read to the end");
    assert_eq!(got.len(), 300);
    assert_eq!(got.last().unwrap().number, 300);
    assert_eq!(graphql_queries(&fake), 3);

    let fake = Fake::default();
    let mut repo = FakeRepo::new("111", clm_repo());
    repo.pulls = pulls(3);
    fake.gh.add_repo("o/a", repo);
    fake.gh.grant("u", "o/a");
    fake.gh.graphql_null.borrow_mut().insert("Pulls".into());
    assert_eq!(block_on(live::pulls_direct(&fake, &auth, "o/a")), None);
}
