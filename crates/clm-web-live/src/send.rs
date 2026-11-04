//! `POST /send`: Web Push messages the Worker chose the recipients of. Sending runs in the
//! object, so its crypto never shares the webhook's CPU budget.

use std::collections::HashMap;

use futures::future::join_all;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::hub::{Answer, Host};
use crate::push::{Signer, Subscription, origin, signed_push};

/// The most messages one send takes.
const MAX_MESSAGES: usize = 10;

#[derive(Deserialize)]
struct Body {
    messages: Vec<Message>,
}

#[derive(Deserialize)]
struct Message {
    subscription: Subscription,
    payload: Value,
}

/// Sends each message, signing one VAPID JWT per endpoint origin, and answers the endpoints
/// whose push service answered 404 or 410 under `gone`. A message whose subscription
/// `web_push` can't use is skipped. Without the VAPID keys or randomness it sends nothing.
pub(crate) async fn send<H: Host>(host: &H, body: &str) -> Answer<H::Socket> {
    let Ok(Body { messages }) = serde_json::from_str(body) else {
        return Answer::Status(422);
    };
    if messages.len() > MAX_MESSAGES {
        return Answer::Status(422);
    }
    let Some(signer) = host.vapid().and_then(|v| Signer::new(&v).ok()) else {
        return Answer::Status(503);
    };
    let Some(randoms) = messages
        .iter()
        .map(|_| host.random())
        .collect::<Option<Vec<_>>>()
    else {
        return Answer::Status(503);
    };
    let now = host.now();
    let mut jwts: HashMap<String, String> = HashMap::new();
    let mut requests = Vec::new();
    for (message, random) in messages.into_iter().zip(randoms) {
        let Ok(origin) = origin(&message.subscription.endpoint) else {
            continue;
        };
        let authorization = jwts
            .entry(origin)
            .or_insert_with_key(|origin| signer.authorization(origin, now))
            .clone();
        let payload = message.payload.to_string();
        if let Ok(request) = signed_push(
            &message.subscription,
            payload.as_bytes(),
            authorization,
            random,
        ) {
            requests.push((message.subscription.endpoint, request));
        }
    }
    // Sent together, so a send takes about one push service's round trip.
    let statuses = join_all(
        requests
            .into_iter()
            .map(async |(endpoint, request)| (endpoint, host.fetch(request).await)),
    )
    .await;
    let gone: Vec<_> = statuses
        .into_iter()
        .filter(|(_, status)| matches!(status, Some(404 | 410)))
        .map(|(endpoint, _)| endpoint)
        .collect();
    Answer::Json(200, json!({ "gone": gone }).to_string())
}
