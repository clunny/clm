//! What the Worker adapter decides about an asset request (`asset-adapter`): what reaches the
//! `ASSETS` binding, the 503 and 502 answers, and what is kept. `worker.rs` supplies the
//! binding and keeps only its calls.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use clm_web_platform::http::{Request, Response};

/// What the binding answered: its status, every header, and the body's bytes, or `Err` when
/// the body couldn't be read.
#[derive(Clone)]
pub struct BindingAnswer {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Result<Vec<u8>, ()>,
}

/// The `ASSETS` binding: `Err` when the fetch fails.
#[allow(async_fn_in_trait)]
pub trait AssetBinding {
    async fn fetch(
        &self,
        method: &str,
        path: &str,
        query: &str,
        headers: &BTreeMap<String, String>,
    ) -> Result<BindingAnswer, ()>;
}

/// The `/_clm/*` answers kept for an isolate's life.
#[derive(Clone, Default)]
pub struct AssetCache(Rc<RefCell<HashMap<String, Response>>>);

/// The binding's answer to `req`, or 503 without a binding and 502 when its fetch or its
/// body fails. Only a 200 to a `GET` of a `/_clm/*` path with no query is kept: it changes
/// only with a deploy, which starts new isolates.
pub async fn fetch_asset<B: AssetBinding>(
    binding: Option<&B>,
    cache: &AssetCache,
    req: &Request,
) -> Response {
    let keep = req.method == "GET" && req.query.is_empty() && req.path.starts_with("/_clm/");
    if keep && let Some(hit) = cache.0.borrow().get(&req.path) {
        return hit.clone();
    }
    let Some(binding) = binding else {
        return Response::empty(503);
    };
    // The session cookie and credentials stay with the Worker.
    let headers: BTreeMap<String, String> = req
        .headers
        .iter()
        .filter(|(k, _)| !matches!(k.as_str(), "cookie" | "authorization"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let Ok(answer) = binding
        .fetch(&req.method, &req.path, &req.query, &headers)
        .await
    else {
        return Response::empty(502);
    };
    let Ok(body) = answer.body else {
        return Response::empty(502);
    };
    let out = Response {
        status: answer.status,
        headers: answer.headers,
        body,
        upgrade: None,
    };
    if keep && out.status == 200 {
        cache.0.borrow_mut().insert(req.path.clone(), out.clone());
    }
    out
}
