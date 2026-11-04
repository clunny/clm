//! A decision on a capability or an architectural constraint (`web.worker/decide`,
//! `web.worker/architecture-decide`): the body check of a decision, which the object appends
//! as a `decided` record to the requester's web ledger (`writes-to-object`). The body names
//! the version the person saw, as approve names the fingerprint, so a decision is never made
//! on an entry reworded since.

use clm_ledger::{Decision, Rating};
use serde::Deserialize;
use serde_json::json;

use crate::replicas::Replicas;
use crate::writes;
use clm_web_platform::access::Access;
use clm_web_platform::http::Response;

/// What a decide request names: the version seen, the value and the decision.
#[derive(Debug, Deserialize)]
pub struct Decide {
    version: String,
    value: Rating,
    decision: Decision,
}

/// A decide request's body, `None` when it isn't one: checked before any request to GitHub.
/// A version is 8 lowercase hex digits, as markers cite it.
pub fn parse(body: &[u8]) -> Option<Decide> {
    let decide: Decide = serde_json::from_slice(body).ok()?;
    let hex = decide.version.len() == 8
        && decide
            .version
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    hex.then_some(decide)
}

/// What a decision is on: a capability or an architecture requirement.
#[derive(Clone, Copy)]
pub enum Layer {
    Capability,
    Architecture,
}

impl Layer {
    /// The ID prefix of the entries of this layer.
    fn prefix(self) -> &'static str {
        match self {
            Self::Capability => "capability",
            Self::Architecture => "architecture",
        }
    }
}

/// `POST .../capabilities/<slug>/decide` or `.../architecture/<slug>/decide` for `login`: the
/// checked body, passed to the object, which refuses a tip whose specs don't load, an entry
/// it doesn't hold, a signed repository and a stale version, and appends the record
/// (`web.worker/architecture-decide`).
pub async fn decide<P: Replicas>(
    p: &P,
    access: &Access,
    login: &str,
    layer: Layer,
    slug: &str,
    decide: Decide,
) -> Response {
    let write = json!({
        "kind": "decide",
        "id": format!("{}/{slug}", layer.prefix()),
        "version": decide.version,
        "value": decide.value.as_str(),
        "decision": decide.decision.as_str(),
    });
    writes::through_object(p, access, login, write).await
}
