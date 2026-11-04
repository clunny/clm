//! Proposals for the backlog (`web-proposals-as-ledger-records`): a `proposed` record in
//! the requester's web ledger, which `clm sync` files as `backlog/<slug>.md` on the next
//! build, its withdrawal, and the open proposals the implementation response lists.

use std::collections::BTreeSet;

use clm_ledger::Body;
use serde_json::{Value, json};

use crate::replicas::Replicas;
use crate::views;
use clm_web_platform::access::Access;
use clm_web_platform::http::Response;
use clm_web_platform::ledger;
use clm_web_platform::platform::Platform;

/// A title's characters at most, once trimmed.
const TITLE_MAX: usize = 120;
/// A body's characters at most.
const BODY_MAX: usize = 8_000;
/// The nodes a proposal names at most.
const NODES_MAX: usize = 10;
/// A slug's bytes at most, as `clm defect` cuts one.
const SLUG_MAX: usize = 64;

/// A proposal's body, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Proposal {
    title: String,
    body: String,
    nodes: Vec<String>,
    slug: String,
}

/// 422 with `{"error": <field>}`.
fn refused(field: &str) -> Response {
    let mut res = Response::json(&json!({ "error": field }));
    res.status = 422;
    res
}

/// The request's body as a proposal, or the 422 naming the field it fails on: checked
/// before any request to GitHub, except whether the graph holds the nodes.
pub(crate) fn parse(body: &[u8]) -> Result<Proposal, Response> {
    let v: Value = serde_json::from_slice(body).map_err(|_| refused("title"))?;
    let title = v["title"]
        .as_str()
        .map(str::trim)
        .ok_or_else(|| refused("title"))?;
    let chars = title.chars().count();
    if chars == 0 || chars > TITLE_MAX || title.contains(['\n', '\r']) {
        return Err(refused("title"));
    }
    let text = v["body"]
        .as_str()
        .map(str::trim)
        .ok_or_else(|| refused("body"))?;
    if text.chars().count() > BODY_MAX {
        return Err(refused("body"));
    }
    let listed = v["nodes"].as_array().ok_or_else(|| refused("nodes"))?;
    if listed.len() > NODES_MAX {
        return Err(refused("nodes"));
    }
    let mut nodes = Vec::new();
    for node in listed {
        let node = node.as_str().ok_or_else(|| refused("nodes"))?;
        if !nodes.iter().any(|n| n == node) {
            nodes.push(node.to_string());
        }
    }
    let slug = slug(title);
    if slug.is_empty() {
        return Err(refused("slug"));
    }
    Ok(Proposal {
        title: title.into(),
        body: text.into(),
        nodes,
        slug,
    })
}

/// The title in kebab case as `clm defect` slugs one (`plugin.hooks/defect-backlog`):
/// lowercased, split on anything but ASCII letters and digits, the words joined with `-`
/// while the result stays within [`SLUG_MAX`] bytes, a first word longer than that cut to
/// it; empty when the title has no such word.
fn slug(title: &str) -> String {
    let mut slug = String::new();
    let lower = title.to_lowercase();
    for word in lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
    {
        let sep = usize::from(!slug.is_empty());
        if slug.len() + sep + word.len() > SLUG_MAX {
            if slug.is_empty() {
                slug.push_str(&word[..SLUG_MAX]);
            }
            break;
        }
        if sep == 1 {
            slug.push('-');
        }
        slug.push_str(word);
    }
    slug
}

/// `POST /api/repos/{o}/{r}/proposals`: 422 for a node the snapshot's graph doesn't hold,
/// 409 when the slug is taken: a backlog item's, one a change resolves, or an open
/// proposal's the implementation response lists; else a `proposed` record in the
/// requester's web ledger and `{"id"}`.
pub(crate) async fn propose<P: Platform + Replicas>(
    p: &P,
    access: &Access,
    login: &str,
    proposal: Proposal,
) -> Response {
    let (snapshot, implementation) = futures::join!(
        views::fetch(p, access, "snapshot", false),
        views::fetch(p, access, "implementation", false)
    );
    let snap = match snapshot {
        Ok(view) => Facts::of(&view),
        Err(res) => return res,
    };
    let listed = match implementation {
        Ok(view) => view,
        Err(res) => return res,
    };
    if proposal
        .nodes
        .iter()
        .any(|n| !snap.components.contains(n.as_str()))
    {
        return refused("nodes");
    }
    // An open proposal is one the implementation view lists; one that was filed or
    // resolved takes its slug only through the snapshot.
    let taken = snap.filed.contains(proposal.slug.as_str())
        || listed["proposals"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|p| p["slug"].as_str() == Some(proposal.slug.as_str()));
    if taken {
        let mut res = Response::json(&json!({ "error": "taken" }));
        res.status = 409;
        return res;
    }
    let body = Body::Proposed {
        slug: proposal.slug,
        title: proposal.title,
        body: proposal.body,
        nodes: proposal.nodes,
    };
    let record = ledger::record(p, login, body);
    match ledger::append(p, access, &record).await {
        Some(()) => {
            views::ledger_moved(p, access, login).await;
            Response::json(&json!({ "id": record.id }))
        }
        None => Response::empty(502),
    }
}

/// `DELETE /api/repos/{o}/{r}/proposals/{id}`: a `withdrawn` record when the requester's
/// own web ledger, read at its tip, holds an open proposal `id`; 404 for any other.
pub(crate) async fn withdraw<P: Platform + Replicas>(
    p: &P,
    access: &Access,
    login: &str,
    id: &str,
) -> Response {
    let Some(own) = ledger::own(p, access, login).await else {
        return Response::empty(502);
    };
    if !own.proposals.contains_key(id) {
        return Response::empty(404);
    }
    let body = Body::Withdrawn {
        proposal: id.into(),
    };
    match ledger::append(p, access, &ledger::record(p, login, body)).await {
        Some(()) => {
            views::ledger_moved(p, access, login).await;
            Response::empty(200)
        }
        None => Response::empty(502),
    }
}

/// What the snapshot view says that proposals are judged by.
pub(crate) struct Facts {
    /// The graph's component IDs.
    components: BTreeSet<String>,
    /// The slugs a backlog item has or its `resolved` holds: spent, since `clm sync` files
    /// none of them.
    filed: BTreeSet<String>,
}

impl Facts {
    pub(crate) fn of(snapshot: &Value) -> Facts {
        let strings = |list: &Value, key: Option<&str>| -> Vec<String> {
            list.as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| match key {
                    Some(key) => v[key].as_str(),
                    None => v.as_str(),
                })
                .map(String::from)
                .collect()
        };
        let backlog = strings(&snapshot["backlog"], Some("slug"));
        let resolved = strings(&snapshot["resolved"], None);
        Facts {
            components: strings(&snapshot["nodes"], Some("id"))
                .into_iter()
                .collect(),
            filed: backlog.into_iter().chain(resolved).collect(),
        }
    }
}

/// The implementation view with `mine` added to each of its `proposals`: whether the writer
/// is `login`'s `<login>/web`. A view that carries none answers `[]`.
pub(crate) fn with_mine(mut view: Value, login: &str) -> Value {
    if !view.is_object() {
        return view;
    }
    let writer = ledger::web_writer(login);
    let mut listed = view["proposals"].take();
    for proposal in listed.as_array_mut().into_iter().flatten() {
        proposal["mine"] = json!(proposal["writer"].as_str() == Some(writer.as_str()));
    }
    view["proposals"] = if listed.is_array() { listed } else { json!([]) };
    view
}

#[cfg(test)]
mod tests {
    use super::slug;

    #[test]
    fn slugs_as_clm_defect_does() {
        // req:web.worker/proposals@ec27f8e2
        assert_eq!(slug("Export CSV"), "export-csv");
        assert_eq!(slug("  Dark mode: on/off!  "), "dark-mode-on-off");
        assert_eq!(slug("Ünïcode only"), "n-code-only");
        assert_eq!(slug("!!!"), "");
        let long = "a".repeat(70);
        assert_eq!(slug(&long), "a".repeat(64));
        let words = format!("{} bb", "a".repeat(62));
        assert_eq!(
            slug(&words),
            "a".repeat(62),
            "a word that doesn't fit is left out"
        );
    }
}
