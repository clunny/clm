//! Live state, fetched on demand and kept in the Cache API for 60 seconds
//! (`live-state-cache`), and a repo's open pull requests (`open-pulls`).

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::access::Access;
use crate::gh;
use crate::platform::{Auth, GitHub, Platform};

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

/// Pull requests one GraphQL page holds.
const PULLS_PER_PAGE: usize = 100;
/// Pages `pulls_direct` reads at most, within the subrequest budget.
const PULL_PAGES: usize = 3;

/// `repo`'s open pull requests in GitHub's order, read now in pages of 100, at most 3, and
/// whether the listing was read to its end; `None` when a page can't be read.
pub async fn pulls_direct<P: GitHub>(p: &P, auth: &Auth, repo: &str) -> Option<(Vec<Pull>, bool)> {
    let (owner, name) = repo.split_once('/')?;
    let query = format!(
        "query Pulls($owner: String!, $name: String!, $after: String) {{ repository(owner: $owner, name: $name) {{ pullRequests(states: OPEN, first: {PULLS_PER_PAGE}, after: $after) {{ nodes {{ number title url headRefName baseRefName headRefOid isCrossRepository }} pageInfo {{ hasNextPage endCursor }} }} }} }}"
    );
    let mut pulls = Vec::new();
    let mut after = Value::Null;
    for _ in 0..PULL_PAGES {
        let vars = json!({"owner": owner, "name": name, "after": after});
        let data = gh::graphql(p, auth, &query, vars).await?;
        let page = &data["repository"]["pullRequests"];
        pulls.extend(page["nodes"].as_array()?.iter().filter_map(pull));
        match page["pageInfo"]["endCursor"].as_str() {
            Some(cursor) if page["pageInfo"]["hasNextPage"] == true => after = cursor.into(),
            _ => return Some((pulls, true)),
        }
    }
    Some((pulls, false))
}

/// [`pulls_direct`]'s pull requests, kept with the live state.
pub async fn pulls<P: Platform>(p: &P, access: &Access) -> Option<Vec<Pull>> {
    cached(p, access, "pulls", async || {
        Some(pulls_direct(p, access.auth(), &access.full_name()).await?.0)
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
