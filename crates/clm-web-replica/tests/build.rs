//! The first build of a repo's replica (`first-build`) and its resync (`resync`).

mod common;

use clm_web_platform::fake::files;
use common::*;
use futures::executor::block_on;

#[test]
fn a_repo_of_1000_files() {
    // req:web.worker.replica/first-build@d07421b0
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
    // req:web.worker.replica/first-build@d07421b0
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
