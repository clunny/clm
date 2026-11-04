//! The `Replica` Durable Object: the core in `object` bound to `worker`'s storage, alarm,
//! `Fetch` and WebCrypto. It decides nothing (`object-class`).

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use clm_replica::Store;
use clm_web_platform::http;
use clm_web_platform::platform::{Auth, Clock, GhRequest, GhResponse, GitHub, Live, Log, Secrets};
use futures::lock::Mutex;
use serde::Deserialize;
use serde_json::Value;
use worker::js_sys::{self, Array, Function, Object as JsObject, Reflect, Uint8Array};

use worker::wasm_bindgen::{self, JsCast, JsValue};
use worker::wasm_bindgen_futures::JsFuture;
use worker::{
    DurableObject, Env, Fetch, Headers, Method, Request, RequestInit, Response, Result, SqlStorage,
    SqlStorageValue, State, durable_object,
};

use crate::{Alarm, Answer, Object};

/// The Durable Object binding of the `Live` objects in `wrangler.toml` (`web.live`).
const LIVE_BINDING: &str = "LIVE";

/// One object per repo, addressed by `id_from_name("<owner>/<repo>")`. It holds the repo's
/// replica in memory while it is awake and in its SQLite storage between wakes.
#[durable_object]
pub struct Replica {
    state: State,
    env: Env,
    /// The loaded object; the lock keeps a request that arrives while another awaits
    /// GitHub from running over its state.
    loaded: Mutex<Option<Loaded>>,
}

struct Loaded {
    object: Object<SqlStore, Slot>,
    /// The alarm the runtime holds, in Unix seconds.
    synced: Option<u64>,
}

impl DurableObject for Replica {
    fn new(state: State, env: Env) -> Self {
        Replica {
            state,
            env,
            loaded: Mutex::new(None),
        }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        let url = req.url()?;
        let method = req.method();
        let path = url.path().to_string();
        let body = match method {
            Method::Post => match req.json::<Value>().await {
                Ok(body) => body,
                Err(_) => return answer(Answer::empty(422)),
            },
            _ => Value::Null,
        };
        let host = Cloudflare {
            env: self.env.clone(),
        };
        let mut guard = self.loaded.lock().await;
        let Some(loaded) = self.load(&mut guard).await else {
            return answer(Answer::empty(404));
        };
        let reply = match (method, path.as_str()) {
            (Method::Post, "/event") => loaded.object.event(&host, &body).await,
            (Method::Post, "/write") => loaded.object.write(&host, &body).await,
            (Method::Get, view) if view.starts_with("/view/") => {
                let fresh = url.query_pairs().any(|(k, v)| k == "fresh" && v == "1");
                let name = &view["/view/".len()..];
                loaded.object.view(&host, name, fresh).await
            }
            _ => Answer::empty(404),
        };
        self.settle(&mut guard).await;
        answer(reply)
    }

    async fn alarm(&self) -> Result<Response> {
        let host = Cloudflare {
            env: self.env.clone(),
        };
        let mut guard = self.loaded.lock().await;
        if let Some(loaded) = self.load(&mut guard).await {
            // The runtime consumed the alarm, so whatever the object sets is new.
            loaded.synced = None;
            loaded.object.on_alarm(&host).await;
        }
        self.settle(&mut guard).await;
        Response::empty()
    }
}

impl Replica {
    /// The object, loaded from storage on first use; `None` for an object without a name.
    async fn load<'a>(&self, slot: &'a mut Option<Loaded>) -> Option<&'a mut Loaded> {
        if slot.is_none() {
            let name = self.state.id().name()?;
            let storage = self.state.storage();
            let sql = storage.sql();
            sql.exec(
                "CREATE TABLE IF NOT EXISTS blobs (k TEXT PRIMARY KEY, v TEXT NOT NULL)",
                None,
            )
            .ok()?;
            let synced = storage
                .get_alarm()
                .await
                .ok()
                .flatten()
                .map(|ms| (ms / 1000) as u64);
            let store = SqlStore { sql };
            *slot = Some(Loaded {
                object: Object::load(&name, store, Slot(synced)),
                synced,
            });
        }
        slot.as_mut()
    }

    /// Brings the runtime's alarm to the one the object holds, and frees an object an
    /// uninstall emptied.
    async fn settle(&self, guard: &mut Option<Loaded>) {
        let Some(loaded) = guard.as_mut() else {
            return;
        };
        let storage = self.state.storage();
        if loaded.object.removed() {
            let _ = storage.delete_alarm().await;
            let _ = storage.delete_all().await;
            *guard = None;
            return;
        }
        let want = loaded.object.alarm().get();
        if want == loaded.synced {
            return;
        }
        let done = match want {
            Some(at) => {
                let ahead = at.saturating_sub(worker::Date::now().as_millis() / 1000);
                storage.set_alarm((ahead * 1000) as i64).await
            }
            None => storage.delete_alarm().await,
        };
        match done {
            Ok(()) => loaded.synced = want,
            Err(e) => worker::console_error!("replica: the alarm wasn't set: {e}"),
        }
    }
}

fn answer(reply: Answer) -> Result<Response> {
    if reply.body.is_empty() {
        return Ok(Response::empty()?.with_status(reply.status));
    }
    let headers = Headers::new();
    headers.set("content-type", "application/json")?;
    Ok(Response::ok(reply.body)?
        .with_status(reply.status)
        .with_headers(headers))
}

/// The alarm as the object sees it; `settle` carries it to the runtime.
struct Slot(Option<u64>);

impl Alarm for Slot {
    fn get(&self) -> Option<u64> {
        self.0
    }
    fn set(&mut self, at: u64) {
        self.0 = Some(at);
    }
    fn clear(&mut self) {
        self.0 = None;
    }
}

/// The object's SQLite storage as a key-value store: values are base64, at most 1 MiB
/// before encoding, under the 2 MB a row holds.
struct SqlStore {
    sql: SqlStorage,
}

#[derive(Deserialize)]
struct Row {
    v: String,
}

impl Store for SqlStore {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        let rows = self
            .sql
            .exec(
                "SELECT v FROM blobs WHERE k = ?",
                vec![SqlStorageValue::String(key.into())],
            )
            .ok()?
            .to_array::<Row>()
            .ok()?;
        STANDARD.decode(&rows.first()?.v).ok()
    }

    fn put(&mut self, key: &str, bytes: Vec<u8>) {
        let done = self.sql.exec(
            "INSERT OR REPLACE INTO blobs (k, v) VALUES (?, ?)",
            vec![
                SqlStorageValue::String(key.into()),
                SqlStorageValue::String(STANDARD.encode(bytes)),
            ],
        );
        if let Err(e) = done {
            worker::console_error!("replica: {key} wasn't stored: {e}");
        }
    }

    fn delete_all(&mut self) {
        if let Err(e) = self.sql.exec("DELETE FROM blobs", None) {
            worker::console_error!("replica: the storage wasn't emptied: {e}");
        }
    }
}

/// The platform the object runs on: `Fetch` for GitHub and the `Live` objects, the clock,
/// the secrets and WebCrypto, and the log. Nothing else of the Worker's bindings.
struct Cloudflare {
    env: Env,
}

fn method_of(name: &str) -> Method {
    match name {
        "POST" => Method::Post,
        "PATCH" => Method::Patch,
        "HEAD" => Method::Head,
        "DELETE" => Method::Delete,
        _ => Method::Get,
    }
}

impl GitHub for Cloudflare {
    async fn send(&self, req: GhRequest) -> GhResponse {
        let failed = GhResponse {
            status: 502,
            body: String::new(),
        };
        let headers = Headers::new();
        let _ = headers.set("user-agent", "clm-web");
        let accept = if req.url.starts_with(clm_web_platform::gh::API) {
            "application/vnd.github+json"
        } else {
            "application/json"
        };
        let _ = headers.set("accept", accept);
        match &req.auth {
            Auth::User(t) | Auth::App(t) | Auth::Installation(t) => {
                let _ = headers.set("authorization", &format!("Bearer {t}"));
            }
            Auth::None => {}
        }
        let mut init = RequestInit::new();
        init.with_method(method_of(req.method));
        if let Some(body) = &req.body {
            let _ = headers.set("content-type", "application/json");
            init.with_body(Some(JsValue::from_str(body)));
        }
        init.with_headers(headers);
        let Ok(request) = Request::new_with_init(&req.url, &init) else {
            return failed;
        };
        let Ok(mut res) = Fetch::Request(request).send().await else {
            return failed;
        };
        GhResponse {
            status: res.status_code(),
            body: res.text().await.unwrap_or_default(),
        }
    }
}

impl Live for Cloudflare {
    /// The object forwards no socket; only the Worker does.
    async fn live_forward(&self, _name: &str, _req: &http::Request) -> Option<http::Response> {
        None
    }

    async fn live_post(&self, name: &str, path: &str, body: &str) -> Option<(u16, String)> {
        let namespace = self.env.durable_object(LIVE_BINDING).ok()?;
        let stub = namespace.id_from_name(name).ok()?.get_stub().ok()?;
        let mut init = RequestInit::new();
        init.with_method(Method::Post);
        init.with_body(Some(JsValue::from_str(body)));
        let request = Request::new_with_init(&format!("https://live{path}"), &init).ok()?;
        let mut res = stub.fetch_with_request(request).await.ok()?;
        Some((res.status_code(), res.text().await.unwrap_or_default()))
    }
}

impl Log for Cloudflare {
    fn log(&self, line: &str) {
        worker::console_log!("{line}");
    }
}

impl Clock for Cloudflare {
    fn now(&self) -> u64 {
        worker::Date::now().as_millis() / 1000
    }
}

fn crypto() -> Option<JsValue> {
    Reflect::get(&js_sys::global(), &"crypto".into()).ok()
}

fn method(target: &JsValue, name: &str) -> Option<Function> {
    Reflect::get(target, &name.into()).ok()?.dyn_into().ok()
}

impl Secrets for Cloudflare {
    fn secret(&self, name: &str) -> Option<String> {
        self.env.secret(name).ok().map(|s| s.to_string())
    }

    /// Fails closed: without randomness a nonce would repeat, so the request aborts.
    fn random(&self, buf: &mut [u8]) {
        let array = Uint8Array::new_with_length(buf.len() as u32);
        let filled = crypto().and_then(|crypto| {
            let f = method(&crypto, "getRandomValues")?;
            f.call1(&crypto, &array).ok()
        });
        assert!(filled.is_some(), "crypto.getRandomValues is unavailable");
        array.copy_to(buf);
    }

    async fn sign_rs256(&self, data: &[u8]) -> Option<Vec<u8>> {
        let pem = self.secret("GITHUB_APP_PRIVATE_KEY")?;
        let body: String = pem
            .lines()
            .filter(|l| !l.starts_with("-----"))
            .collect::<Vec<_>>()
            .concat();
        let der = STANDARD.decode(body.trim()).ok()?;
        let subtle = Reflect::get(&crypto()?, &"subtle".into()).ok()?;

        let algorithm = JsObject::new();
        Reflect::set(&algorithm, &"name".into(), &"RSASSA-PKCS1-v1_5".into()).ok()?;
        Reflect::set(&algorithm, &"hash".into(), &"SHA-256".into()).ok()?;
        let usages = Array::of1(&"sign".into());
        let import = method(&subtle, "importKey")?;
        let args = Array::of5(
            &"pkcs8".into(),
            &Uint8Array::from(der.as_slice()),
            &algorithm,
            &JsValue::FALSE,
            &usages,
        );
        let promise = import.apply(&subtle, &args).ok()?;
        let key = JsFuture::from(js_sys::Promise::from(promise)).await.ok()?;

        let sign = method(&subtle, "sign")?;
        let promise = sign
            .call3(
                &subtle,
                &"RSASSA-PKCS1-v1_5".into(),
                &key,
                &Uint8Array::from(data),
            )
            .ok()?;
        let signature = JsFuture::from(js_sys::Promise::from(promise)).await.ok()?;
        Some(Uint8Array::new(&signature).to_vec())
    }
}
