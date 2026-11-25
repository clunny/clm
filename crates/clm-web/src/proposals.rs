//! Proposals for the backlog (`web-proposals-as-ledger-records`): the body check of a
//! proposal, which the object appends as a `proposed` record in the requester's web ledger
//! (`writes-to-object`), its withdrawal, and the open proposals the implementation response
//! lists.

use serde_json::{Value, json};

use crate::replicas::Replicas;
use crate::writes;
use clm_web_platform::access::Access;
use clm_web_platform::http::Response;
use clm_web_platform::ledger;

/// A title's characters at most, once trimmed.
const TITLE_MAX: usize = 120;
/// A body's characters at most.
const BODY_MAX: usize = 8_000;
/// The nodes a proposal names at most.
const NODES_MAX: usize = 10;

/// A proposal's body, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Proposal {
    title: String,
    body: String,
    nodes: Vec<String>,
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
    // The slug is the object's (`replica/validate-writes`); it is empty only for a title
    // that lowercases to no ASCII letter or digit (lowercased first: the Kelvin sign).
    if !title
        .to_lowercase()
        .chars()
        .any(|c| c.is_ascii_alphanumeric())
    {
        return Err(refused("slug"));
    }
    Ok(Proposal {
        title: title.into(),
        body: text.into(),
        nodes,
    })
}

/// `POST /api/repos/{o}/{r}/proposals`: the checked proposal, passed to the object.
pub(crate) async fn propose<P: Replicas>(
    p: &P,
    access: &Access,
    login: &str,
    proposal: Proposal,
) -> Response {
    let write = json!({
        "kind": "propose",
        "title": proposal.title,
        "body": proposal.body,
        "nodes": proposal.nodes,
    });
    writes::through_object(p, access, login, write).await
}

/// `DELETE /api/repos/{o}/{r}/proposals/{id}`: passed to the object.
pub(crate) async fn withdraw<P: Replicas>(
    p: &P,
    access: &Access,
    login: &str,
    id: &str,
) -> Response {
    writes::through_object(p, access, login, json!({"kind": "withdraw", "id": id})).await
}

/// The implementation view with `mine` added to each entry of its `list` (`proposals` or
/// `answers`): whether the writer is `login`'s `<login>/web`. A view that carries none
/// answers `[]`.
pub(crate) fn with_mine(mut view: Value, list: &str, login: &str) -> Value {
    if !view.is_object() {
        return view;
    }
    let writer = ledger::web_writer(login);
    let mut listed = view[list].take();
    for entry in listed.as_array_mut().into_iter().flatten() {
        entry["mine"] = json!(entry["writer"].as_str() == Some(writer.as_str()));
    }
    view[list] = if listed.is_array() { listed } else { json!([]) };
    view
}
