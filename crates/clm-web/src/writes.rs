//! The writes that need the replica to validate them (`web.worker/writes-to-object`): a
//! proposal, a withdrawal and a decision. The Worker checks the body and the access, makes
//! one post to the repo's `Replica` object and answers what the object answers.

use serde_json::{Value, json};

use crate::replicas::Replicas;
use clm_web_platform::access::Access;
use clm_web_platform::http::Response;
use clm_web_platform::platform::Auth;

/// Posts `write`'s fields, with the requester's `login` and token, to the repo's object
/// and answers its status and body as it gave them; 503 `{"error": "unavailable"}` when the
/// object can't be reached.
pub(crate) async fn through_object<P: Replicas>(
    p: &P,
    access: &Access,
    login: &str,
    mut write: Value,
) -> Response {
    let Auth::User(token) = access.auth() else {
        return Response::empty(404);
    };
    write["login"] = json!(login);
    write["token"] = json!(token);
    let sent = p
        .replica(
            &access.full_name(),
            "POST",
            "/write",
            Some(&write.to_string()),
        )
        .await;
    match sent {
        Some((status, body)) if body.is_empty() => Response::empty(status),
        Some((status, body)) => {
            let mut res =
                Response::empty(status).with_header("content-type", "application/json".into());
            res.body = body.into_bytes();
            res
        }
        None => {
            let mut res = Response::json(&json!({ "error": "unavailable" }));
            res.status = 503;
            res
        }
    }
}
