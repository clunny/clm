//! The view routes (`views-from-replica`): after the access check, the snapshot,
//! implementation, capabilities and architecture routes answer the view the repo's `Replica`
//! object computed, kept in the Cache API for 30 seconds. They compute none of it.

use serde_json::{Value, json};

use crate::proposals;
use crate::replicas::Replicas;
use clm_web_platform::access::Access;
use clm_web_platform::http::{Request, Response};
use clm_web_platform::platform::Platform;

/// How long the Worker keeps an object's answer.
const TTL: u64 = 30;

/// Whether the request is a read with a valid `fresh` (`fresh-live-state`): 1 to 64
/// characters of `[0-9a-f-]`. Any other value is ignored.
pub(crate) fn is_fresh(req: &Request) -> bool {
    req.method == "GET"
        && req.param("fresh").is_some_and(|d| {
            (1..=64).contains(&d.len())
                && d.bytes()
                    .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f' | b'-'))
        })
}

fn unavailable() -> Response {
    let mut res = Response::json(&json!({ "error": "unavailable" }));
    res.status = 503;
    res
}

/// The view `name` of the repo's object: from the Cache API unless `fresh`, else asked of
/// the object (with `fresh=1` when `fresh`, which brings it up to the tip first) and kept.
/// The error is the response to give: the object's 503 `{"preparing"}` as it gave it, or
/// 503 `{"error": "unavailable"}`.
pub(crate) async fn fetch<P: Platform + Replicas>(
    p: &P,
    access: &Access,
    name: &str,
    fresh: bool,
) -> Result<Value, Response> {
    let repo = access.full_name();
    let key = format!("view:{repo}:{name}");
    if !fresh
        && let Some(view) = p
            .cache_get(&key)
            .await
            .and_then(|v| serde_json::from_str(&v).ok())
    {
        return Ok(view);
    }
    let path = format!("/view/{name}?fresh={}", u8::from(fresh));
    let (status, body) = p
        .replica(&repo, "GET", &path, None)
        .await
        .ok_or_else(unavailable)?;
    let answer: Value = serde_json::from_str(&body).map_err(|_| unavailable())?;
    match status {
        200 => {
            let view = answer.get("view").cloned().ok_or_else(unavailable)?;
            p.cache_put(&key, &view.to_string(), TTL).await;
            Ok(view)
        }
        503 if answer.get("preparing").is_some() => {
            let mut res = Response::json(&answer);
            res.status = 503;
            Err(res)
        }
        _ => Err(unavailable()),
    }
}

/// `GET .../snapshot`, `.../capabilities` and `.../architecture`: the object's view.
pub(crate) async fn get<P: Platform + Replicas>(
    p: &P,
    access: &Access,
    name: &str,
    fresh: bool,
) -> Response {
    match fetch(p, access, name, fresh).await {
        Ok(view) => Response::json(&view),
        Err(res) => res,
    }
}

/// `GET .../implementation`: the object's view with `mine` added to each of its
/// `proposals` for `login` (`proposals`), asking for no other view and reading no ledger.
pub(crate) async fn implementation<P: Platform + Replicas>(
    p: &P,
    access: &Access,
    login: &str,
    fresh: bool,
) -> Response {
    match fetch(p, access, "implementation", fresh).await {
        Ok(view) => Response::json(&proposals::with_mine(view, login)),
        Err(res) => res,
    }
}
