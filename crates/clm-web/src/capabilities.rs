//! A decision on a capability or an architectural constraint (`web.worker/decide`,
//! `web.worker/architecture-decide`): a `decided` record appended to the requester's web
//! ledger as approve appends. The body names the version the person saw, as approve names
//! the fingerprint, so a decision is never made on an entry reworded since.

use clm_ledger::{Body, Decision, Rating};
use serde::Deserialize;

use crate::replicas::Replicas;
use crate::views;
use clm_web_platform::access::Access;
use clm_web_platform::http::Response;
use clm_web_platform::platform::Platform;
use clm_web_platform::{files, ledger};
use clm_web_review::review;

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
    /// The ID prefix and the `clm` command that makes the same decision.
    fn names(self) -> (&'static str, &'static str) {
        match self {
            Self::Capability => ("capability", "clm capabilities decide"),
            Self::Architecture => ("architecture", "clm architecture decide"),
        }
    }
}

/// `POST .../capabilities/<slug>/decide` or `.../architecture/<slug>/decide` for `login`: 422
/// for a tip whose specs don't load or an entry it doesn't hold, then 422 `signed`, then 409
/// when its version isn't the one named; none writes (`web.worker/architecture-decide`).
pub async fn decide<P: Platform + Replicas>(
    p: &P,
    access: &Access,
    login: &str,
    layer: Layer,
    slug: &str,
    decide: Decide,
) -> Response {
    let (prefix, command) = layer.names();
    let repo = access.full_name();
    let Some(tip) = ledger::tip(p, access, access.default_branch()).await else {
        return Response::empty(502);
    };
    // The ledgers as they are now, not the minute-old fold, so the flags settled are those
    // open when the person decided.
    let (files, signed, sources) = futures::join!(
        files::spec_files(p, access.auth(), &repo, &tip, None),
        review::signed(p, access, &tip),
        ledger::read(p, access.auth(), &repo),
    );
    let (Some(files), Some(signed), Some(sources)) = (files, signed, sources) else {
        return Response::empty(502);
    };
    let (graph, violations) = clm_model::load_files(&files);
    let id = format!("{prefix}/{slug}");
    let Some(entry) = graph.entry(&id).filter(|_| violations.is_empty()) else {
        return Response::empty(422);
    };
    if signed {
        return review::signed_refusal(format!(
            "{command} {slug} --value {} --decision {}",
            decide.value.as_str(),
            decide.decision.as_str()
        ));
    }
    if decide.version != entry.version() {
        return Response::empty(409);
    }
    let settles = clm_effort::unsettled(&graph, &ledger::folded(&sources), &id);
    let record = ledger::record(
        p,
        login,
        Body::Decided {
            capability: id,
            value: decide.value,
            decision: decide.decision,
            settles,
        },
    );
    match ledger::append(p, access, &record).await {
        Some(()) => {
            views::ledger_moved(p, access, login).await;
            Response::json(&record)
        }
        None => Response::empty(502),
    }
}
