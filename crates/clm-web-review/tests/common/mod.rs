//! What this crate's tests share: the fake GitHub and platform live in
//! `clm_web_platform::fake`; what calls this crate's routes stays here, as the extension
//! trait [`FakeExt`] and the request helpers. [`FakeExt::handle`] stands in for the router
//! (`web.worker`'s): it makes the access check the router makes, runs the body parsers it
//! runs before any request to GitHub, and calls the route function. It makes no session or
//! origin check; those are the router's and its tests'.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};

use clm_web_platform::access;
use clm_web_platform::http::{Request, Response, percent_decode};
use clm_web_review::{approve, branches, feedback, review};

pub use clm_web_platform::fake::*;

/// The route functions on [`Fake`], a foreign type, so an extension trait.
pub trait FakeExt {
    /// Calls the route function `req` names, asserting it stayed within 50 subrequests.
    /// The `cookie` header holds `<token>` or `<token>|<login>`.
    fn handle(&self, req: Request) -> Response;
    /// A cookie value for `token`, signed in as `clunny`.
    fn cookie(&self, token: &str) -> String;
    /// A cookie value for `token`, signed in as `login`.
    fn cookie_as(&self, token: &str, login: &str) -> String;
    /// A user who can read `repos`.
    fn user(&self, token: &str, repos: &[&str]) -> String;
}

impl FakeExt for Fake {
    fn handle(&self, req: Request) -> Response {
        let before = self.gh.calls.borrow().len();
        let res = block_on(route(self, &req));
        let used = self.gh.calls.borrow().len() - before;
        assert!(used <= 50, "{used} subrequests");
        res
    }

    fn cookie(&self, token: &str) -> String {
        self.cookie_as(token, "clunny")
    }

    fn cookie_as(&self, token: &str, login: &str) -> String {
        format!("{token}|{login}")
    }

    fn user(&self, token: &str, repos: &[&str]) -> String {
        for r in repos {
            self.gh.grant(token, r);
        }
        self.cookie(token)
    }
}

/// The requirement IDs of the default branch's tip, as the router's snapshot gives them.
async fn graph(p: &Fake, access: &access::Access) -> Option<BTreeSet<String>> {
    let repo = access.full_name();
    let tip = clm_web_platform::ledger::tip(p, access, access.default_branch()).await?;
    let files = clm_web_platform::files::spec_files(p, access.auth(), &repo, &tip, None).await?;
    let (graph, _) = clm_model::load_files(&files);
    Some(graph.requirements().map(|r| r.id.clone()).collect())
}

async fn route(p: &Fake, req: &Request) -> Response {
    let cookie = req.header("cookie").unwrap_or_default();
    let (token, login) = cookie.split_once('|').unwrap_or((cookie, "clunny"));
    let parts: Vec<&str> = req.path.trim_start_matches('/').split('/').collect();
    let ["api", "repos", owner, repo, rest @ ..] = parts.as_slice() else {
        return Response::empty(404);
    };
    // Bodies are checked before any request to GitHub, as the router does.
    enum Body {
        None,
        Pin(String),
        Comment(feedback::Comment),
        Request(feedback::Request),
    }
    let body = match (req.method.as_str(), rest) {
        ("POST", ["branches", .., "approve"]) => match approve::fingerprint(&req.body) {
            Some(pin) => Body::Pin(pin),
            None => return Response::empty(422),
        },
        ("POST", ["branches", .., "comments"]) => match feedback::parse_comment(&req.body) {
            Ok(c) => Body::Comment(c),
            Err(res) => return res,
        },
        ("POST", ["branches", .., "request-changes"]) => match feedback::parse_request(&req.body) {
            Ok(r) => Body::Request(r),
            Err(res) => return res,
        },
        _ => Body::None,
    };
    let Some(access) = access::check(p, token, owner, repo).await else {
        return Response::empty(404);
    };
    let fresh = req.param("fresh").filter(|_| req.method == "GET");
    let access = access.fresh(fresh.as_deref());
    let name = |parts: &[&str]| percent_decode(&parts.join("/"));
    match (req.method.as_str(), rest, body) {
        ("GET", ["branches"], _) => match branches::list(p, &access).await {
            Some(list) => Response::json(&list),
            None => Response::empty(502),
        },
        ("GET", ["branches", n @ .., "review"], _) => review::branch(p, &access, &name(n)).await,
        ("POST", ["branches", n @ .., "approve"], Body::Pin(pin)) => {
            approve::branch(p, &access, login, &name(n), &pin).await
        }
        ("POST", ["branches", n @ .., "comments"], Body::Comment(comment)) => {
            let graph = graph(p, &access);
            feedback::comment(p, &access, login, &name(n), comment, graph).await
        }
        ("POST", ["branches", n @ .., "request-changes"], Body::Request(request)) => {
            feedback::request_changes(p, &access, login, &name(n), request).await
        }
        _ => Response::empty(404),
    }
}

pub fn get(path: &str, cookie: &str) -> Request {
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    let mut headers = BTreeMap::new();
    if !cookie.is_empty() {
        headers.insert("cookie".to_string(), cookie.to_string());
    }
    Request {
        method: "GET".into(),
        path: path.into(),
        query: query.into(),
        headers,
        body: Vec::new(),
        origin: ORIGIN.into(),
    }
}

pub fn post(path: &str, cookie: &str, body: &[u8]) -> Request {
    let mut req = get(path, cookie);
    req.method = "POST".into();
    req.body = body.to_vec();
    req.headers.insert("origin".into(), ORIGIN.into());
    req
}
