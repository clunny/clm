//! Comments and change requests on a spec branch's diff
//! (`web-review-feedback-as-ledger-records`): `commented` and `changes_requested` records in
//! the requester's web ledger, and the entries a branch's review lists for them.

use clm_ledger::{Body, Fold};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::approve::{self, Refusal, Spec};
use crate::review;
use clm_web_platform::access::{self, Access};
use clm_web_platform::http::Response;
use clm_web_platform::platform::Platform;
use clm_web_platform::{install, ledger};
use std::collections::BTreeSet;

/// A text's characters at most, once trimmed.
const TEXT_MAX: usize = 4_000;

/// A comment's body, checked.
pub struct Comment {
    fingerprint: String,
    target: Option<String>,
    text: String,
}

/// A change request's body, checked.
pub struct Request {
    fingerprint: String,
    text: String,
}

/// `status` with `{"error": <field>}`.
fn refused(status: u16, field: &str) -> Response {
    let mut res = Response::json(&json!({ "error": field }));
    res.status = status;
    res
}

/// The body's JSON, its non-empty `fingerprint`, and its `text` trimmed, `None` when that
/// is missing, empty or over [`TEXT_MAX`] characters.
fn fields(body: &[u8]) -> Result<(Value, String, Option<String>), Response> {
    let v: Value = serde_json::from_slice(body).map_err(|_| refused(422, "fingerprint"))?;
    let fingerprint = v["fingerprint"]
        .as_str()
        .filter(|f| !f.is_empty())
        .ok_or_else(|| refused(422, "fingerprint"))?
        .to_string();
    let text = v["text"]
        .as_str()
        .map(str::trim)
        .filter(|t| (1..=TEXT_MAX).contains(&t.chars().count()))
        .map(String::from);
    Ok((v, fingerprint, text))
}

/// A comment's body, or the 422 naming the field it fails on, before any request to GitHub.
pub fn parse_comment(body: &[u8]) -> Result<Comment, Response> {
    let (v, fingerprint, text) = fields(body)?;
    let target = match &v["target"] {
        Value::Null => None,
        Value::String(t) => Some(t.clone()),
        _ => return Err(refused(422, "target")),
    };
    let text = text.ok_or_else(|| refused(422, "text"))?;
    Ok(Comment {
        fingerprint,
        target,
        text,
    })
}

/// A change request's body, or the 422 naming the field it fails on.
pub fn parse_request(body: &[u8]) -> Result<Request, Response> {
    let (_, fingerprint, text) = fields(body)?;
    let text = text.ok_or_else(|| refused(422, "text"))?;
    Ok(Request { fingerprint, text })
}

/// The branch as a spec branch, or the answer refusing it.
async fn spec<P: Platform>(p: &P, access: &Access, branch: &str) -> Result<Spec, Response> {
    approve::spec_branch(p, access, branch)
        .await
        .map_err(|refusal| match refusal {
            Refusal::Branch => refused(422, "branch"),
            Refusal::Failed => Response::empty(502),
        })
}

/// Appends `body` as `login`'s record and answers `{"id"}`.
async fn append<P: Platform>(p: &P, access: &Access, login: &str, body: Body) -> Response {
    let record = ledger::record(p, login, body);
    match ledger::append(p, access, &record).await {
        Some(()) => Response::json(&json!({ "id": record.id })),
        None => Response::empty(502),
    }
}

/// `POST .../branches/{branch}/comments`: a `commented` record, whatever the branch's
/// fingerprint now, for a target that is null or a requirement the branch's review lists
/// or the snapshot's graph holds: `graph` is the snapshot's requirement IDs, awaited only
/// for a comment with a target, `None` when the snapshot can't be read.
pub async fn comment<P: Platform>(
    p: &P,
    access: &Access,
    login: &str,
    branch: &str,
    comment: Comment,
    graph: impl Future<Output = Option<BTreeSet<String>>>,
) -> Response {
    let spec = match spec(p, access, branch).await {
        Ok(spec) => spec,
        Err(res) => return res,
    };
    if let Some(target) = &comment.target {
        // The graph's requirements first; the branch's review only for one it doesn't hold.
        let Some(graph) = graph.await else {
            return Response::empty(502);
        };
        let held = graph.contains(target);
        if !held {
            let comparison = review::compare(p, access, &spec.base, &spec.trees.head_sha);
            let Some(comparison) = comparison.await else {
                return Response::empty(502);
            };
            let reviewed = comparison.nodes.iter().any(|n| {
                let r = &n.requirements;
                r.added
                    .iter()
                    .chain(&r.modified)
                    .chain(&r.removed)
                    .any(|e| e.id.as_ref() == Some(target))
            });
            if !reviewed {
                return refused(422, "target");
            }
        }
    }
    let body = Body::Commented {
        branch: branch.into(),
        fingerprint: comment.fingerprint,
        target: comment.target,
        text: comment.text,
    };
    append(p, access, login, body).await
}

/// `POST .../branches/{branch}/request-changes`: a `changes_requested` record, checked as
/// Approve is but for a signed repository, which it accepts, for a requester GitHub
/// answers `write` or `admin` for, naming the branch's fingerprint now.
pub async fn request_changes<P: Platform>(
    p: &P,
    access: &Access,
    login: &str,
    branch: &str,
    request: Request,
) -> Response {
    let spec = match spec(p, access, branch).await {
        Ok(spec) => spec,
        Err(res) => return res,
    };
    // A truncated listing gives no fingerprint, so nothing can pin the request.
    let Some(fingerprint) = spec.trees.fingerprint() else {
        return refused(422, "fingerprint");
    };
    // A hold on a land is for those who could land: asked with the installation's token,
    // as the webhook asks for push recipients.
    let repo = access.full_name();
    let auth = match install::installation_id(p, &repo).await {
        Some(id) => install::token(p, id).await,
        None => None,
    };
    let Some(auth) = auth else {
        return Response::empty(502);
    };
    match access::permission(p, &auth, &repo, login).await.as_deref() {
        None => return Response::empty(502),
        Some("write" | "admin") => {}
        Some(_) => return refused(403, "permission"),
    }
    if request.fingerprint != fingerprint {
        return refused(409, "moved");
    }
    let body = Body::ChangesRequested {
        branch: branch.into(),
        fingerprint,
        text: request.text,
    };
    append(p, access, login, body).await
}

/// A `commented` record, as a branch's review lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commented {
    id: String,
    writer: String,
    at: String,
    fingerprint: String,
    /// Null for the change as a whole.
    target: Option<String>,
    text: String,
    /// Its fingerprint is the review's.
    current: bool,
}

/// A `changes_requested` record, as a branch's review lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Requested {
    id: String,
    writer: String,
    at: String,
    fingerprint: String,
    text: String,
    current: bool,
    /// Current, and not cleared as `Fold::cleared` decides, the rule `clm land` holds the
    /// branch by (`ledger/request-cleared`).
    held: bool,
}

/// The fold's comments on `branch`, oldest first; `current` against `fingerprint`, the
/// review's, false on every entry without one.
pub fn comments(fold: &Fold, branch: &str, fingerprint: Option<&str>) -> Vec<Commented> {
    fold.comments
        .get(branch)
        .into_iter()
        .flatten()
        .filter_map(|r| match &r.body {
            Body::Commented {
                fingerprint: f,
                target,
                text,
                ..
            } => Some(Commented {
                id: r.id.clone(),
                writer: r.writer.clone(),
                at: r.at.clone(),
                fingerprint: f.clone(),
                target: target.clone(),
                text: text.clone(),
                current: fingerprint == Some(f.as_str()),
            }),
            _ => None,
        })
        .collect()
}

/// The fold's change requests on `branch`, oldest first, with `held`.
pub fn change_requests(fold: &Fold, branch: &str, fingerprint: Option<&str>) -> Vec<Requested> {
    fold.change_requests
        .get(branch)
        .into_iter()
        .flatten()
        .filter_map(|r| match &r.body {
            Body::ChangesRequested {
                fingerprint: f,
                text,
                ..
            } => {
                let current = fingerprint == Some(f.as_str());
                Some(Requested {
                    id: r.id.clone(),
                    writer: r.writer.clone(),
                    at: r.at.clone(),
                    fingerprint: f.clone(),
                    text: text.clone(),
                    current,
                    held: current && !fold.cleared(r),
                })
            }
            _ => None,
        })
        .collect()
}
