//! The times the object gives the replica are instants, offsets counted (`times-as-instants`).

mod common;

use clm_replica::{Answer, Replica};
use clm_web_platform::fake::{Fake, iso};
use clm_web_platform::http::{Request, Response};
use clm_web_platform::platform::{Clock, GhRequest, GhResponse, GitHub, Live, Log, Secrets};
use common::*;
use futures::executor::block_on;
use serde_json::{Value, json};

/// 2026-10-01T10:00:00Z.
const TEN_UTC: u64 = 1_790_848_800;
const ELEVEN_UTC: u64 = TEN_UTC + 3600;
const FOURTEEN_UTC: u64 = TEN_UTC + 4 * 3600;

/// The fake GitHub, but its refs listing gives each commit time `iso(TEN_UTC)` as its
/// committer two hours east wrote it.
/// It counts the listings it rewrote, so the test knows an offset reached the object.
struct East<'a>(&'a Fake, std::cell::Cell<usize>);

impl GitHub for East<'_> {
    async fn send(&self, req: GhRequest) -> GhResponse {
        let refs = req
            .body
            .as_deref()
            .is_some_and(|b| b.contains("query Refs"));
        let mut res = self.0.send(req).await;
        let utc = format!("\"committedDate\":\"{}\"", iso(TEN_UTC));
        if refs && res.body.contains(&utc) {
            res.body = res
                .body
                .replace(&utc, "\"committedDate\":\"2026-10-01T12:00:00+02:00\"");
            self.1.set(self.1.get() + 1);
        }
        res
    }
}

impl Clock for East<'_> {
    fn now(&self) -> u64 {
        self.0.now()
    }
}

impl Secrets for East<'_> {
    fn secret(&self, name: &str) -> Option<String> {
        self.0.secret(name)
    }
    fn random(&self, buf: &mut [u8]) {
        self.0.random(buf)
    }
    async fn sign_rs256(&self, data: &[u8]) -> Option<Vec<u8>> {
        self.0.sign_rs256(data).await
    }
}

impl Live for East<'_> {
    async fn live_forward(&self, name: &str, req: &Request) -> Option<Response> {
        self.0.live_forward(name, req).await
    }
    async fn live_post(&self, name: &str, path: &str, body: &str) -> Option<(u16, String)> {
        self.0.live_post(name, path, body).await
    }
}

impl Log for East<'_> {
    fn log(&self, line: &str) {
        self.0.log(line)
    }
}

/// The replica `mem` holds, loaded and recomputed at 11:00 UTC: the time the implementation
/// view next changes, and that view.
fn recomputed_at_eleven(mem: &Mem) -> (Option<u64>, Value) {
    let mut replica = Replica::load(mem);
    let next = replica.recompute(ELEVEN_UTC).next;
    let Some(Answer::Ready { value, .. }) = replica.view("implementation") else {
        panic!("the saved replica has no implementation view");
    };
    (next, value)
}

#[test]
fn a_commit_made_two_hours_east() {
    // req:web.worker.replica/times-as-instants@5944fea6
    // A push creating `impl/a`, whose head commit was made at 12:00:00.5 two hours east.
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
    let mut body = push("impl/a", "0", "c1");
    body["created"] = json!(true);
    body["head_commit"] = json!({"timestamp": "2026-10-01T12:00:00.5+02:00"});
    block_on(object.event(&fake, &event("push", "d1", body)));
    let (next, view) = recomputed_at_eleven(&mem);
    let claims: Vec<&Value> = view["activity"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["kind"] == "claim")
        .collect();
    assert_eq!(claims.len(), 1, "{}", view["activity"]);
    assert_eq!(claims[0]["ref"], "a");
    assert_eq!(claims[0]["at"], TEN_UTC, "the claim is at 10:00 UTC");
    assert_eq!(next, Some(FOURTEEN_UTC), "the claim lapses at 14:00 UTC");

    // A resync listing `impl/b`, which the replica doesn't hold, committed at 12:00 two hours
    // east.
    let fake = fake_with("111", repo_files(&[]));
    let east = East(&fake, Default::default());
    let (mut object, mem, slot) = built(&fake);
    fake.now.set(fake.now.get() + 60);
    block_on(object.view(&east, "implementation", false));
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        repo.claims.push(("b".into(), "c2".into(), TEN_UTC));
        repo.commits
            .insert("c2".into(), repo_files(&[("y.rs", "fn y() {}")]));
    }
    let mut resynced = false;
    for _ in 0..20 {
        let Some(at) = slot.0.get() else { break };
        fake.now.set(at);
        // The pages stay open while the test reads.
        block_on(object.view(&east, "implementation", false));
        block_on(object.on_alarm(&east));
        let (next, _) = recomputed_at_eleven(&mem);
        if next.is_some() {
            resynced = true;
            assert_eq!(next, Some(FOURTEEN_UTC), "the claim lapses at 14:00 UTC");
            break;
        }
    }
    assert!(resynced, "the resync applied `impl/b`");
    assert!(east.1.get() > 0, "the refs listing gave the offset");
}

/// Whether the implementation view lists a claim on `node`.
fn claimed(view: &Value, node: &str) -> bool {
    view["view"]["flight"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["node"] == node && n["state"] == "claimed")
}

#[test]
fn a_time_github_gives_that_cant_be_read() {
    // req:web.worker.replica/times-as-instants@5944fea6
    // A built replica holding `impl/b`, read a minute ago.
    let fake = fake_with("111", repo_files(&[]));
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        repo.claims.push(("b".into(), "cb".into(), fake.now.get()));
        repo.commits
            .insert("cb".into(), repo_files(&[("y.rs", "fn y() {}")]));
        repo.commits
            .insert("c1".into(), repo_files(&[("x.rs", "fn x() {}")]));
        // GitHub lists `impl/a` too, once it is pushed below.
        repo.claims.push(("a".into(), "c1".into(), fake.now.get()));
    }
    let (mut object, _mem, slot) = built(&fake);
    fake.now.set(fake.now.get() + 60);
    assert!(claimed(&view(&fake, &mut object, "implementation"), "b"));

    // A push creating `impl/a` whose head commit's time is `yesterday`, applied at `ten`.
    let ten = fake.now.get();
    let mut body = push("impl/a", "0", "c1");
    body["created"] = json!(true);
    body["head_commit"] = json!({"timestamp": "yesterday"});
    block_on(object.event(&fake, &event("push", "d1", body)));
    fake.now.set(slot.0.get().unwrap());
    block_on(object.on_alarm(&fake));
    let seen = view(&fake, &mut object, "implementation");
    let claim: Vec<&Value> = seen["view"]["activity"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["kind"] == "claim" && a["ref"] == "a")
        .collect();
    assert_eq!(claim.len(), 1, "{seen}");
    assert_eq!(claim[0]["at"], ten, "recorded at the time it was applied");

    // A resync whose refs listing gives `impl/b` a `committedDate` of `soon`: still held.
    let listed = std::cell::Cell::new(0);
    let over = Over::new(&fake);
    *over.github.borrow_mut() = Box::new(|req| {
        let body = req.body.as_deref()?;
        if !(body.contains("query Refs") && body.contains("refs/heads/impl/")) {
            return None;
        }
        listed.set(listed.get() + 1);
        let nodes = json!([
            {"name": "a", "target": {"oid": "c1", "committedDate": iso(ten)}},
            {"name": "b", "target": {"oid": "cb", "committedDate": "soon"}},
        ]);
        let refs = json!({"nodes": nodes, "pageInfo": {"hasNextPage": false, "endCursor": "2"}});
        Some(GhResponse {
            status: 200,
            body: json!({"data": {"repository": {"refs": refs}}}).to_string(),
        })
    });
    for _ in 0..20 {
        let Some(at) = slot.0.get() else { break };
        fake.now.set(at);
        // The pages stay open while the test reads.
        block_on(object.view(&over, "implementation", false));
        block_on(object.on_alarm(&over));
        if listed.get() > 0 {
            break;
        }
    }
    assert!(listed.get() > 0, "a resync listed the refs");
    // The views recompute after the resync's change, if any.
    fake.now.set(fake.now.get() + 1);
    block_on(object.view(&over, "implementation", true));
    let seen = view(&fake, &mut object, "implementation");
    assert!(claimed(&seen, "b"), "`impl/b` is still held: {seen}");

    // An object that holds nothing builds over an events listing holding a push whose
    // `created_at` is `soon`: the feed holds nothing for it.
    let fake = fake_with("111", repo_files(&[]));
    fake.gh.repos.borrow_mut().get_mut("o/a").unwrap().events = json!([
        {"type": "PushEvent", "created_at": "soon", "actor": {"login": "alice"},
            "payload": {"ref": "refs/heads/impl/q"}},
        {"type": "PushEvent", "created_at": iso(fake.now.get() - 60), "actor": {"login": "bob"},
            "payload": {"ref": "refs/heads/impl/r"}},
    ]);
    let (mut object, _mem, _slot) = built(&fake);
    let seen = view(&fake, &mut object, "implementation");
    let activity = seen["view"]["activity"].as_array().unwrap();
    assert!(
        activity.iter().any(|a| a["ref"] == "impl/r"),
        "the readable push is in the feed: {activity:?}"
    );
    assert!(
        !activity.iter().any(|a| a["ref"] == "impl/q"),
        "nothing for the push whose time can't be read: {activity:?}"
    );
}
