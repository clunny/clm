//! The reads the repo's `Replica` object calls, without storage (`object-reads`).

use clm_ledger::Body;
use clm_web_platform::fake::*;
use clm_web_platform::platform::*;
use clm_web_platform::{access, files, install, ledger};
use serde_json::json;

/// Only what the object has: GitHub, the clock, secrets, `Live` and the log, over the fake.
struct Bare(Fake);

impl GitHub for Bare {
    async fn send(&self, req: GhRequest) -> GhResponse {
        self.0.send(req).await
    }
}
impl Clock for Bare {
    fn now(&self) -> u64 {
        self.0.now()
    }
}
impl Secrets for Bare {
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
impl Live for Bare {
    async fn live_forward(
        &self,
        name: &str,
        req: &clm_web_platform::http::Request,
    ) -> Option<clm_web_platform::http::Response> {
        self.0.live_forward(name, req).await
    }
    async fn live_post(&self, name: &str, path: &str, body: &str) -> Option<(u16, String)> {
        self.0.live_post(name, path, body).await
    }
}
impl Log for Bare {
    fn log(&self, line: &str) {
        self.0.log(line)
    }
}

fn setup() -> Fake {
    let fake = Fake::default();
    let mut repo = FakeRepo::new("111", clm_repo());
    repo.permissions.insert("alice".into(), "write".into());
    fake.gh.add_repo("o/a", repo);
    fake.gh.grant("u", "o/a");
    let mut r = fake.gh.repos.borrow_mut();
    let repo = r.get_mut("o/a").unwrap();
    let rec = json!({"id": "r1", "kind": "subscribed", "writer": "alice/web", "at": iso(NOW),
        "endpoint": "https://fcm.googleapis.com/x", "p256dh": "k", "auth": "a"});
    ledger_of(repo, rec);
    drop(r);
    fake
}

fn ledger_of(repo: &mut FakeRepo, rec: serde_json::Value) {
    clm_web_platform::fake::ledger(repo, "alice/web", "l1", &[rec]);
}

fn calls(f: &Fake) -> usize {
    f.gh.calls.borrow().len()
}

#[test]
fn a_platform_without_storage() {
    // req:web.worker.platform/object-reads@e46c07e5
    let bare = Bare(setup());
    let f = &bare.0;
    let auth = Auth::Installation("inst-9".into());
    let mut seen = Vec::new();
    for _ in 0..2 {
        let before = calls(f);
        let token = block_on(install::token_direct(&bare, 7));
        assert_eq!(token, Some(Auth::Installation("inst-7".into())));
        let acc = block_on(access::check_direct(&bare, "u", "o", "a")).unwrap();
        assert_eq!(acc.full_name(), "o/a");
        let perm = block_on(access::permission_direct(&bare, &auth, "o/a", "alice"));
        assert_eq!(perm.as_deref(), Some("write"));
        let sha = block_on(files::listing(&bare, acc.auth(), "o/a", "111")).unwrap();
        assert!(!sha.entries.is_empty());
        let exprs = vec!["l1:records.jsonl".to_string()];
        let objs = block_on(files::objects_direct(
            &bare,
            acc.auth(),
            "o/a",
            "Ledgers",
            &exprs,
        ))
        .unwrap();
        assert!(objs[0].is_some());
        let own = block_on(ledger::own(&bare, &acc, "alice")).unwrap();
        assert_eq!(own.subscriptions.len(), 1);
        let tip = block_on(ledger::tip(&bare, &acc, "clm/ledger/alice/web"));
        assert_eq!(tip.as_deref(), Some("l1"));
        let rec = ledger::record(
            &bare,
            "alice",
            Body::Unsubscribed {
                subscription: "r1".into(),
            },
        );
        block_on(ledger::append(&bare, &acc, &rec)).unwrap();
        seen.push(calls(f) - before);
        // Put the ledger back so the second round appends to the same tip.
        let mut r = f.gh.repos.borrow_mut();
        let repo = r.get_mut("o/a").unwrap();
        let rec = json!({"id": "r1", "kind": "subscribed", "writer": "alice/web", "at": iso(NOW),
            "endpoint": "https://fcm.googleapis.com/x", "p256dh": "k", "auth": "a"});
        ledger_of(repo, rec);
    }
    assert_eq!(seen[0], seen[1], "each request made twice");
    assert!(
        f.cache.borrow().is_empty() && f.kv.borrow().is_empty(),
        "kept nothing"
    );
}

#[test]
fn the_workers_form_still_caches() {
    // req:web.worker.platform/object-reads@e46c07e5
    let fake = Fake::default();
    block_on(install::token(&fake, 7)).unwrap();
    block_on(install::token(&fake, 7)).unwrap();
    let tokens = fake
        .gh
        .calls
        .borrow()
        .iter()
        .filter(|c| c.url.ends_with("/access_tokens"))
        .count();
    assert_eq!(tokens, 1);
}

#[test]
fn a_truncated_ledger_is_read_whole() {
    // req:web.worker.platform/object-reads@e46c07e5
    let bare = Bare(setup());
    let recs: Vec<_> = (0..400)
        .map(|i| {
            json!({"id": format!("r{i}"), "kind": "subscribed", "writer": "alice/web", "at": iso(NOW),
                "endpoint": "https://fcm.googleapis.com/x", "p256dh": "k", "auth": "a"})
        })
        .collect();
    {
        let mut r = bare.0.gh.repos.borrow_mut();
        clm_web_platform::fake::ledger(r.get_mut("o/a").unwrap(), "alice/web", "l2", &recs);
    }
    let acc = block_on(access::check_direct(&bare, "u", "o", "a")).unwrap();
    let own = block_on(ledger::own(&bare, &acc, "alice")).unwrap();
    assert_eq!(own.subscriptions.len(), 400);
    let all = block_on(ledger::read(&bare, acc.auth(), "o/a")).unwrap();
    assert_eq!(all[0].1.lines().count(), 400);
}
