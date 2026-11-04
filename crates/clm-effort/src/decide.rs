//! `clm capabilities decide` and `clm architecture decide` (`effort/decide-command`,
//! `effort/architecture-decide-command`).

use std::path::Path;

use clm_ledger::{Body, Decision, Rating};

use crate::history::graph;
use crate::{Error, Recorded, unsettled};

/// Appends a `decided` record for `capability/<slug>`, settling the flags open and unsettled
/// on it now. Refuses a slug `CAPABILITIES.md` doesn't hold.
pub fn decide(
    root: &Path,
    slug: &str,
    value: Rating,
    decision: Decision,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Recorded, Error> {
    let missing = Error::NoCapability(slug.to_string());
    decided(root, "capability", slug, value, decision, env, missing)
}

/// Appends a `decided` record for `architecture/<slug>`, settling the flags open and
/// unsettled on it now. Refuses a slug `ARCHITECTURE.md` doesn't hold.
pub fn decide_architecture(
    root: &Path,
    slug: &str,
    value: Rating,
    decision: Decision,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Recorded, Error> {
    let missing = Error::NoArchitecture(slug.to_string());
    decided(root, "architecture", slug, value, decision, env, missing)
}

fn decided(
    root: &Path,
    layer: &str,
    slug: &str,
    value: Rating,
    decision: Decision,
    env: &dyn Fn(&str) -> Option<String>,
    missing: Error,
) -> Result<Recorded, Error> {
    let graph = graph(root)?;
    let capability = format!("{layer}/{slug}");
    if graph.entry(&capability).is_none() {
        return Err(missing);
    }
    let fold = clm_ledger::fold(root)?;
    let settles = unsettled(&graph, &fold, &capability);
    let line = format!(
        "decided {capability} {} (value {}), settles {} flags",
        decision.as_str(),
        value.as_str(),
        settles.len()
    );
    clm_ledger::append(
        root,
        Body::Decided {
            capability,
            value,
            decision,
            settles,
        },
        env,
    )?;
    Ok(Recorded {
        lines: vec![line],
        warnings: Vec::new(),
    })
}
