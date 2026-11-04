//! Typed helpers over [`GitHub`]: REST calls returning parsed JSON, and GraphQL.

use serde_json::{Value, json};

use crate::platform::{Auth, GhRequest, GitHub};

pub const API: &str = "https://api.github.com";

/// `GET <API><path>`: the status and the parsed body (`Null` if it isn't JSON).
pub async fn get<P: GitHub>(p: &P, auth: &Auth, path: &str) -> (u16, Value) {
    send(p, "GET", auth, &format!("{API}{path}"), None).await
}

/// `GET <API><path>`, or `None` unless it answers 200.
pub async fn get_ok<P: GitHub>(p: &P, auth: &Auth, path: &str) -> Option<Value> {
    match get(p, auth, path).await {
        (200, v) => Some(v),
        _ => None,
    }
}

pub async fn post<P: GitHub>(p: &P, auth: &Auth, url: &str, body: &Value) -> (u16, Value) {
    send(p, "POST", auth, url, Some(body.to_string())).await
}

pub async fn patch<P: GitHub>(p: &P, auth: &Auth, url: &str, body: &Value) -> (u16, Value) {
    send(p, "PATCH", auth, url, Some(body.to_string())).await
}

async fn send<P: GitHub>(
    p: &P,
    method: &'static str,
    auth: &Auth,
    url: &str,
    body: Option<String>,
) -> (u16, Value) {
    let res = p
        .send(GhRequest {
            method,
            url: url.into(),
            auth: auth.clone(),
            body,
        })
        .await;
    let value = serde_json::from_str(&res.body).unwrap_or(Value::Null);
    (res.status, value)
}

/// Runs a GraphQL query and returns its `data`, which may be partial: a repository the
/// token can't read comes back `null` beside an error.
pub async fn graphql<P: GitHub>(p: &P, auth: &Auth, query: &str, vars: Value) -> Option<Value> {
    let body = json!({"query": query, "variables": vars});
    match post(p, auth, &format!("{API}/graphql"), &body).await {
        (200, v) if !v["data"].is_null() => Some(v["data"].clone()),
        _ => None,
    }
}
