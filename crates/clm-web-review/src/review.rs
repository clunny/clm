//! A spec branch's review: the spec files at the merge base and at the head, loaded with
//! the model and compared node by node, element by element, by content hash, with the
//! branch's fingerprint and its head's code key. The comparison is kept in the Cache API
//! under the default branch's tip and the branch's head, so while they stand a request
//! fetches no spec files (`web.worker/review-cached`); the ledger's approvals and checks
//! and the page are live.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use clm_model::{Files, Graph, Node, outline, split_frontmatter};
use serde::{Deserialize, Serialize};

use crate::feedback;
use clm_web_platform::access::Access;
use clm_web_platform::http::{Response, percent_encode};
use clm_web_platform::platform::Platform;
use clm_web_platform::{files, gh, keys, ledger, live};

/// An element's text before and after; `None` on the side where it doesn't exist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Texts {
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Element {
    /// A requirement's ID, or an asset's path under the node's `assets/`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(flatten)]
    pub texts: Texts,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Changes {
    pub added: Vec<Element>,
    pub modified: Vec<Element>,
    pub removed: Vec<Element>,
}

impl Changes {
    fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.removed.is_empty()
    }

    fn push(&mut self, element: Element) {
        match (&element.texts.before, &element.texts.after) {
            (None, _) => self.added.push(element),
            (_, None) => self.removed.push(element),
            _ => self.modified.push(element),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeChanges {
    pub id: String,
    /// `added`, `removed` or `modified`.
    pub status: String,
    pub frontmatter: Option<Texts>,
    pub contract: Option<Texts>,
    pub design: Option<Texts>,
    pub requirements: Changes,
    pub assets: Changes,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeInfo {
    pub id: String,
    pub status: String,
    pub from: Option<String>,
    pub resolves: Vec<String>,
    /// The whole body of `change.md` after its frontmatter.
    pub intent: String,
    /// The `## Intent` section's body.
    pub intent_section: Option<String>,
}

/// A decision or backlog item added, modified or removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// A decision's name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// A backlog item's slug.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    pub status: String,
    #[serde(flatten)]
    pub texts: Texts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub path: String,
    #[serde(flatten)]
    pub texts: Texts,
}

fn text(files: &Files, path: &Path) -> Option<String> {
    files
        .get(path)
        .and_then(|b| std::str::from_utf8(b).ok())
        .map(String::from)
}

fn texts_if_changed(before: Option<String>, after: Option<String>) -> Option<Texts> {
    (before != after).then_some(Texts { before, after })
}

fn frontmatter(node: &Node) -> String {
    split_frontmatter(&node.source)
        .map(|(fm, _)| fm.to_string())
        .unwrap_or_default()
}

/// The repo-relative directory holding a node's `assets/`.
fn assets_dir(node: &Node) -> PathBuf {
    node.path.parent().unwrap_or(Path::new("")).join("assets")
}

fn node_changes(
    id: &str,
    base: Option<&Node>,
    head: Option<&Node>,
    base_files: &Files,
    head_files: &Files,
) -> Option<NodeChanges> {
    let changed_by_hash = |pick: fn(&Node) -> &str, which: fn(&Node) -> String| {
        let same = matches!((base, head), (Some(b), Some(h)) if pick(b) == pick(h));
        if same {
            None
        } else {
            texts_if_changed(base.map(which), head.map(which))
        }
    };
    let frontmatter = changed_by_hash(|n| &n.hashes.frontmatter, frontmatter);
    let contract = changed_by_hash(|n| &n.hashes.contract, |n| n.contract.clone());
    let design = changed_by_hash(|n| &n.hashes.design, |n| n.design.clone());

    let mut requirements = Changes::default();
    let reqs = |n: Option<&Node>| -> BTreeMap<String, (String, String)> {
        n.into_iter()
            .flat_map(|n| &n.requirements)
            .map(|r| (r.id.clone(), (r.hash.clone(), r.text.clone())))
            .collect()
    };
    let (before, after) = (reqs(base), reqs(head));
    let ids: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    for rid in ids {
        let (b, a) = (before.get(rid), after.get(rid));
        if b.map(|x| &x.0) != a.map(|x| &x.0) {
            requirements.push(Element {
                id: Some(rid.clone()),
                path: None,
                texts: Texts {
                    before: b.map(|x| x.1.clone()),
                    after: a.map(|x| x.1.clone()),
                },
            });
        }
    }

    let mut assets = Changes::default();
    let empty = BTreeMap::new();
    let (ba, ha) = (
        base.map_or(&empty, |n| &n.assets),
        head.map_or(&empty, |n| &n.assets),
    );
    let paths: BTreeSet<&String> = ba.keys().chain(ha.keys()).collect();
    for path in paths {
        if ba.get(path) != ha.get(path) {
            let read = |n: Option<&Node>, files: &Files| {
                n.filter(|n| n.assets.contains_key(path))
                    .and_then(|n| text(files, &assets_dir(n).join(path)))
            };
            let in_before = ba.contains_key(path);
            let in_after = ha.contains_key(path);
            // A binary asset has no text, but still exists on its side.
            let side = |exists: bool, t: Option<String>| exists.then(|| t.unwrap_or_default());
            assets.push(Element {
                id: None,
                path: Some(path.clone()),
                texts: Texts {
                    before: side(in_before, read(base, base_files)),
                    after: side(in_after, read(head, head_files)),
                },
            });
        }
    }

    let untouched = frontmatter.is_none()
        && contract.is_none()
        && design.is_none()
        && requirements.is_empty()
        && assets.is_empty();
    if untouched {
        return None;
    }
    Some(NodeChanges {
        id: id.into(),
        status: match (base, head) {
            (None, _) => "added",
            (_, None) => "removed",
            _ => "modified",
        }
        .into(),
        frontmatter,
        contract,
        design,
        requirements,
        assets,
    })
}

/// The body of `## <name>` in `body`, trimmed, or `None` when it's missing or empty.
fn section(body: &str, name: &str) -> Option<String> {
    let lines: Vec<&str> = body.lines().collect();
    let span = outline::section_spans(&lines)
        .into_iter()
        .find(|s| s.name == name)?;
    let text = lines[span.heading + 1..span.end].join("\n");
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn change_info(head: &Graph, changed: &BTreeSet<PathBuf>) -> Option<ChangeInfo> {
    let lower = |d: &dyn std::fmt::Debug| format!("{d:?}").to_lowercase();
    let change = head.changes.values().find(|c| {
        changed
            .iter()
            .any(|p| p.starts_with(c.path.parent().unwrap_or(Path::new("-"))))
    })?;
    let source = change.source.replace("\r\n", "\n");
    let body = split_frontmatter(&source).map_or(source.as_str(), |(_, b)| b);
    Some(ChangeInfo {
        id: change.id.clone(),
        status: lower(&change.frontmatter.status),
        from: change.frontmatter.from.clone(),
        resolves: change.frontmatter.resolves.clone(),
        intent: body.trim().to_string(),
        intent_section: section(body, "Intent"),
    })
}

/// Records for the `*.md` files directly in `dir` that differ.
fn records(
    base: &Files,
    head: &Files,
    changed: &BTreeSet<PathBuf>,
    dir: &str,
    slug: bool,
) -> Vec<Record> {
    changed
        .iter()
        .filter(|p| p.parent() == Some(Path::new(dir)) && p.extension().is_some_and(|e| e == "md"))
        .map(|p| {
            let stem = p
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let (before, after) = (text(base, p), text(head, p));
            let status = match (&before, &after) {
                (None, _) => "added",
                (_, None) => "removed",
                _ => "modified",
            };
            Record {
                name: (!slug).then(|| stem.clone()),
                slug: slug.then_some(stem),
                status: status.into(),
                texts: Texts { before, after },
            }
        })
        .collect()
}

/// The comparison of a base and a head commit: everything in a review that only its commits
/// decide.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comparison {
    pub change: Option<ChangeInfo>,
    pub nodes: Vec<NodeChanges>,
    pub decisions: Vec<Record>,
    pub backlog: Vec<Record>,
    pub documents: Vec<Document>,
    /// `clm fingerprint` of the branch, `None` when a listing was truncated.
    pub fingerprint: Option<String>,
    /// The head's code key, `None` when GitHub truncated its tree.
    pub code_key: Option<String>,
    /// The branch needs a human's review (`needs_review`).
    pub review: bool,
}

/// A commit never changes, so a comparison stays good for as long as it is kept.
const COMPARISON_TTL: u64 = 86_400;

pub async fn compare<P: Platform>(
    p: &P,
    access: &Access,
    base_sha: &str,
    head_sha: &str,
) -> Option<Comparison> {
    let repo = access.full_name();
    let auth = access.auth();
    // Named anew when the comparison changes shape or meaning, so none cached before is
    // served: `compare4:` since `review` became the branch's (`pr-review`).
    let key = format!("compare4:{repo}@{base_sha}:{head_sha}");
    if let Some(c) = p
        .cache_get(&key)
        .await
        .and_then(|v| serde_json::from_str(&v).ok())
    {
        return Some(c);
    }
    let trees = trees(p, access, base_sha, head_sha).await?;
    let base_blobs = files::blobs(&trees.base);
    let base_files =
        files::spec_files(p, auth, &repo, &trees.merge_base, Some(&base_blobs)).await?;
    let head_blobs = files::blobs(&trees.head);
    let head_files = files::spec_files(p, auth, &repo, head_sha, Some(&head_blobs)).await?;
    let (base, _) = clm_model::load_files(&base_files);
    let (head, _) = clm_model::load_files(&head_files);

    let changed: BTreeSet<PathBuf> = base_files
        .keys()
        .chain(head_files.keys())
        .filter(|path| base_files.get(*path) != head_files.get(*path))
        .cloned()
        .collect();

    let ids: BTreeSet<&String> = base.nodes.keys().chain(head.nodes.keys()).collect();
    let nodes = ids
        .into_iter()
        .filter_map(|id| {
            node_changes(
                id,
                base.nodes.get(id),
                head.nodes.get(id),
                &base_files,
                &head_files,
            )
        })
        .collect();

    // `clm.toml` is a spec document only when its approval setting changed; the other
    // files fetched for sizes are none.
    let policy = Path::new(POLICY);
    fn side<'a>(listing: &keys::Listing, files: &'a Files) -> Option<Option<&'a [u8]>> {
        let policy = Path::new(POLICY);
        listing
            .has(POLICY)
            .then(|| files.get(policy).map(Vec::as_slice))
    }
    let setting = trees.policy_differs()
        && setting_changed(
            side(&trees.base, &base_files),
            side(&trees.head, &head_files),
        );
    let documents = changed
        .iter()
        .filter(|path| {
            path.to_str().is_some_and(files::is_spec_path) || (setting && path.as_path() == policy)
        })
        .filter_map(|path| {
            let texts = Texts {
                before: text(&base_files, path),
                after: text(&head_files, path),
            };
            (texts.before.is_some() || texts.after.is_some()).then(|| Document {
                path: path.to_string_lossy().into_owned(),
                texts,
            })
        })
        .collect();

    let comparison = Comparison {
        change: change_info(&head, &changed),
        nodes,
        decisions: records(&base_files, &head_files, &changed, "docs/decisions", false),
        backlog: records(&base_files, &head_files, &changed, "backlog", true),
        documents,
        fingerprint: trees.fingerprint(),
        code_key: trees.head.code_key(),
        review: needs_review(&trees, setting, &head_files),
    };
    put_comparison(p, &key, &comparison).await;
    Some(comparison)
}

/// Whether the branch needs a human's review, by the gate's own rule
/// (`clm_gate::needs_review_of`): over the paths whose entries differ between the two
/// listings, whether the approval setting changed, and the head's `change.md` frontmatter
/// of each change the head holds one of those paths of (a live change, or an archived one
/// whose `change.md` is among them), `None` when it is missing or doesn't parse. True when
/// either listing is truncated, as the gate fails closed on what it can't read.
fn needs_review(trees: &Trees, setting: bool, head_files: &Files) -> bool {
    if trees.base.truncated || trees.head.truncated {
        return true;
    }
    let paths = trees.head.changed_paths(&trees.base);
    // `changes/<id>/` for any of its files, `changes/archive/<id>/` for its `change.md`.
    let dir = |path: &str| -> Option<String> {
        let rest = path.strip_prefix("changes/")?;
        if let Some(archived) = rest.strip_prefix("archive/") {
            let id = archived.strip_suffix("/change.md")?;
            return (!id.contains('/')).then(|| format!("changes/archive/{id}/"));
        }
        let (id, _) = rest.split_once('/')?;
        Some(format!("changes/{id}/"))
    };
    let dirs: BTreeSet<String> = paths
        .iter()
        .filter_map(|p| dir(p).filter(|_| trees.head.has(p)))
        .collect();
    let changes: Vec<_> = dirs
        .iter()
        .map(|d| {
            text(head_files, Path::new(&format!("{d}change.md")))
                .and_then(|t| clm_model::parse_change_frontmatter(&t))
        })
        .collect();
    let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
    clm_gate::needs_review_of(&paths, setting, &changes)
}

/// The listings of a branch's merge base with the default branch and of its head.
pub struct Trees {
    /// The compare API's `merge_base_commit`.
    pub merge_base: String,
    pub head_sha: String,
    pub base: keys::Listing,
    pub head: keys::Listing,
}

impl Trees {
    /// `clm fingerprint` of the branch (`web.worker/fingerprint-from-trees`), `None` when
    /// either listing is truncated.
    pub fn fingerprint(&self) -> Option<String> {
        keys::fingerprint(&self.base, &self.head)
    }

    /// The two listings differ on `clm.toml`: only then can the branch change the approval
    /// setting.
    pub(crate) fn policy_differs(&self) -> bool {
        self.base.entry(POLICY) != self.head.entry(POLICY)
    }
}

/// The file whose top-level `approval` key is a spec setting (`approval-setting-is-spec`).
pub const POLICY: &str = "clm.toml";

/// Whether the branch changes the approval setting, given `clm.toml` on each side: `None`
/// where the listing has no such file, `Some(None)` where GitHub withholds its text, which
/// counts as changed, as the gate fails closed on a file it can't read. The rule is the
/// gate's own (`gate/approval-changed`).
pub fn setting_changed(before: Option<Option<&[u8]>>, after: Option<Option<&[u8]>>) -> bool {
    match (before, after) {
        (Some(None), _) | (_, Some(None)) => true,
        (before, after) => clm_gate::approval_changed(before.flatten(), after.flatten()),
    }
}

/// The merge base of `base_sha` and `head_sha`, and both listings: three requests.
pub async fn trees<P: Platform>(
    p: &P,
    access: &Access,
    base_sha: &str,
    head_sha: &str,
) -> Option<Trees> {
    let repo = access.full_name();
    let auth = access.auth();
    let compare = gh::get_ok(
        p,
        auth,
        &format!("/repos/{repo}/compare/{base_sha}...{head_sha}"),
    )
    .await?;
    let merge_base = compare["merge_base_commit"]["sha"].as_str()?.to_string();
    let (base, head) = futures::join!(
        files::listing(p, auth, &repo, &merge_base),
        files::listing(p, auth, &repo, head_sha)
    );
    Some(Trees {
        merge_base,
        head_sha: head_sha.into(),
        base: base?,
        head: head?,
    })
}

async fn put_comparison<P: Platform>(p: &P, key: &str, comparison: &Comparison) {
    if let Ok(text) = serde_json::to_string(comparison) {
        p.cache_put(key, &text, COMPARISON_TTL).await;
    }
}

/// An `approved` record for the branch's fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approval {
    pub writer: String,
    pub at: String,
    /// Its line has a non-empty string `sig`: one `clm land` will check, not one known to
    /// count, since the Worker verifies no signature.
    pub signed: bool,
}

/// The path whose presence at the default branch's tip makes a repository signed
/// (`signed-ledger-records`).
const SIGNERS: &str = "docs/allowed_signers";

/// A commit never changes, so whether it is signed is kept a day, as a comparison is.
const SIGNED_TTL: u64 = 86_400;

/// The answer to a write a signed repository counts only when signed with a key the Worker
/// doesn't hold: 422 with the command that makes it (`signed-repository`).
pub(crate) fn signed_refusal(command: String) -> Response {
    let mut res = Response::json(&serde_json::json!({"error": "signed", "command": command}));
    res.status = 422;
    res
}

/// Whether the commit `tip` holds an entry at `docs/allowed_signers`, from one GraphQL
/// query, kept in the Cache API by tip for a day; `None` when GitHub doesn't say.
pub async fn signed<P: Platform>(p: &P, access: &Access, tip: &str) -> Option<bool> {
    let key = format!("signed:{}@{tip}", access.full_name());
    if let Some(v) = p.cache_get(&key).await {
        return Some(v == "true");
    }
    let query = "query Signers($owner: String!, $name: String!, $expression: String!) { repository(owner: $owner, name: $name) { object(expression: $expression) { __typename } } }";
    let vars = serde_json::json!({
        "owner": access.owner(),
        "name": access.repo(),
        "expression": format!("{tip}:{SIGNERS}"),
    });
    let data = gh::graphql(p, access.auth(), query, vars).await?;
    let repository = data["repository"].as_object()?;
    let signed = repository.get("object").is_some_and(|o| !o.is_null());
    p.cache_put(&key, &signed.to_string(), SIGNED_TTL).await;
    Some(signed)
}

/// The branch's page on the host, when it has one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    pub number: u64,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BranchReview {
    pub branch: String,
    /// The default branch.
    pub base: String,
    pub base_sha: String,
    pub head_sha: String,
    /// The branch needs a human's review (`gate/needs-review-of`).
    pub review: bool,
    pub change: Option<ChangeInfo>,
    pub nodes: Vec<NodeChanges>,
    pub decisions: Vec<Record>,
    pub backlog: Vec<Record>,
    /// Every changed text file the model reads, whole, for the full-document view.
    pub documents: Vec<Document>,
    pub fingerprint: Option<String>,
    /// The default branch's tip holds `docs/allowed_signers` (`signed-repository`).
    pub signed: bool,
    /// The ledger's `approved` records for the fingerprint, oldest first.
    pub approvals: Vec<Approval>,
    /// A `checked` record names the head's code key.
    pub checked: bool,
    pub page: Option<Page>,
    /// The `commented` records for the branch, oldest first.
    pub comments: Vec<feedback::Commented>,
    /// The `changes_requested` records for the branch, oldest first.
    pub change_requests: Vec<feedback::Requested>,
}

/// `spec/<name>` with each part of `<name>` a plain ref component.
pub(crate) fn is_spec_branch(branch: &str) -> bool {
    branch.strip_prefix("spec/").is_some_and(|name| {
        name.split('/').all(|part| {
            !part.is_empty()
                && !part.starts_with('.')
                && !part.ends_with(".lock")
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        }) && !name.contains("..")
    })
}

/// The open PR whose head is `branch`, kept with the live state.
async fn page<P: Platform>(p: &P, access: &Access, branch: &str) -> Option<Page> {
    let found: Vec<Page> = live::cached(p, access, &format!("page:{branch}"), async || {
        let head = percent_encode(&format!("{}:{branch}", access.owner()));
        let path = format!(
            "/repos/{}/pulls?head={head}&base={}&state=open",
            access.full_name(),
            percent_encode(access.default_branch())
        );
        let pulls = gh::get_ok(p, access.auth(), &path).await?;
        Some(
            pulls
                .as_array()?
                .iter()
                .filter_map(|pr| {
                    Some(Page {
                        number: pr["number"].as_u64()?,
                        url: pr["html_url"].as_str()?.into(),
                    })
                })
                .collect(),
        )
    })
    .await
    .unwrap_or_default();
    found.into_iter().next()
}

/// `GET /api/repos/{o}/{r}/branches/{branch}/review`: 404 unless `branch` is an existing
/// `spec/*` branch.
pub async fn branch<P: Platform>(p: &P, access: &Access, branch: &str) -> Response {
    if !is_spec_branch(branch) {
        return Response::empty(404);
    }
    let Some(head_sha) = ledger::tip(p, access, branch).await else {
        return Response::empty(404);
    };
    let base = access.default_branch();
    let Some(base_sha) = ledger::tip(p, access, base).await else {
        return Response::empty(502);
    };
    let (Some(signed), Some(comparison), Some(fold)) = futures::join!(
        signed(p, access, &base_sha),
        compare(p, access, &base_sha, &head_sha),
        ledger::fold(p, access),
    ) else {
        return Response::empty(502);
    };
    let approvals = comparison
        .fingerprint
        .as_ref()
        .and_then(|f| fold.approvals.get(f))
        .into_iter()
        .flatten()
        .map(|r| Approval {
            writer: r.writer.clone(),
            at: r.at.clone(),
            signed: fold
                .lines
                .get(&r.id)
                .and_then(|line| line.get("sig")?.as_str())
                .is_some_and(|sig| !sig.is_empty()),
        })
        .collect();
    let checked = comparison
        .code_key
        .as_ref()
        .is_some_and(|k| fold.checks.contains_key(k));
    let fingerprint = comparison.fingerprint.as_deref();
    let comments = feedback::comments(&fold, branch, fingerprint);
    let change_requests = feedback::change_requests(&fold, branch, fingerprint);
    Response::json(&BranchReview {
        branch: branch.into(),
        base: base.into(),
        base_sha,
        head_sha,
        review: comparison.review,
        change: comparison.change,
        nodes: comparison.nodes,
        decisions: comparison.decisions,
        backlog: comparison.backlog,
        documents: comparison.documents,
        fingerprint: comparison.fingerprint,
        signed,
        approvals,
        checked,
        page: page(p, access, branch).await,
        comments,
        change_requests,
    })
}
