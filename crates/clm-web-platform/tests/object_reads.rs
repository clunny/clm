//! The reads the repo's `Replica` object calls, without storage (`object-reads`), the
//! Worker's cached token (`installation-token`), and many files read in batches
//! (`batched-fetches`).

use clm_ledger::Body;
use clm_web_platform::fake::*;
use clm_web_platform::platform::*;
use clm_web_platform::{access, files, gh, install, ledger, live};
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

fn subscribed(id: &str) -> serde_json::Value {
    json!({"id": id, "kind": "subscribed", "writer": "alice/web", "at": iso(NOW),
        "endpoint": "https://fcm.googleapis.com/x", "p256dh": "k", "auth": "a"})
}

fn setup() -> Fake {
    let fake = Fake::default();
    let mut repo = FakeRepo::new("111", clm_repo());
    repo.permissions.insert("alice".into(), "write".into());
    repo.pulls = vec![FakePull {
        number: 4,
        state: "open".into(),
        title: "t".into(),
        base_ref: "main".into(),
        head_ref: "spec/x".into(),
        head_sha: "222".into(),
        ..FakePull::default()
    }];
    ledger_of(&mut repo);
    fake.gh.add_repo("o/a", repo);
    fake.gh.grant("u", "o/a");
    fake
}

/// `alice/web`'s ledger at `l1`, holding one subscription.
fn ledger_of(repo: &mut FakeRepo) {
    clm_web_platform::fake::ledger(repo, "alice/web", "l1", &[subscribed("r1")]);
}

fn calls(f: &Fake) -> Vec<GhRequest> {
    f.gh.calls.borrow().clone()
}

/// What each uncached call gave, in a form two rounds can compare.
#[derive(Debug, PartialEq)]
struct Round {
    installation: Option<u64>,
    token: Option<Auth>,
    access: Option<String>,
    permission: Option<String>,
    listing: Option<usize>,
    objects: Option<Vec<Option<String>>>,
    pulls: Option<(Vec<live::Pull>, bool)>,
    read: Option<ledger::Sources>,
    own: Option<usize>,
    branch_tip: Result<Option<String>, gh::Failed>,
    tip: Option<String>,
    append: Option<()>,
    appended: Option<ledger::Appended>,
}

#[test]
fn a_platform_without_storage() {
    // req:web.worker.platform/object-reads@68d26c9f
    let bare = Bare(setup());
    let f = &bare.0;
    let auth = Auth::Installation("inst-9".into());
    let user = Auth::User("u".into());
    let exprs = vec!["l1:records.jsonl".to_string(), "111:specs/a/SPEC.md".into()];
    let body = Body::Unsubscribed {
        subscription: "r1".into(),
    };
    let rec = ledger::record(&bare, "alice", body);
    let acc = block_on(access::check(f, "u", "o", "a")).unwrap();
    f.cache.borrow_mut().clear();
    f.stored.borrow_mut().clear();
    f.gh.calls.borrow_mut().clear();
    let mut rounds = Vec::new();
    let mut requests = Vec::new();
    for _ in 0..2 {
        // The ledger as it was, so the round appends to the same tip.
        ledger_of(f.gh.repos.borrow_mut().get_mut("o/a").unwrap());
        let before = calls(f).len();
        let round = Round {
            installation: block_on(install::installation_id(&bare, "o/a")),
            token: block_on(install::token_direct(&bare, 9)),
            access: block_on(access::checked_direct(&bare, "u", "o", "a"))
                .unwrap()
                .map(|a| format!("{} {}", a.full_name(), a.default_branch())),
            permission: block_on(access::permission_direct(&bare, &auth, "o/a", "alice")),
            listing: block_on(files::listing(&bare, &user, "o/a", "111")).map(|l| l.entries.len()),
            objects: block_on(files::objects_direct(
                &bare, &user, "o/a", "Ledgers", &exprs,
            )),
            pulls: block_on(live::pulls_direct(&bare, &user, "o/a")),
            read: block_on(ledger::read(&bare, &user, "o/a")),
            own: block_on(ledger::own(&bare, &acc, "alice")).map(|o| o.subscriptions.len()),
            branch_tip: block_on(ledger::branch_tip(&bare, &acc, "clm/ledger/alice/web")),
            tip: block_on(ledger::tip(&bare, &acc, "clm/ledger/alice/web")),
            append: block_on(ledger::append(&bare, &acc, &rec)),
            appended: block_on(ledger::appended(&bare, &acc, &rec)),
        };
        requests.push(calls(f)[before..].to_vec());
        rounds.push(round);
    }
    assert_eq!(rounds[0], rounds[1], "each call gives the same twice");
    assert_eq!(
        requests[0], requests[1],
        "the fake GitHub saw each request twice"
    );
    assert!(
        f.cache.borrow().is_empty(),
        "nothing written to the Cache API"
    );
    assert!(f.stored.borrow().is_empty(), "nothing stored at all");
    let round = &rounds[0];
    assert_eq!(round.installation, Some(9));
    assert_eq!(round.own, Some(1));
    assert_eq!(round.branch_tip, Ok(Some("l1".into())));
    assert_eq!(round.tip.as_deref(), Some("l1"));
    assert_eq!(round.append, Some(()));
    assert!(round.appended.is_some());
    assert_eq!(
        round.pulls.as_ref().map(|p| (p.0.len(), p.1)),
        Some((1, true))
    );
    assert_eq!(round.read.as_ref().map(Vec::len), Some(1));
    assert_eq!(round.listing, Some(4));

    // Each cached form, once over the fake platform with the same GitHub, gives the same.
    assert_eq!(block_on(install::token(f, 9)), round.token);
    assert!(round.token.is_some());
    let checked = block_on(access::check(f, "u", "o", "a"))
        .map(|a| format!("{} {}", a.full_name(), a.default_branch()));
    assert_eq!(checked, round.access);
    assert_eq!(checked.as_deref(), Some("o/a main"));
    assert_eq!(
        block_on(access::permission(f, &auth, "o/a", "alice")),
        round.permission
    );
    assert_eq!(round.permission.as_deref(), Some("write"));
    assert_eq!(
        block_on(files::objects(f, &user, "o/a", "Ledgers", &exprs)),
        round.objects
    );
    assert!(round.objects.as_ref().unwrap().iter().all(Option::is_some));
    // A refusal is none from both, and a failed read none from `check`.
    assert_eq!(
        block_on(access::checked_direct(&bare, "u", "o", "none")).map(|a| a.is_none()),
        Ok(true)
    );
    assert!(block_on(access::check(f, "u", "o", "none")).is_none());
}

#[test]
fn the_workers_form_still_caches() {
    // req:web.worker.platform/installation-token@e5f2d889
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
    // req:web.worker.platform/batched-fetches@d19c47cf
    let bare = Bare(setup());
    let recs: Vec<_> = (0..400).map(|i| subscribed(&format!("r{i}"))).collect();
    let mut commit: FileMap = (0..150)
        .map(|i| (format!("f/{i:03}.md"), format!("text {i}").into_bytes()))
        .collect();
    commit.insert("bin.dat".into(), vec![0xff, 0xfe, 0]);
    let ledger_text: String = recs.iter().map(|r| format!("{r}\n")).collect();
    assert!(
        ledger_text.len() > 50_000,
        "longer than GraphQL's text limit"
    );
    commit.insert("records.jsonl".into(), ledger_text.clone().into_bytes());
    {
        let mut r = bare.0.gh.repos.borrow_mut();
        let repo = r.get_mut("o/a").unwrap();
        clm_web_platform::fake::ledger(repo, "alice/web", "l2", &recs);
        repo.commits.insert("big".into(), commit);
    }
    bare.0.gh.graphql_null.borrow_mut().insert("Nulled".into());
    let acc = block_on(access::checked_direct(&bare, "u", "o", "a"))
        .unwrap()
        .unwrap();
    let own = block_on(ledger::own(&bare, &acc, "alice")).unwrap();
    assert_eq!(own.subscriptions.len(), 400);
    let all = block_on(ledger::read(&bare, acc.auth(), "o/a")).unwrap();
    assert_eq!(all[0].1.lines().count(), 400);

    let mut exprs: Vec<String> = (0..150).map(|i| format!("big:f/{i:03}.md")).collect();
    exprs.extend(["big:bin.dat", "big:missing.md", "big:records.jsonl"].map(String::from));
    let before = bare.0.gh.calls.borrow().len();
    let got = block_on(files::objects_direct(
        &bare,
        acc.auth(),
        "o/a",
        "Blobs",
        &exprs,
    ))
    .unwrap();
    let made = bare.0.gh.calls.borrow()[before..].to_vec();
    let graphql = made.iter().filter(|c| c.url.ends_with("/graphql")).count();
    let blobs = made
        .iter()
        .filter(|c| c.url.contains("/git/blobs/"))
        .count();
    assert_eq!(
        (graphql, blobs),
        (2, 1),
        "two queries and one blob API call"
    );
    let texts: Vec<Option<String>> = (0..150).map(|i| Some(format!("text {i}"))).collect();
    assert_eq!(got[..150], texts[..], "the 150 texts in order");
    assert_eq!(
        got[150..152],
        [None, None],
        "the binary file and the missing path"
    );
    assert_eq!(got[152].as_deref(), Some(ledger_text.as_str()));
    assert_eq!(got[152].as_ref().unwrap().lines().count(), 400);
    let failing = block_on(files::objects_direct(
        &bare,
        acc.auth(),
        "o/a",
        "Nulled",
        &exprs,
    ));
    assert_eq!(failing, None, "a null repository");
}
