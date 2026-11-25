//! Sign-in through the GitHub App's user authorization flow, with tokens only in an
//! encrypted cookie.

mod common;

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use clm_web::session::{Session, seal};
use clm_web_platform::http::Response;
use clm_web_platform::platform::Auth;
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
    // req:web.worker/sign-in@1708e84d
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
    // req:web.worker/sign-in@1708e84d
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
    // Using it for a repo fills the Cache API, still without the tokens.
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
    // req:web.worker/sign-in@1708e84d
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
    // req:web.worker/sign-in@1708e84d
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
    // req:web.worker/sign-in@1708e84d
    let fake = Fake::default();
    let session = Session {
        token: "old".into(),
        refresh: "r1".into(),
        expires: NOW - 1,
        login: "clunny".into(),
    };
    let cookie = format!("__Host-clm={}", block_on(seal(&fake, &session)).unwrap());
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
    // req:web.worker/sign-in@1708e84d
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
    // req:web.worker/invited-sign-in@c0b2a54d
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
    // req:web.worker/sign-in-opens-app@4c94f7fb
    let fake = Fake::default();
    let res = sign_in_as(&fake, "clunny");
    assert_eq!(res.status, 302, "{res:?}");
    assert_eq!(res.header("location"), Some("/app/"));
    assert!(cookie_pair(&res, "__Host-clm").is_some());
}

#[test]
fn logins_compare_without_case() {
    // req:web.worker/invited-sign-in@c0b2a54d
    // GitHub logins are case-insensitive, and `/user` gives the account's own casing.
    let fake = Fake::default();
    let res = sign_in_as(&fake, "Clunny");
    assert!(cookie_pair(&res, "__Host-clm").is_some(), "{res:?}");
}

#[test]
fn not_invited_login_gets_the_page_and_no_session() {
    // req:web.worker/invited-sign-in@c0b2a54d
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
    // With every cookie the callback set, the API stays shut.
    let jar = set_cookies(&res)
        .into_iter()
        .filter_map(|c| c.split(';').next())
        .collect::<Vec<_>>()
        .join("; ");
    assert_eq!(fake.handle(get("/api/repos", &jar)).status, 401);
    // Nothing of the token was kept.
    for value in fake.stored.borrow().iter() {
        assert!(!value.contains("tok-someone-else"), "stored: {value}");
    }
}

#[test]
fn not_invited_page_points_to_quickstart_and_early_access() {
    // req:web.worker/not-invited-page@6cef779f
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
    // req:web.worker/invited-sign-in@c0b2a54d
    // Removing a login from the Contract revokes access at the next deploy.
    let fake = Fake::default();
    let cookie = fake.cookie_as("t", "someone-else");
    assert_eq!(fake.handle(get("/api/repos", &cookie)).status, 401);
    // Every API route, not only the list, reads the same session check.
    assert_eq!(
        fake.handle(get("/api/repos/o/r/snapshot", &cookie)).status,
        401
    );
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
    // req:web.worker/invited-sign-in@c0b2a54d
    let fake = Fake::default();
    let res = sign_in_as(&fake, "");
    assert_eq!(res.status, 502);
    assert!(cookie_pair(&res, "__Host-clm").is_none(), "{res:?}");
    let body = String::from_utf8_lossy(&res.body);
    assert!(body.contains("href=\"/auth/login\""), "{body}");
}

#[test]
fn without_the_invited_list_nobody_gets_in() {
    // req:web.worker/invited-sign-in@c0b2a54d
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
    // req:web.worker/invited-sign-in@c0b2a54d
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

/// A session cookie for `token` and `refresh` that expired a minute ago.
fn expired(fake: &Fake, token: &str, refresh: &str) -> String {
    let session = Session {
        token: token.into(),
        refresh: refresh.into(),
        expires: fake.now.get() - 60,
        login: "clunny".into(),
    };
    format!("__Host-clm={}", block_on(seal(fake, &session)).unwrap())
}

#[test]
fn the_flows_edges() {
    // req:web.worker/sign-in@1708e84d
    let fake = Fake::default();
    fake.gh
        .codes
        .borrow_mut()
        .insert("c1".into(), ("t1".into(), "r1".into()));
    // The login: GitHub's authorize URL, with the client ID, the callback and a state
    // equal to the state cookie's.
    let res = fake.handle(get("/auth/login", ""));
    assert_eq!(res.status, 302);
    let location = res.header("location").unwrap();
    let callback = clm_web_platform::http::percent_encode(&format!("{ORIGIN}/auth/callback"));
    assert!(
        location.starts_with("https://github.com/login/oauth/authorize?"),
        "{location}"
    );
    let query = location.split_once('?').unwrap().1;
    let params: Vec<(&str, &str)> = query.split('&').filter_map(|p| p.split_once('=')).collect();
    let param = |name: &str| params.iter().find(|(k, _)| *k == name).map(|(_, v)| *v);
    assert_eq!(param("client_id"), Some("client-id"));
    assert_eq!(param("redirect_uri"), Some(callback.as_str()));
    let state_cookie = set_cookies(&res)
        .into_iter()
        .find(|c| c.starts_with("__Host-clm-state="))
        .unwrap();
    for attr in ["HttpOnly", "Max-Age=600"] {
        assert!(state_cookie.contains(attr), "{state_cookie}");
    }
    let state = cookie_pair(&res, "__Host-clm-state").unwrap();
    let state = state.split_once('=').unwrap().1;
    assert!(!state.is_empty());
    assert_eq!(param("state"), Some(state));
    let jar = format!("__Host-clm-state={state}");
    // A forged state, no code, and a code GitHub refuses: 400, and no session cookie.
    for path in [
        "/auth/callback?code=c1&state=forged".to_string(),
        format!("/auth/callback?state={state}"),
        format!("/auth/callback?code=refused&state={state}"),
    ] {
        let res = fake.handle(get(&path, &jar));
        assert_eq!(res.status, 400, "{path}");
        assert!(cookie_pair(&res, "__Host-clm").is_none(), "{path}");
    }
    // An expired session that GitHub renews: 200, the cookie re-set, the renewed token used.
    let res = fake.handle(get("/api/repos", &expired(&fake, "t1", "r1")));
    assert_eq!(res.status, 200);
    assert!(cookie_pair(&res, "__Host-clm").is_some(), "{res:?}");
    let last = fake.gh.calls.borrow().last().cloned().unwrap();
    assert_eq!(last.auth, Auth::User("r1-renewed".into()));
    // One GitHub refuses to renew: 401.
    fake.refused_refresh.borrow_mut().insert("r2".into());
    let res = fake.handle(get("/api/repos", &expired(&fake, "t2", "r2")));
    assert_eq!(res.status, 401);
    assert!(cookie_pair(&res, "__Host-clm").is_none(), "{res:?}");
    // A token GitHub gave no expiry is used as it is, 30 days on.
    fake.gh
        .codes
        .borrow_mut()
        .insert("c2".into(), ("forever".into(), String::new()));
    let pair = cookie_pair(&sign_in(&fake, "c2"), "__Host-clm")
        .unwrap()
        .to_string();
    fake.now.set(NOW + 30 * 86_400);
    let res = fake.handle(get("/api/repos", &pair));
    assert_eq!(res.status, 200);
    assert!(cookie_pair(&res, "__Host-clm").is_none(), "not renewed");
    let last = fake.gh.calls.borrow().last().cloned().unwrap();
    assert_eq!(last.auth, Auth::User("forever".into()));
}

#[test]
fn no_usable_cookie_key() {
    // req:web.worker/sign-in@1708e84d
    for key in [None, Some(STANDARD.encode([7u8; 16]))] {
        let mut fake = Fake::default();
        match &key {
            None => fake.secrets.remove("COOKIE_KEY"),
            Some(k) => fake.secrets.insert("COOKIE_KEY".into(), k.clone()),
        };
        let res = sign_in_as(&fake, "clunny");
        assert_eq!(res.status, 500, "{key:?}: {res:?}");
        assert!(res.header("location").is_none(), "{key:?}: no redirect");
        assert!(
            cookie_pair(&res, "__Host-clm").is_none(),
            "{key:?}: {res:?}"
        );
        assert_eq!(
            cookie_pair(&res, "__Host-clm-state"),
            Some("__Host-clm-state="),
            "{key:?}: the state cookie is cleared"
        );
        let body = String::from_utf8_lossy(&res.body).to_lowercase();
        assert!(body.contains("sign-in isn't available"), "{body}");
    }
}
