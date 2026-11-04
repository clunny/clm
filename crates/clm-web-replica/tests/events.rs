//! The events the object applies (`applies-events`) and the batching of its recompute
//! (`alarm-batches`).

mod common;

use clm_web_platform::fake::{FakeRepo, files, ledger};
use common::*;
use futures::executor::block_on;
use serde_json::json;

fn gh_calls(fake: &clm_web_platform::fake::Fake) -> usize {
    fake.gh.calls.borrow().len()
}

#[test]
fn a_push_to_main() {
    // req:web.worker.replica/applies-events@f75e1a7c
    let before = repo_files(&[("t.rs", "fn t() {}"), ("u.rs", "fn u() {}")]);
    let fake = fake_with("111", before);
    let (mut object, _mem, slot) = built(&fake);
    let after = repo_files(&[("t.rs", "fn t2() {}")]);
    commit(&fake, "222", after);
    let from = gh_calls(&fake);
    let answer = block_on(object.event(&fake, &event("push", "d1", push("main", "111", "222"))));
    assert_eq!(answer.status, 202);
    let queries = graphql_calls(&fake, from);
    assert_eq!(queries.len(), 1, "{queries:?}");
    assert!(queries[0].contains("222:t.rs"), "{}", queries[0]);
    assert!(!queries[0].contains("u.rs"));
    assert_eq!(
        fake.gh.fetched_paths().last().map(String::as_str),
        Some("t.rs")
    );
    let urls = urls(&fake, from);
    assert!(
        !urls.iter().any(|u| u.contains("/git/trees/")),
        "no tree was listed: {urls:?}"
    );
    assert!(slot.0.get().is_some(), "the alarm is set");
}

#[test]
fn a_ledger_append() {
    // req:web.worker.replica/applies-events@f75e1a7c
    let mut repo = FakeRepo::new("111", repo_files(&[]));
    let record = |id: &str| {
        json!({"id": id, "writer": "alice/web", "at": "2027-01-15T07:00:00.000Z",
            "kind": "unsubscribed", "subscription": "x"})
    };
    ledger(&mut repo, "alice/web", "LA", &[record("r1")]);
    let fake = clm_web_platform::fake::Fake::default();
    fake.gh.add_repo("o/a", repo);
    let (mut object, _mem, _slot) = built(&fake);
    {
        let mut repos = fake.gh.repos.borrow_mut();
        ledger(
            repos.get_mut("o/a").unwrap(),
            "alice/web",
            "LB",
            &[record("r1"), record("r2")],
        );
    }
    let from = gh_calls(&fake);
    let body = push("clm/ledger/alice/web", "LA", "LB");
    block_on(object.event(&fake, &event("push", "d1", body)));
    let calls = fake.gh.calls.borrow();
    let made: Vec<_> = calls[from..].iter().collect();
    assert_eq!(made.len(), 1, "{made:?}");
    assert!(
        made[0]
            .body
            .as_deref()
            .unwrap()
            .contains("LB:records.jsonl")
    );
}

#[test]
fn a_truncated_compare() {
    // req:web.worker.replica/applies-events@f75e1a7c
    let name = |i: usize| format!("src/f{i}.rs");
    let make = |gen_of: &dyn Fn(usize) -> &'static str| {
        let mut all = repo_files(&[]);
        for i in 0..310 {
            all.extend(files(&[(
                &name(i),
                &format!("fn f{i}() {{ /* {} */ }}", gen_of(i)),
            )]));
        }
        all
    };
    let fake = fake_with("111", make(&|_| "one"));
    let (mut object, _mem, _slot) = built(&fake);
    // 301 of the 310 files change: the compare lists only 300 of them.
    commit(&fake, "222", make(&|i| if i < 301 { "two" } else { "one" }));
    let from = gh_calls(&fake);
    block_on(object.event(&fake, &event("push", "d1", push("main", "111", "222"))));
    let urls = urls(&fake, from);
    assert!(
        urls.iter().any(|u| u.contains("/git/trees/222")),
        "the tree at 222 is listed: {urls:?}"
    );
    let mut fetched = fake.gh.fetched_paths();
    let first_new = fetched.len() - 301;
    let tail = fetched.split_off(first_new);
    assert!(
        tail.iter().all(|p| {
            let n: usize = p
                .trim_start_matches("src/f")
                .trim_end_matches(".rs")
                .parse()
                .unwrap();
            n < 301
        }),
        "only blobs whose SHAs weren't held are fetched"
    );
    assert_eq!(tail.len(), 301);
}

#[test]
fn a_burst_of_pushes() {
    // req:web.worker.replica/alarm-batches@7c4c8c7a
    let fake = fake_with("111", repo_files(&[("t.rs", "fn t() {}")]));
    let (mut object, _mem, slot) = built(&fake);
    let described = view(&fake, &mut object, "snapshot");
    assert_eq!(described["view"]["sha"], "111");
    let computed = described["computed_at"].clone();
    // The read armed an idle alarm for a resync; the burst starts with none set.
    slot.0.set(None);
    let notices_before = notices(&fake);
    let start = fake.now.get();
    for (i, (from, to)) in [("111", "222"), ("222", "333"), ("333", "444")]
        .iter()
        .enumerate()
    {
        commit(
            &fake,
            to,
            repo_files(&[("t.rs", &format!("fn t{to}() {{}}"))]),
        );
        fake.now.set(start + 10 * i as u64);
        let body = push("main", from, to);
        block_on(object.event(&fake, &event("push", &format!("d{i}"), body)));
        // An applied event recomputes nothing.
        let now = view(&fake, &mut object, "snapshot");
        assert_eq!(now["view"]["sha"], "111");
        assert_eq!(now["computed_at"], computed);
    }
    assert_eq!(slot.0.get(), Some(start + 60), "one minute after the first");
    run_alarms(&fake, &mut object, &slot);
    let after = view(&fake, &mut object, "snapshot");
    assert_eq!(after["view"]["sha"], "444");
    let notices = notices(&fake);
    assert_eq!(notices.len(), notices_before.len() + 1, "{notices:?}");
    assert_eq!(
        notices.last().unwrap(),
        &json!({"delivery": "d2"}).to_string()
    );
}

/// The bodies posted to the `Live` object's `/notify`.
fn notices(fake: &clm_web_platform::fake::Fake) -> Vec<String> {
    fake.live
        .calls
        .borrow()
        .iter()
        .filter(|c| c.path == "/notify")
        .map(|c| c.body.clone())
        .collect()
}

#[test]
fn a_question_lands_through_the_alarm() {
    // req:web.worker.replica/push-waiting@8e710ff8
    let mut fake = fake_with("111", repo_files(&[]));
    fake.secrets
        .insert("VAPID_PUBLIC_KEY".into(), "BPublicKey".into());
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        let sub = json!({"id": "sa", "writer": "alice/web", "at": "2027-01-15T07:00:00.000Z",
            "kind": "subscribed", "endpoint": "https://fcm.googleapis.com/fcm/send/alice",
            "p256dh": "BKey", "auth": "AuthSecret"});
        ledger(repo, "alice/web", "LA", &[sub]);
        repo.permissions.insert("alice".into(), "read".into());
    }
    let (mut object, _mem, slot) = built(&fake);
    slot.0.set(None);
    let sends = |fake: &clm_web_platform::fake::Fake| {
        fake.live
            .calls
            .borrow()
            .iter()
            .filter(|c| c.path == "/send")
            .count()
    };
    assert_eq!(sends(&fake), 0);
    let item = "---\ntitle: Which region?\nkind: question\ncomponents: [a]\n---\nBody.\n";
    commit(&fake, "222", repo_files(&[("backlog/region.md", item)]));
    block_on(object.event(&fake, &event("push", "d1", push("main", "111", "222"))));
    run_alarms(&fake, &mut object, &slot);
    assert_eq!(sends(&fake), 1);
}

#[test]
fn a_ref_the_views_read_moves() {
    // req:web.worker.replica/applies-events@f75e1a7c
    let fake = fake_with("111", repo_files(&[]));
    let (mut object, _mem, slot) = built(&fake);
    slot.0.set(None);
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        repo.claims.push(("a".into(), "c1".into(), fake.now.get()));
        repo.commits
            .insert("c1".into(), repo_files(&[("x.rs", "fn x() {}")]));
    }
    let from = gh_calls(&fake);
    let mut body = push("impl/a", "0", "c1");
    body["created"] = json!(true);
    block_on(object.event(&fake, &event("push", "d1", body)));
    let urls = urls(&fake, from);
    // Only the head's tree, for its code key: no compare, no file fetched.
    assert_eq!(urls.len(), 1, "{urls:?}");
    assert!(urls[0].contains("/git/trees/c1"));
    run_alarms(&fake, &mut object, &slot);
    let now = view(&fake, &mut object, "implementation");
    assert_eq!(now["view"]["flight"]["totals"]["by_state"]["claimed"], 1);
}

#[test]
fn a_pull_request_and_an_uninstall() {
    // req:web.worker.replica/push-review@0b5b98e1
    let mut fake = fake_with("111", repo_files(&[]));
    fake.secrets
        .insert("VAPID_PUBLIC_KEY".into(), "BPublicKey".into());
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        let sub = json!({"id": "sa", "writer": "alice/web", "at": "2027-01-15T07:00:00.000Z",
            "kind": "subscribed", "endpoint": "https://fcm.googleapis.com/fcm/send/alice",
            "p256dh": "BKey", "auth": "AuthSecret"});
        ledger(repo, "alice/web", "LA", &[sub]);
        repo.permissions.insert("alice".into(), "write".into());
    }
    let (mut object, mem, _slot) = built(&fake);
    let pr = json!({"action": "opened", "pull_request": {"number": 3, "title": "Add x", "draft": false,
        "state": "open", "html_url": "https://github.com/o/a/pull/3",
        "head": {"ref": "spec/x", "sha": "s1", "repo": {"full_name": "o/a"}},
        "base": {"ref": "main", "repo": {"full_name": "o/a"}}}});
    block_on(object.event(&fake, &event("pull_request", "d1", pr)));
    let sends = fake
        .live
        .calls
        .borrow()
        .iter()
        .filter(|c| c.path == "/send")
        .count();
    assert_eq!(sends, 1);
    // An uninstall deletes what the object holds.
    assert!(!mem.0.borrow().is_empty());
    let removed = json!({"action": "removed", "repositories_removed": [{"full_name": "o/a"}]});
    let answer = block_on(object.event(&fake, &event("installation_repositories", "d2", removed)));
    assert_eq!(answer.status, 202);
    assert!(mem.0.borrow().is_empty());
    assert_eq!(block_on(object.view(&fake, "snapshot", false)).status, 503);
    fake.secrets.remove("VAPID_PUBLIC_KEY");
}
