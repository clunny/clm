//! The repo list: every repo of the user's App installations whose default branch has
//! `specs/SPEC.md`, memoised for 5 minutes per token in the Cache API, unless empty.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use clm_web_platform::access::token_hash;
use clm_web_platform::gh;
use clm_web_platform::platform::{Auth, Platform};

/// How long a user's list is remembered. A newly installed repo can take this long to
/// appear, except when the list was empty, which isn't remembered.
const MEMO_SECS: u64 = 300;

/// At most this many repos are checked for `specs/SPEC.md` per GraphQL query.
const PER_QUERY: usize = 50;
/// At most this many pages of repositories are read, across installations, and at most
/// this many repos checked, so the list stays within the subrequest budget (25 pages and
/// 20 queries); repos beyond them are left out.
const MAX_PAGES: usize = 25;
const MAX_REPOS: usize = 20 * PER_QUERY;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repo {
    pub owner: String,
    pub name: String,
    pub default_branch: String,
}

/// The requester's list, from the Cache API under a hash of their token, else from
/// GitHub. Each token has its own memo, so one user never sees another's repos.
pub async fn list<P: Platform>(p: &P, auth: &Auth) -> Option<Vec<Repo>> {
    let Auth::User(token) = auth else {
        return fetch(p, auth).await;
    };
    let key = format!("repos:{}", token_hash(token));
    if let Some(memo) = p
        .cache_get(&key)
        .await
        .and_then(|m| serde_json::from_str(&m).ok())
    {
        return Some(memo);
    }
    let repos = fetch(p, auth).await?;
    if !repos.is_empty()
        && let Ok(text) = serde_json::to_string(&repos)
    {
        p.cache_put(&key, &text, MEMO_SECS).await;
    }
    Some(repos)
}

async fn fetch<P: Platform>(p: &P, auth: &Auth) -> Option<Vec<Repo>> {
    let installs = gh::get_ok(p, auth, "/user/installations?per_page=100").await?;
    let mut repos = Vec::new();
    let mut pages = 0;
    for id in installs["installations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|i| i["id"].as_u64())
    {
        for page in 1.. {
            if pages == MAX_PAGES {
                break;
            }
            pages += 1;
            let path = format!("/user/installations/{id}/repositories?per_page=100&page={page}");
            let v = gh::get_ok(p, auth, &path).await?;
            let batch = v["repositories"].as_array().cloned().unwrap_or_default();
            repos.extend(batch.iter().filter_map(repo));
            let total = v["total_count"].as_u64().unwrap_or(0) as usize;
            if batch.is_empty() || page * 100 >= total {
                break;
            }
        }
    }
    repos.sort_by(|a, b| (&a.owner, &a.name).cmp(&(&b.owner, &b.name)));
    repos.dedup();
    repos.truncate(MAX_REPOS);

    let mut out = Vec::new();
    for chunk in repos.chunks(PER_QUERY) {
        let mut vars = serde_json::Map::new();
        let mut fields = String::new();
        let mut params = Vec::new();
        for (i, r) in chunk.iter().enumerate() {
            vars.insert(format!("o{i}"), json!(r.owner));
            vars.insert(format!("n{i}"), json!(r.name));
            params.push(format!("$o{i}: String!, $n{i}: String!"));
            fields.push_str(&format!(
                "r{i}: repository(owner: $o{i}, name: $n{i}) {{ object(expression: \"HEAD:specs/SPEC.md\") {{ __typename }} }}\n"
            ));
        }
        let query = format!("query Specs({}) {{\n{fields}}}", params.join(", "));
        let data = gh::graphql(p, auth, &query, Value::Object(vars)).await?;
        for (i, r) in chunk.iter().enumerate() {
            if data[format!("r{i}")]["object"]["__typename"] == "Blob" {
                out.push(r.clone());
            }
        }
    }
    Some(out)
}

fn repo(v: &Value) -> Option<Repo> {
    Some(Repo {
        owner: v["owner"]["login"].as_str()?.into(),
        name: v["name"].as_str()?.into(),
        default_branch: v["default_branch"].as_str().unwrap_or("main").into(),
    })
}
