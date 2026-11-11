//! The `workers-rs` adapter: the fetch event, KV, the Cache API, `Fetch` and
//! WebCrypto behind the core's traits. It decides nothing.

use std::collections::BTreeMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use worker::js_sys::{self, Array, Function, Object, Reflect, Uint8Array};
use worker::wasm_bindgen::{JsCast, JsValue};
use worker::wasm_bindgen_futures::JsFuture;
use worker::{
    Context, Env, Headers, KvStore, Method, Request, RequestInit, Response, WebSocket, event,
};

use crate::Replicas;
use crate::assets::{AssetBinding, AssetCache, BindingAnswer, fetch_asset};
use clm_web_platform::http::{self, percent_encode};
use clm_web_platform::platform::{
    Assets, Auth, Cache, Clock, GhRequest, GhResponse, GitHub, Kv, Live, Log, Secrets,
};

/// The KV namespace binding in `wrangler.toml`.
const KV_BINDING: &str = "CLM";
/// The static assets binding in `wrangler.toml`.
const ASSETS_BINDING: &str = "ASSETS";
/// The Durable Object binding of the `Live` objects in `wrangler.toml` (`web.live`).
const LIVE_BINDING: &str = "LIVE";
/// The Durable Object binding of the `Replica` objects in `wrangler.toml`
/// (`web.worker.replica`).
const REPLICA_BINDING: &str = "REPLICA";
/// Cache API keys are URLs; they live under the app's own host.
const CACHE_PREFIX: &str = "https://clm-app.dev/__cache/";

struct Cloudflare {
    env: Env,
    kv: Option<KvStore>,
}

impl Cloudflare {
    fn new(env: Env) -> Cloudflare {
        let kv = env.kv(KV_BINDING).ok();
        Cloudflare { env, kv }
    }
}

impl Kv for Cloudflare {
    async fn kv_get(&self, key: &str) -> Option<String> {
        self.kv.as_ref()?.get(key).text().await.ok().flatten()
    }

    async fn kv_put(&self, key: &str, value: &str) {
        if let Some(Ok(put)) = self.kv.as_ref().map(|kv| kv.put(key, value)) {
            let _ = put.execute().await;
        }
    }
}

impl Cache for Cloudflare {
    async fn cache_get(&self, key: &str) -> Option<String> {
        let url = format!("{CACHE_PREFIX}{}", percent_encode(key));
        let mut res = worker::Cache::default().get(url, false).await.ok()??;
        res.text().await.ok()
    }

    async fn cache_put(&self, key: &str, value: &str, ttl_secs: u64) {
        let url = format!("{CACHE_PREFIX}{}", percent_encode(key));
        let Ok(res) = Response::from_bytes(value.as_bytes().to_vec()) else {
            return;
        };
        let headers = Headers::new();
        let _ = headers.set("cache-control", &format!("max-age={ttl_secs}"));
        let _ = worker::Cache::default()
            .put(url, res.with_headers(headers))
            .await;
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
        let Ok(mut res) = worker::Fetch::Request(request).send().await else {
            return failed;
        };
        GhResponse {
            status: res.status_code(),
            body: res.text().await.unwrap_or_default(),
        }
    }
}

thread_local! {
    /// The `/_clm/*` answers kept for the isolate's life (`asset-adapter`).
    static ASSET_CACHE: AssetCache = AssetCache::default();
}

/// The `ASSETS` binding's calls.
struct Binding(worker::Fetcher);

impl AssetBinding for Binding {
    async fn fetch(
        &self,
        method: &str,
        path: &str,
        query: &str,
        headers: &BTreeMap<String, String>,
    ) -> Result<BindingAnswer, ()> {
        let out = Headers::new();
        for (name, value) in headers {
            let _ = out.set(name, value);
        }
        let mut init = RequestInit::new();
        init.with_method(Method::from(method.to_string()));
        init.with_headers(out);
        let query = if query.is_empty() {
            String::new()
        } else {
            format!("?{query}")
        };
        let mut res = self
            .0
            .fetch(format!("https://assets.local{path}{query}"), Some(init))
            .await
            .map_err(|_| ())?;
        Ok(BindingAnswer {
            status: res.status_code(),
            headers: res.headers().entries().collect(),
            body: res.bytes().await.map_err(|_| ()),
        })
    }
}

impl Assets for Cloudflare {
    async fn fetch_asset(&self, req: &http::Request) -> http::Response {
        let binding = self.env.assets(ASSETS_BINDING).ok().map(Binding);
        let cache = ASSET_CACHE.with(|c| c.clone());
        fetch_asset(binding.as_ref(), &cache, req).await
    }
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

impl Cloudflare {
    /// The `Live` object named `name`: `None` without the binding, else the stub or the
    /// error getting it.
    fn live(&self, name: &str) -> Option<worker::Result<worker::Stub>> {
        let namespace = self.env.durable_object(LIVE_BINDING).ok()?;
        Some(namespace.id_from_name(name).and_then(|id| id.get_stub()))
    }
}

impl Live for Cloudflare {
    async fn live_forward(&self, name: &str, req: &http::Request) -> Option<http::Response> {
        let failed = Some(http::Response::empty(502));
        let Ok(stub) = self.live(name)? else {
            return failed;
        };
        // The object needs the upgrade and nothing else: never the session cookie.
        let headers = Headers::new();
        if let Some(upgrade) = req.header("upgrade") {
            let _ = headers.set("upgrade", upgrade);
        }
        let mut init = RequestInit::new();
        init.with_method(method_of(&req.method));
        init.with_headers(headers);
        let Ok(request) = Request::new_with_init("https://live/", &init) else {
            return failed;
        };
        let Ok(res) = stub.fetch_with_request(request).await else {
            return failed;
        };
        let status = res.status_code();
        let Some(socket) = res.websocket() else {
            // A 101 without a socket can't be sent on; anything else is the object's answer.
            return Some(http::Response::empty(if status == 101 {
                502
            } else {
                status
            }));
        };
        let mut out = http::Response::empty(status);
        out.upgrade = Some(http::Upgrade::new(socket));
        Some(out)
    }

    async fn live_post(&self, name: &str, path: &str, body: &str) -> Option<(u16, String)> {
        let stub = self.live(name)?.ok()?;
        let mut init = RequestInit::new();
        init.with_method(Method::Post);
        init.with_body(Some(JsValue::from_str(body)));
        let request = Request::new_with_init(&format!("https://live{path}"), &init).ok()?;
        let mut res = stub.fetch_with_request(request).await.ok()?;
        Some((res.status_code(), res.text().await.unwrap_or_default()))
    }
}

impl Replicas for Cloudflare {
    async fn replica(
        &self,
        name: &str,
        method: &str,
        path: &str,
        body: Option<&str>,
    ) -> Option<(u16, String)> {
        let namespace = self.env.durable_object(REPLICA_BINDING).ok()?;
        let stub = namespace.id_from_name(name).ok()?.get_stub().ok()?;
        let mut init = RequestInit::new();
        init.with_method(method_of(method));
        if let Some(body) = body {
            init.with_body(Some(JsValue::from_str(body)));
        }
        let request = Request::new_with_init(&format!("https://replica{path}"), &init).ok()?;
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

        let algorithm = Object::new();
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

async fn to_core(mut req: Request) -> Option<http::Request> {
    let url = req.url().ok()?;
    let headers = req
        .headers()
        .entries()
        .map(|(k, v)| (k.to_lowercase(), v))
        .collect();
    let method = req.method().to_string();
    let body = if method == "GET" {
        Vec::new()
    } else {
        req.bytes().await.unwrap_or_default()
    };
    Some(http::Request {
        method,
        path: url.path().into(),
        query: url.query().unwrap_or_default().into(),
        headers,
        body,
        origin: url.origin().ascii_serialization(),
    })
}

fn from_core(res: http::Response) -> worker::Result<Response> {
    let headers = Headers::new();
    for (k, v) in &res.headers {
        headers.append(k, v)?;
    }
    // A 101 carries the `Live` object's socket through to the browser.
    if let Some(socket) = res.upgrade.as_ref().and_then(|u| u.get::<WebSocket>()) {
        return Ok(Response::from_websocket(socket.clone())?.with_headers(headers));
    }
    Ok(Response::from_bytes(res.body)?
        .with_status(res.status)
        .with_headers(headers))
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> worker::Result<Response> {
    let Some(req) = to_core(req).await else {
        return Response::error("bad request", 400);
    };
    from_core(crate::handle(&Cloudflare::new(env), req).await)
}
