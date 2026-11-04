//! `web_push`: one browser's push message, encrypted as RFC 8291 says and signed with a
//! VAPID key as RFC 8292 says. It builds the request and sends nothing.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A browser's push subscription: its endpoint, and its `p256dh` key and `auth` secret in
/// base64url, as `PushSubscription.toJSON()` gives them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
}

/// The App's VAPID key pair, base64url without padding: the uncompressed public point and
/// the 32-byte private scalar, as `npx web-push generate-vapid-keys` prints them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vapid {
    pub public_key: String,
    pub private_key: String,
}

/// The HTTP `POST` that delivers a push message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Which input `web_push` couldn't use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushError {
    /// `p256dh` isn't a 65-byte uncompressed P-256 point.
    P256dh,
    /// `auth` isn't 16 bytes.
    Auth,
    /// The endpoint isn't an `https` URL.
    Endpoint,
    /// A VAPID key doesn't parse.
    Vapid,
    /// `random`'s first 32 bytes aren't a valid P-256 private key.
    Random,
    /// The payload doesn't fit in a 4096-byte body.
    Payload,
}

impl fmt::Display for PushError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            PushError::P256dh => "p256dh is not a 65-byte uncompressed P-256 point",
            PushError::Auth => "auth is not 16 bytes",
            PushError::Endpoint => "endpoint is not an https URL",
            PushError::Vapid => "a VAPID key does not parse",
            PushError::Random => "random's first 32 bytes are not a P-256 private key",
            PushError::Payload => "payload does not fit in a 4096-byte body",
        })
    }
}

impl std::error::Error for PushError {}

/// The request that delivers `payload` to `subscription`, encrypted with `random`'s first 32
/// bytes as the sender's private key and the next 16 as the salt, and signed with `vapid`
/// for `now_secs`.
pub fn web_push(
    subscription: &Subscription,
    payload: &[u8],
    vapid: &Vapid,
    now_secs: u64,
    random: [u8; 48],
) -> Result<PushRequest, PushError> {
    let origin = origin(&subscription.endpoint)?;
    let authorization = Signer::new(vapid)?.authorization(&origin, now_secs);
    signed_push(subscription, payload, authorization, random)
}

/// The record size the header declares; the whole message is one record.
const RECORD_SIZE: u32 = 4096;
/// The most body push services must accept (RFC 8030): header, ciphertext and tag.
const MAX_BODY: usize = 4096;
/// The header block: salt, record size, key length and the 65-byte key.
const HEADER: usize = 16 + 4 + 1 + 65;
/// AES-GCM's tag, which each record carries.
const TAG: usize = 16;
/// How long the JWT is valid: RFC 8292 allows at most 24 hours.
const JWT_LIFETIME: u64 = 12 * 3600;
const SUBJECT: &str = "https://clm-app.dev";

/// base64url, with or without padding.
fn decode(s: &str) -> Option<Vec<u8>> {
    use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
    const ENGINE: GeneralPurpose = GeneralPurpose::new(
        &base64::alphabet::URL_SAFE,
        GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
    );
    base64::Engine::decode(&ENGINE, s).ok()
}

fn encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// A P-256 public key given as a 65-byte uncompressed point.
fn uncompressed_point(s: &str) -> Option<p256::PublicKey> {
    let bytes = decode(s).filter(|b| b.len() == 65 && b[0] == 4)?;
    p256::PublicKey::from_sec1_bytes(&bytes).ok()
}

/// The endpoint's origin, as the push service compares the JWT's `aud` with it: `https://`
/// and the host in lower case, with its port unless it is the default 443. An endpoint
/// with user info isn't one a browser gives.
pub(crate) fn origin(endpoint: &str) -> Result<String, PushError> {
    let rest = endpoint
        .strip_prefix("https://")
        .ok_or(PushError::Endpoint)?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority.strip_suffix(":443").unwrap_or(authority);
    if host.is_empty() || host.contains(|c: char| c.is_whitespace() || c == '@') {
        return Err(PushError::Endpoint);
    }
    Ok(format!("https://{}", host.to_ascii_lowercase()))
}

/// RFC 8291: `payload` as one `aes128gcm` record, with its header in front.
fn encrypt(
    subscription: &Subscription,
    payload: &[u8],
    random: [u8; 48],
) -> Result<Vec<u8>, PushError> {
    use aes_gcm::aead::{Aead, KeyInit};
    use hkdf::Hkdf;
    use p256::elliptic_curve::sec1::ToEncodedPoint;
    use sha2::Sha256;

    let ua_public = uncompressed_point(&subscription.p256dh).ok_or(PushError::P256dh)?;
    let auth = decode(&subscription.auth)
        .filter(|a| a.len() == 16)
        .ok_or(PushError::Auth)?;
    if HEADER + payload.len() + 1 + TAG > MAX_BODY {
        return Err(PushError::Payload);
    }
    let (key, salt) = random.split_at(32);
    let as_private = p256::SecretKey::from_slice(key).map_err(|_| PushError::Random)?;
    let as_public = as_private.public_key().to_encoded_point(false);
    let shared = p256::ecdh::diffie_hellman(as_private.to_nonzero_scalar(), ua_public.as_affine());

    let mut key_info = b"WebPush: info\0".to_vec();
    key_info.extend_from_slice(ua_public.to_encoded_point(false).as_bytes());
    key_info.extend_from_slice(as_public.as_bytes());
    let mut ikm = [0; 32];
    Hkdf::<Sha256>::new(Some(&auth), shared.raw_secret_bytes())
        .expand(&key_info, &mut ikm)
        .expect("32 bytes is a valid HKDF-SHA-256 length");
    let prk = Hkdf::<Sha256>::new(Some(salt), &ikm);
    let (mut cek, mut nonce) = ([0; 16], [0; 12]);
    prk.expand(b"Content-Encoding: aes128gcm\0", &mut cek)
        .expect("16 bytes is a valid HKDF-SHA-256 length");
    prk.expand(b"Content-Encoding: nonce\0", &mut nonce)
        .expect("12 bytes is a valid HKDF-SHA-256 length");

    let mut plaintext = payload.to_vec();
    plaintext.push(2); // the last record's padding delimiter
    let ciphertext = aes_gcm::Aes128Gcm::new(&cek.into())
        .encrypt(&nonce.into(), plaintext.as_slice())
        .expect("one record is within AES-GCM's limits");

    let mut body = salt.to_vec();
    body.extend_from_slice(&RECORD_SIZE.to_be_bytes());
    body.push(as_public.len() as u8);
    body.extend_from_slice(as_public.as_bytes());
    body.extend(ciphertext);
    Ok(body)
}

/// The VAPID key, parsed once to sign JWTs for any number of origins.
pub(crate) struct Signer {
    key: p256::ecdsa::SigningKey,
    public_key: String,
}

impl Signer {
    pub(crate) fn new(vapid: &Vapid) -> Result<Self, PushError> {
        use p256::elliptic_curve::sec1::ToEncodedPoint;
        let public = uncompressed_point(&vapid.public_key).ok_or(PushError::Vapid)?;
        let key = decode(&vapid.private_key)
            .filter(|k| k.len() == 32)
            .and_then(|k| p256::ecdsa::SigningKey::from_slice(&k).ok())
            .ok_or(PushError::Vapid)?;
        Ok(Signer {
            key,
            public_key: encode(public.to_encoded_point(false).as_bytes()),
        })
    }

    /// The `Authorization` header for a push service at `origin`: `vapid t=<jwt>, k=<key>`.
    pub(crate) fn authorization(&self, origin: &str, now_secs: u64) -> String {
        use p256::ecdsa::Signature;
        use p256::ecdsa::signature::Signer as _;
        let header = encode(br#"{"typ":"JWT","alg":"ES256"}"#);
        let claims = serde_json::json!({
            "aud": origin,
            "exp": now_secs + JWT_LIFETIME,
            "sub": SUBJECT,
        });
        let signed = format!("{header}.{}", encode(claims.to_string().as_bytes()));
        let signature: Signature = self.key.sign(signed.as_bytes());
        let jwt = format!("{signed}.{}", encode(&signature.to_bytes()));
        format!("vapid t={jwt}, k={}", self.public_key)
    }
}

/// `web_push` with an `Authorization` header already signed for the endpoint's origin, so a
/// send signs one JWT per origin.
pub(crate) fn signed_push(
    subscription: &Subscription,
    payload: &[u8],
    authorization: String,
    random: [u8; 48],
) -> Result<PushRequest, PushError> {
    let body = encrypt(subscription, payload, random)?;
    let headers = [
        ("Authorization", authorization.as_str()),
        ("Content-Encoding", "aes128gcm"),
        ("Content-Type", "application/octet-stream"),
        ("TTL", "86400"),
        ("Urgency", "normal"),
    ];
    Ok(PushRequest {
        url: subscription.endpoint.clone(),
        headers: headers
            .into_iter()
            .map(|(n, v)| (n.to_owned(), v.to_owned()))
            .collect(),
        body,
    })
}
