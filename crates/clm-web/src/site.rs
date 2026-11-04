//! The public site, behind sign-in until launch (`site-behind-sign-in`): the assets'
//! answer for a signed-in invited person, the sign-in page for anyone else.

use crate::auth;
use clm_web_platform::http::{Request, Response};
use clm_web_platform::platform::Platform;

/// The page a request without a valid invited session gets: it holds nothing of the site.
const SIGN_IN: &str = "<!doctype html><meta charset=utf-8><meta name=viewport content=\"width=device-width\"><title>CLM</title><h1>Sign in</h1><p>This site is private for now. <a href=\"/auth/login\">Sign in with GitHub</a>.</p>";

/// The answer for a path the router doesn't know: the site, or 404 for the paths the
/// Worker reserves (`/_clm/*`, which is never served raw, `/sw.js` and `/app/*`, which the
/// assets serve directly).
pub(crate) async fn serve<P: Platform>(p: &P, req: &Request) -> Response {
    let path = req.path.as_str();
    if path == "/app" {
        return Response::redirect("/app/");
    }
    let reserved = ["/_clm/", "/app/", "/auth/"];
    if path == "/sw.js" || path == "/webhook" || reserved.iter().any(|r| path.starts_with(r)) {
        return Response::empty(404);
    }
    let authed = match auth::session(p, req).await {
        Ok(authed) => authed,
        Err(_) => {
            return Response::html(401, SIGN_IN).with_header("cache-control", "private".into());
        }
    };
    let mut res = p.fetch_asset(req).await;
    if res.status == 404 {
        let page = Request {
            method: "GET".into(),
            path: "/404.html".into(),
            ..Request::default()
        };
        let missing = p.fetch_asset(&page).await;
        res = if missing.status == 200 {
            Response {
                status: 404,
                ..missing
            }
        } else {
            Response::empty(404)
        };
    }
    res.headers
        .retain(|(k, _)| !k.eq_ignore_ascii_case("cache-control"));
    authed.respond(res.with_header("cache-control", "private".into()))
}
