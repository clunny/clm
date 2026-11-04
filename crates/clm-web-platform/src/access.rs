//! The access guard. Repos of many organisations share one cache, so every repo route
//! takes an [`Access`], which only [`check`] makes, after GitHub confirms with the
//! requester's own token that they can read the repo.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::gh;
use crate::platform::{Auth, GitHub, Platform};

/// How long a check is remembered, per token and repo.
const MEMO_SECS: u64 = 300;

/// Proof that the requester can read `owner/repo`.
#[derive(Debug, Clone)]
pub struct Access {
    owner: String,
    repo: String,
    auth: Auth,
    default_branch: String,
    /// The webhook delivery a live notice named, so the live state is read anew for it
    /// (`fresh-live-state`).
    fresh: Option<String>,
}

impl Access {
    /// This access, reading the live state anew for the delivery `fresh` names when it is
    /// 1 to 64 characters of `[0-9a-f-]`; any other value is ignored.
    pub fn fresh(mut self, fresh: Option<&str>) -> Access {
        let valid = |d: &&str| {
            (1..=64).contains(&d.len())
                && d.bytes()
                    .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f' | b'-'))
        };
        self.fresh = fresh.filter(valid).map(String::from);
        self
    }

    /// The Cache API key of the live state `what`: `live:<o>/<r>:<what>`, or, for a
    /// fresh read, `live:<o>/<r>#<delivery>:<what>`.
    pub(crate) fn live_key(&self, what: &str) -> String {
        match &self.fresh {
            Some(d) => format!("live:{}#{d}:{what}", self.full_name()),
            None => self.plain_live_key(what),
        }
    }

    /// `live:<o>/<r>:<what>`, the key of a read without `fresh`.
    pub(crate) fn plain_live_key(&self, what: &str) -> String {
        format!("live:{}:{what}", self.full_name())
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn repo(&self) -> &str {
        &self.repo
    }

    /// `<owner>/<repo>`, the part of every cache key that names the repo.
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }

    /// The requester's own token.
    pub fn auth(&self) -> &Auth {
        &self.auth
    }

    pub fn default_branch(&self) -> &str {
        &self.default_branch
    }
}

#[derive(Serialize, Deserialize)]
struct Memo {
    ok: bool,
    /// GitHub's own `<owner>/<repo>`, whatever casing the request used.
    full_name: String,
    default_branch: String,
}

/// GitHub's characters for owner and repo names; anything else can't be a repo.
pub fn valid_name(s: &str) -> bool {
    !s.is_empty()
        && s != "."
        && s != ".."
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// How long a login's permission on a repo is remembered.
const PERMISSION_TTL: u64 = 300;

fn permission_key(repo: &str, login: &str) -> String {
    format!("permission2:{repo}:{login}")
}

/// `login`'s permission on `repo` as it is remembered, without asking GitHub.
pub(crate) async fn cached_permission<P: Platform>(
    p: &P,
    repo: &str,
    login: &str,
) -> Option<String> {
    p.cache_get(&permission_key(repo, login)).await
}

/// `login`'s permission on `repo` as GitHub answers it with the installation's token
/// `auth`, `none` for a 404, remembered for 5 minutes; `None` when GitHub doesn't answer,
/// which is never remembered.
pub async fn permission<P: Platform>(
    p: &P,
    auth: &Auth,
    repo: &str,
    login: &str,
) -> Option<String> {
    if let Some(known) = cached_permission(p, repo, login).await {
        return Some(known);
    }
    ask(p, auth, repo, login).await
}

/// `login`'s permission on `repo` asked of GitHub, remembered when it answers.
pub async fn ask<P: Platform>(p: &P, auth: &Auth, repo: &str, login: &str) -> Option<String> {
    let known = permission_direct(p, auth, repo, login).await?;
    p.cache_put(&permission_key(repo, login), &known, PERMISSION_TTL)
        .await;
    Some(known)
}

/// `login`'s permission on `repo` asked of GitHub every time, remembering nothing
/// (`object-reads`); `None` when GitHub doesn't answer.
pub async fn permission_direct<P: GitHub>(
    p: &P,
    auth: &Auth,
    repo: &str,
    login: &str,
) -> Option<String> {
    let path = format!("/repos/{repo}/collaborators/{login}/permission");
    match gh::get(p, auth, &path).await {
        (200, v) => Some(v["permission"].as_str()?.to_string()),
        (404, _) => Some("none".to_string()),
        _ => None,
    }
}

/// The hash under which a token's memos are kept, so no cache key holds a token.
pub fn token_hash(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// `Some` when `token` can read `owner/repo`, by `GET /repos/{o}/{r}`, memoised for 5
/// minutes in the Cache API under the token's hash. GitHub matches names in any casing;
/// the `Access` carries its canonical name, so every cache key is the same for all of them.
pub async fn check<P: Platform>(p: &P, token: &str, owner: &str, repo: &str) -> Option<Access> {
    if !valid_name(owner) || !valid_name(repo) {
        return None;
    }
    let key = format!("access:{}:{owner}/{repo}", token_hash(token));
    let memo = match p
        .cache_get(&key)
        .await
        .and_then(|m| serde_json::from_str(&m).ok())
    {
        Some(m) => m,
        None => {
            let (status, memo) = ask_repo(p, token, owner, repo).await;
            // Only GitHub's answer is remembered, not a failure to get one.
            if matches!(status, 200 | 403 | 404)
                && let Ok(value) = serde_json::to_string(&memo)
            {
                p.cache_put(&key, &value, MEMO_SECS).await;
            }
            memo
        }
    };
    access_of(memo, token)
}

/// `Some` when `token` can read `owner/repo`, asked of GitHub every time and remembered
/// nowhere (`object-reads`).
pub async fn check_direct<P: GitHub>(
    p: &P,
    token: &str,
    owner: &str,
    repo: &str,
) -> Option<Access> {
    if !valid_name(owner) || !valid_name(repo) {
        return None;
    }
    access_of(ask_repo(p, token, owner, repo).await.1, token)
}

/// `GET /repos/{o}/{r}` as `token`: GitHub's status and what it says of the repo.
async fn ask_repo<P: GitHub>(p: &P, token: &str, owner: &str, repo: &str) -> (u16, Memo) {
    let auth = Auth::User(token.into());
    let (status, v) = gh::get(p, &auth, &format!("/repos/{owner}/{repo}")).await;
    let full_name = v["full_name"].as_str().unwrap_or_default();
    let canonical = full_name
        .split_once('/')
        .is_some_and(|(o, r)| valid_name(o) && valid_name(r));
    let memo = Memo {
        ok: status == 200 && canonical,
        full_name: full_name.into(),
        default_branch: v["default_branch"].as_str().unwrap_or("main").into(),
    };
    (status, memo)
}

fn access_of(memo: Memo, token: &str) -> Option<Access> {
    if !memo.ok {
        return None;
    }
    let (owner, repo) = memo.full_name.split_once('/')?;
    Some(Access {
        owner: owner.into(),
        repo: repo.into(),
        auth: Auth::User(token.into()),
        default_branch: memo.default_branch,
        fresh: None,
    })
}
