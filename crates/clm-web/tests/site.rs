//! The public site is served only to a signed-in invited person (web.worker/site-behind-sign-in).

mod common;

use common::*;

fn site(fake: &Fake) {
    let mut assets = fake.assets.borrow_mut();
    assets.insert(
        "/".into(),
        ("text/html".into(), b"<h1>CLM home</h1>".to_vec()),
    );
    assets.insert(
        "/guide/quickstart".into(),
        ("text/html".into(), b"quickstart text".to_vec()),
    );
    assets.insert(
        "/fonts/inter.woff2".into(),
        ("font/woff2".into(), vec![0, 1, 2, 3]),
    );
    assets.insert(
        "/404.html".into(),
        ("text/html".into(), b"not here".to_vec()),
    );
}

#[test]
fn signed_in_gets_the_assets_private() {
    // req:web.worker/site-behind-sign-in@a4a23004
    let fake = Fake::default();
    site(&fake);
    let cookie = fake.cookie("tok");
    for (path, ctype, body) in [
        ("/", "text/html", b"<h1>CLM home</h1>".to_vec()),
        (
            "/guide/quickstart",
            "text/html",
            b"quickstart text".to_vec(),
        ),
        ("/fonts/inter.woff2", "font/woff2", vec![0, 1, 2, 3]),
    ] {
        let res = fake.handle(get(path, &cookie));
        assert_eq!(res.status, 200, "{path}");
        assert_eq!(res.header("content-type"), Some(ctype));
        assert_eq!(res.body, body);
        assert_eq!(res.header("cache-control"), Some("private"));
    }
}

#[test]
fn signed_out_gets_the_sign_in_page() {
    // req:web.worker/site-behind-sign-in@a4a23004
    let fake = Fake::default();
    site(&fake);
    let stranger = fake.cookie_as("tok", "mallory");
    for cookie in ["", "__Host-clm=garbage", stranger.as_str()] {
        for path in ["/", "/guide/quickstart", "/assets/site.js"] {
            let res = fake.handle(get(path, cookie));
            assert_eq!(res.status, 401, "{path}");
            let body = String::from_utf8(res.body).unwrap();
            assert!(body.contains("/auth/login"));
            assert!(!body.contains("CLM home") && !body.contains("quickstart text"));
        }
    }
}

#[test]
fn missing_page_and_bare_app_path() {
    // req:web.worker/site-behind-sign-in@a4a23004
    let fake = Fake::default();
    site(&fake);
    let cookie = fake.cookie("tok");
    let res = fake.handle(get("/guide/nothing-here", &cookie));
    assert_eq!(res.status, 404);
    assert_eq!(res.body, b"not here");
    assert_eq!(res.header("cache-control"), Some("private"));

    fake.assets.borrow_mut().remove("/404.html");
    let res = fake.handle(get("/guide/nothing-here", &cookie));
    assert_eq!((res.status, res.body.len()), (404, 0));

    for cookie in [cookie.as_str(), ""] {
        let res = fake.handle(get("/app", cookie));
        assert_eq!(res.status, 302);
        assert_eq!(res.header("location"), Some("/app/"));
    }
}

#[test]
fn reserved_paths_are_not_the_site() {
    // req:web.worker/site-behind-sign-in@a4a23004
    let fake = Fake::default();
    site(&fake);
    let cookie = fake.cookie("tok");
    for path in [
        "/_clm/invited.json",
        "/sw.js",
        "/app/x",
        "/auth/nope",
        "/webhook",
    ] {
        let res = fake.handle(get(path, &cookie));
        assert_eq!(res.status, 404, "{path}");
    }
}

#[test]
fn a_renewed_session_cookie_is_sent() {
    // req:web.worker/site-behind-sign-in@a4a23004
    let fake = Fake::default();
    site(&fake);
    let session = clm_web::session::Session {
        token: "old".into(),
        refresh: "r".into(),
        expires: fake.now.get() - 1,
        login: "clunny".into(),
    };
    let cookie = format!(
        "__Host-clm={}",
        block_on(clm_web::session::seal(&fake, &session))
    );
    let res = fake.handle(get("/", &cookie));
    assert_eq!(res.status, 200);
    assert!(res.headers.iter().any(|(k, _)| k == "set-cookie"));
}
