//! Push subscriptions (`push-subscriptions-in-ledger`): the VAPID public key the app
//! subscribes with, and a repo's Notify me switch, kept as `subscribed` and
//! `unsubscribed` records in the user's web ledger.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use clm_ledger::{Body, Record};
use serde::Deserialize;
use serde_json::json;

use clm_web_platform::access::Access;
use clm_web_platform::http::{Request, Response};
use clm_web_platform::ledger;
use clm_web_platform::platform::Platform;

/// `GET /api/push/key`: `{"key"}`, or 404 without the secret, which the app reads as
/// notifications being unavailable.
pub(crate) async fn key<P: Platform>(p: &P) -> Response {
    match p.secret("VAPID_PUBLIC_KEY") {
        Some(key) => Response::json(&json!({ "key": key })),
        None => Response::empty(404),
    }
}

/// `GET .../push?endpoint=`: whether the user's web subscriptions, their own branch's
/// records among the live state's ledger sources folded alone, hold one for the endpoint
/// (`web-subscriptions-by-branch`).
pub(crate) async fn state<P: Platform>(
    p: &P,
    req: &Request,
    access: &Access,
    login: &str,
) -> Response {
    let endpoint = req.param("endpoint").unwrap_or_default();
    let Some(sources) = ledger::sources(p, access).await else {
        return Response::empty(502);
    };
    let subscribed = ledger::web_fold(&sources, login)
        .subscriptions
        .values()
        .any(|r| endpoint_of(r) == Some(endpoint.as_str()));
    Response::json(&json!({ "subscribed": subscribed }))
}

#[derive(Deserialize)]
struct Keys {
    p256dh: String,
    auth: String,
}

/// The browser's `PushSubscription.toJSON()`.
#[derive(Deserialize)]
struct Subscription {
    endpoint: String,
    keys: Keys,
}

#[derive(Deserialize)]
struct Endpoint {
    endpoint: String,
}

/// `POST .../push`: a `subscribed` record, unless the user's ledger, read at its tip,
/// already holds a live one for the endpoint; 422 for anything that isn't a subscription
/// on a known push service.
pub(crate) async fn subscribe<P: Platform>(
    p: &P,
    req: &Request,
    access: &Access,
    login: &str,
) -> Response {
    let Ok(sub) = serde_json::from_slice::<Subscription>(&req.body) else {
        return Response::empty(422);
    };
    let valid = known_service(&sub.endpoint)
        && decoded_len(&sub.keys.p256dh, Some(0x04)) == Some(65)
        && decoded_len(&sub.keys.auth, None) == Some(16);
    if !valid {
        return Response::empty(422);
    }
    let Some(live) = own(p, access, login).await else {
        return Response::empty(502);
    };
    if live
        .iter()
        .any(|r| endpoint_of(r) == Some(sub.endpoint.as_str()))
    {
        return Response::empty(200);
    }
    let body = Body::Subscribed {
        endpoint: sub.endpoint,
        p256dh: sub.keys.p256dh,
        auth: sub.keys.auth,
    };
    appended(ledger::append(p, access, &ledger::record(p, login, body)).await)
}

/// `DELETE .../push`: an `unsubscribed` record for each live subscription the user's
/// ledger, read at its tip, holds for the endpoint; nothing when it holds none.
pub(crate) async fn unsubscribe<P: Platform>(
    p: &P,
    req: &Request,
    access: &Access,
    login: &str,
) -> Response {
    let Ok(Endpoint { endpoint }) = serde_json::from_slice(&req.body) else {
        return Response::empty(422);
    };
    let Some(live) = own(p, access, login).await else {
        return Response::empty(502);
    };
    for record in live
        .iter()
        .filter(|r| endpoint_of(r) == Some(endpoint.as_str()))
    {
        let body = Body::Unsubscribed {
            subscription: record.id.clone(),
        };
        let record = ledger::record(p, login, body);
        if ledger::append(p, access, &record).await.is_none() {
            return Response::empty(502);
        }
    }
    Response::empty(200)
}

fn appended(done: Option<()>) -> Response {
    match done {
        Some(()) => Response::empty(200),
        None => Response::empty(502),
    }
}

fn endpoint_of(record: &Record) -> Option<&str> {
    match &record.body {
        Body::Subscribed { endpoint, .. } => Some(endpoint),
        _ => None,
    }
}

/// The live subscriptions in `clm/ledger/<login>/web` at its tip, so a quick off after an
/// on is decided by what the ledger holds now, not the minute-old fold.
async fn own<P: Platform>(p: &P, access: &Access, login: &str) -> Option<Vec<Record>> {
    let fold = ledger::own(p, access, login).await?;
    Some(fold.subscriptions.into_values().collect())
}

/// Whether `endpoint` is an `https` URL whose host, with no user information or port, is a
/// known push service's, so the Worker never sends to an address someone made up.
fn known_service(endpoint: &str) -> bool {
    let Some(rest) = endpoint.strip_prefix("https://") else {
        return false;
    };
    let host = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if host.contains(['@', ':']) {
        return false;
    }
    host == "fcm.googleapis.com"
        || [
            ".push.services.mozilla.com",
            ".push.apple.com",
            ".notify.windows.com",
        ]
        .iter()
        .any(|suffix| host.len() > suffix.len() && host.ends_with(suffix))
}

/// The length of base64url `text` decoded, padded or not, when it decodes and, given
/// `first`, starts with that byte.
fn decoded_len(text: &str, first: Option<u8>) -> Option<usize> {
    let bytes = URL_SAFE_NO_PAD.decode(text.trim_end_matches('=')).ok()?;
    match first {
        Some(b) if bytes.first() != Some(&b) => None,
        _ => Some(bytes.len()),
    }
}
