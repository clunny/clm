//! What the object reads from GitHub, as the changes `replica` takes in: the default
//! branch's tip, its compare, a tree listing, the files of a slice, the refs, the open pull
//! requests and a ledger branch's records. Every call is charged to a [`Budget`], so an
//! invocation stays within the Free plan's 50 subrequests (`first-build`).

use std::cell::Cell;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use clm_ledger::unix_time;
use clm_replica::{Entry, Event, File, Pull, RefChange};
use clm_web_platform::platform::Auth;

use crate::Host;
use clm_web_platform::{files, gh, live};
use serde_json::{Value, json};

/// The subrequests one invocation spends at most, under the plan's 50.
pub(crate) const BUDGET: usize = 45;
/// Pages of 100 refs read at most per prefix.
const REF_PAGES: usize = 3;
/// Spec refs one GraphQL query reads at most.
const PER_SPEC_QUERY: usize = 50;
/// A compare lists at most this many files, so a list this long may be cut short.
const COMPARE_FILES: usize = 300;
/// The largest binary asset read whole: a blob over 10 MiB is held without its bytes.
const MAX_ASSET: u64 = 10 << 20;
/// The pull requests a page of `live::pulls_direct` holds, and the pages it reads at most
/// (`web.worker.platform/open-pulls`).
const PULLS_PER_PAGE: usize = 100;
const PULL_PAGES: usize = 3;

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
/// the source could read are binary. An entry with no mode, as a compare gives it, takes the
/// mode its parent tree gives, asked in the same query (`fetches-what-the-replica-wants`).
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
                "f{i}: object(expression: $e{i}) {{ ... on Blob {{ text isBinary isTruncated oid byteSize }} }}\n"
            ));
        }
        // Each distinct parent directory of an entry with no mode: `t<j>`.
        let mut dirs: Vec<&str> = chunk
            .iter()
            .filter(|e| e.mode.is_empty())
            .map(|e| parent(&e.path).0)
            .collect();
        dirs.sort_unstable();
        dirs.dedup();
        for (j, dir) in dirs.iter().enumerate() {
            vars.insert(format!("t{j}"), json!(format!("{sha}:{dir}")));
            params.push(format!("$t{j}: String!"));
            fields.push_str(&format!(
                "t{j}: object(expression: $t{j}) {{ ... on Tree {{ entries {{ name mode }} }} }}\n"
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
            let mut entry = entry.clone();
            if entry.mode.is_empty() {
                let (dir, file) = parent(&entry.path);
                let j = dirs.binary_search(&dir).unwrap_or_default();
                entry.mode = mode_in(&repository[format!("t{j}")], file);
            }
            if entry.size == 0 {
                entry.size = object["byteSize"].as_u64().unwrap_or_default();
            }
            let blob = if entry.sha.is_empty() {
                object["oid"].as_str().unwrap_or_default().to_string()
            } else {
                entry.sha.clone()
            };
            if object["isBinary"] == true && clm_replica::keeps_hash(&entry.path) {
                // A binary asset is read whole, one call each, and given with its bytes;
                // one too big, or one the blob API fails, is held without them
                // (`fetches-binary-assets`).
                if entry.size > MAX_ASSET || blob.is_empty() {
                    out.push(File::binary(entry));
                    continue;
                }
                if b.left() == 0 {
                    continue;
                }
                b.spend(1);
                out.push(match blob_bytes(p, auth, repo, &blob).await {
                    Some(bytes) => File::asset(entry, bytes),
                    None => File::binary(entry),
                });
                continue;
            }
            let text = match (object["text"].as_str(), object["isTruncated"] == true) {
                (Some(_), true) => {
                    if blob.is_empty() || b.left() == 0 {
                        continue;
                    }
                    b.spend(1);
                    let Some(bytes) = blob_bytes(p, auth, repo, &blob).await else {
                        continue;
                    };
                    String::from_utf8(bytes).ok()
                }
                (text, _) => text.map(String::from),
            };
            if entry.size == 0 {
                entry.size = text.as_ref().map_or(0, |t| t.len() as u64);
            }
            // A text with a NUL byte near its start is held binary by the replica itself
            // (`replica/files-a-source-gives`).
            out.push(match text {
                Some(text) if object["isBinary"] != true => File::text(entry, text),
                _ => File::binary(entry),
            });
        }
    }
    out
}

/// A path's directory, empty at the top, and its name.
fn parent(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}

/// The mode a tree's `entries` give `name`, as the octal text a tree listing writes, from
/// GraphQL's integer (33188 is `100644`, 40960 `120000`); `100644` when it gives none.
fn mode_in(tree: &Value, name: &str) -> String {
    tree["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|e| e["name"] == name)
        .and_then(|e| e["mode"].as_u64())
        .map_or_else(|| "100644".into(), |m| format!("{m:o}"))
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
            // A commit time that can't be read holds the ref at the time it is listed.
            let time = n["target"]["committedDate"].as_str().and_then(unix_time);
            Some(Tip {
                name: name.into(),
                sha: n["target"]["oid"].as_str()?.into(),
                time: time.unwrap_or_else(|| p.now()),
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

/// The open pull requests, through the platform's pages of 100, at most 300, with whether
/// the listing was read to its end.
pub(crate) async fn pulls<P: Host>(
    p: &P,
    auth: &Auth,
    repo: &str,
    b: &Budget,
) -> Option<(Vec<Pull>, bool)> {
    let listed = live::pulls_direct(p, auth, repo).await;
    // One query a page: a listing read to its end took one for each 100 it holds, and one
    // cut short, or a failed one, is counted at the most it can take.
    b.spend(match &listed {
        Some((pulls, true)) => pulls.len().div_ceil(PULLS_PER_PAGE).max(1),
        _ => PULL_PAGES,
    });
    let (pulls, complete) = listed?;
    let pulls = pulls
        .into_iter()
        .map(|v| Pull {
            number: v.number,
            title: v.title,
            head: v.head,
            base: v.base,
            head_sha: v.head_sha,
            url: v.url,
            fork: v.fork,
            open: true,
        })
        .collect();
    Some((pulls, complete))
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
