//! What the Worker's tests share: the fakes of Cloudflare and GitHub live in
//! `clm_web_platform::fake`; what calls `clm_web` stays here, as the wrapper [`Fake`] with
//! the knobs only these tests need, the extension trait [`FakeExt`] and the request helpers.
#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use clm_replica::Store;
use clm_web::Replicas;
use clm_web_platform::http::{Request, Response};
use clm_web_platform::platform::{
    Assets, Cache, Clock, GhRequest, GhResponse, GitHub, Live, Log, Secrets,
};
use clm_web_replica::{Alarm, Object};
use serde_json::{Value, json};

pub use clm_web_platform::fake::*;

/// An in-memory store for a real [`Object`].
#[derive(Clone, Default)]
pub struct Mem(Rc<RefCell<BTreeMap<String, Vec<u8>>>>);

impl Store for Mem {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.0.borrow().get(key).cloned()
    }
    fn put(&mut self, key: &str, bytes: Vec<u8>) {
        self.0.borrow_mut().insert(key.into(), bytes);
    }
    fn delete_all(&mut self) {
        self.0.borrow_mut().clear();
    }
}

/// An alarm slot for a real [`Object`].
#[derive(Clone, Default)]
pub struct Slot(Rc<Cell<Option<u64>>>);

impl Alarm for Slot {
    fn get(&self) -> Option<u64> {
        self.0.get()
    }
    fn set(&mut self, at: u64) {
        self.0.set(Some(at));
    }
    fn clear(&mut self) {
        self.0.set(None);
    }
}

/// A real `Replica` object over the fake GitHub, in place of the scripted one.
pub struct RealObject {
    name: String,
    object: Object<Mem, Slot>,
    slot: Slot,
}

/// A request the Worker sent to a `Replica` object.
#[derive(Debug, Clone)]
pub struct ReplicaCall {
    pub name: String,
    pub method: String,
    pub path: String,
    pub body: Option<Value>,
}

/// The `REPLICA` binding: each object answers what the test scripted for its views, and
/// records what it was asked.
pub struct FakeReplicas {
    pub bound: Cell<bool>,
    /// The scripted answer to `GET /view/<name>` of the object named `<o>/<r>`: a status and a
    /// body; a 200's body is the view, which the fake wraps as the object does.
    pub views: RefCell<BTreeMap<(String, String), (u16, Value)>>,
    pub calls: RefCell<Vec<ReplicaCall>>,
    /// The scripted answer to `POST /write`, when the test scripts the object's writes.
    pub write: RefCell<Option<(u16, String)>>,
    /// The real object, once [`Fake::real_object`] made it: it answers what isn't scripted.
    pub real: RefCell<Option<RealObject>>,
}

impl FakeReplicas {
    /// The calls made to the object `name`, in order.
    pub fn asked(&self, name: &str) -> Vec<ReplicaCall> {
        let calls = self.calls.borrow();
        calls.iter().filter(|c| c.name == name).cloned().collect()
    }
}

/// [`clm_web_platform::fake::Fake`] with the `REPLICA` binding, which the platform crate
/// doesn't fake: the traits it implements are delegated.
pub struct Fake {
    inner: clm_web_platform::fake::Fake,
    pub replicas: FakeReplicas,
    /// GitHub calls whose URL contains one of these, with the method given, answer 502
    /// (`"*"` matches any method).
    pub failing: RefCell<Vec<(String, String)>>,
    /// Refresh tokens GitHub's token endpoint refuses.
    pub refused_refresh: RefCell<BTreeSet<String>>,
    /// Changes what the fake GitHub answers, where the fake can't be told to.
    pub rewrite: RefCell<Option<Rewrite>>,
    /// Every request forwarded to a `Live` object, as the Worker built it.
    pub forwarded: RefCell<Vec<Request>>,
    /// What a `Live` object answers a forwarded request, in place of the platform fake's
    /// 101 with a socket.
    pub live_answer: RefCell<Option<Response>>,
}

/// A change to a fake GitHub answer, given the request.
pub type Rewrite = Box<dyn Fn(&GhRequest, GhResponse) -> GhResponse>;

impl Default for Fake {
    fn default() -> Fake {
        Fake {
            inner: clm_web_platform::fake::Fake::default(),
            replicas: FakeReplicas {
                bound: Cell::new(true),
                views: RefCell::default(),
                calls: RefCell::default(),
                write: RefCell::default(),
                real: RefCell::default(),
            },
            failing: RefCell::default(),
            refused_refresh: RefCell::default(),
            rewrite: RefCell::default(),
            forwarded: RefCell::default(),
            live_answer: RefCell::default(),
        }
    }
}

impl Deref for Fake {
    type Target = clm_web_platform::fake::Fake;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Fake {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Fake {
    /// Scripts the view `name` of the object `repo` to answer 200 with `view`.
    pub fn set_view(&self, repo: &str, name: &str, view: Value) {
        self.replicas
            .views
            .borrow_mut()
            .insert((repo.into(), name.into()), (200, view));
    }

    /// Scripts `POST /write` to answer `status` with `body`.
    pub fn set_write(&self, status: u16, body: Value) {
        *self.replicas.write.borrow_mut() = Some((status, body.to_string()));
    }

    /// Replaces the scripted object with a real `Replica` object for `repo`, built from the
    /// fake GitHub through its first read and its alarms, as Cloudflare runs them.
    pub fn real_object(&self, repo: &str) {
        let (mem, slot) = (Mem::default(), Slot::default());
        let object = Object::load(repo, mem, slot.clone());
        *self.replicas.real.borrow_mut() = Some(RealObject {
            name: repo.into(),
            object,
            slot,
        });
        let first = block_on(self.replica(repo, "GET", "/view/snapshot?fresh=0", None));
        assert_eq!(
            first.map(|f| f.0),
            Some(503),
            "the first read starts the build"
        );
        let mut real = self.replicas.real.borrow_mut();
        let real = real.as_mut().unwrap();
        for _ in 0..200 {
            let Some(at) = real.slot.get() else {
                self.replicas.calls.borrow_mut().clear();
                return;
            };
            self.inner.now.set(self.inner.now.get().max(at));
            block_on(real.object.on_alarm(&self.inner));
        }
        panic!("the alarm never stopped");
    }

    /// Scripts the view `name` of the object `repo` to answer `status` with `body`.
    pub fn set_answer(&self, repo: &str, name: &str, status: u16, body: Value) {
        self.replicas
            .views
            .borrow_mut()
            .insert((repo.into(), name.into()), (status, body));
    }
}

impl Replicas for Fake {
    async fn replica(
        &self,
        name: &str,
        method: &str,
        path: &str,
        body: Option<&str>,
    ) -> Option<(u16, String)> {
        if !self.replicas.bound.get() {
            return None;
        }
        self.replicas.calls.borrow_mut().push(ReplicaCall {
            name: name.into(),
            method: method.into(),
            path: path.into(),
            body: body.and_then(|b| serde_json::from_str(b).ok()),
        });
        if path == "/write"
            && let Some((status, body)) = self.replicas.write.borrow().clone()
        {
            return Some((status, body));
        }
        // The object is taken out while it answers, so no borrow is held across the await.
        let taken = self.replicas.real.borrow_mut().take();
        if let Some(mut real) = taken {
            if real.name != name {
                *self.replicas.real.borrow_mut() = Some(real);
            } else {
                let sent: Value = body
                    .and_then(|b| serde_json::from_str(b).ok())
                    .unwrap_or(Value::Null);
                let answer = match (method, path.split('?').next().unwrap_or_default()) {
                    ("POST", "/event") => Some(real.object.event(&self.inner, &sent).await),
                    ("POST", "/write") => Some(real.object.write(&self.inner, &sent).await),
                    ("GET", p) if p.starts_with("/view/") => {
                        let fresh = path.ends_with("fresh=1");
                        let name = &p["/view/".len()..];
                        Some(real.object.view(&self.inner, name, fresh).await)
                    }
                    _ => None,
                };
                *self.replicas.real.borrow_mut() = Some(real);
                return answer.map(|a| (a.status, a.body));
            }
        }
        if path == "/event" {
            return Some((202, String::new()));
        }
        let view = path.strip_prefix("/view/")?.split('?').next()?;
        let scripted = self
            .replicas
            .views
            .borrow()
            .get(&(name.to_string(), view.to_string()))
            .cloned();
        Some(match scripted {
            Some((200, view)) => (
                200,
                json!({"sha": "111", "computed_at": NOW, "view": view}).to_string(),
            ),
            Some((status, body)) => (status, body.to_string()),
            None => (404, String::new()),
        })
    }
}

impl Assets for Fake {
    async fn fetch_asset(&self, req: &Request) -> Response {
        self.inner.fetch_asset(req).await
    }
}

impl Cache for Fake {
    async fn cache_get(&self, key: &str) -> Option<String> {
        self.inner.cache_get(key).await
    }
    async fn cache_put(&self, key: &str, value: &str, ttl_secs: u64) {
        self.inner.cache_put(key, value, ttl_secs).await
    }
}

impl Fake {
    /// Makes GitHub answer 502 to every `method` call whose URL contains `url`.
    pub fn fail(&self, method: &str, url: &str) {
        self.failing.borrow_mut().push((method.into(), url.into()));
    }

    /// The answer GitHub gives in place of the fake's, by the knobs above.
    fn knob(&self, req: &GhRequest) -> Option<GhResponse> {
        let failing = self.failing.borrow();
        if failing
            .iter()
            .any(|(m, u)| (m == "*" || m == req.method) && req.url.contains(u.as_str()))
        {
            return Some(GhResponse {
                status: 502,
                body: String::new(),
            });
        }
        let body: Value = serde_json::from_str(req.body.as_deref()?).ok()?;
        let refresh = body["refresh_token"].as_str()?;
        // GitHub refuses a refresh token with a 200 that names the error.
        self.refused_refresh
            .borrow()
            .contains(refresh)
            .then(|| GhResponse {
                status: 200,
                body: json!({"error": "bad_refresh_token"}).to_string(),
            })
    }
}

impl GitHub for Fake {
    async fn send(&self, req: GhRequest) -> GhResponse {
        match self.knob(&req) {
            Some(res) => {
                self.gh.calls.borrow_mut().push(req);
                res
            }
            None => {
                let asked = req.clone();
                let res = self.inner.send(req).await;
                match self.rewrite.borrow().as_ref() {
                    Some(f) => f(&asked, res),
                    None => res,
                }
            }
        }
    }
}

impl Live for Fake {
    async fn live_forward(&self, name: &str, req: &Request) -> Option<Response> {
        if self.inner.live.bound.get() {
            self.forwarded.borrow_mut().push(req.clone());
        }
        let scripted = self.live_answer.borrow().clone();
        match scripted {
            Some(res) if self.inner.live.bound.get() => Some(res),
            _ => self.inner.live_forward(name, req).await,
        }
    }
    async fn live_post(&self, name: &str, path: &str, body: &str) -> Option<(u16, String)> {
        self.inner.live_post(name, path, body).await
    }
}

impl Log for Fake {
    fn log(&self, line: &str) {
        self.inner.log(line)
    }
}

impl Clock for Fake {
    fn now(&self) -> u64 {
        self.inner.now()
    }
}

impl Secrets for Fake {
    fn secret(&self, name: &str) -> Option<String> {
        self.inner.secret(name)
    }
    fn random(&self, buf: &mut [u8]) {
        self.inner.random(buf)
    }
    async fn sign_rs256(&self, data: &[u8]) -> Option<Vec<u8>> {
        self.inner.sign_rs256(data).await
    }
}

/// The `clm_web` entry points on [`Fake`], a foreign type, so an extension trait.
pub trait FakeExt {
    /// Handles `req`, asserting it stayed within 50 subrequests.
    fn handle(&self, req: Request) -> Response;
    /// A session cookie header value for `token`, signed in as `clunny`, valid for 8 hours.
    fn cookie(&self, token: &str) -> String;
    /// A session cookie header value for `token`, signed in as `login`.
    fn cookie_as(&self, token: &str, login: &str) -> String;
    /// A user who can read `repos`.
    fn user(&self, token: &str, repos: &[&str]) -> String;
}

impl FakeExt for Fake {
    fn handle(&self, req: Request) -> Response {
        let before = self.gh.calls.borrow().len();
        let res = block_on(clm_web::handle(self, req));
        let used = self.gh.calls.borrow().len() - before;
        assert!(used <= 50, "{used} subrequests");
        res
    }
    fn cookie(&self, token: &str) -> String {
        self.cookie_as(token, "clunny")
    }

    fn cookie_as(&self, token: &str, login: &str) -> String {
        let session = clm_web::session::Session {
            token: token.into(),
            refresh: format!("{token}-refresh"),
            expires: self.now.get() + 8 * 3600,
            login: login.into(),
        };
        let sealed = block_on(clm_web::session::seal(self, &session)).expect("a cookie key");
        format!("__Host-clm={sealed}")
    }

    fn user(&self, token: &str, repos: &[&str]) -> String {
        for r in repos {
            self.gh.grant(token, r);
        }
        self.cookie(token)
    }
}

pub fn get(path: &str, cookie: &str) -> Request {
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    let mut headers = BTreeMap::new();
    if !cookie.is_empty() {
        headers.insert("cookie".to_string(), cookie.to_string());
    }
    Request {
        method: "GET".into(),
        path: path.into(),
        query: query.into(),
        headers,
        body: Vec::new(),
        origin: ORIGIN.into(),
    }
}

pub fn post(path: &str, cookie: &str, body: &[u8]) -> Request {
    let mut req = get(path, cookie);
    req.method = "POST".into();
    req.body = body.to_vec();
    req.headers.insert("origin".into(), ORIGIN.into());
    req
}

pub fn webhook(event: &str, body: &Value, secret: &str) -> Request {
    let bytes = body.to_string().into_bytes();
    let mut req = post("/webhook", "", &bytes);
    req.headers.remove("origin");
    req.headers.insert("x-github-event".into(), event.into());
    req.headers
        .insert("x-hub-signature-256".into(), signature(secret, &bytes));
    req
}

pub fn push_body(repo: &str, before: &str, after: &str) -> Value {
    let (o, r) = repo.split_once('/').unwrap();
    json!({
        "ref": "refs/heads/main",
        "before": before,
        "after": after,
        "repository": {"full_name": repo, "name": r, "owner": {"login": o}, "default_branch": "main"},
        "installation": {"id": 9},
    })
}
