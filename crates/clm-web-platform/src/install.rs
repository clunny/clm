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
    let expires = v["expires_at"].as_str().and_then(unix_time).unwrap_or(0);
    let ttl = expires.saturating_sub(p.now()).saturating_sub(TOKEN_MARGIN);
    Some((token, ttl))
}

/// `YYYY-MM-DDTHH:MM:SS.000Z` for Unix seconds, as `clm-ledger` writes `at`.
pub fn rfc3339(secs: u64) -> String {
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    // Civil from days (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.000Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Unix seconds for GitHub's `YYYY-MM-DDTHH:MM:SSZ`.
pub fn unix_time(iso: &str) -> Option<u64> {
    let b = iso.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[10] != b'T' {
        return None;
    }
    let num = |r: std::ops::Range<usize>| iso.get(r)?.parse::<i64>().ok();
    let (y, m, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (hh, mm, ss) = (num(11..13)?, num(14..16)?, num(17..19)?);
    // Days from civil (Howard Hinnant).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + hh * 3600 + mm * 60 + ss).ok()
}
