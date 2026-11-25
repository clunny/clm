//! The App's own credentials, for the webhook and Cron runs, which have no user: a JWT
//! signed with the App's private key, exchanged for installation tokens.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::json;

use crate::gh;
use crate::platform::{Auth, Clock, GitHub, Platform, Secrets};

/// An installation token is dropped from the cache this long before it expires.
const TOKEN_MARGIN: u64 = 300;

/// A JWT for the App: RS256, issued a minute ago for clock skew, valid for 10 minutes.
async fn app_jwt<P: GitHub + Clock + Secrets>(p: &P) -> Option<Auth> {
    let now = p.now();
    let header = URL_SAFE_NO_PAD.encode(json!({"alg": "RS256", "typ": "JWT"}).to_string());
    let claims = json!({
        "iat": now.saturating_sub(60),
        "exp": now + 540,
        "iss": p.secret("GITHUB_APP_ID")?,
    });
    let unsigned = format!("{header}.{}", URL_SAFE_NO_PAD.encode(claims.to_string()));
    let signature = p.sign_rs256(unsigned.as_bytes()).await?;
    Some(Auth::App(format!(
        "{unsigned}.{}",
        URL_SAFE_NO_PAD.encode(signature)
    )))
}

/// The ID of the App's installation on `repo`.
pub async fn installation_id<P: GitHub + Clock + Secrets>(p: &P, repo: &str) -> Option<u64> {
    let jwt = app_jwt(p).await?;
    gh::get_ok(p, &jwt, &format!("/repos/{repo}/installation")).await?["id"].as_u64()
}

/// A token for installation `id`, cached in the Cache API until shortly before it expires.
pub async fn token<P: Platform>(p: &P, id: u64) -> Option<Auth> {
    let key = format!("installation-token:{id}");
    if let Some(token) = p.cache_get(&key).await {
        return Some(Auth::Installation(token));
    }
    let (token, ttl) = mint(p, id).await?;
    if ttl > 0 {
        p.cache_put(&key, &token, ttl).await;
    }
    Some(Auth::Installation(token))
}

/// A token for installation `id`, minted anew every time and kept nowhere
/// (`object-reads`).
pub async fn token_direct<P: GitHub + Clock + Secrets>(p: &P, id: u64) -> Option<Auth> {
    Some(Auth::Installation(mint(p, id).await?.0))
}

/// A new installation token and the seconds it may be kept, short of its expiry.
async fn mint<P: GitHub + Clock + Secrets>(p: &P, id: u64) -> Option<(String, u64)> {
    let jwt = app_jwt(p).await?;
    let url = format!("{}/app/installations/{id}/access_tokens", gh::API);
    let (status, v) = gh::post(p, &jwt, &url, &json!({})).await;
    if status != 201 && status != 200 {
        return None;
    }
    let token = v["token"].as_str()?.to_string();
    let expires = v["expires_at"]
        .as_str()
        .and_then(clm_ledger::unix_time)
        .unwrap_or(0);
    let ttl = expires.saturating_sub(p.now()).saturating_sub(TOKEN_MARGIN);
    Some((token, ttl))
}
