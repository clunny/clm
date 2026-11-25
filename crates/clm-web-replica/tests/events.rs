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
    // req:web.worker.replica/applies-events@fad75cef
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
fn a_push_listing_every_kind_of_file() {
    // req:web.worker.replica/fetches-what-the-replica-wants@bb23e1f0
    let paths = [
        "web/.gitattributes",
        "clm.toml",
        "docs/decisions/d.md",
        "src/b.rs",
        "README.md",
        "docs/guide.md",
    ];
    let at = |round: &str| {
        let texts: Vec<String> = paths.iter().map(|p| format!("{p} {round}\n")).collect();
        let pairs: Vec<(&str, &str)> = paths
            .iter()
            .zip(&texts)
            .map(|(p, t)| (*p, t.as_str()))
            .collect();
        repo_files(&pairs)
    };
    let fake = fake_with("111", at("one"));
    let (mut object, _mem, _slot) = built(&fake);
    commit(&fake, "222", at("two"));
    let from = gh_calls(&fake);
    let fetched_before = fake.gh.fetched_paths().len();
    block_on(object.event(&fake, &event("push", "d1", push("main", "111", "222"))));
    let queries = graphql_calls(&fake, from);
    assert_eq!(queries.len(), 1, "{queries:?}");
    let mut fetched = fake.gh.fetched_paths().split_off(fetched_before);
    fetched.sort();
    assert_eq!(
        fetched,
        [
            "clm.toml",
            "docs/decisions/d.md",
            "src/b.rs",
            "web/.gitattributes"
        ]
    );
    for p in &paths[..4] {
        assert!(queries[0].contains(&format!("222:{p}")), "{}", queries[0]);
    }
    assert!(!queries[0].contains("README.md"));
    assert!(!queries[0].contains("docs/guide.md"));
}

/// Files of every kind the replica's rule sorts, beside the repo's spec files.
fn every_kind(round: &str) -> clm_web_platform::fake::FileMap {
    let a = format!("fn a() {{ /* {round} */ }}");
    let attrs = format!("* text # {round}\n");
    let toml = format!("# {round}\n");
    let readme = format!("# r {round}\n");
    let d = format!("# d {round}\n");
    repo_files(&[
        ("a.rs", &a),
        ("web/.gitattributes", &attrs),
        ("clm.toml", &toml),
        ("README.md", &readme),
        ("docs/decisions/d.md", &d),
    ])
}

/// The paths each blob query since `from` calls fetched, query by query.
fn blob_queries(fake: &clm_web_platform::fake::Fake, from: usize) -> Vec<Vec<String>> {
    graphql_calls(fake, from)
        .iter()
        .filter(|body| body.contains("query Blobs"))
        .map(|body| {
            let v: serde_json::Value = serde_json::from_str(body).unwrap();
            v["variables"]
                .as_object()
                .unwrap()
                .iter()
                .filter(|(k, _)| k.starts_with('e'))
                .map(|(_, e)| e.as_str().unwrap().split_once(':').unwrap().1.to_string())
                .collect()
        })
        .collect()
}

/// Every file `wanted` accepts was fetched, nothing else, and the kept text came first.
fn kept_text_first(fetched: &[String]) {
    let last_kept = fetched
        .iter()
        .rposition(|p| clm_replica::keeps_text(p))
        .expect("kept text fetched");
    let first_other = fetched
        .iter()
        .position(|p| !clm_replica::keeps_text(p))
        .expect("code fetched");
    assert!(last_kept < first_other, "{fetched:?}");
    for p in [
        "web/.gitattributes",
        "clm.toml",
        "docs/decisions/d.md",
        "a.rs",
    ] {
        assert!(fetched.contains(&p.to_string()), "{p}: {fetched:?}");
    }
    assert!(!fetched.contains(&"README.md".to_string()), "{fetched:?}");
}

#[test]
fn a_first_build_fetches_the_kept_text_first() {
    // req:web.worker.replica/fetches-what-the-replica-wants@bb23e1f0
    let fake = fake_with("111", every_kind("one"));
    built(&fake);
    let queries = blob_queries(&fake, 0);
    assert!(
        queries[0].iter().all(|p| clm_replica::keeps_text(p)),
        "the first slice holds only kept text: {queries:?}"
    );
    kept_text_first(&queries.concat());
}

#[test]
fn a_recovery_fetches_the_kept_text_first() {
    // req:web.worker.replica/fetches-what-the-replica-wants@bb23e1f0
    let fake = fake_with("111", every_kind("one"));
    let (mut object, _mem, _slot) = built(&fake);
    // A force-push: the compare isn't a fast-forward, so the object lists the tree.
    commit(&fake, "222", every_kind("two"));
    fake.gh
        .repos
        .borrow_mut()
        .get_mut("o/a")
        .unwrap()
        .compare_status
        .insert("111...222".into(), "diverged".into());
    let from = gh_calls(&fake);
    block_on(object.event(&fake, &event("push", "d1", push("main", "111", "222"))));
    assert!(
        urls(&fake, from)
            .iter()
            .any(|u| u.contains("/git/trees/222")),
        "the tree at 222 is listed"
    );
    kept_text_first(&blob_queries(&fake, from).concat());
}

#[test]
fn a_ledger_append() {
    // req:web.worker.replica/applies-events@fad75cef
    let mut repo = FakeRepo::new("111", repo_files(&[]));
    let record = |id: &str| {
        json!({"id": id, "writer": "alice/web", "at": "2027-01-15T07:00:00.000Z",
            "kind": "proposed", "slug": id, "title": id, "body": "", "nodes": []})
    };
    ledger(&mut repo, "alice/web", "LA", &[record("r1")]);
    let fake = clm_web_platform::fake::Fake::default();
    fake.gh.add_repo("o/a", repo);
    let (mut object, mem, slot) = built(&fake);
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
    drop(calls);
    run_alarms(&fake, &mut object, &slot);
    let seen = view(&fake, &mut object, "implementation");
    assert_eq!(
        seen["view"]["proposals"].as_array().map(Vec::len),
        Some(2),
        "{seen}"
    );

    // The branch is deleted: nothing is fetched, and its records leave the views.
    fake.gh
        .repos
        .borrow_mut()
        .get_mut("o/a")
        .unwrap()
        .branches
        .remove("clm/ledger/alice/web");
    let from = gh_calls(&fake);
    let mut body = push("clm/ledger/alice/web", "LB", &"0".repeat(40));
    body["deleted"] = json!(true);
    block_on(object.event(&fake, &event("push", "d2", body)));
    assert_eq!(gh_calls(&fake), from, "nothing was fetched");
    // What the event saved, recomputed with no resync to read GitHub again.
    let mut saved = clm_replica::Replica::load(&mem);
    saved.recompute(fake.now.get());
    let Some(clm_replica::Answer::Ready { value, .. }) = saved.view("implementation") else {
        panic!("no implementation view");
    };
    assert_eq!(value["proposals"], json!([]), "{value}");
}

#[test]
fn a_truncated_compare() {
    // req:web.worker.replica/applies-events@fad75cef
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
    // req:web.worker.replica/alarm-batches@d6095ba0
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
fn an_idle_alarm_pulled_in() {
    // req:web.worker.replica/alarm-batches@d6095ba0
    let fake = fake_with("111", repo_files(&[("t.rs", "fn t() {}")]));
    let (mut object, _mem, slot) = built(&fake);
    // Read a minute ago; the alarm is set ten minutes ahead for a resync.
    view(&fake, &mut object, "snapshot");
    fake.now.set(fake.now.get() + 60);
    let now = fake.now.get();
    slot.0.set(Some(now + 600));
    commit(&fake, "222", repo_files(&[("t.rs", "fn t2() {}")]));
    block_on(object.event(&fake, &event("push", "d1", push("main", "111", "222"))));
    assert_eq!(slot.0.get(), Some(now + 60), "one minute after the push");
    // An alarm due within that minute is kept.
    slot.0.set(Some(now + 30));
    commit(&fake, "333", repo_files(&[("t.rs", "fn t3() {}")]));
    block_on(object.event(&fake, &event("push", "d2", push("main", "222", "333"))));
    assert_eq!(slot.0.get(), Some(now + 30));
}

#[test]
fn a_claim_lapses_while_a_page_is_open() {
    // req:web.worker.replica/alarm-batches@d6095ba0
    let fake = fake_with("111", repo_files(&[]));
    let (mut object, mem, slot) = built(&fake);
    // A claim on `a` pushed at "10:00", four hours less 200 seconds before now.
    let ten = fake.now.get() + 200 - 4 * 3600;
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        repo.commits
            .insert("c1".into(), repo_files(&[("x.rs", "fn x() {}")]));
        // GitHub lists the claim too, so a resync keeps it.
        repo.claims.push(("a".into(), "c1".into(), ten));
    }
    let mut body = push("impl/a", "0", "c1");
    body["created"] = json!(true);
    body["head_commit"] = json!({"timestamp": clm_web_platform::fake::iso(ten)});
    block_on(object.event(&fake, &event("push", "d1", body)));
    fake.now.set(slot.0.get().unwrap());
    block_on(object.on_alarm(&fake));
    let claimed = |v: &serde_json::Value| {
        v["view"]["flight"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["node"] == "a" && n["state"] == "claimed")
    };
    // Read at 13:58, as it were.
    fake.now.set(ten + 4 * 3600 - 120);
    let before = view(&fake, &mut object, "implementation");
    assert!(claimed(&before), "{before}");
    let earlier = notices(&fake);
    // `t`: the time the last recompute said the implementation view next changes.
    let t = clm_replica::Replica::load(&mem)
        .recompute(fake.now.get())
        .next
        .expect("the claim is live, so the view changes when it lapses");
    assert!(t > fake.now.get(), "{t}");
    // A tag's push applies nothing: its delivery is no event the next notice may name.
    let alarm = slot.0.get();
    let mut tag = push("main", "0", "c1");
    tag["ref"] = json!("refs/tags/v1");
    block_on(object.event(&fake, &event("push", "dt", tag)));
    assert_eq!(slot.0.get(), alarm, "nothing applied sets no alarm");
    let mut fired = Vec::new();
    // The alarms run until a second past `t`.
    while let Some(at) = slot.0.get().filter(|at| *at <= t + 1) {
        fake.now.set(at);
        block_on(object.on_alarm(&fake));
        fired.push(at);
    }
    fake.now.set(t + 1);
    assert!(fired.contains(&t), "an alarm fired at {t}: {fired:?}");
    let after = view(&fake, &mut object, "implementation");
    assert!(!claimed(&after), "{after}");
    let posted = notices(&fake).split_off(earlier.len());
    assert!(!posted.is_empty(), "a notice was posted");
    for body in &posted {
        let notice: serde_json::Value = serde_json::from_str(body).unwrap();
        assert_eq!(notice.as_object().unwrap().len(), 1, "{notice}");
        let delivery = notice["delivery"].as_str().unwrap();
        assert_eq!(delivery.len(), 32, "{delivery}");
        assert!(
            delivery
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "{delivery}"
        );
        // It names no event: neither the push's delivery nor an earlier notice's.
        assert!(delivery != "d1" && delivery != "dt", "{delivery}");
        assert!(
            !earlier.iter().any(|e| e.contains(delivery)),
            "{delivery} named before"
        );
    }
}

#[test]
fn a_question_lands_through_the_alarm() {
    // req:web.worker.replica/push-waiting@e5643acc
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
    assert!(sent(&fake).is_empty());
    commit(&fake, "222", questions(&["Which region?"]));
    block_on(object.event(&fake, &event("push", "d1", push("main", "111", "222"))));
    run_alarms(&fake, &mut object, &slot);
    let sent = sent(&fake);
    assert_eq!(sent.len(), 1, "{sent:?}");
    let messages = sent[0]["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert_eq!(
        messages[0]["subscription"]["endpoint"],
        "https://fcm.googleapis.com/fcm/send/alice"
    );
    assert_eq!(messages[0]["payload"]["body"], "Which region?");
}

/// The bodies posted to the `Live` object's `/send`.
fn sent(fake: &clm_web_platform::fake::Fake) -> Vec<serde_json::Value> {
    fake.live
        .calls
        .borrow()
        .iter()
        .filter(|c| c.path == "/send")
        .map(|c| serde_json::from_str(&c.body).unwrap())
        .collect()
}

/// The repo with a `question` item titled each of `titles`.
fn questions(titles: &[&str]) -> clm_web_platform::fake::FileMap {
    let items: Vec<(String, String)> = titles
        .iter()
        .enumerate()
        .map(|(i, t)| {
            (
                format!("backlog/q{i}.md"),
                format!("---\ntitle: {t}\nkind: question\ncomponents: [a]\n---\nBody.\n"),
            )
        })
        .collect();
    let pairs: Vec<(&str, &str)> = items
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    repo_files(&pairs)
}

/// A `subscribed` record `id` of `alice/web`'s, for `endpoint` with key `p256dh`.
fn alices(id: &str, endpoint: &str, p256dh: &str) -> serde_json::Value {
    json!({"id": id, "writer": "alice/web", "at": "2027-01-15T07:00:00.000Z",
        "kind": "subscribed", "endpoint": endpoint, "p256dh": p256dh, "auth": "AuthSecret"})
}

/// `o/a` at `111`, with alice's subscriptions and her `read` permission.
fn subscribed_repo(records: &[serde_json::Value]) -> clm_web_platform::fake::Fake {
    let mut fake = fake_with("111", repo_files(&[]));
    fake.secrets
        .insert("VAPID_PUBLIC_KEY".into(), "BPublicKey".into());
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        ledger(repo, "alice/web", "LA", records);
        repo.permissions.insert("alice".into(), "read".into());
    }
    fake
}

#[test]
fn nothing_to_send() {
    // req:web.worker.replica/push-waiting@e5643acc
    let sub = alices("s1", "https://fcm.googleapis.com/fcm/send/alice", "BKey");
    // No VAPID key set.
    let mut fake = subscribed_repo(std::slice::from_ref(&sub));
    fake.secrets.remove("VAPID_PUBLIC_KEY");
    let (mut object, _mem, slot) = built(&fake);
    commit(&fake, "222", questions(&["Which region?"]));
    block_on(object.event(&fake, &event("push", "d1", push("main", "111", "222"))));
    run_alarms(&fake, &mut object, &slot);
    assert_eq!(view(&fake, &mut object, "snapshot")["sha"], "222");
    assert!(sent(&fake).is_empty(), "no send without a VAPID key");

    // A VAPID key, and no recompute before `222`'s: the push reaches an object that holds
    // nothing, and its first build is the first recompute.
    let fake = subscribed_repo(&[sub]);
    commit(&fake, "222", questions(&["Which region?"]));
    let (mut object, _mem, slot) = common::object();
    block_on(object.event(&fake, &event("push", "d1", push("main", "111", "222"))));
    run_alarms(&fake, &mut object, &slot);
    assert_eq!(view(&fake, &mut object, "snapshot")["sha"], "222");
    assert!(
        sent(&fake).is_empty(),
        "no send without a previous recompute"
    );
}

#[test]
fn a_gone_endpoint() {
    // req:web.worker.replica/push-waiting@e5643acc
    let (e1, e2) = (
        "https://fcm.googleapis.com/fcm/send/one",
        "https://fcm.googleapis.com/fcm/send/two",
    );
    let fake = subscribed_repo(&[alices("s1", e1, "K1"), alices("s2", e2, "K2")]);
    let (mut object, _mem, slot) = built(&fake);
    let over = Over::new(&fake);
    *over.send.borrow_mut() = Some(json!({"gone": [e1]}).to_string());
    let keys = |body: &serde_json::Value| -> Vec<String> {
        let mut keys: Vec<String> = body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["subscription"]["p256dh"].as_str().unwrap().to_string())
            .collect();
        keys.sort();
        keys
    };
    let titles = ["One?", "Two?", "Three?"];
    let mut tip = "111".to_string();
    for (n, round) in ["222", "333", "444"].iter().enumerate() {
        if n == 2 {
            // Alice subscribes again as `s3`, with `s1`'s endpoint.
            let records = [
                alices("s1", e1, "K1"),
                alices("s2", e2, "K2"),
                alices("s3", e1, "K3"),
            ];
            ledger(
                fake.gh.repos.borrow_mut().get_mut("o/a").unwrap(),
                "alice/web",
                "LA3",
                &records,
            );
        }
        commit(&fake, round, questions(&titles[..=n]));
        let body = push("main", &tip, round);
        block_on(object.event(&over, &event("push", &format!("d{n}"), body)));
        run_alarms_on(&over, &fake, &mut object, &slot);
        tip = round.to_string();
    }
    let sent = sent(&fake);
    assert_eq!(sent.len(), 3, "{sent:?}");
    assert_eq!(keys(&sent[0]), ["K1", "K2"]);
    assert_eq!(keys(&sent[1]), ["K2"]);
    assert_eq!(keys(&sent[2]), ["K2", "K3"]);
}

#[test]
fn a_ref_the_views_read_moves() {
    // req:web.worker.replica/applies-events@fad75cef
    let fake = fake_with("111", repo_files(&[]));
    let (mut object, mem, slot) = built(&fake);
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
    let made = urls(&fake, from);
    // Only the head's tree, for its code key: no compare, no file fetched.
    assert_eq!(made.len(), 1, "{made:?}");
    assert!(made[0].contains("/git/trees/c1"));
    run_alarms(&fake, &mut object, &slot);
    let now = view(&fake, &mut object, "implementation");
    assert_eq!(now["view"]["flight"]["totals"]["by_state"]["claimed"], 1);
    assert!(claimed_on(&now, "a"), "{now}");

    // The branch is deleted: nothing is fetched, and the claim goes.
    fake.gh
        .repos
        .borrow_mut()
        .get_mut("o/a")
        .unwrap()
        .claims
        .clear();
    let from = gh_calls(&fake);
    let mut body = push("impl/a", "c1", &"0".repeat(40));
    body["deleted"] = json!(true);
    block_on(object.event(&fake, &event("push", "d2", body)));
    assert_eq!(urls(&fake, from), Vec::<String>::new());
    // What the event saved, recomputed with no resync to read GitHub again.
    let mut saved = clm_replica::Replica::load(&mem);
    saved.recompute(fake.now.get());
    let Some(clm_replica::Answer::Ready { value, .. }) = saved.view("implementation") else {
        panic!("no implementation view");
    };
    assert!(!claimed_on(&json!({ "view": value }), "a"), "{value}");
}

/// Whether the implementation view `seen` lists a claim on `node`.
fn claimed_on(seen: &serde_json::Value, node: &str) -> bool {
    seen["view"]["flight"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["node"] == node && n["state"] == "claimed")
}

#[test]
fn a_push_in_the_feed() {
    // req:web.worker.replica/applies-events@fad75cef
    let fake = fake_with("111", repo_files(&[]));
    let (mut object, _mem, slot) = built(&fake);
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        repo.commits
            .insert("c1".into(), repo_files(&[("x.rs", "fn x() {}")]));
        repo.claims.push(("a".into(), "c1".into(), fake.now.get()));
        ledger(repo, "alice/web", "LA", &[]);
    }
    let base = fake.now.get() - 3600;
    let pushed = |branch: &str, after: &str, created: bool, at: u64| {
        let mut body = push(branch, "0", after);
        body["created"] = json!(created);
        body["head_commit"] = json!({"timestamp": clm_web_platform::fake::iso(at)});
        body
    };
    let pushes = [
        pushed("impl/a", "c1", true, base),
        // A push that names no pusher is its sender's.
        {
            let mut body = pushed("spec/x", "c1", true, base + 60);
            body.as_object_mut().unwrap().remove("pusher");
            body["sender"] = json!({"login": "alice"});
            body
        },
        pushed("clm/ledger/alice/web", "LA", false, base + 120),
    ];
    for (i, body) in pushes.into_iter().enumerate() {
        block_on(object.event(&fake, &event("push", &format!("d{i}"), body)));
    }
    let mut tag = pushed("main", "c1", true, base + 180);
    tag["ref"] = json!("refs/tags/v1");
    block_on(object.event(&fake, &event("push", "dt", tag)));
    run_alarms(&fake, &mut object, &slot);
    let seen = view(&fake, &mut object, "implementation");
    let mut feed: Vec<(String, String, String, u64)> = seen["view"]["activity"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["at"].as_u64().unwrap() >= base)
        .map(|a| {
            (
                a["kind"].as_str().unwrap().into(),
                a["ref"].as_str().unwrap().into(),
                a["actor"].as_str().unwrap().into(),
                a["at"].as_u64().unwrap(),
            )
        })
        .collect();
    feed.sort();
    let entry = |kind: &str, r: &str, at: u64| (kind.into(), r.into(), "alice".into(), at);
    assert_eq!(
        feed,
        vec![
            entry("claim", "a", base),
            entry("push", "impl/a", base),
            entry("push", "spec/x", base + 60),
        ],
        "{}",
        seen["view"]["activity"]
    );
}

#[test]
fn a_pull_request_and_an_uninstall() {
    // req:web.worker.replica/push-review@692d328c
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
    let sends: Vec<serde_json::Value> = fake
        .live
        .calls
        .borrow()
        .iter()
        .filter(|c| c.path == "/send")
        .map(|c| serde_json::from_str(&c.body).unwrap())
        .collect();
    assert_eq!(sends.len(), 1);
    let messages = sends[0]["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert_eq!(
        messages[0]["subscription"]["endpoint"],
        "https://fcm.googleapis.com/fcm/send/alice"
    );
    assert_eq!(messages[0]["payload"]["body"], "spec/x: Add x");

    // An uninstall that removes `o/a` deletes what its object holds, and not `o/b`'s.
    // req:web.worker.replica/uninstall-deletes@5353d3fd
    let other = fake_with("111", repo_files(&[]));
    let (_built, b_mem, _b_slot) = built(&other);
    let b_held = b_mem.0.borrow().clone();
    let mut b = clm_web_replica::Object::load("o/b", b_mem.clone(), Slot::default());
    assert!(!mem.0.borrow().is_empty());
    let removed = json!({"action": "removed", "repositories_removed": [{"full_name": "o/a"}]});
    let uninstall = event("installation_repositories", "d2", removed);
    let answer = block_on(object.event(&fake, &uninstall));
    assert_eq!(answer.status, 202);
    assert!(mem.0.borrow().is_empty());
    assert_eq!(block_on(object.view(&fake, "snapshot", false)).status, 503);
    assert_eq!(block_on(b.event(&other, &uninstall)).status, 202);
    assert!(!b.removed());
    assert_eq!(*b_mem.0.borrow(), b_held, "o/b's object is unchanged");
    fake.secrets.remove("VAPID_PUBLIC_KEY");
}
