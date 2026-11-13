//! Answers to backlog items (`web-answers-as-ledger-records`): the body check of an answer,
//! which the object appends as an `answered` record in the requester's web ledger
//! (`writes-to-object`), and the open answers the implementation response lists.

use serde_json::{Value, json};

use crate::replicas::Replicas;
use crate::writes;
use clm_web_platform::access::Access;
use clm_web_platform::http::Response;

/// An answer request's `/answer`, or 422 `{"error": "answer"}` when the body has none that
/// is a string: checked before any request to GitHub. Whether the item offers it is the
/// object's to say.
pub(crate) fn parse(body: &[u8]) -> Result<String, Response> {
    serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|v| v["answer"].as_str().map(String::from))
        .ok_or_else(|| {
            let mut res = Response::json(&json!({ "error": "answer" }));
            res.status = 422;
            res
        })
}

/// `POST /api/repos/{o}/{r}/backlog/{slug}/answer`: the checked answer, passed to the
/// object with the item's slug, percent-decoded by the router.
pub(crate) async fn answer<P: Replicas>(
    p: &P,
    access: &Access,
    login: &str,
    item: &str,
    answer: String,
) -> Response {
    let write = json!({ "kind": "answer", "item": item, "answer": answer });
    writes::through_object(p, access, login, write).await
}
