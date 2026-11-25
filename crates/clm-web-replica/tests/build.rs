//! The first build of a repo's replica (`first-build`) and its resync (`resync`).

mod common;

use clm_web_platform::fake::{files, iso};
use common::*;
use futures::executor::block_on;
use serde_json::{Value, json};

#[test]
fn github_not_ready() {
    // req:web.worker.replica/first-build@36cba39d
    let fake = fake_with("111", repo_files(&[]));
    commit(&fake, "222", repo_files(&[("t.rs", "fn t() {}")]));
    let over = Over::new(&fake);
    // GitHub fails the tree listing.
    *over.github.borrow_mut() =
        Box::new(|req| req.url.contains("/git/trees/").then(|| status(502)));
    let (mut object, mem, slot) = object();

    // A push reaches the object that holds nothing: 202, the alarm set to now, nothing of
    // it applied.
    let mut body = push("impl/z", "0", "c9");
    body["created"] = json!(true);
    let answer = block_on(object.event(&over, &event("push", "d1", body)));
    assert_eq!((answer.status, answer.body.as_str()), (202, ""));
    let now = fake.now.get();
    assert_eq!(slot.0.get(), Some(now), "the alarm is set to now");
    assert!(fake.gh.calls.borrow().is_empty(), "nothing was fetched");

    // The first alarm: the tree can't be listed, so it re-arms a minute on.
    block_on(object.on_alarm(&over));
    assert_eq!(slot.0.get(), Some(now + 60));
    let read = block_on(object.view(&over, "snapshot", false));
    assert_eq!(
        (read.status, read.body.as_str()),
        (503, r#"{"preparing":0}"#)
    );
    assert_eq!(slot.0.get(), Some(now + 60), "the read keeps the alarm");

    // GitHub answers again: the next alarm builds the tip GitHub gives.
    *over.github.borrow_mut() = Box::new(|_| None);
    run_alarms(&fake, &mut object, &slot);
    let snapshot = view(&fake, &mut object, "snapshot");
    assert_eq!(snapshot["sha"], "222");
    let implementation = view(&fake, &mut object, "implementation");
    let activity = implementation["view"]["activity"].as_array().unwrap();
    assert!(
        !activity
            .iter()
            .any(|a| a["ref"] == "z" || a["ref"] == "impl/z"),
        "the event wasn't applied: {activity:?}"
    );
    // The first notice names no delivery the object didn't apply.
    let notices: Vec<String> = fake
        .live
        .calls
        .borrow()
        .iter()
        .filter(|c| c.path == "/notify")
        .map(|c| c.body.clone())
        .collect();
    assert!(!notices.is_empty());
    assert!(notices.iter().all(|n| !n.contains("\"d1\"")), "{notices:?}");
    drop(mem);
}

#[test]
fn no_installation_token() {
    // req:web.worker.replica/first-build@36cba39d
    let fake = fake_with("111", repo_files(&[]));
    let over = Over::new(&fake);
    *over.github.borrow_mut() =
        Box::new(|req| req.url.ends_with("/access_tokens").then(|| status(500)));
    let (mut object, _mem, slot) = object();
    block_on(object.view(&over, "snapshot", false));
    let now = fake.now.get();
    block_on(object.on_alarm(&over));
    assert_eq!(slot.0.get(), Some(now + 60), "re-armed a minute on");
    let read = block_on(object.view(&over, "snapshot", false));
    assert_eq!(
        (read.status, read.body.as_str()),
        (503, r#"{"preparing":0}"#)
    );
    // Nor can it read the default branch's ref.
    *over.github.borrow_mut() =
        Box::new(|req| req.url.contains("/git/ref/heads/").then(|| status(502)));
    fake.now.set(now + 60);
    block_on(object.on_alarm(&over));
    assert_eq!(slot.0.get(), Some(now + 120), "re-armed a minute on");
    let read = block_on(object.view(&over, "snapshot", false));
    assert_eq!(read.status, 503);
}

#[test]
fn a_claim_made_before_the_first_build() {
    // req:web.worker.replica/first-build@36cba39d
    let fake = fake_with("111", repo_files(&[]));
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        let at = fake.now.get() - 600;
        for (name, sha) in [("a", "ca"), ("b", "cb")] {
            repo.claims.push((name.into(), sha.into(), at));
            repo.commits
                .insert(sha.into(), repo_files(&[("x.rs", "fn x() {}")]));
        }
        repo.events = json!([{
            "type": "CreateEvent", "created_at": iso(at), "actor": {"login": "alice"},
            "payload": {"ref": "impl/a", "ref_type": "branch"},
        }]);
    }
    let (mut object, _mem, _slot) = built(&fake);
    let implementation = view(&fake, &mut object, "implementation");
    let claims: Vec<&Value> = implementation["view"]["activity"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["kind"] == "claim")
        .collect();
    assert_eq!(claims.len(), 1, "{claims:?}");
    assert_eq!(
        (&claims[0]["ref"], &claims[0]["actor"]),
        (&json!("a"), &json!("alice"))
    );
}

#[test]
fn a_repo_of_1000_files() {
    // req:web.worker.replica/first-build@36cba39d
    let mut all = repo_files(&[]);
    // Six more spec files the replica reads, so ten in all.
    for d in 0..6 {
        all.extend(files(&[(
            &format!("d{d}/.gitattributes"),
            "*.lock linguist-generated",
        )]));
    }
    for i in 0..990 {
        all.extend(files(&[(
            &format!("src/f{i}.rs"),
            &format!("fn f{i}() {{}}"),
        )]));
    }
    let fake = fake_with("111", all);
    let (mut object, _mem, slot) = object();

    let first = block_on(object.view(&fake, "snapshot", false));
    assert_eq!(
        (first.status, first.body.as_str()),
        (503, r#"{"preparing":0}"#)
    );
    assert!(slot.0.get().is_some(), "the build started");

    // The first alarm lists the tree and fetches the spec files, and the views are computed.
    let from = fake.gh.calls.borrow().len();
    let at = slot.0.get().unwrap();
    fake.now.set(at);
    block_on(object.on_alarm(&fake));
    let calls = fake.gh.calls.borrow().len() - from;
    assert!(calls <= 50, "{calls} subrequests");
    let snapshot = view(&fake, &mut object, "snapshot");
    assert_eq!(
        snapshot["view"]["coverage"]["state"], "indexing",
        "{snapshot}"
    );

    // The rest of the alarms scan the non-Markdown files.
    let mut counts = vec![calls];
    counts.extend(run_alarms(&fake, &mut object, &slot));
    assert!(counts.iter().all(|c| *c <= 50), "{counts:?}");
    assert!(counts.len() > 2, "slices: {counts:?}");
    let snapshot = view(&fake, &mut object, "snapshot");
    assert_eq!(
        snapshot["view"]["coverage"]["state"], "complete",
        "{snapshot}"
    );
}

#[test]
fn a_big_repo_stays_within_the_budget() {
    // req:web.worker.replica/first-build@36cba39d
    let mut all = repo_files(&[]);
    for i in 0..6000 {
        all.extend(files(&[(
            &format!("src/f{i}.rs"),
            &format!("fn f{i}() {{}}"),
        )]));
    }
    let fake = fake_with("111", all);
    let (mut object, _mem, slot) = object();
    block_on(object.view(&fake, "snapshot", false));
    let counts = run_alarms(&fake, &mut object, &slot);
    assert!(counts.iter().all(|c| *c <= 50), "{counts:?}");
    assert!(counts.len() > 3, "{counts:?}");
    let snapshot = view(&fake, &mut object, "snapshot");
    assert_eq!(snapshot["view"]["coverage"]["state"], "complete");
}

#[test]
fn a_lost_webhook() {
    // req:web.worker.replica/resync@69cbabef
    let fake = fake_with("111", repo_files(&[]));
    let (mut object, _mem, slot) = built(&fake);
    // A read a minute ago keeps the pages open.
    fake.now.set(fake.now.get() + 60);
    block_on(object.view(&fake, "implementation", false));
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        repo.claims.push(("a".into(), "c1".into(), fake.now.get()));
    }
    let before = view(&fake, &mut object, "implementation");
    assert_eq!(
        before["view"]["flight"]["totals"]["by_state"]["claimed"],
        serde_json::Value::Null
    );
    // The resync is about every 10 minutes: it runs when the alarm fires.
    let mut ran = false;
    for _ in 0..20 {
        let Some(at) = slot.0.get() else { break };
        fake.now.set(at);
        // The pages stay open while the test reads.
        block_on(object.view(&fake, "implementation", false));
        block_on(object.on_alarm(&fake));
        let now = view(&fake, &mut object, "implementation");
        let claimed = now["view"]["flight"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["node"] == "a" && n["state"] == "claimed");
        if claimed {
            ran = true;
            break;
        }
    }
    assert!(ran, "the claim on `a` is listed after the resync");
}

#[test]
fn nobody_looking() {
    // req:web.worker.replica/resync@69cbabef
    let fake = fake_with("111", repo_files(&[]));
    let (mut object, _mem, slot) = built(&fake);
    // Last read an hour ago; the alarm then fires 20 minutes on with no event.
    let hour = 3600;
    fake.now.set(fake.now.get() + hour);
    slot.0.set(Some(fake.now.get() + 20 * 60));
    let from = fake.gh.calls.borrow().len();
    fake.now.set(fake.now.get() + 20 * 60);
    block_on(object.on_alarm(&fake));
    assert_eq!(fake.gh.calls.borrow().len(), from, "no request to GitHub");
    assert_eq!(slot.0.get(), None, "nothing is re-armed for nobody");
}
