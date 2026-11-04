//! Sign-in through the GitHub App's user authorization flow, with tokens only in an
//! encrypted cookie.

mod common;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use clm_web::session::{Session, seal};
use clm_web_platform::http::Response;
use common::*;

fn set_cookies(res: &Response) -> Vec<&str> {
    res.headers
        .iter()
        .filter(|(k, _)| k == "set-cookie")
        .map(|(_, v)| v.as_str())
        .collect()
}

/// `name=value` of the first `Set-Cookie` for `name`.
fn cookie_pair<'a>(res: &'a Response, name: &str) -> Option<&'a str> {
    set_cookies(res)
        .into_iter()
        .find(|c| c.starts_with(&format!("{name}=")))
        .map(|c| c.split(';').next().unwrap())
}

/// Runs login, then the callback with `code`, returning the callback's response.
fn sign_in(fake: &Fake, code: &str) -> Response {
    let login = fake.handle(get("/auth/login", ""));
    assert_eq!(login.status, 302);
    let state_cookie = cookie_pair(&login, "__Host-clm-state").unwrap().to_string();
    let state = state_cookie.split_once('=').unwrap().1;
    fake.handle(get(
        &format!("/auth/callback?code={code}&state={state}"),
        &state_cookie,
    ))
}

#[test]
fn login_redirects_to_github_with_state() {
    // req:web.worker/sign-in@17761a38
    let fake = Fake::default();
    let res = fake.handle(get("/auth/login", ""));
    let location = res.header("location").unwrap();
    assert!(
        location.starts_with("https://github.com/login/oauth/authorize?client_id=client-id&"),
        "{location}"
    );
    let state = cookie_pair(&res, "__Host-clm-state").unwrap();
    assert!(location.contains(&format!("state={}", state.split_once('=').unwrap().1)));
}

#[test]
fn callback_sets_encrypted_cookie() {
    // req:web.worker/sign-in@17761a38
    let fake = Fake::default();
    fake.gh.codes.borrow_mut().insert(
        "c1".into(),
        ("tok-secret-123".into(), "ref-secret-456".into()),
    );
    let res = sign_in(&fake, "c1");
    assert_eq!(res.status, 302, "{res:?}");
    let cookie = set_cookies(&res)
        .into_iter()
        .find(|c| c.starts_with("__Host-clm="))
        .unwrap();
    for attr in ["HttpOnly", "Secure", "SameSite=Lax", "Path=/"] {
        assert!(cookie.contains(attr), "{cookie}");
    }
    // The cookie is a working session: the GitHub call carries the token.
    let pair = cookie.split(';').next().unwrap();
    let res = fake.handle(get("/api/repos", pair));
    assert_eq!(res.status, 200);
    let last = fake.gh.calls.borrow().last().cloned().unwrap();
    assert_eq!(
        last.auth,
        clm_web_platform::platform::Auth::User("tok-secret-123".into())
    );
    // Using it for a repo fills KV and the Cache API, still without the tokens.
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    fake.gh.grant("tok-secret-123", "o/a");
    fake.set_view("o/a", "implementation", serde_json::json!({"sha": "111"}));
    let res = fake.handle(get("/api/repos/o/a/implementation", pair));
    assert_eq!(res.status, 200);
    assert!(!fake.stored.borrow().is_empty());
    // Encrypted, not merely encoded: the decoded value doesn't hold the tokens either.
    let value = pair.split_once('=').unwrap().1;
    let decoded = URL_SAFE_NO_PAD.decode(value).unwrap();
    for secret in ["tok-secret-123", "ref-secret-456"] {
        assert!(!cookie.contains(secret));
        assert!(
            !decoded
                .windows(secret.len())
                .any(|w| w == secret.as_bytes()),
            "decoded cookie holds {secret}"
        );
        for value in fake.stored.borrow().iter() {
            assert!(!value.contains(secret), "stored: {value}");
        }
    }
}

#[test]
fn callback_rejects_wrong_state() {
    // req:web.worker/sign-in@17761a38
    let fake = Fake::default();
    fake.gh
        .codes
        .borrow_mut()
        .insert("c1".into(), ("t".into(), "r".into()));
    let res = fake.handle(get(
        "/auth/callback?code=c1&state=forged",
        "__Host-clm-state=real",
    ));
    assert_eq!(res.status, 400);
    assert!(cookie_pair(&res, "__Host-clm").is_none());
}

#[test]
fn tampered_cookie_is_401() {
    // req:web.worker/sign-in@17761a38
    let fake = Fake::default();
    let cookie = fake.cookie("t");
    assert_eq!(fake.handle(get("/api/repos", &cookie)).status, 200);
    let mut bytes = cookie.into_bytes();
    let i = bytes.len() - 5;
    bytes[i] = if bytes[i] == b'A' { b'B' } else { b'A' };
    let tampered = String::from_utf8(bytes).unwrap();
    assert_eq!(fake.handle(get("/api/repos", &tampered)).status, 401);
    assert_eq!(fake.handle(get("/api/repos", "")).status, 401);
    assert_eq!(
        fake.handle(get("/api/repos/o/a/snapshot", &tampered))
            .status,
        401
    );
}

#[test]
fn expired_session_refreshes() {
    // req:web.worker/sign-in@17761a38
    let fake = Fake::default();
    let session = Session {
        token: "old".into(),
        refresh: "r1".into(),
        expires: NOW - 1,
        login: "clunny".into(),
    };
    let cookie = format!("__Host-clm={}", block_on(seal(&fake, &session)));
    let res = fake.handle(get("/api/repos", &cookie));
    assert_eq!(res.status, 200);
    let renewed = cookie_pair(&res, "__Host-clm").expect("cookie re-set");
    let last = fake.gh.calls.borrow().last().cloned().unwrap();
    assert_eq!(
        last.auth,
        clm_web_platform::platform::Auth::User("r1-renewed".into())
    );
    assert_eq!(fake.handle(get("/api/repos", renewed)).status, 200);
    // The renewed tokens are kept nowhere else either.
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    fake.gh.grant("r1-renewed", "o/a");
    fake.set_view("o/a", "snapshot", serde_json::json!({"sha": "111"}));
    let res = fake.handle(get("/api/repos/o/a/snapshot", renewed));
    assert_eq!(res.status, 200);
    for secret in ["r1-renewed", "r1-next"] {
        for value in fake.stored.borrow().iter() {
            assert!(!value.contains(secret), "stored: {value}");
        }
    }
}

#[test]
fn token_without_expiry_never_refreshes() {
    // req:web.worker/sign-in@17761a38
    // An App with user-token expiry turned off gives no `expires_in` and no refresh token.
    let fake = Fake::default();
    fake.gh
        .codes
        .borrow_mut()
        .insert("c1".into(), ("forever".into(), String::new()));
    let res = sign_in(&fake, "c1");
    let cookie = cookie_pair(&res, "__Host-clm").unwrap().to_string();
    fake.now.set(NOW + 30 * 86_400);
    let res = fake.handle(get("/api/repos", &cookie));
    assert_eq!(res.status, 200);
    let last = fake.gh.calls.borrow().last().cloned().unwrap();
    assert_eq!(
        last.auth,
        clm_web_platform::platform::Auth::User("forever".into())
    );
}

/// Signs in as `login` with a fresh code, returning the callback's response.
fn sign_in_as(fake: &Fake, login: &str) -> Response {
    let token = format!("tok-{login}");
    fake.gh
        .codes
        .borrow_mut()
        .insert("c1".into(), (token.clone(), "r".into()));
    fake.gh.logins.borrow_mut().insert(token, login.into());
    sign_in(fake, "c1")
}

#[test]
fn invited_login_gets_the_session_cookie() {
    // req:web/invited-users@cfd18219
    let fake = Fake::default();
    let res = sign_in_as(&fake, "clunny");
    assert_eq!(res.status, 302, "{res:?}");
    let pair = cookie_pair(&res, "__Host-clm").expect("session cookie");
    assert_eq!(fake.handle(get("/api/repos", pair)).status, 200);
    // The login was asked of GitHub with the new token.
    assert!(
        fake.gh
            .calls
            .borrow()
            .iter()
            .any(|c| c.url == "https://api.github.com/user"
                && c.auth == clm_web_platform::platform::Auth::User("tok-clunny".into()))
    );
}

#[test]
fn callback_redirects_to_the_app() {
    // req:web.worker/sign-in-opens-app@bf3c8137
    let fake = Fake::default();
    let res = sign_in_as(&fake, "clunny");
    assert_eq!(res.status, 302, "{res:?}");
    assert_eq!(res.header("location"), Some("/app/"));
    assert!(cookie_pair(&res, "__Host-clm").is_some());
}

#[test]
fn logins_compare_without_case() {
    // req:web/invited-users@cfd18219
    // GitHub logins are case-insensitive, and `/user` gives the account's own casing.
    let fake = Fake::default();
    let res = sign_in_as(&fake, "Clunny");
    assert!(cookie_pair(&res, "__Host-clm").is_some(), "{res:?}");
}

#[test]
fn not_invited_login_gets_the_page_and_no_session() {
    // req:web/invited-users@cfd18219
    let fake = Fake::default();
    let res = sign_in_as(&fake, "someone-else");
    assert_eq!(res.status, 403);
    assert!(
        String::from_utf8_lossy(&res.body).contains("not invited"),
        "{res:?}"
    );
    assert!(cookie_pair(&res, "__Host-clm").is_none(), "{res:?}");
    // The state cookie is cleared.
    assert_eq!(
        cookie_pair(&res, "__Host-clm-state"),
        Some("__Host-clm-state=")
    );
    // Whatever cookie the browser has left, the API stays shut.
    assert_eq!(fake.handle(get("/api/repos", "")).status, 401);
    // Nothing of the token was kept.
    for value in fake.stored.borrow().iter() {
        assert!(!value.contains("tok-someone-else"), "stored: {value}");
    }
}

#[test]
fn not_invited_page_points_to_quickstart_and_early_access() {
    // req:web.worker/not-invited-page@82a29a2e
    let fake = Fake::default();
    let res = sign_in_as(&fake, "someone-else");
    assert_eq!(res.status, 403);
    let html = String::from_utf8_lossy(&res.body).to_lowercase();
    for want in [
        "href=\"https://github.com/clunny/clm/issues/new?template=early-access.yml\"",
        "href=\"/guide/quickstart\"",
        "href=\"/\"",
        "not invited",
        "is in early access",
        "optional",
    ] {
        assert!(html.contains(want), "missing {want}: {html}");
    }
    assert!(cookie_pair(&res, "__Host-clm").is_none(), "{res:?}");
}

#[test]
fn session_of_a_login_no_longer_invited_is_401() {
    // req:web/invited-users@cfd18219
    // Removing a login from the Contract revokes access at the next deploy.
    let fake = Fake::default();
    let cookie = fake.cookie_as("t", "someone-else");
    assert_eq!(fake.handle(get("/api/repos", &cookie)).status, 401);
    fake.assets.borrow_mut().insert(
        "/_clm/invited.json".into(),
        (
            "application/json".into(),
            br#"["clunny","someone-else"]"#.to_vec(),
        ),
    );
    assert_eq!(fake.handle(get("/api/repos", &cookie)).status, 200);
}

#[test]
fn github_failing_to_name_the_user_leaves_a_way_back() {
    // req:web/invited-users@cfd18219
    let fake = Fake::default();
    let res = sign_in_as(&fake, "");
    assert_eq!(res.status, 502);
    assert!(cookie_pair(&res, "__Host-clm").is_none(), "{res:?}");
    let body = String::from_utf8_lossy(&res.body);
    assert!(body.contains("href=\"/auth/login\""), "{body}");
}

#[test]
fn without_the_invited_list_nobody_gets_in() {
    // req:web/invited-users@cfd18219
    let fake = Fake::default();
    fake.assets.borrow_mut().remove("/_clm/invited.json");
    let res = sign_in_as(&fake, "clunny");
    assert!(cookie_pair(&res, "__Host-clm").is_none(), "{res:?}");
    assert_eq!(
        fake.handle(get("/api/repos", &fake.cookie("t"))).status,
        401
    );
}

#[test]
fn cookie_from_before_logins_is_401() {
    // req:web/invited-users@cfd18219
    // Sessions sealed before the login was kept must sign in again, and be checked.
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Nonce};
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let fake = Fake::default();
    let cipher = Aes256Gcm::new_from_slice(&[7u8; 32]).unwrap();
    let nonce = [1u8; 12];
    let plain = br#"{"token":"t","refresh":"r","expires":9999999999}"#;
    let sealed = cipher
        .encrypt(Nonce::from_slice(&nonce), plain.as_slice())
        .unwrap();
    let cookie = format!(
        "__Host-clm={}",
        URL_SAFE_NO_PAD.encode([nonce.as_slice(), &sealed].concat())
    );
    assert_eq!(fake.handle(get("/api/repos", &cookie)).status, 401);
}

#[test]
fn the_build_assets_are_never_served_directly() {
    // req:web/invited-users@cfd18219
    let fake = Fake::default();
    let cookie = fake.cookie("t");
    assert_eq!(fake.handle(get("/_clm/invited.json", &cookie)).status, 404);
}
