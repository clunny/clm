//! A platform is its seven parts, with no KV store (`platform-parts`).

use std::cell::RefCell;
use std::collections::BTreeMap;

use clm_web_platform::fake::*;
use clm_web_platform::http::{Request, Response};
use clm_web_platform::platform::*;
use clm_web_platform::{access, live};

/// The seven parts over the fake GitHub, its Cache API a map of its own, and no KV.
struct Seven {
    fake: Fake,
    cache: RefCell<BTreeMap<String, String>>,
}

impl Assets for Seven {
    async fn fetch_asset(&self, req: &Request) -> Response {
        self.fake.fetch_asset(req).await
    }
}
impl Cache for Seven {
    async fn cache_get(&self, key: &str) -> Option<String> {
        self.cache.borrow().get(key).cloned()
    }
    async fn cache_put(&self, key: &str, value: &str, _ttl_secs: u64) {
        self.cache.borrow_mut().insert(key.into(), value.into());
    }
}
impl GitHub for Seven {
    async fn send(&self, req: GhRequest) -> GhResponse {
        self.fake.send(req).await
    }
}
impl Clock for Seven {
    fn now(&self) -> u64 {
        self.fake.now()
    }
}
impl Secrets for Seven {
    fn secret(&self, name: &str) -> Option<String> {
        self.fake.secret(name)
    }
    fn random(&self, buf: &mut [u8]) {
        self.fake.random(buf)
    }
    async fn sign_rs256(&self, data: &[u8]) -> Option<Vec<u8>> {
        self.fake.sign_rs256(data).await
    }
}
impl Live for Seven {
    async fn live_forward(&self, name: &str, req: &Request) -> Option<Response> {
        self.fake.live_forward(name, req).await
    }
    async fn live_post(&self, name: &str, path: &str, body: &str) -> Option<(u16, String)> {
        self.fake.live_post(name, path, body).await
    }
}
impl Log for Seven {
    fn log(&self, line: &str) {
        self.fake.log(line)
    }
}

fn fake() -> Fake {
    let fake = Fake::default();
    let mut repo = FakeRepo::new("111", clm_repo());
    repo.default_branch = "trunk".into();
    fake.gh.add_repo("o/a", repo);
    fake.gh.grant("u", "o/a");
    fake
}

fn calls(f: &Fake) -> usize {
    f.gh.calls.borrow().len()
}

/// What `check` and `cached` give on `p`, as text.
async fn both<P: Platform>(p: &P) -> (String, Option<u32>) {
    let acc = access::check(p, "u", "o", "a").await.unwrap();
    let named = format!("{} {}", acc.full_name(), acc.default_branch());
    let item = live::cached(p, &acc, "n", async || Some(7u32)).await;
    (named, item)
}

#[test]
fn a_platform_with_no_kv() {
    // req:web.worker.platform/platform-parts@85cd127b
    let seven = Seven {
        fake: fake(),
        cache: RefCell::default(),
    };
    let over_fake = block_on(both(&fake()));
    assert_eq!(block_on(both(&seven)), over_fake);
    assert_eq!(over_fake, ("o/a trunk".to_string(), Some(7)));
    assert!(calls(&seven.fake) > 0, "the first round asked GitHub");
    let before = calls(&seven.fake);
    let acc = block_on(access::check(&seven, "u", "o", "a")).unwrap();
    let again = async || -> Option<u32> { panic!("fetched again") };
    assert_eq!(block_on(live::cached(&seven, &acc, "n", again)), Some(7));
    assert_eq!(
        calls(&seven.fake),
        before,
        "neither second call asks GitHub"
    );
}
