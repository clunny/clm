//! Fakes of Cloudflare and GitHub for the Worker's core: KV with a write counter, the Cache
//! API, a clock, secrets, and a GitHub that serves repos held in memory and counts every
//! call, so each test can assert the subrequest budget.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::http::{Request, Response, Upgrade};
use crate::platform::{
    Assets, Auth, Cache, Clock, GhRequest, GhResponse, GitHub, Kv, Live, Log, Secrets,
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub use futures_executor::block_on;

/// 2027-01-15T08:00:00Z.
pub const NOW: u64 = 1_800_000_000;
pub const ORIGIN: &str = "https://clm-app.dev";
pub const WEBHOOK_SECRET: &str = "whsec";
/// GraphQL cuts a blob's `text` at this many bytes and sets `isTruncated`.
const TRUNCATE: usize = 50_000;

pub type FileMap = BTreeMap<String, Vec<u8>>;

#[derive(Debug, Clone, Default)]
pub struct FakePull {
    pub number: u64,
    pub state: String,
    pub title: String,
    pub base_ref: String,
    pub base_sha: String,
    pub head_ref: String,
    pub head_sha: String,
    /// Its head is a fork's branch.
    pub fork: bool,
}

#[derive(Debug, Clone, Default)]
pub struct FakeRepo {
    pub default_branch: String,
    /// Branch name to commit SHA.
    pub branches: BTreeMap<String, String>,
    /// Commit SHA to its files.
    pub commits: BTreeMap<String, FileMap>,
    /// `impl/*` branches: (name without `impl/`, tip SHA, committer time).
    pub claims: Vec<(String, String, u64)>,
    pub pulls: Vec<FakePull>,
    pub events: Value,
    pub installation: u64,
    /// The compare API's `status` for `<a>...<b>`; `ahead` when absent.
    pub compare_status: BTreeMap<String, String>,
    /// The compare API's `merge_base_commit` for `<a>...<b>`; `<a>` when absent.
    pub compare_base: BTreeMap<String, String>,
    /// Open PRs' web pages: (head branch, number, URL).
    pub pages: Vec<(String, u64, String)>,
    /// Branches' tip commit times, for the refs query; `NOW` when absent.
    pub ref_times: BTreeMap<String, u64>,
    /// Commits whose Trees API listing GitHub reports as truncated.
    pub truncated_trees: BTreeSet<String>,
    /// Each path's mode in tree listings, when it isn't `100644`: `120000` for a symlink,
    /// whose bytes are its target.
    pub modes: BTreeMap<String, String>,
    /// Blobs the Git Data API created, by SHA.
    pub blobs: BTreeMap<String, Vec<u8>>,
    /// Trees the Git Data API created, by SHA.
    pub trees: BTreeMap<String, FileMap>,
    /// Each commit the Git Data API created, to its parents.
    pub parents: BTreeMap<String, Vec<String>>,
    /// GitHub fails the `Signers` query, which asks whether a tip holds
    /// `docs/allowed_signers`.
    pub signers_fail: bool,
    /// Each login's permission on the repo, as the collaborators API answers it; a login
    /// absent here gets 404.
    pub permissions: BTreeMap<String, String>,
    /// Branches whose ref lookup GitHub fails with a 502.
    pub ref_fails: BTreeSet<String>,
}

impl FakeRepo {
    /// A repo whose default branch `main` is at `sha` with `files`.
    pub fn new(sha: &str, files: FileMap) -> FakeRepo {
        FakeRepo {
            default_branch: "main".into(),
            branches: [("main".to_string(), sha.to_string())].into(),
            commits: [(sha.to_string(), files)].into(),
            events: json!([]),
            installation: 9,
            ..FakeRepo::default()
        }
    }
}

/// An App installation: its ID and the repos it covers.
pub type Installation = (u64, Vec<String>);

#[derive(Default)]
pub struct FakeGitHub {
    pub repos: RefCell<BTreeMap<String, FakeRepo>>,
    /// User token to the repos it can read.
    pub readers: RefCell<BTreeMap<String, BTreeSet<String>>>,
    /// User token to its installations.
    pub installations: RefCell<BTreeMap<String, Vec<Installation>>>,
    /// OAuth code to (token, refresh token).
    pub codes: RefCell<BTreeMap<String, (String, String)>>,
    /// User token to the login `GET /user` names; `clunny` when absent, and a 502 when
    /// empty.
    pub logins: RefCell<BTreeMap<String, String>>,
    pub calls: RefCell<Vec<GhRequest>>,
    /// GraphQL query names (`Signers`, ...) answered with a null `repository`.
    pub graphql_null: RefCell<BTreeSet<String>>,
}

pub fn blob_sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)[..20]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn ok(body: Value) -> GhResponse {
    GhResponse {
        status: 200,
        body: body.to_string(),
    }
}

fn status(code: u16) -> GhResponse {
    GhResponse {
        status: code,
        body: json!({"message": "Not Found"}).to_string(),
    }
}

impl FakeGitHub {
    pub fn add_repo(&self, name: &str, repo: FakeRepo) {
        self.repos.borrow_mut().insert(name.into(), repo);
    }

    pub fn grant(&self, token: &str, repo: &str) {
        self.readers
            .borrow_mut()
            .entry(token.into())
            .or_default()
            .insert(repo.into());
    }

    /// Paths of every file fetched as a blob, through GraphQL or the blob API.
    pub fn fetched_paths(&self) -> Vec<String> {
        let mut out = Vec::new();
        for call in self.calls.borrow().iter() {
            if let Some(body) = &call.body {
                let v: Value = serde_json::from_str(body).unwrap_or_default();
                if v["query"]
                    .as_str()
                    .is_some_and(|q| q.contains("query Blobs"))
                {
                    for (k, e) in v["variables"].as_object().unwrap() {
                        if k.starts_with('e') {
                            let e = e.as_str().unwrap();
                            out.push(e.split_once(':').unwrap().1.to_string());
                        }
                    }
                }
            }
        }
        out
    }

    /// The calls that change something on GitHub (not reads, not token exchanges).
    pub fn writes(&self) -> Vec<GhRequest> {
        self.calls
            .borrow()
            .iter()
            .filter(|c| c.method != "GET")
            .filter(|c| !c.url.ends_with("/graphql"))
            .filter(|c| !c.url.contains("/access_token"))
            .cloned()
            .collect()
    }

    fn can_read(&self, auth: &Auth, repo: &str) -> bool {
        match auth {
            Auth::User(t) => self
                .readers
                .borrow()
                .get(t)
                .is_some_and(|r| r.contains(repo)),
            Auth::Installation(t) => self
                .repos
                .borrow()
                .get(repo)
                .is_some_and(|r| *t == format!("inst-{}", r.installation)),
            _ => false,
        }
    }

    fn route(&self, req: &GhRequest) -> GhResponse {
        if let Some(rest) = req.url.strip_prefix("https://github.com/") {
            return self.oauth(rest, req);
        }
        let path = req.url.strip_prefix("https://api.github.com").unwrap();
        let (path, query) = path.split_once('?').unwrap_or((path, ""));
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        match (req.method, parts.as_slice()) {
            ("POST", ["graphql"]) => self.graphql(req),
            ("GET", ["user"]) => match &req.auth {
                Auth::User(t) => match self.logins.borrow().get(t).map(String::as_str) {
                    Some("") => status(502),
                    login => ok(json!({"login": login.unwrap_or("clunny")})),
                },
                _ => status(401),
            },
            ("GET", ["user", "installations"]) => self.user_installations(&req.auth),
            ("GET", ["user", "installations", id, "repositories"]) => {
                self.installation_repos(&req.auth, id, query)
            }
            ("POST", ["app", "installations", id, "access_tokens"]) => {
                assert!(matches!(req.auth, Auth::App(_)), "{req:?}");
                ok(json!({"token": format!("inst-{id}"), "expires_at": "2027-01-15T09:00:00Z"}))
            }
            (_, ["repos", o, r, rest @ ..]) => {
                // GitHub matches owner and repo names case-insensitively.
                let asked = format!("{o}/{r}");
                let name = self
                    .repos
                    .borrow()
                    .keys()
                    .find(|k| k.eq_ignore_ascii_case(&asked))
                    .cloned()
                    .unwrap_or(asked);
                if let (["installation"], Auth::App(_)) = (rest, &req.auth) {
                    return match self.repos.borrow().get(&name) {
                        Some(repo) => ok(json!({"id": repo.installation})),
                        None => status(404),
                    };
                }
                if !self.can_read(&req.auth, &name) {
                    return status(404);
                }
                if let ("POST" | "PATCH", ["git", ..]) = (req.method, rest) {
                    let mut repos = self.repos.borrow_mut();
                    return git_write(req, repos.get_mut(&name).unwrap(), &rest[1..]);
                }
                let repos = self.repos.borrow();
                let Some(repo) = repos.get(&name) else {
                    return status(404);
                };
                self.repo_route(req, &name, repo, rest, query)
            }
            _ => panic!("fake GitHub has no route for {} {}", req.method, req.url),
        }
    }

    fn oauth(&self, rest: &str, req: &GhRequest) -> GhResponse {
        assert_eq!(rest, "login/oauth/access_token");
        let body: Value = serde_json::from_str(req.body.as_deref().unwrap()).unwrap();
        if let Some(refresh) = body["refresh_token"].as_str() {
            return ok(json!({
                "access_token": format!("{refresh}-renewed"),
                "refresh_token": format!("{refresh}-next"),
                "expires_in": 28800,
            }));
        }
        match self.codes.borrow().get(body["code"].as_str().unwrap()) {
            // An App with user-token expiry turned off sends neither.
            Some((token, refresh)) if refresh.is_empty() => ok(json!({"access_token": token})),
            Some((token, refresh)) => ok(json!({
                "access_token": token,
                "refresh_token": refresh,
                "expires_in": 28800,
            })),
            None => ok(json!({"error": "bad_verification_code"})),
        }
    }

    fn user_installations(&self, auth: &Auth) -> GhResponse {
        let Auth::User(t) = auth else {
            return status(401);
        };
        let ids: Vec<Value> = self
            .installations
            .borrow()
            .get(t)
            .into_iter()
            .flatten()
            .map(|(id, _)| json!({"id": id}))
            .collect();
        ok(json!({"total_count": ids.len(), "installations": ids}))
    }

    fn installation_repos(&self, auth: &Auth, id: &str, query: &str) -> GhResponse {
        let Auth::User(t) = auth else {
            return status(401);
        };
        let page: usize = query
            .split('&')
            .find_map(|p| p.strip_prefix("page="))
            .map_or(1, |p| p.parse().unwrap());
        let installs = self.installations.borrow();
        let names: Vec<&String> = installs
            .get(t)
            .into_iter()
            .flatten()
            .filter(|(i, _)| i.to_string() == id)
            .flat_map(|(_, r)| r)
            .collect();
        let repos = self.repos.borrow();
        let list: Vec<Value> = names
            .iter()
            .skip((page - 1) * 100)
            .take(100)
            .map(|n| {
                let (o, r) = n.split_once('/').unwrap();
                let branch = repos
                    .get(*n)
                    .map_or("main".into(), |r| r.default_branch.clone());
                json!({"full_name": n, "name": r, "owner": {"login": o}, "default_branch": branch})
            })
            .collect();
        ok(json!({"total_count": names.len(), "repositories": list}))
    }

    fn repo_route(
        &self,
        req: &GhRequest,
        name: &str,
        repo: &FakeRepo,
        rest: &[&str],
        query: &str,
    ) -> GhResponse {
        let commit = |sha: &str| repo.commits.get(sha).cloned();
        match (req.method, rest) {
            ("GET", []) => ok(json!({
                "full_name": name,
                "default_branch": repo.default_branch,
                "private": true,
            })),
            ("GET", ["git", "ref", "heads", branch @ ..]) => {
                if repo.ref_fails.contains(&branch.join("/")) {
                    return status(502);
                }
                match repo.branches.get(&branch.join("/")) {
                    Some(sha) => ok(json!({"object": {"sha": sha}})),
                    None => status(404),
                }
            }
            ("GET", ["git", "matching-refs", prefix @ ..]) => {
                let prefix = prefix.join("/");
                let refs: Vec<Value> = repo
                    .branches
                    .iter()
                    .map(|(b, sha)| (format!("refs/heads/{b}"), sha))
                    .filter(|(r, _)| r.starts_with(&format!("refs/{prefix}")))
                    .map(|(r, sha)| json!({"ref": r, "object": {"sha": sha, "type": "commit"}}))
                    .collect();
                ok(Value::Array(refs))
            }
            ("GET", ["pulls"]) => {
                let head = query
                    .split('&')
                    .find_map(|p| p.strip_prefix("head="))
                    .map(|h| h.replace("%2F", "/").replace("%3A", ":"));
                let pages: Vec<Value> = repo
                    .pages
                    .iter()
                    .filter(|(branch, _, _)| {
                        head.as_deref()
                            == Some(&format!("{}:{branch}", name.split('/').next().unwrap()))
                    })
                    .map(|(_, n, url)| json!({"number": n, "html_url": url}))
                    .collect();
                ok(Value::Array(pages))
            }
            ("GET", ["git", "trees", sha]) => match commit(sha) {
                Some(files) => {
                    let tree: Vec<Value> = files
                        .iter()
                        .map(|(p, b)| {
                            let mode = repo.modes.get(p).map_or("100644", String::as_str);
                            json!({"path": p, "mode": mode, "type": "blob", "sha": blob_sha(b), "size": b.len()})
                        })
                        .collect();
                    let truncated = repo.truncated_trees.contains(*sha);
                    ok(json!({"sha": sha, "truncated": truncated, "tree": tree}))
                }
                None => status(404),
            },
            ("GET", ["git", "blobs", sha]) => {
                let found = repo
                    .commits
                    .values()
                    .flat_map(|f| f.values())
                    .find(|b| blob_sha(b) == *sha);
                match found {
                    Some(b) => ok(json!({"content": STANDARD.encode(b), "encoding": "base64"})),
                    None => status(404),
                }
            }
            ("GET", ["compare", range]) => {
                let (a, b) = range.split_once("...").unwrap();
                let (Some(before), Some(after)) = (commit(a), commit(b)) else {
                    return status(404);
                };
                let mut files = Vec::new();
                for (p, bytes) in &after {
                    match before.get(p) {
                        None => files.push(json!({"filename": p, "status": "added"})),
                        Some(old) if old != bytes => {
                            files.push(json!({"filename": p, "status": "modified"}))
                        }
                        _ => {}
                    }
                }
                for p in before.keys().filter(|p| !after.contains_key(*p)) {
                    files.push(json!({"filename": p, "status": "removed"}));
                }
                files.truncate(300);
                let status = repo
                    .compare_status
                    .get(*range)
                    .map_or("ahead", String::as_str);
                let base = repo.compare_base.get(*range).map_or(a, String::as_str);
                ok(json!({"status": status, "merge_base_commit": {"sha": base}, "files": files}))
            }
            ("GET", ["events"]) => ok(repo.events.clone()),
            ("GET", ["collaborators", login, "permission"]) => {
                assert!(
                    matches!(req.auth, Auth::Installation(_)),
                    "asked with the installation's token"
                );
                match repo.permissions.get(*login) {
                    Some(p) => ok(json!({"permission": p})),
                    None => status(404),
                }
            }
            _ => panic!("fake GitHub has no route for {} {}", req.method, req.url),
        }
    }

    fn graphql(&self, req: &GhRequest) -> GhResponse {
        let body: Value = serde_json::from_str(req.body.as_deref().unwrap()).unwrap();
        let query = body["query"].as_str().unwrap();
        let vars = body["variables"].as_object().cloned().unwrap_or_default();
        let var = |k: &str| vars[k].as_str().unwrap().to_string();
        if query.contains("query Specs") {
            let mut data = serde_json::Map::new();
            for i in 0.. {
                let Some(o) = vars.get(&format!("o{i}")) else {
                    break;
                };
                let name = format!("{}/{}", o.as_str().unwrap(), var(&format!("n{i}")));
                let has = self.can_read(&req.auth, &name)
                    && self.repos.borrow().get(&name).is_some_and(|r| {
                        let head = &r.branches[&r.default_branch];
                        r.commits[head].contains_key("specs/SPEC.md")
                    });
                let object = if has {
                    json!({"__typename": "Blob"})
                } else {
                    Value::Null
                };
                data.insert(format!("r{i}"), json!({"object": object}));
            }
            return ok(json!({"data": data}));
        }
        let name = format!("{}/{}", var("owner"), var("name"));
        let nulled = self
            .graphql_null
            .borrow()
            .iter()
            .any(|q| query.contains(&format!("query {q}")));
        if nulled || !self.can_read(&req.auth, &name) {
            return ok(json!({"data": {"repository": null}}));
        }
        let repos = self.repos.borrow();
        let repo = &repos[&name];
        let data = if query.contains("query Signers") {
            if repo.signers_fail {
                return status(502);
            }
            // Signed when the tip holds an entry at the path, a file or a directory.
            let (sha, path) = var("expression")
                .split_once(':')
                .map(|(s, p)| (s.to_string(), p.to_string()))
                .unwrap();
            let held = repo.commits.get(&sha).is_some_and(|f| {
                f.keys()
                    .any(|k| *k == path || k.starts_with(&format!("{path}/")))
            });
            let object = if held {
                json!({"__typename": "Blob"})
            } else {
                Value::Null
            };
            json!({"object": object})
        } else if query.contains("query Blobs") || query.contains("query Ledgers") {
            let mut fields = serde_json::Map::new();
            for (k, e) in &vars {
                let Some(i) = k.strip_prefix('e') else {
                    continue;
                };
                let (sha, path) = e.as_str().unwrap().split_once(':').unwrap();
                let value = match repo.commits.get(sha).and_then(|f| f.get(path)) {
                    Some(b) => match std::str::from_utf8(b) {
                        Ok(t) if t.len() > TRUNCATE => json!({
                            "text": &t[..TRUNCATE],
                            "isBinary": false,
                            "isTruncated": true,
                            "oid": blob_sha(b),
                        }),
                        Ok(t) => json!({"text": t, "isBinary": false, "isTruncated": false}),
                        Err(_) => json!({"text": null, "isBinary": true, "isTruncated": false}),
                    },
                    None => Value::Null,
                };
                fields.insert(format!("f{i}"), value);
            }
            Value::Object(fields)
        } else if query.contains("query Taken") {
            // Each `c<i>` a blob's text, each `d<i>` a tree's entries.
            let mut fields = serde_json::Map::new();
            for (k, e) in vars.iter().filter(|(k, _)| k.starts_with(['c', 'd'])) {
                let (sha, path) = e.as_str().unwrap().split_once(':').unwrap();
                let Some(files) = repo.commits.get(sha) else {
                    fields.insert(k.clone(), Value::Null);
                    continue;
                };
                let value = if k.starts_with('c') {
                    match files.get(path) {
                        Some(b) => json!({"text": std::str::from_utf8(b).unwrap()}),
                        None => Value::Null,
                    }
                } else {
                    let names: BTreeSet<&str> = files
                        .keys()
                        .filter_map(|f| f.strip_prefix(&format!("{path}/")))
                        .map(|rest| rest.split('/').next().unwrap())
                        .collect();
                    if names.is_empty() {
                        Value::Null
                    } else {
                        json!({"entries": names.iter().map(|n| json!({"name": n})).collect::<Vec<_>>()})
                    }
                };
                fields.insert(k.clone(), value);
            }
            Value::Object(fields)
        } else if query.contains("query Refs") {
            // GitHub lists refs in name order, at most `first` (100) a page, from the
            // `after` cursor. `impl/*` claims come from `claims`; other prefixes from
            // `branches`, at their `ref_times` or `NOW`.
            let prefix = var("prefix");
            let rest = prefix.strip_prefix("refs/heads/").unwrap();
            let mut claims = if rest == "impl/" {
                repo.claims.clone()
            } else {
                repo.branches
                    .iter()
                    .filter_map(|(b, sha)| {
                        let name = b.strip_prefix(rest)?;
                        let time = repo.ref_times.get(b).copied().unwrap_or(NOW);
                        Some((name.to_string(), sha.clone(), time))
                    })
                    .collect()
            };
            claims.sort_by(|a, b| a.0.cmp(&b.0));
            let first = query
                .split("first: ")
                .nth(1)
                .and_then(|s| s.split(|c: char| !c.is_ascii_digit()).next())
                .and_then(|n| n.parse::<usize>().ok())
                .expect("the claims query names `first`");
            assert!(first <= 100, "GitHub serves at most 100 refs a page");
            let start = vars
                .get("after")
                .and_then(Value::as_str)
                .map_or(0, |c| c.parse::<usize>().unwrap());
            let end = (start + first).min(claims.len());
            let nodes: Vec<Value> = claims[start..end]
                .iter()
                .map(|(item, sha, time)| {
                    json!({"name": item, "target": {"oid": sha, "committedDate": iso(*time)}})
                })
                .collect();
            json!({"refs": {
                "nodes": nodes,
                "pageInfo": {"hasNextPage": end < claims.len(), "endCursor": end.to_string()},
            }})
        } else if query.contains("query Pulls") {
            let nodes: Vec<Value> = repo
                .pulls
                .iter()
                .filter(|p| p.state == "open")
                .map(|p| {
                    assert!(
                        !query.contains("statusCheckRollup"),
                        "the App can't read checks"
                    );
                    json!({
                        "number": p.number,
                        "url": format!("https://github.com/{name}/pull/{}", p.number),
                        "title": p.title,
                        "headRefName": p.head_ref,
                        "baseRefName": p.base_ref,
                        "headRefOid": p.head_sha,
                        "isCrossRepository": p.fork,
                    })
                })
                .collect();
            json!({"pullRequests": {"nodes": nodes}})
        } else {
            panic!("fake GitHub doesn't know the query {query}");
        };
        ok(json!({"data": {"repository": data}}))
    }
}

fn created(sha: &str) -> GhResponse {
    GhResponse {
        status: 201,
        body: json!({"sha": sha}).to_string(),
    }
}

/// The Git Data API's writes: blobs, trees, commits and refs.
fn git_write(req: &GhRequest, repo: &mut FakeRepo, rest: &[&str]) -> GhResponse {
    let body: Value = serde_json::from_str(req.body.as_deref().unwrap()).unwrap();
    match (req.method, rest) {
        ("POST", ["blobs"]) => {
            assert_eq!(body["encoding"], "utf-8");
            let bytes = body["content"].as_str().unwrap().as_bytes().to_vec();
            let sha = blob_sha(&bytes);
            repo.blobs.insert(sha.clone(), bytes);
            created(&sha)
        }
        ("POST", ["trees"]) => {
            assert!(
                body.get("base_tree").is_none(),
                "a ledger's tree is its file alone"
            );
            let mut files = FileMap::new();
            for e in body["tree"].as_array().unwrap() {
                assert_eq!(
                    (e["mode"].as_str(), e["type"].as_str()),
                    (Some("100644"), Some("blob"))
                );
                let bytes = repo.blobs[e["sha"].as_str().unwrap()].clone();
                files.insert(e["path"].as_str().unwrap().into(), bytes);
            }
            let sha = format!("t{}", blob_sha(format!("{files:?}").as_bytes()));
            repo.trees.insert(sha.clone(), files);
            created(&sha)
        }
        ("POST", ["commits"]) => {
            let files = repo.trees[body["tree"].as_str().unwrap()].clone();
            let parents: Vec<String> = body["parents"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p.as_str().unwrap().to_string())
                .collect();
            let sha = format!("c{}", blob_sha(body.to_string().as_bytes()));
            repo.commits.insert(sha.clone(), files);
            repo.parents.insert(sha.clone(), parents);
            created(&sha)
        }
        ("POST", ["refs"]) => {
            let name = body["ref"]
                .as_str()
                .unwrap()
                .strip_prefix("refs/heads/")
                .unwrap();
            if repo.branches.contains_key(name) {
                return status(422);
            }
            repo.branches
                .insert(name.into(), body["sha"].as_str().unwrap().into());
            created(body["sha"].as_str().unwrap())
        }
        ("PATCH", ["refs", "heads", branch @ ..]) => {
            let name = branch.join("/");
            let sha = body["sha"].as_str().unwrap().to_string();
            let Some(old) = repo.branches.get(&name) else {
                return status(422);
            };
            let forward = repo.parents.get(&sha).is_some_and(|p| p.contains(old));
            if body["force"] != true && !forward {
                return status(422);
            }
            repo.branches.insert(name, sha.clone());
            ok(json!({"object": {"sha": sha}}))
        }
        _ => panic!("fake GitHub has no route for {} {}", req.method, req.url),
    }
}

/// A ledger branch `clm/ledger/<writer>` at commit `sha` holding `records`.
pub fn ledger(repo: &mut FakeRepo, writer: &str, sha: &str, records: &[Value]) {
    let text: String = records.iter().map(|r| format!("{r}\n")).collect();
    repo.branches
        .insert(format!("clm/ledger/{writer}"), sha.to_string());
    repo.commits
        .insert(sha.to_string(), files(&[("records.jsonl", &text)]));
}

/// The entries the fake's Trees API lists for `files`.
pub fn entries(files: &FileMap) -> Vec<clm_gate::Entry> {
    files
        .iter()
        .map(|(p, b)| clm_gate::Entry {
            mode: "100644".into(),
            kind: "blob".into(),
            id: blob_sha(b),
            path: p.as_bytes().to_vec(),
        })
        .collect()
}

/// `clm fingerprint` of `head` against `base`, over the entries the fake lists for them.
pub fn fingerprint_of(base: &FileMap, head: &FileMap) -> String {
    clm_gate::fingerprint_of(&entries(base), &entries(head))
}

/// RFC 3339 for Unix seconds, as the Worker writes it.
pub fn iso(t: u64) -> String {
    crate::install::rfc3339(t)
}

/// A request that reached a `Live` object.
#[derive(Debug, Clone)]
pub struct LiveCall {
    /// The object's name, `<owner>/<repo>`.
    pub name: String,
    pub method: String,
    pub path: String,
    /// It carried `Upgrade: websocket`.
    pub websocket: bool,
    pub body: String,
    /// How many KV writes had been made when it arrived.
    pub kv_writes_before: usize,
}

/// The `LIVE` binding: every request its objects received.
pub struct FakeLive {
    /// The Worker has the binding.
    pub bound: Cell<bool>,
    pub calls: RefCell<Vec<LiveCall>>,
}

impl Default for FakeLive {
    fn default() -> FakeLive {
        FakeLive {
            bound: Cell::new(true),
            calls: RefCell::default(),
        }
    }
}

pub struct Fake {
    pub live: FakeLive,
    /// Every line the Worker logged.
    pub logs: RefCell<Vec<String>>,
    pub kv: RefCell<BTreeMap<String, String>>,
    pub kv_writes: RefCell<Vec<String>>,
    pub cache: RefCell<BTreeMap<String, (String, u64)>>,
    /// Every value ever written to KV or the Cache API.
    pub stored: RefCell<Vec<String>>,
    pub now: Cell<u64>,
    pub secrets: BTreeMap<String, String>,
    pub random: Cell<u8>,
    pub gh: FakeGitHub,
    /// The build's static assets, by path: a content type and bytes each, the invited
    /// list (`["clunny"]`) among them.
    pub assets: RefCell<BTreeMap<String, (String, Vec<u8>)>>,
}

impl Default for Fake {
    fn default() -> Fake {
        let secrets = [
            ("GITHUB_APP_ID", "1".to_string()),
            ("GITHUB_CLIENT_ID", "client-id".into()),
            ("GITHUB_CLIENT_SECRET", "client-secret".into()),
            ("GITHUB_WEBHOOK_SECRET", WEBHOOK_SECRET.into()),
            ("COOKIE_KEY", STANDARD.encode([7u8; 32])),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
        Fake {
            live: FakeLive::default(),
            logs: RefCell::default(),
            kv: RefCell::default(),
            kv_writes: RefCell::default(),
            cache: RefCell::default(),
            stored: RefCell::default(),
            now: Cell::new(NOW),
            secrets,
            random: Cell::new(0),
            gh: FakeGitHub::default(),
            assets: RefCell::new(
                [(
                    "/_clm/invited.json".to_string(),
                    ("application/json".to_string(), br#"["clunny"]"#.to_vec()),
                )]
                .into_iter()
                .collect(),
            ),
        }
    }
}

impl Assets for Fake {
    async fn fetch_asset(&self, req: &Request) -> Response {
        match self.assets.borrow().get(&req.path) {
            Some((content_type, bytes)) => Response {
                status: 200,
                headers: vec![("content-type".into(), content_type.clone())],
                body: bytes.clone(),
                upgrade: None,
            },
            None => Response::empty(404),
        }
    }
}

impl Kv for Fake {
    async fn kv_get(&self, key: &str) -> Option<String> {
        self.kv.borrow().get(key).cloned()
    }
    async fn kv_put(&self, key: &str, value: &str) {
        self.kv_writes.borrow_mut().push(key.into());
        self.stored.borrow_mut().push(format!("{key} {value}"));
        self.kv.borrow_mut().insert(key.into(), value.into());
    }
}

impl Cache for Fake {
    async fn cache_get(&self, key: &str) -> Option<String> {
        let cache = self.cache.borrow();
        let (value, expires) = cache.get(key)?;
        (self.now.get() < *expires).then(|| value.clone())
    }
    async fn cache_put(&self, key: &str, value: &str, ttl_secs: u64) {
        self.stored.borrow_mut().push(format!("{key} {value}"));
        self.cache
            .borrow_mut()
            .insert(key.into(), (value.into(), self.now.get() + ttl_secs));
    }
}

impl GitHub for Fake {
    async fn send(&self, req: GhRequest) -> GhResponse {
        let res = self.gh.route(&req);
        self.gh.calls.borrow_mut().push(req);
        res
    }
}

impl Live for Fake {
    async fn live_forward(&self, name: &str, req: &Request) -> Option<Response> {
        if !self.live.bound.get() {
            return None;
        }
        self.live.calls.borrow_mut().push(LiveCall {
            name: name.into(),
            method: req.method.clone(),
            path: req.path.clone(),
            websocket: req.header("upgrade") == Some("websocket"),
            body: String::from_utf8_lossy(&req.body).into_owned(),
            kv_writes_before: self.kv_writes.borrow().len(),
        });
        let mut res = Response::empty(101);
        res.upgrade = Some(Upgrade::new("socket"));
        Some(res)
    }

    async fn live_post(&self, name: &str, path: &str, body: &str) -> Option<(u16, String)> {
        if !self.live.bound.get() {
            return None;
        }
        self.live.calls.borrow_mut().push(LiveCall {
            name: name.into(),
            method: "POST".into(),
            path: path.into(),
            websocket: false,
            body: body.into(),
            kv_writes_before: self.kv_writes.borrow().len(),
        });
        Some((204, String::new()))
    }
}

impl Log for Fake {
    fn log(&self, line: &str) {
        self.logs.borrow_mut().push(line.into());
    }
}

impl Clock for Fake {
    fn now(&self) -> u64 {
        self.now.get()
    }
}

impl Secrets for Fake {
    fn secret(&self, name: &str) -> Option<String> {
        self.secrets.get(name).cloned()
    }
    fn random(&self, buf: &mut [u8]) {
        for b in buf {
            self.random.set(self.random.get().wrapping_add(1));
            *b = self.random.get();
        }
    }
    async fn sign_rs256(&self, data: &[u8]) -> Option<Vec<u8>> {
        Some(Sha256::digest(data).to_vec())
    }
}

pub fn body_json(res: &Response) -> Value {
    serde_json::from_slice(&res.body)
        .unwrap_or_else(|e| panic!("{e}: {} {}", res.status, String::from_utf8_lossy(&res.body)))
}

pub fn files(entries: &[(&str, &str)]) -> FileMap {
    entries
        .iter()
        .map(|(p, t)| (p.to_string(), t.as_bytes().to_vec()))
        .collect()
}

/// Writes `files` under `dir`.
pub fn write_checkout(dir: &Path, files: &FileMap) {
    for (p, b) in files {
        let path = dir.join(p);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b).unwrap();
    }
}

pub fn node(deps: &str, reqs: &[&str], status: &str) -> String {
    let mut s = format!(
        "---\ntitle: T\nstatus: {status}\nowners: []\ndepends_on: [{deps}]\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n"
    );
    for r in reqs {
        s.push_str(&format!(
            "\n### Requirement: {r}\nIt SHALL {r}.\n\n#### Scenario: s\n- ok\n"
        ));
    }
    s
}

/// A small CLM repo: root, `a` (active, requirement `p`) and `b` (specd, depends on `a`,
/// requirement `q`).
pub fn clm_repo() -> FileMap {
    files(&[
        ("ARCHITECTURE.md", "# Arch\n"),
        ("specs/SPEC.md", &node("", &[], "active")),
        ("specs/a/SPEC.md", &node("", &["p"], "active")),
        ("specs/b/SPEC.md", &node("a", &["q"], "specd")),
    ])
}

/// HMAC-SHA256 signature header value for a webhook body.
pub fn signature(secret: &str, body: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body);
    let tag = mac.finalize().into_bytes();
    format!(
        "sha256={}",
        tag.iter().map(|b| format!("{b:02x}")).collect::<String>()
    )
}

/// The first 8 hex digits of requirement `id`'s hash in `files`: its version
/// (`versioned-markers`).
pub fn version_of(files: &FileMap, id: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    write_checkout(dir.path(), files);
    let (graph, _) = clm_model::load(dir.path());
    let (node, _) = id.split_once('/').unwrap();
    graph.nodes[node]
        .requirements
        .iter()
        .find(|r| r.id == id)
        .unwrap()
        .hash[..8]
        .to_string()
}

/// A test file citing each of `ids` at its version in `files`.
pub fn citing(files: &FileMap, ids: &[&str]) -> String {
    let mut s = String::from("#[test]\nfn t() {\n");
    for id in ids {
        s.push_str(&format!(
            "    // {}:{id}@{}\n",
            "req",
            version_of(files, id)
        ));
    }
    s.push_str("}\n");
    s
}
