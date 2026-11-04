//! Sign-in through the GitHub App's user authorization flow, and the session every API
//! request needs.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::{Value, json};

use crate::invited;
use crate::session::{self, Session};
use clm_web_platform::gh;
use clm_web_platform::http::{Request, Response, percent_encode};
use clm_web_platform::platform::{Auth, Platform};

const STATE_COOKIE: &str = "__Host-clm-state";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
/// Refresh this long before the token expires.
const REFRESH_MARGIN: u64 = 60;

fn redirect_uri(req: &Request) -> String {
    format!("{}/auth/callback", req.origin)
}

/// `GET /auth/login`: sends the browser to GitHub with a fresh `state`, also kept in a
/// short-lived cookie the callback compares it with.
pub fn login<P: Platform>(p: &P, req: &Request) -> Response {
    let mut state = [0u8; 16];
    p.random(&mut state);
    let state = URL_SAFE_NO_PAD.encode(state);
    let client_id = p.secret("GITHUB_CLIENT_ID").unwrap_or_default();
    let to = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&state={state}",
        percent_encode(&client_id),
        percent_encode(&redirect_uri(req)),
    );
    Response::redirect(&to).with_header(
        "set-cookie",
        format!("{STATE_COOKIE}={state}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=600"),
    )
}

/// `GET /auth/callback`: exchanges the code for tokens and signs the person in.
pub async fn callback<P: Platform>(p: &P, req: &Request) -> Response {
    let (Some(code), Some(state)) = (req.param("code"), req.param("state")) else {
        return Response::empty(400);
    };
    if req.cookie(STATE_COOKIE) != Some(state.as_str()) {
        return Response::empty(400);
    }
    let body = json!({
        "client_id": p.secret("GITHUB_CLIENT_ID"),
        "client_secret": p.secret("GITHUB_CLIENT_SECRET"),
        "code": code,
        "redirect_uri": redirect_uri(req),
    });
    match exchange(p, &body).await {
        Some(session) => complete_sign_in(p, session).await,
        None => Response::empty(400),
    }
}

/// With the tokens in hand: asks GitHub whose they are, and signs that person in only if
/// they're invited. Anyone else gets the "not invited" page and no session cookie.
async fn complete_sign_in<P: Platform>(p: &P, mut session: Session) -> Response {
    let clear_state = format!("{STATE_COOKIE}=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0");
    let user = gh::get_ok(p, &Auth::User(session.token.clone()), "/user").await;
    let Some(login) = user.as_ref().and_then(|u| u["login"].as_str()) else {
        return Response::html(502, invited::NO_LOGIN).with_header("set-cookie", clear_state);
    };
    if !invited::is_invited(p, login).await {
        return Response::html(403, &invited::not_invited_page())
            .with_header("set-cookie", clear_state);
    }
    session.login = login.into();
    let sealed = session::seal(p, &session).await;
    Response::redirect("/app/")
        .with_header("set-cookie", session::set_cookie(&sealed))
        .with_header("set-cookie", clear_state)
}

/// Posts to GitHub's token endpoint, for a code or a refresh token.
async fn exchange<P: Platform>(p: &P, body: &Value) -> Option<Session> {
    let (status, v) = gh::post(p, &Auth::None, TOKEN_URL, body).await;
    if status != 200 {
        return None;
    }
    let token = v["access_token"].as_str()?;
    Some(Session {
        token: token.into(),
        refresh: v["refresh_token"].as_str().unwrap_or_default().into(),
        // Without `expires_in` the App doesn't expire user tokens, so neither do we.
        expires: v["expires_in"]
            .as_u64()
            .map_or(u64::MAX, |s| p.now().saturating_add(s)),
        // The token endpoint doesn't say whose tokens they are; the caller fills it in.
        login: String::new(),
    })
}

/// A request's valid session, refreshed if it has expired.
pub(crate) struct Authed {
    pub session: Session,
    /// A `Set-Cookie` value to send back when the session was refreshed.
    pub renewed: Option<String>,
}

impl Authed {
    pub fn auth(&self) -> Auth {
        Auth::User(self.session.token.clone())
    }

    /// `res`, carrying the refreshed cookie if there is one.
    pub(crate) fn respond(&self, res: Response) -> Response {
        match &self.renewed {
            Some(c) => res.with_header("set-cookie", c.clone()),
            None => res,
        }
    }
}

/// The request's session, or 401 when it has none, it was altered, its login is no longer
/// invited, or it can't be renewed.
pub(crate) async fn session<P: Platform>(p: &P, req: &Request) -> Result<Authed, Response> {
    let unauthorized = || Response::empty(401);
    let value = req.cookie(session::COOKIE).ok_or_else(unauthorized)?;
    let current = session::open(p, value).ok_or_else(unauthorized)?;
    if !invited::is_invited(p, &current.login).await {
        return Err(unauthorized());
    }
    if p.now() + REFRESH_MARGIN < current.expires {
        return Ok(Authed {
            session: current,
            renewed: None,
        });
    }
    let body = json!({
        "client_id": p.secret("GITHUB_CLIENT_ID"),
        "client_secret": p.secret("GITHUB_CLIENT_SECRET"),
        "grant_type": "refresh_token",
        "refresh_token": current.refresh,
    });
    let mut fresh = exchange(p, &body).await.ok_or_else(unauthorized)?;
    fresh.login = current.login;
    let sealed = session::seal(p, &fresh).await;
    Ok(Authed {
        session: fresh,
        renewed: Some(session::set_cookie(&sealed)),
    })
}
