//! Paths the router doesn't answer (`reserved-paths`).

use clm_web_platform::http::{Request, Response};

/// `/app` redirects to `/app/`; any other path gets a bare 404, with no session read and
/// no asset fetched, so `/_clm/*` is never served raw and the Worker holds no page of the
/// site (the assets answer it directly, `web/site-served-directly`).
pub(crate) fn serve(req: &Request) -> Response {
    if req.path == "/app" {
        Response::redirect("/app/")
    } else {
        Response::empty(404)
    }
}
