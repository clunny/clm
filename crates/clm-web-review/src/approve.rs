//! `POST /api/repos/{o}/{r}/branches/{branch}/approve`: appends an `approved` record with
//! the branch's fingerprint to the requester's web ledger, `clm/ledger/<login>/web`,
//! through the Git Data API with their own token, and writes nothing else. The request
//! names the fingerprint of the review the person was shown, so an approval only ever
//! names a diff they saw (`approve-pins-the-review`).

use clm_ledger::Body;
use serde_json::Value;

use crate::review;
use clm_web_platform::access::Access;
use clm_web_platform::http::Response;
use clm_web_platform::platform::Platform;
use clm_web_platform::{files, keys, ledger};

/// The fingerprint an approve request names as reviewed: the body's string
/// `fingerprint`, `None` when the body is missing, isn't JSON or has none.
pub fn fingerprint(body: &[u8]) -> Option<String> {
    let v: Value = serde_json::from_slice(body).ok()?;
    v.get("fingerprint")?.as_str().map(String::from)
}

/// Approves `branch` for `login` when its fingerprint is still `pin`: 422 for a branch
/// that can't be approved at all, checked first, then 409 when it moved since the review
/// was drawn. Neither writes anything.
pub async fn branch<P: Platform>(
    p: &P,
    access: &Access,
    login: &str,
    branch: &str,
    pin: &str,
) -> Response {
    let refuse = || Response::empty(422);
    let Spec {
        base,
        trees,
        changed,
    } = match spec_branch(p, access, branch).await {
        Ok(spec) => spec,
        Err(Refusal::Branch) => return refuse(),
        Err(Refusal::Failed) => return Response::empty(502),
    };
    // A signed repository counts only approvals signed with the reviewer's SSH key, which
    // the Worker doesn't hold (`signed-repository`).
    match review::signed(p, access, &base).await {
        None => return Response::empty(502),
        Some(true) => return review::signed_refusal(format!("clm approve {branch}")),
        Some(false) => {}
    }
    // A truncated listing gives no fingerprint, so nothing can pin the approval.
    let Some(fingerprint) = trees.fingerprint() else {
        return refuse();
    };
    if pin != fingerprint {
        return Response::empty(409);
    }
    let record = ledger::record(
        p,
        login,
        Body::Approved {
            fingerprint,
            branch: branch.into(),
            change: change_of(&changed, &trees.head),
        },
    );
    match ledger::append(p, access, &record).await {
        Some(()) => Response::empty(200),
        None => Response::empty(502),
    }
}

/// A spec branch a reviewer can approve, comment on or request changes of.
pub(crate) struct Spec {
    /// The default branch's tip.
    pub(crate) base: String,
    pub(crate) trees: review::Trees,
    /// The paths the branch changes.
    pub(crate) changed: Vec<String>,
}

/// Why a branch isn't a [`Spec`].
pub(crate) enum Refusal {
    /// It isn't `spec/*`, doesn't exist, or neither touches a spec path nor changes the
    /// approval setting.
    Branch,
    /// GitHub didn't answer.
    Failed,
}

/// `branch` as a spec branch, checked as Approve checks it.
pub(crate) async fn spec_branch<P: Platform>(
    p: &P,
    access: &Access,
    branch: &str,
) -> Result<Spec, Refusal> {
    if !review::is_spec_branch(branch) {
        return Err(Refusal::Branch);
    }
    let tip = ledger::tip(p, access, branch)
        .await
        .ok_or(Refusal::Branch)?;
    let base = ledger::tip(p, access, access.default_branch())
        .await
        .ok_or(Refusal::Failed)?;
    let trees = review::trees(p, access, &base, &tip)
        .await
        .ok_or(Refusal::Failed)?;
    let changed = trees.head.changed_paths(&trees.base);
    if !changed.iter().any(|path| clm_gate::is_spec_path(path)) {
        match changes_setting(p, access, &trees).await {
            None => return Err(Refusal::Failed),
            Some(false) => return Err(Refusal::Branch),
            Some(true) => {}
        }
    }
    Ok(Spec {
        base,
        trees,
        changed,
    })
}

/// Whether the branch changes `clm.toml`'s approval setting (`approval-setting-branch`),
/// fetching the file's two texts in one query only when the listings differ on it; `None`
/// when GitHub doesn't answer.
async fn changes_setting<P: Platform>(
    p: &P,
    access: &Access,
    trees: &review::Trees,
) -> Option<bool> {
    if !trees.policy_differs() {
        return Some(false);
    }
    let sides = [
        (&trees.merge_base, &trees.base),
        (&trees.head_sha, &trees.head),
    ];
    let expressions: Vec<String> = sides
        .iter()
        .filter(|(_, listing)| listing.has(review::POLICY))
        .map(|(sha, _)| format!("{sha}:{}", review::POLICY))
        .collect();
    let repo = access.full_name();
    let mut texts = files::objects(p, access.auth(), &repo, "Blobs", &expressions)
        .await?
        .into_iter();
    let [before, after] =
        sides.map(|(_, listing)| listing.has(review::POLICY).then(|| texts.next().flatten()));
    Some(review::setting_changed(bytes(&before), bytes(&after)))
}

fn bytes(side: &Option<Option<String>>) -> Option<Option<&[u8]>> {
    side.as_ref().map(|t| t.as_deref().map(str::as_bytes))
}

/// The change a branch adds or edits, as `clm approve` names it: the first
/// `changes/<id>/change.md` the head has and the merge base lacks or differs on.
fn change_of(changed: &[String], head: &keys::Listing) -> Option<String> {
    let kept = |path: &String| head.entries.iter().any(|e| e.path == path.as_bytes());
    changed.iter().filter(|path| kept(path)).find_map(|path| {
        let id = path.strip_prefix("changes/")?.strip_suffix("/change.md")?;
        (id != "archive" && !id.contains('/')).then(|| id.to_string())
    })
}
