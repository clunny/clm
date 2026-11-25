//! The first build of a repo's replica (`first-build`) and its resync (`resync`).

mod common;

use clm_web_platform::fake::{Fake, FakePull, files, iso, node};
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
    let (mut object, _mem, slot) = object();

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
        }
        // `impl/b`'s code key can't be read: GitHub has no tree for its head.
        repo.commits
            .insert("ca".into(), repo_files(&[("x.rs", "fn x() {}")]));
        repo.events = json!([{
            "type": "CreateEvent", "created_at": iso(at), "actor": {"login": "alice"},
            "payload": {"ref": "impl/a", "ref_type": "branch"},
        }]);
    }
    let (mut object, _mem, slot) = built(&fake);
    let claims = |object: &mut Obj| -> Vec<Value> {
        let implementation = view(&fake, object, "implementation");
        implementation["view"]["activity"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| a["kind"] == "claim")
            .cloned()
            .collect()
    };
    let first = claims(&mut object);
    assert_eq!(first.len(), 1, "{first:?}");
    assert_eq!(
        (&first[0]["ref"], &first[0]["actor"]),
        (&json!("a"), &json!("alice"))
    );
    // Nor does a later resync take a ref the first build listed for a new one.
    let resyncs = |fake: &clm_web_platform::fake::Fake| {
        fake.gh
            .calls
            .borrow()
            .iter()
            .filter(|c| c.body.as_deref().is_some_and(|b| b.contains("query Refs")))
            .count()
    };
    let before = resyncs(&fake);
    for _ in 0..20 {
        // The pages stay open while the test reads.
        block_on(object.view(&fake, "implementation", false));
        let Some(at) = slot.0.get() else { break };
        fake.now.set(at);
        block_on(object.on_alarm(&fake));
        if resyncs(&fake) > before {
            break;
        }
    }
    assert!(resyncs(&fake) > before, "a resync ran");
    assert_eq!(claims(&mut object), first);
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
    // req:web.worker.replica/resync@bdd4c4e3
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
    // req:web.worker.replica/resync@bdd4c4e3
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

/// The names of the refs the stored replica holds under `prefix`.
fn held_refs(mem: &Mem, prefix: &str) -> Vec<String> {
    let state = stored_state(mem);
    state["state"]["refs"]
        .as_object()
        .unwrap()
        .keys()
        .filter(|k| k.starts_with(prefix))
        .cloned()
        .collect()
}

/// The numbers of the pull requests the stored replica holds open, in order.
fn open_pulls(mem: &Mem) -> Vec<u64> {
    let state = stored_state(mem);
    let mut open: Vec<u64> = state["state"]["pulls"]
        .as_object()
        .unwrap()
        .keys()
        .map(|k| k.parse().unwrap())
        .collect();
    open.sort();
    open
}

/// An open pull request `#number` of `impl/p<number>` on `main`.
fn fake_pull(number: u64) -> FakePull {
    FakePull {
        number,
        state: "open".into(),
        title: format!("P{number}"),
        base_ref: "main".into(),
        base_sha: "111".into(),
        head_ref: format!("impl/p{number}"),
        head_sha: format!("h{number}"),
        fork: false,
    }
}

/// The `pull_request` webhook body that opens `pull`.
fn opened(pull: &FakePull) -> Value {
    json!({"action": "opened", "pull_request": {
        "number": pull.number, "state": "open", "title": pull.title,
        "html_url": format!("https://github.com/o/a/pull/{}", pull.number),
        "head": {"ref": pull.head_ref, "sha": pull.head_sha, "repo": {"full_name": "o/a"}},
        "base": {"ref": pull.base_ref, "repo": {"full_name": "o/a"}},
    }})
}

fn set_pull_state(fake: &Fake, number: u64, state: &str) {
    let mut repos = fake.gh.repos.borrow_mut();
    let repo = repos.get_mut("o/a").unwrap();
    repo.pulls
        .iter_mut()
        .find(|p| p.number == number)
        .unwrap()
        .state = state.into();
}

fn claims_mut(fake: &Fake, f: impl FnOnce(&mut Vec<(String, String, u64)>)) {
    f(&mut fake.gh.repos.borrow_mut().get_mut("o/a").unwrap().claims);
}

#[test]
fn what_a_resync_cant_list() {
    // req:web.worker.replica/resync@bdd4c4e3
    let fake = fake_with("111", repo_files(&[]));
    let at = fake.now.get();
    claims_mut(&fake, |c| {
        for name in ["a", "z"] {
            c.push((name.into(), format!("c{name}"), at));
        }
    });
    let (mut object, mem, slot) = built(&fake);
    assert_eq!(held_refs(&mem, "impl/"), ["impl/a", "impl/z"]);
    let claimed_a = |object: &mut Obj| -> bool {
        let seen = view(&fake, object, "implementation");
        seen["view"]["flight"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["node"] == "a" && n["state"] == "claimed")
    };
    assert!(claimed_a(&mut object));

    // `impl/a` deleted with its webhook lost: the resync drops it, and its claim goes.
    claims_mut(&fake, |c| c.retain(|c| c.0 != "a"));
    resync(&fake, &mut object, &slot);
    assert_eq!(held_refs(&mem, "impl/"), ["impl/z"]);
    recompute(&fake, &mut object, &slot);
    assert!(!claimed_a(&mut object), "the claim on `a` goes");

    // `impl/` comes to hold 301 refs, `z` not among them: nothing under `impl/` changes.
    let at = fake.now.get();
    claims_mut(&fake, |c| {
        *c = (0..301)
            .map(|i| (format!("n{i:03}"), format!("cn{i}"), at))
            .collect();
    });
    resync(&fake, &mut object, &slot);
    assert_eq!(held_refs(&mem, "impl/"), ["impl/z"]);

    // 151 pull requests open, all known from webhooks: all stay open.
    let pulls: Vec<FakePull> = (1..=151).map(fake_pull).collect();
    fake.gh.repos.borrow_mut().get_mut("o/a").unwrap().pulls = pulls.clone();
    for (i, pull) in pulls.iter().enumerate() {
        let delivery = format!("p{i}");
        let answer = block_on(object.event(&fake, &event("pull_request", &delivery, opened(pull))));
        assert_eq!(answer.status, 202);
    }
    let all: Vec<u64> = (1..=151).collect();
    assert_eq!(open_pulls(&mem), all);
    resync(&fake, &mut object, &slot);
    assert_eq!(open_pulls(&mem), all, "a listing of 151 closes none");

    // #3 closed with its webhook lost: it closes, the other 150 stay open.
    set_pull_state(&fake, 3, "closed");
    resync(&fake, &mut object, &slot);
    let without_3: Vec<u64> = all.iter().copied().filter(|n| *n != 3).collect();
    assert_eq!(open_pulls(&mem), without_3);

    // 301 open and #4 closed with its webhook lost: the resync changes no pull request.
    fake.gh
        .repos
        .borrow_mut()
        .get_mut("o/a")
        .unwrap()
        .pulls
        .extend((152..=303).map(fake_pull));
    set_pull_state(&fake, 4, "closed");
    let open = fake.gh.repos.borrow()["o/a"]
        .pulls
        .iter()
        .filter(|p| p.state == "open")
        .count();
    assert_eq!(open, 301);
    resync(&fake, &mut object, &slot);
    assert_eq!(
        open_pulls(&mem),
        without_3,
        "#4 stays open, and none is added"
    );
}

/// Runs the alarm due, so what the last resync applied is recomputed.
fn recompute(fake: &Fake, object: &mut Obj, slot: &Slot) {
    let at = slot.0.get().expect("an alarm is set");
    fake.now.set(fake.now.get().max(at));
    block_on(object.on_alarm(fake));
}

#[test]
fn a_code_key_that_cant_be_read() {
    // req:web.worker.replica/resync@bdd4c4e3
    let spec = node("", &["r"], "active");
    let fake = fake_with("111", repo_files(&[("specs/y/SPEC.md", &spec)]));
    let (mut object, _mem, slot) = built(&fake);
    // `impl/y` created by `bob`, its webhook lost; GitHub has no tree for its head, so its
    // code key can't be read.
    let at = fake.now.get();
    claims_mut(&fake, |c| c.push(("y".into(), "cy".into(), at)));
    let claims_of_y = |object: &mut Obj| -> usize {
        let seen = view(&fake, object, "implementation");
        seen["view"]["activity"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| a["kind"] == "claim" && a["ref"] == "y")
            .count()
    };
    let lists_y = |object: &mut Obj| -> bool {
        let seen = view(&fake, object, "implementation");
        seen["view"]["flight"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["node"] == "y" && n["state"] == "claimed")
    };
    let trees_of_y = |fake: &Fake| {
        fake.gh
            .calls
            .borrow()
            .iter()
            .filter(|c| c.url.contains("/git/trees/cy"))
            .count()
    };
    resync(&fake, &mut object, &slot);
    let first = trees_of_y(&fake);
    assert!(first >= 1, "the code key is asked for");
    resync(&fake, &mut object, &slot);
    assert!(trees_of_y(&fake) > first, "and asked for again");
    recompute(&fake, &mut object, &slot);
    assert_eq!(claims_of_y(&mut object), 1, "one claim in the feed");
    assert!(lists_y(&mut object), "the view lists the claim");

    // `impl/y` deleted with its webhook lost: the third resync drops it.
    claims_mut(&fake, |c| c.retain(|c| c.0 != "y"));
    resync(&fake, &mut object, &slot);
    recompute(&fake, &mut object, &slot);
    assert!(!lists_y(&mut object), "no claim on `y`");
}
