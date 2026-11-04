//! Live state, fetched on demand and kept in the Cache API for 60 seconds: claim refs,
//! open pull requests, and recent repo events.

use clm_orchestrator::ClaimRef;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::access::Access;
use crate::gh;
use crate::install::unix_time;
use crate::platform::Platform;

/// Live state is never older than this.
const LIVE_TTL: u64 = 60;

/// `live:<owner>/<repo>:<what>` (`Access::live_key`) from the Cache API, else fetched
/// and cached, under the delivery's key and the plain one both when `access` is fresh.
pub async fn cached<P, T, F>(p: &P, access: &Access, what: &str, fetch: F) -> Option<T>
where
    P: Platform,
    T: Serialize + DeserializeOwned,
    F: AsyncFnOnce() -> Option<T>,
{
    let key = access.live_key(what);
    if let Some(v) = p
        .cache_get(&key)
        .await
        .and_then(|v| serde_json::from_str(&v).ok())
    {
        return Some(v);
    }
    let value = fetch().await?;
    if let Ok(text) = serde_json::to_string(&value) {
        p.cache_put(&key, &text, LIVE_TTL).await;
        // A fresh read replaces the plain entry, so the next read without `fresh` never
        // goes back to the older value (`fresh-replaces-plain`).
        let plain = access.plain_live_key(what);
        if plain != key {
            p.cache_put(&plain, &text, LIVE_TTL).await;
        }
    }
    Some(value)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    /// The branch name without `impl/`: a node ID. Nested branches such as
    /// `impl/archive/<id>` claim no node (`orchestrator/nested-branches-not-claims`), so
    /// none is a `Claim`.
    pub item: String,
    /// The tip commit's time, in Unix seconds: the last push.
    pub tip_time: u64,
}

/// Pages of 100 refs `tips` reads at most per prefix, within the subrequest budget.
const REF_PAGES: usize = 3;

/// A branch under a prefix: its name without the prefix, its tip commit and the tip's
/// time in Unix seconds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tip {
    pub name: String,
    pub sha: String,
    pub time: u64,
}

/// The branches under `refs/heads/<prefix>` with their tips, from GraphQL pages of 100
/// refs, cached as `<prefix>`. GitHub lists refs in name order, so nested branches such as
/// `impl/archive/<id>` can fill the first page: every page is read, up to `REF_PAGES`,
/// and the nested names are left out.
pub async fn tips<P: Platform>(p: &P, access: &Access, prefix: &str) -> Option<Vec<Tip>> {
    cached(p, access, prefix, async || {
        let query = "query Refs($owner: String!, $name: String!, $prefix: String!, $after: String) { repository(owner: $owner, name: $name) { refs(refPrefix: $prefix, first: 100, after: $after) { nodes { name target { ... on Commit { oid committedDate } } } pageInfo { hasNextPage endCursor } } } }";
        let mut tips = Vec::new();
        let mut after = Value::Null;
        for _ in 0..REF_PAGES {
            let vars = json!({
                "owner": access.owner(),
                "name": access.repo(),
                "prefix": format!("refs/heads/{prefix}"),
                "after": after,
            });
            let data = gh::graphql(p, access.auth(), query, vars).await?;
            let refs = &data["repository"]["refs"];
            tips.extend(refs["nodes"].as_array()?.iter().filter_map(tip));
            match refs["pageInfo"]["endCursor"].as_str() {
                Some(cursor) if refs["pageInfo"]["hasNextPage"] == true => {
                    after = cursor.into();
                }
                _ => break,
            }
        }
        Some(tips)
    })
    .await
}

/// A ref of the refs query as a `Tip`, unless it is nested.
fn tip(n: &Value) -> Option<Tip> {
    let name = n["name"].as_str()?;
    if name.contains('/') {
        return None;
    }
    Some(Tip {
        name: name.into(),
        sha: n["target"]["oid"].as_str()?.into(),
        time: unix_time(n["target"]["committedDate"].as_str()?)?,
    })
}

/// The `impl/*` claim branches with their tip commit times. Nested branches such as
/// `impl/archive/<id>` claim no node (`orchestrator/nested-branches-not-claims`). A merged
/// claim's branch is deleted on merge (`ci/delete-merged-head`), so none is filtered here.
pub async fn claims<P: Platform>(p: &P, access: &Access) -> Option<Vec<Claim>> {
    let tips = tips(p, access, "impl/").await?;
    Some(
        tips.into_iter()
            .map(|t| Claim {
                item: t.name,
                tip_time: t.time,
            })
            .collect(),
    )
}

pub fn claim_refs(claims: &[Claim]) -> Vec<ClaimRef> {
    claims
        .iter()
        .map(|c| ClaimRef {
            node: c.item.clone(),
            tip_time: c.tip_time,
            // The merge test is git's (`merged-claims-released`); the Worker's claims are
            // branches GitHub still has, which `ci/delete-merged-head` removes on merge.
            merged: false,
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pull {
    pub number: u64,
    pub title: String,
    pub head: String,
    pub base: String,
    pub head_sha: String,
    /// The page's web address.
    pub url: String,
    /// Its head is a fork's branch, not this repository's.
    #[serde(default)]
    pub fork: bool,
}

/// Open pull requests, in one GraphQL query.
pub async fn pulls<P: Platform>(p: &P, access: &Access) -> Option<Vec<Pull>> {
    cached(p, access, "pulls", async || {
        let query = "query Pulls($owner: String!, $name: String!) { repository(owner: $owner, name: $name) { pullRequests(states: OPEN, first: 50) { nodes { number title url headRefName baseRefName headRefOid isCrossRepository } } } }";
        let vars = json!({"owner": access.owner(), "name": access.repo()});
        let data = gh::graphql(p, access.auth(), query, vars).await?;
        let nodes = data["repository"]["pullRequests"]["nodes"].as_array()?.clone();
        Some(nodes.iter().filter_map(pull).collect())
    })
    .await
}

fn pull(v: &Value) -> Option<Pull> {
    Some(Pull {
        number: v["number"].as_u64()?,
        title: v["title"].as_str().unwrap_or_default().into(),
        head: v["headRefName"].as_str()?.into(),
        base: v["baseRefName"].as_str()?.into(),
        head_sha: v["headRefOid"].as_str().unwrap_or_default().into(),
        url: v["url"].as_str().unwrap_or_default().into(),
        fork: v["isCrossRepository"] == true,
    })
}

/// One entry of the activity feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    /// `push`, `claim`, `land` or `approval`.
    pub kind: String,
    /// Unix seconds.
    pub at: u64,
    /// A GitHub login, or the person part of a record's writer.
    pub actor: String,
    /// The branch pushed, landed or approved, or the item claimed.
    pub r#ref: String,
}

/// Recent pushes and claims from the repo's events, newest first. Pushes to
/// `clm/ledger/*` are left out: every append is one, and the fold has the records.
pub async fn events<P: Platform>(p: &P, access: &Access) -> Option<Vec<Activity>> {
    cached(p, access, "events", async || {
        let path = format!("/repos/{}/events?per_page=100", access.full_name());
        let events = gh::get_ok(p, access.auth(), &path).await?;
        let mut feed: Vec<Activity> = events.as_array()?.iter().filter_map(activity_of).collect();
        feed.sort_by_key(|a| std::cmp::Reverse(a.at));
        Some(feed)
    })
    .await
}

fn activity_of(e: &Value) -> Option<Activity> {
    let payload = &e["payload"];
    let (kind, r#ref) = match e["type"].as_str()? {
        "PushEvent" => {
            let branch = payload["ref"].as_str()?.strip_prefix("refs/heads/")?;
            if branch.starts_with("clm/ledger/") {
                return None;
            }
            ("push", branch)
        }
        "CreateEvent" if payload["ref_type"] == "branch" => {
            ("claim", payload["ref"].as_str()?.strip_prefix("impl/")?)
        }
        _ => return None,
    };
    Some(Activity {
        kind: kind.into(),
        at: unix_time(e["created_at"].as_str()?)?,
        actor: e["actor"]["login"].as_str().unwrap_or_default().into(),
        r#ref: r#ref.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_branches_are_left_out() {
        let at = json!({"oid": "abc", "committedDate": "2026-10-27T00:00:00Z"});
        let node = tip(&json!({"name": "orchestrator.run", "target": at})).unwrap();
        assert_eq!(node.name, "orchestrator.run");
        assert_eq!(
            tip(&json!({"name": "archive/20261026-x", "target": at})),
            None
        );
    }
}
