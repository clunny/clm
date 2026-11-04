//! `POST /webhook`: GitHub's events. The signature is checked before anything is parsed.
//! A verified event is passed to the `Replica` object of each repo it names
//! (`webhook-refresh`); the Worker computes no view and writes no KV.

use std::collections::BTreeSet;

use hmac::{Hmac, Mac};
use serde_json::{Value, json};
use sha2::Sha256;

use crate::replicas::Replicas;
use clm_web_platform::access::valid_name;
use clm_web_platform::http::{Request, Response};
use clm_web_platform::platform::Platform;

/// Whether `X-Hub-Signature-256` is the HMAC-SHA256 of the body under the webhook secret,
/// compared in constant time.
fn verified<P: Platform>(p: &P, req: &Request) -> bool {
    let Some(secret) = p.secret("GITHUB_WEBHOOK_SECRET") else {
        return false;
    };
    let Some(hex) = req
        .header("x-hub-signature-256")
        .and_then(|s| s.strip_prefix("sha256="))
    else {
        return false;
    };
    let Some(tag) = decode_hex(hex) else {
        return false;
    };
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(&req.body);
    mac.verify_slice(&tag).is_ok()
}

fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

pub async fn handle<P: Platform + Replicas>(p: &P, req: &Request) -> Response {
    if !verified(p, req) {
        return Response::empty(401);
    }
    let event = req.header("x-github-event").unwrap_or_default();
    let Some(body) = serde_json::from_slice::<Value>(&req.body).ok() else {
        return Response::empty(if event == "push" { 400 } else { 204 });
    };
    if ![
        "push",
        "pull_request",
        "installation",
        "installation_repositories",
    ]
    .contains(&event)
    {
        return Response::empty(204);
    }
    let post = json!({
        "event": event,
        "delivery": req.header("x-github-delivery"),
        "installation": body["installation"]["id"],
        "body": body,
    })
    .to_string();
    for repo in named(&body) {
        match p.replica(&repo, "POST", "/event", Some(&post)).await {
            Some((202, _)) => {}
            other => p.log(&format!("webhook {repo}: the object answered {other:?}")),
        }
    }
    Response::empty(200)
}

/// The repos an event names, whose names GitHub could have given: the one a `push` or
/// `pull_request` is about, and those an installation event lists as given or taken away.
fn named(body: &Value) -> BTreeSet<String> {
    let lists = ["repositories", "repositories_added", "repositories_removed"];
    let repos = lists
        .iter()
        .flat_map(|l| body[l].as_array().into_iter().flatten())
        .chain(std::iter::once(&body["repository"]));
    repos
        .filter_map(|r| r["full_name"].as_str())
        .filter(|name| {
            name.split_once('/')
                .is_some_and(|(o, r)| valid_name(o) && valid_name(r))
        })
        .map(String::from)
        .collect()
}
