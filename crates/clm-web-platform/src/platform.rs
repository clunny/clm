//! What the Worker needs from Cloudflare and GitHub, as traits the core is generic over.
//! The `worker` module implements them on wasm32; tests implement them with fakes.

use crate::http::{Request, Response};

/// Workers KV: durable, but the Free plan allows 1,000 writes a day.
#[allow(async_fn_in_trait)]
pub trait Kv {
    async fn kv_get(&self, key: &str) -> Option<String>;
    async fn kv_put(&self, key: &str, value: &str);
}

/// The Workers Cache API: short-lived, per data centre, and free to write.
#[allow(async_fn_in_trait)]
pub trait Cache {
    async fn cache_get(&self, key: &str) -> Option<String>;
    async fn cache_put(&self, key: &str, value: &str, ttl_secs: u64);
}

/// Who a GitHub call is made as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Auth {
    /// The signed-in person's user-to-server token.
    User(String),
    /// The App's own JWT.
    App(String),
    /// An installation token, for the webhook and Cron runs, which have no user.
    Installation(String),
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhRequest {
    pub method: &'static str,
    /// Absolute URL: `https://api.github.com/...` or `https://github.com/login/...`.
    pub url: String,
    pub auth: Auth,
    /// A JSON body.
    pub body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhResponse {
    pub status: u16,
    pub body: String,
}

/// One HTTP call to GitHub. Each call is a subrequest, of which a request may make 50.
#[allow(async_fn_in_trait)]
pub trait GitHub {
    async fn send(&self, req: GhRequest) -> GhResponse;
}

pub trait Clock {
    /// Unix seconds.
    fn now(&self) -> u64;
}

/// The Worker's secrets and the crypto that only the platform has.
#[allow(async_fn_in_trait)]
pub trait Secrets {
    /// `GITHUB_APP_ID`, `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, `GITHUB_WEBHOOK_SECRET`
    /// or `COOKIE_KEY`.
    fn secret(&self, name: &str) -> Option<String>;
    fn random(&self, buf: &mut [u8]);
    /// RSASSA-PKCS1-v1_5 with SHA-256 under `GITHUB_APP_PRIVATE_KEY`.
    async fn sign_rs256(&self, data: &[u8]) -> Option<Vec<u8>>;
}

/// The static assets the app was built with, through the Worker's `ASSETS` binding. Only
/// the Worker reads `/_clm/*`: the invited logins (web/invited-users).
#[allow(async_fn_in_trait)]
pub trait Assets {
    /// The binding's answer to `req`, whole: status, headers and bytes as it gives them
    /// (`asset-responses`).
    async fn fetch_asset(&self, req: &Request) -> Response;

    /// The text of the asset at `path`, such as `/_clm/invited.json`: `Some` only for a
    /// 200 whose body is UTF-8.
    async fn asset(&self, path: &str) -> Option<String> {
        let req = Request {
            method: "GET".into(),
            path: path.into(),
            ..Request::default()
        };
        let res = self.fetch_asset(&req).await;
        if res.status != 200 {
            return None;
        }
        String::from_utf8(res.body).ok()
    }
}

/// The `LIVE` Durable Object binding: one `Live` object per repo, named `<owner>/<repo>`
/// (`web.live`), which holds the repo's open pages' sockets.
#[allow(async_fn_in_trait)]
pub trait Live {
    /// Forwards `req` to the object named `name` and returns its answer, a 101 carrying
    /// the socket; `None` without the binding.
    async fn live_forward(&self, name: &str, req: &Request) -> Option<Response>;
    /// Posts `body` to `path` (`/notify` or `/send`) of the object named `name`: its
    /// status and body, or `None` without the binding or when the post fails.
    async fn live_post(&self, name: &str, path: &str, body: &str) -> Option<(u16, String)>;
}

/// The Worker's log, which `wrangler tail` shows.
pub trait Log {
    fn log(&self, line: &str);
}

pub trait Platform: Kv + Cache + GitHub + Clock + Secrets + Assets + Live + Log {}

impl<T: Kv + Cache + GitHub + Clock + Secrets + Assets + Live + Log> Platform for T {}
