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
/// A slug's bytes at most, as `clm defect` cuts one.
const SLUG_MAX: usize = 64;

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
    if slug(title).is_empty() {
        return Err(refused("slug"));
    }
    Ok(Proposal {
        title: title.into(),
        body: text.into(),
        nodes,
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

#[cfg(test)]
mod tests {
    use super::slug;

    #[test]
    fn slugs_as_clm_defect_does() {
        // req:web.worker/proposals@e608c36e
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
