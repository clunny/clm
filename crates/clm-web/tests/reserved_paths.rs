//! The Worker answers `/app` with a redirect and every other path its routes don't answer
//! with a bare 404 (web.worker/reserved-paths).

mod common;

use common::*;

fn assets(fake: &Fake) {
    let mut assets = fake.assets.borrow_mut();
    assets.insert(
        "/_clm/invited.json".into(),
        ("application/json".into(), b"[\"clunny\"]".to_vec()),
    );
    assets.insert(
        "/guide/quickstart".into(),
        ("text/html".into(), b"quickstart text".to_vec()),
    );
    assets.insert(
        "/404.html".into(),
        ("text/html".into(), b"not here".to_vec()),
    );
}

#[test]
fn bare_app_path() {
    // req:web.worker/reserved-paths@04959fbe
    let fake = Fake::default();
    let cookie = fake.cookie("tok");
    for cookie in [cookie.as_str(), ""] {
        let res = fake.handle(get("/app", cookie));
        assert_eq!(res.status, 302);
        assert_eq!(res.header("location"), Some("/app/"));
    }
}

#[test]
fn the_invited_list() {
    // req:web.worker/reserved-paths@04959fbe
    let fake = Fake::default();
    assets(&fake);
    let cookie = fake.cookie("tok");
    for cookie in [cookie.as_str(), ""] {
        let res = fake.handle(get("/_clm/invited.json", cookie));
        assert_eq!((res.status, res.body.len()), (404, 0));
    }
}

#[test]
fn a_path_the_assets_should_have_answered() {
    // req:web.worker/reserved-paths@04959fbe
    let fake = Fake::default();
    assets(&fake);
    let cookie = fake.cookie("tok");
    for cookie in [cookie.as_str(), ""] {
        for path in ["/guide/quickstart", "/", "/sw.js", "/app/x", "/auth/nope"] {
            let res = fake.handle(get(path, cookie));
            assert_eq!((res.status, res.body.len()), (404, 0), "{path}");
        }
    }
}
