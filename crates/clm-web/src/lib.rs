//! The `web` node's Cloudflare Worker (see specs/web/SPEC.md).
//!
//! Everything here is plain Rust that `cargo test` runs on the host, driven through the
//! traits in `clm_web_platform::platform`. The `worker` module, built only for `wasm32`, adapts
//! `workers-rs` to those traits and does nothing else.

pub mod auth;
mod capabilities;
mod invited;
mod proposals;
mod push;
mod replicas;
mod repos;
pub mod session;
mod site;
mod views;
mod webhook;
mod writes;

#[cfg(target_arch = "wasm32")]
mod worker;

/// The Durable Object classes, exported by the Worker's module so that `web/wrangler.toml`
/// can bind them as `LIVE` (`web.live`) and `REPLICA` (`web.worker.replica`).
#[cfg(target_arch = "wasm32")]
pub use clm_web_live::Live;
#[cfg(target_arch = "wasm32")]
pub use clm_web_replica::Replica;
pub use replicas::Replicas;

use clm_web_platform::access::{self, Access};
use clm_web_platform::http::{self, Request, Response};
use clm_web_platform::platform::Platform;
use clm_web_review::{approve, branches, feedback, review};

/// Handles one request the Worker is routed: `/api/*`, `/auth/*`, `/webhook`, and the
/// paths no route answers, which get a bare 404 (`site`).
pub async fn handle<P: Platform + Replicas>(p: &P, req: Request) -> Response {
    let parts: Vec<&str> = req.path.trim_start_matches('/').split('/').collect();
    match (req.method.as_str(), parts.as_slice()) {
        ("GET", ["auth", "login"]) => auth::login(p, &req),
        ("GET", ["auth", "callback"]) => auth::callback(p, &req).await,
        ("POST", ["webhook"]) => webhook::handle(p, &req).await,
        (_, ["api", ..]) => api(p, &req, &parts[1..]).await,
        _ => site::serve(&req),
    }
}

/// Every API route needs a session. The session cookie authorises writes, so a request
/// that writes must also come from the app's own origin, as must the live socket: a
/// WebSocket handshake isn't bound by CORS, and a browser always sends `Origin` on one
/// (`live-socket`).
async fn api<P: Platform + Replicas>(p: &P, req: &Request, parts: &[&str]) -> Response {
    let needs_origin = req.method != "GET" || matches!(parts, ["repos", _, _, "live"]);
    if needs_origin && req.header("origin") != Some(req.origin.as_str()) {
        return Response::empty(403);
    }
    let authed = match auth::session(p, req).await {
        Ok(a) => a,
        Err(res) => return res,
    };
    let res = match (req.method.as_str(), parts) {
        ("GET", ["push", "key"]) => push::key(p).await,
        ("GET", ["repos"]) => match repos::list(p, &authed.auth()).await {
            Some(list) => Response::json(&list),
            None => Response::empty(502),
        },
        (_, ["repos", owner, repo, rest @ ..]) => match checked_body(req, rest) {
            // A body that names nothing reviewed is refused before any request to GitHub.
            Err(res) => res,
            Ok(body) => match access::check(p, &authed.session.token, owner, repo).await {
                Some(access) => {
                    // Every route that reads live state is a GET (`fresh-live-state`).
                    let fresh = req.param("fresh").filter(|_| req.method == "GET");
                    let access = access.fresh(fresh.as_deref());
                    let login = &authed.session.login;
                    repo_route(p, req, &access, login, rest, body).await
                }
                None => Response::empty(404),
            },
        },
        _ => Response::empty(404),
    };
    authed.respond(res)
}

/// A write's body, checked before any request to GitHub.
enum Body {
    None,
    /// The fingerprint an approve request names as reviewed.
    Pin(String),
    Proposal(proposals::Proposal),
    Comment(feedback::Comment),
    Request(feedback::Request),
    /// A decision on a capability or an architecture requirement, naming the version seen.
    Decide(capabilities::Decide),
}

/// The body of an approve, a proposal, a comment, a change request or a decision, checked,
/// and 422 for one that names nothing reviewed or isn't what the route takes.
fn checked_body(req: &Request, parts: &[&str]) -> Result<Body, Response> {
    match (req.method.as_str(), parts) {
        ("POST", ["branches", .., "approve"]) => approve::fingerprint(&req.body)
            .map(Body::Pin)
            .ok_or_else(|| Response::empty(422)),
        ("POST", ["branches", .., "comments"]) => {
            feedback::parse_comment(&req.body).map(Body::Comment)
        }
        ("POST", ["branches", .., "request-changes"]) => {
            feedback::parse_request(&req.body).map(Body::Request)
        }
        ("POST", ["proposals"]) => proposals::parse(&req.body).map(Body::Proposal),
        ("POST", ["capabilities" | "architecture", _, "decide"]) => capabilities::parse(&req.body)
            .map(Body::Decide)
            .ok_or_else(|| Response::empty(422)),
        _ => Ok(Body::None),
    }
}

/// The routes for one repo, each taking the guard's proof of access; `login` is the
/// signed-in person's, and `body` a write's body, checked.
async fn repo_route<P: Platform + Replicas>(
    p: &P,
    req: &Request,
    access: &Access,
    login: &str,
    parts: &[&str],
    body: Body,
) -> Response {
    let fresh = views::is_fresh(req);
    match (req.method.as_str(), parts, body) {
        ("GET", ["snapshot"], _) => views::get(p, access, "snapshot", fresh).await,
        ("GET", ["live"], _) => live_socket(p, req, access).await,
        ("GET", ["push"], _) => push::state(p, req, access, login).await,
        ("POST", ["push"], _) => push::subscribe(p, req, access, login).await,
        ("DELETE", ["push"], _) => push::unsubscribe(p, req, access, login).await,
        ("GET", ["branches"], _) => json_or_502(branches::list(p, access).await),
        ("GET", ["implementation"], _) => views::implementation(p, access, login, fresh).await,
        ("POST", ["proposals"], Body::Proposal(proposal)) => {
            proposals::propose(p, access, login, proposal).await
        }
        ("DELETE", ["proposals", id], _) => proposals::withdraw(p, access, login, id).await,
        ("GET", [view @ ("capabilities" | "architecture")], _) => {
            views::get(p, access, view, fresh).await
        }
        (
            "POST",
            [which @ ("capabilities" | "architecture"), slug, "decide"],
            Body::Decide(decide),
        ) => {
            let layer = match *which {
                "capabilities" => capabilities::Layer::Capability,
                "architecture" => capabilities::Layer::Architecture,
                _ => return Response::empty(404),
            };
            let slug = http::percent_decode(slug);
            capabilities::decide(p, access, login, layer, &slug, decide).await
        }
        ("GET", ["branches", name @ .., "review"], _) => {
            review::branch(p, access, &http::percent_decode(&name.join("/"))).await
        }
        ("POST", ["branches", name @ .., "approve"], Body::Pin(pin)) => {
            let name = http::percent_decode(&name.join("/"));
            approve::branch(p, access, login, &name, &pin).await
        }
        ("POST", ["branches", name @ .., "comments"], Body::Comment(comment)) => {
            let name = http::percent_decode(&name.join("/"));
            let graph = async {
                let snap = views::fetch(p, access, "snapshot", false).await.ok()?;
                let nodes = snap["nodes"].as_array()?;
                let ids = nodes
                    .iter()
                    .flat_map(|n| n["requirements"].as_array().into_iter().flatten());
                Some(
                    ids.filter_map(|r| r["id"].as_str().map(String::from))
                        .collect(),
                )
            };
            feedback::comment(p, access, login, &name, comment, graph).await
        }
        ("POST", ["branches", name @ .., "request-changes"], Body::Request(request)) => {
            let name = http::percent_decode(&name.join("/"));
            feedback::request_changes(p, access, login, &name, request).await
        }
        _ => Response::empty(404),
    }
}

/// `GET .../live`: the request forwarded to the repo's `Live` object, whose 101 carries
/// the socket (`live-socket`): 426 without `Upgrade: websocket`, 503 without the binding.
async fn live_socket<P: Platform>(p: &P, req: &Request, access: &Access) -> Response {
    let websocket = req
        .header("upgrade")
        .is_some_and(|u| u.eq_ignore_ascii_case("websocket"));
    if !websocket {
        return Response::empty(426);
    }
    p.live_forward(&access.full_name(), req)
        .await
        .unwrap_or_else(|| Response::empty(503))
}

fn json_or_502<T: serde::Serialize>(value: Option<T>) -> Response {
    match value {
        Some(v) => Response::json(&v),
        None => Response::empty(502),
    }
}
