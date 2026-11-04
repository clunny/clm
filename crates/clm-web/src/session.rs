//! The session cookie: the user's tokens, AES-256-GCM encrypted under `COOKIE_KEY`. There
//! is no server-side session store, and the tokens are never written anywhere else.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};

use clm_web_platform::platform::Secrets;

pub(crate) const COOKIE: &str = "__Host-clm";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// The user-to-server token.
    pub token: String,
    pub refresh: String,
    /// When `token` expires, in Unix seconds.
    pub expires: u64,
    /// The GitHub login, checked against the invited list on every request. Required, so
    /// a cookie sealed before it was kept fails to open and its owner signs in again.
    pub login: String,
}

fn cipher<P: Secrets>(p: &P) -> Option<Aes256Gcm> {
    let key = STANDARD.decode(p.secret("COOKIE_KEY")?.trim()).ok()?;
    Aes256Gcm::new_from_slice(&key).ok()
}

/// The cookie value: base64url of a random 96-bit nonce followed by the ciphertext.
pub async fn seal<P: Secrets>(p: &P, session: &Session) -> String {
    let Some(cipher) = cipher(p) else {
        return String::new();
    };
    let mut nonce = [0u8; 12];
    p.random(&mut nonce);
    let plain = serde_json::to_vec(session).unwrap_or_default();
    let Ok(sealed) = cipher.encrypt(Nonce::from_slice(&nonce), plain.as_slice()) else {
        return String::new();
    };
    URL_SAFE_NO_PAD.encode([nonce.as_slice(), &sealed].concat())
}

/// The session in a cookie value, or `None` if it was altered or isn't one of ours.
pub fn open<P: Secrets>(p: &P, value: &str) -> Option<Session> {
    let bytes = URL_SAFE_NO_PAD.decode(value).ok()?;
    if bytes.len() < 12 {
        return None;
    }
    let (nonce, sealed) = bytes.split_at(12);
    let plain = cipher(p)?.decrypt(Nonce::from_slice(nonce), sealed).ok()?;
    serde_json::from_slice(&plain).ok()
}

/// A `Set-Cookie` value carrying the sealed session.
pub(crate) fn set_cookie(value: &str) -> String {
    // The refresh token lasts six months, so the cookie may too.
    format!("{COOKIE}={value}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=15552000")
}
