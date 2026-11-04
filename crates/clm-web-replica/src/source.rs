//! What the object reads from GitHub, as the changes `replica` takes in: the default
//! branch's tip, its compare, a tree listing, the files of a slice, the refs, the open pull
//! requests and a ledger branch's records. Every call is charged to a [`Budget`], so an
//! invocation stays within the Free plan's 50 subrequests (`first-build`).

use std::cell::Cell;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use clm_replica::{Entry, Event, File, Pull, RefChange};
use clm_web_platform::install::unix_time;
use clm_web_platform::platform::Auth;

use crate::Host;
use clm_web_platform::{files, gh};
use serde_json::{Value, json};

/// The subrequests one invocation spends at most, under the plan's 50.
pub(crate) const BUDGET: usize = 45;
/// Pages of 100 refs read at most per prefix.
const REF_PAGES: usize = 3;
/// Spec refs one GraphQL query reads at most.
const PER_SPEC_QUERY: usize = 50;
/// A compare lists at most this many files, so a list this long may be cut short.
const COMPARE_FILES: usize = 300;
/// Bytes from the start of a file whose NUL decides it is binary.
const BINARY_PROBE: usize = 8000;

/// The subrequests an invocation has made.
#[derive(Debug, Default)]
pub(crate) struct Budget(Cell<usize>);

impl Budget {
    pub(crate) fn spend(&self, calls: usize) {
        self.0.set(self.0.get() + calls);
    }

    pub(crate) fn left(&self) -> usize {
        BUDGET.saturating_sub(self.0.get())
    }
}

/// Whether the replica keeps a file's text: the model's files, `clm.toml` and every
/// `.gitattributes`.
pub(crate) fn is_spec(path: &str) -> bool {
    files::is_spec_path(path)
        || path == "clm.toml"
        || path == ".gitattributes"
        || path.ends_with("/.gitattributes")
}

/// Whether a file's text is read at all: Markdown outside the spec files holds no markers.
pub(crate) fn wanted(path: &str) -> bool {
    is_spec(path) || !path.ends_with(".md")
}

/// The default branch's name.
pub(crate) async fn default_branch<P: Host>(
    p: &P,
    auth: &Auth,
    repo: &str,
    b: &Budget,
) -> Option<String> {
    b.spend(1);
    let v = gh::get_ok(p, auth, &format!("/repos/{repo}")).await?;
    v["default_branch"].as_str().map(String::from)
}

/// A branch's tip commit.
pub(crate) async fn branch_tip<P: Host>(
    p: &P,
    auth: &Auth,
    repo: &str,
    branch: &str,
    b: &Budget,
) -> Option<String> {
    b.spend(1);
    let v = gh::get_ok(p, auth, &format!("/repos/{repo}/git/ref/heads/{branch}")).await?;
    v["object"]["sha"].as_str().map(String::from)
}

/// A commit's tree: each regular file's blob SHA, size and mode.
pub(crate) async fn tree<P: Host>(
    p: &P,
    auth: &Auth,
    repo: &str,
    sha: &str,
    b: &Budget,
) -> Option<Vec<Entry>> {
    b.spend(1);
    let v = gh::get_ok(
        p,
        auth,
        &format!("/repos/{repo}/git/trees/{sha}?recursive=1"),
    )
    .await?;
    let entries = v["tree"].as_array()?;
    Some(
        entries
            .iter()
            .filter(|e| e["type"] == "blob")
            .filter_map(|e| {
                Some(Entry {
                    path: e["path"].as_str()?.into(),
                    sha: e["sha"].as_str()?.into(),
                    size: e["size"].as_u64().unwrap_or(0),
                    mode: e["mode"].as_str()?.into(),
                })
            })
            .collect(),
    )
}

/// A file the compare lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Changed {
    pub path: String,
    /// The blob's SHA, empty when the compare doesn't give it.
    pub sha: String,
}

/// What a compare of the replica's tip with a push's `after` lists, when the push is a
/// fast-forward of fewer files than the compare's limit: the files added or modified, and the
/// paths removed. `None` for anything else, and a list as long as the limit may be cut
/// short, so it counts as anything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Compare {
    pub changed: Vec<Changed>,
    pub removed: Vec<String>,
}

pub(crate) async fn compare<P: Host>(
    p: &P,
    auth: &Auth,
    repo: &str,
    base: &str,
    head: &str,
    b: &Budget,
) -> Option<Compare> {
    b.spend(1);
    let v = gh::get_ok(p, auth, &format!("/repos/{repo}/compare/{base}...{head}")).await?;
    let listed = v["files"].as_array()?;
    if v["status"] != "ahead" || listed.len() >= COMPARE_FILES {
        return None;
    }
    let (mut changed, mut removed) = (Vec::new(), Vec::new());
    for f in listed {
        let path = f["filename"].as_str()?;
        match f["status"].as_str()? {
            "removed" => removed.push(path.to_string()),
            status => {
                if status == "renamed" {
                    removed.extend(f["previous_filename"].as_str().map(String::from));
                }
                changed.push(Changed {
                    path: path.into(),
                    sha: f["sha"].as_str().unwrap_or_default().into(),
                });
            }
        }
    }
    Some(Compare { changed, removed })
}

/// The files of `entries` at `sha`, as many as the budget allows, in GraphQL queries of at
/// most 100 paths. A blob GraphQL cut short is read whole from the blob API, one call each;
/// one the budget can't afford is left out, so it stays to fetch. The entries without a text
/// the source could read are binary.
pub(crate) async fn fetch<P: Host>(
    p: &P,
    auth: &Auth,
    repo: &str,
    sha: &str,
    entries: &[Entry],
    b: &Budget,
) -> Vec<File> {
    let Some((owner, name)) = repo.split_once('/') else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for chunk in entries.chunks(files::PER_QUERY) {
        if b.left() < 2 {
            break;
        }
        let mut vars = serde_json::Map::new();
        vars.insert("owner".into(), json!(owner));
        vars.insert("name".into(), json!(name));
        let mut params = vec!["$owner: String!".to_string(), "$name: String!".to_string()];
        let mut fields = String::new();
        for (i, entry) in chunk.iter().enumerate() {
            vars.insert(format!("e{i}"), json!(format!("{sha}:{}", entry.path)));
            params.push(format!("$e{i}: String!"));
            fields.push_str(&format!(
                "f{i}: object(expression: $e{i}) {{ ... on Blob {{ text isBinary isTruncated oid }} }}\n"
            ));
        }
        let query = format!(
            "query Blobs({}) {{ repository(owner: $owner, name: $name) {{\n{fields}}} }}",
            params.join(", ")
        );
        b.spend(1);
        let Some(data) = gh::graphql(p, auth, &query, Value::Object(vars)).await else {
            break;
        };
        let repository = &data["repository"];
        if repository.is_null() {
            break;
        }
        for (i, entry) in chunk.iter().enumerate() {
            let object = &repository[format!("f{i}")];
            let text = match (object["text"].as_str(), object["isTruncated"] == true) {
                (Some(_), true) => {
                    let blob = if entry.sha.is_empty() {
                        object["oid"].as_str().unwrap_or_default()
                    } else {
                        entry.sha.as_str()
                    };
                    if blob.is_empty() || b.left() == 0 {
                        continue;
                    }
                    b.spend(1);
                    let Some(bytes) = blob_bytes(p, auth, repo, blob).await else {
                        continue;
                    };
                    String::from_utf8(bytes).ok()
                }
                (text, _) => text.map(String::from),
            };
            let nul = text
                .as_ref()
                .is_some_and(|t| t.bytes().take(BINARY_PROBE).any(|c| c == 0));
            let mut entry = entry.clone();
            if entry.size == 0 {
                entry.size = text.as_ref().map_or(0, |t| t.len() as u64);
            }
            out.push(File {
                binary: object["isBinary"] == true || nul || text.is_none(),
                text,
                entry,
            });
        }
    }
    out
}

async fn blob_bytes<P: Host>(p: &P, auth: &Auth, repo: &str, sha: &str) -> Option<Vec<u8>> {
    let v = gh::get_ok(p, auth, &format!("/repos/{repo}/git/blobs/{sha}")).await?;
    let content: String = v["content"].as_str()?.split_whitespace().collect();
    STANDARD.decode(content).ok()
}

/// A branch under a prefix: its name without the prefix, its tip and the tip's commit time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tip {
    pub name: String,
    pub sha: String,
    pub time: u64,
}

/// The branches under `refs/heads/<prefix>`, from GraphQL pages of 100 refs. Nested names
/// such as `impl/archive/<id>` name no component and are left out.
pub(crate) async fn tips<P: Host>(
    p: &P,
    auth: &Auth,
    repo: &str,
    prefix: &str,
    b: &Budget,
) -> Option<Vec<Tip>> {
    let (owner, name) = repo.split_once('/')?;
    let query = "query Refs($owner: String!, $name: String!, $prefix: String!, $after: String) { repository(owner: $owner, name: $name) { refs(refPrefix: $prefix, first: 100, after: $after) { nodes { name target { ... on Commit { oid committedDate } } } pageInfo { hasNextPage endCursor } } } }";
    let mut tips = Vec::new();
    let mut after = Value::Null;
    let mut complete = false;
    for _ in 0..REF_PAGES {
        let vars = json!({
            "owner": owner,
            "name": name,
            "prefix": format!("refs/heads/{prefix}"),
            "after": after,
        });
        b.spend(1);
        let data = gh::graphql(p, auth, query, vars).await?;
        let refs = &data["repository"]["refs"];
        tips.extend(refs["nodes"].as_array()?.iter().filter_map(|n| {
            let name = n["name"].as_str().filter(|n| !n.contains('/'))?;
            Some(Tip {
                name: name.into(),
                sha: n["target"]["oid"].as_str()?.into(),
                time: unix_time(n["target"]["committedDate"].as_str()?)?,
            })
        }));
        match refs["pageInfo"]["endCursor"].as_str() {
            Some(cursor) if refs["pageInfo"]["hasNextPage"] == true => after = cursor.into(),
            _ => {
                complete = true;
                break;
            }
        }
    }
    // A list cut short at the last page would read as refs deleted.
    complete.then_some(tips)
}

/// The open pull requests, in one GraphQL query.
pub(crate) async fn pulls<P: Host>(
    p: &P,
    auth: &Auth,
    repo: &str,
    b: &Budget,
) -> Option<Vec<Pull>> {
    let (owner, name) = repo.split_once('/')?;
    let query = "query Pulls($owner: String!, $name: String!) { repository(owner: $owner, name: $name) { pullRequests(states: OPEN, first: 50) { nodes { number title url headRefName baseRefName headRefOid isCrossRepository } } } }";
    b.spend(1);
    let data = gh::graphql(p, auth, query, json!({"owner": owner, "name": name})).await?;
    let nodes = data["repository"]["pullRequests"]["nodes"].as_array()?;
    Some(
        nodes
            .iter()
            .filter_map(|v| {
                Some(Pull {
                    number: v["number"].as_u64()?,
                    title: v["title"].as_str().unwrap_or_default().into(),
                    head: v["headRefName"].as_str()?.into(),
                    base: v["baseRefName"].as_str()?.into(),
                    head_sha: v["headRefOid"].as_str().unwrap_or_default().into(),
                    url: v["url"].as_str().unwrap_or_default().into(),
                    fork: v["isCrossRepository"] == true,
                    open: true,
                })
            })
            .collect(),
    )
}

/// Each spec ref's `changes/<id>/change.md` and `deltas/` names at its tip, or the archived
/// ones when the tip has no live change, in the order of `refs` (name and tip). `spec/<id>`
/// and `autosync/spec/<id>` name change `<id>`.
pub(crate) async fn spec_changes<P: Host>(
    p: &P,
    auth: &Auth,
    repo: &str,
    refs: &[(String, String)],
    b: &Budget,
) -> Option<Vec<RefChange>> {
    let (owner, name) = repo.split_once('/')?;
    let id = |name: &str| name.rsplit('/').next().unwrap_or_default().to_string();
    let mut out = Vec::new();
    for chunk in refs.chunks(PER_SPEC_QUERY) {
        let mut vars = serde_json::Map::new();
        vars.insert("owner".into(), json!(owner));
        vars.insert("name".into(), json!(name));
        let mut params = vec!["$owner: String!".to_string(), "$name: String!".into()];
        let mut fields = String::new();
        for (i, (branch, sha)) in chunk.iter().enumerate() {
            let dir = format!("{sha}:changes/{}", id(branch));
            vars.insert(format!("c{i}"), json!(format!("{dir}/change.md")));
            vars.insert(format!("d{i}"), json!(format!("{dir}/deltas")));
            let archive = format!("{sha}:changes/archive/{}", id(branch));
            vars.insert(format!("ca{i}"), json!(format!("{archive}/change.md")));
            vars.insert(format!("da{i}"), json!(format!("{archive}/deltas")));
            params.push(format!(
                "$c{i}: String!, $d{i}: String!, $ca{i}: String!, $da{i}: String!"
            ));
            fields.push_str(&format!(
                "c{i}: object(expression: $c{i}) {{ ... on Blob {{ text }} }}\n\
                 d{i}: object(expression: $d{i}) {{ ... on Tree {{ entries {{ name }} }} }}\n\
                 ca{i}: object(expression: $ca{i}) {{ ... on Blob {{ text }} }}\n\
                 da{i}: object(expression: $da{i}) {{ ... on Tree {{ entries {{ name }} }} }}\n"
            ));
        }
        let query = format!(
            "query Taken({}) {{ repository(owner: $owner, name: $name) {{\n{fields}}} }}",
            params.join(", ")
        );
        b.spend(1);
        let data = gh::graphql(p, auth, &query, Value::Object(vars)).await?;
        let repository = &data["repository"];
        if repository.is_null() {
            return None;
        }
        for i in 0..chunk.len() {
            let (change_key, deltas_key) = if repository[format!("c{i}")]["text"].is_string() {
                (format!("c{i}"), format!("d{i}"))
            } else {
                (format!("ca{i}"), format!("da{i}"))
            };
            out.push(RefChange::Spec {
                change: repository[change_key]["text"].as_str().map(String::from),
                deltas: repository[deltas_key]["entries"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|e| e["name"].as_str().map(String::from))
                    .collect(),
            });
        }
    }
    Some(out)
}

/// An `impl/*` head's code key.
pub(crate) async fn code_key<P: Host>(
    p: &P,
    auth: &Auth,
    repo: &str,
    sha: &str,
    b: &Budget,
) -> Option<RefChange> {
    b.spend(1);
    let listing = files::listing(p, auth, repo, sha).await?;
    Some(RefChange::Code {
        key: listing.code_key()?,
    })
}

/// Every ledger branch with its tip: `clm/ledger/*`.
pub(crate) async fn ledger_tips<P: Host>(
    p: &P,
    auth: &Auth,
    repo: &str,
    b: &Budget,
) -> Option<Vec<(String, String)>> {
    b.spend(1);
    let path = format!("/repos/{repo}/git/matching-refs/heads/clm/ledger/");
    let refs = gh::get_ok(p, auth, &path).await?;
    Some(
        refs.as_array()?
            .iter()
            .filter_map(|r| {
                let name = r["ref"].as_str()?.strip_prefix("refs/heads/")?;
                Some((name.to_string(), r["object"]["sha"].as_str()?.to_string()))
            })
            .collect(),
    )
}

/// The `records.jsonl` at each `(branch, tip)`, in order, `None` for a branch without one.
pub(crate) async fn ledger_texts<P: Host>(
    p: &P,
    auth: &Auth,
    repo: &str,
    branches: &[(String, String)],
    b: &Budget,
) -> Option<Vec<Option<String>>> {
    if branches.is_empty() {
        return Some(Vec::new());
    }
    let expressions: Vec<String> = branches
        .iter()
        .map(|(_, sha)| format!("{sha}:records.jsonl"))
        .collect();
    b.spend(expressions.len().div_ceil(files::PER_QUERY));
    files::objects_direct(p, auth, repo, "Ledgers", &expressions).await
}

/// The repo's recent pushes and branch creations, newest first.
pub(crate) async fn events<P: Host>(
    p: &P,
    auth: &Auth,
    repo: &str,
    b: &Budget,
) -> Option<Vec<Event>> {
    b.spend(1);
    let events = gh::get_ok(p, auth, &format!("/repos/{repo}/events?per_page=100")).await?;
    Some(events.as_array()?.iter().filter_map(event_of).collect())
}

fn event_of(e: &Value) -> Option<Event> {
    let payload = &e["payload"];
    let at = unix_time(e["created_at"].as_str()?)?;
    let actor = e["actor"]["login"].as_str().unwrap_or_default().to_string();
    match e["type"].as_str()? {
        "PushEvent" => Some(Event::Push {
            branch: payload["ref"].as_str()?.strip_prefix("refs/heads/")?.into(),
            at,
            actor,
        }),
        "CreateEvent" if payload["ref_type"] == "branch" => Some(Event::Create {
            branch: payload["ref"].as_str()?.into(),
            at,
            actor,
        }),
        _ => None,
    }
}
