use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes128Gcm, Nonce};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use clm_web_live::{PushError, Subscription, Vapid, web_push};
use hkdf::Hkdf;
use p256::ecdsa::signature::Verifier;
use p256::ecdsa::{Signature, VerifyingKey};
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::{PublicKey, SecretKey};
use serde_json::Value;
use sha2::Sha256;

fn b64(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn unb64(s: &str) -> Vec<u8> {
    URL_SAFE_NO_PAD.decode(s.replace([' ', '\n'], "")).unwrap()
}

fn public(secret: &SecretKey) -> Vec<u8> {
    secret
        .public_key()
        .to_encoded_point(false)
        .as_bytes()
        .to_vec()
}

const NOW: u64 = 1_800_000_000;

/// The browser's key pair and auth secret, from fixed bytes.
fn browser() -> (SecretKey, [u8; 16]) {
    (SecretKey::from_slice(&[7; 32]).unwrap(), [9; 16])
}

fn vapid_secret() -> SecretKey {
    SecretKey::from_slice(&[3; 32]).unwrap()
}

fn vapid() -> Vapid {
    let secret = vapid_secret();
    Vapid {
        public_key: b64(&public(&secret)),
        private_key: b64(&secret.to_bytes()),
    }
}

fn subscription(endpoint: &str) -> Subscription {
    let (secret, auth) = browser();
    Subscription {
        endpoint: endpoint.into(),
        p256dh: b64(&public(&secret)),
        auth: b64(&auth),
    }
}

fn random() -> [u8; 48] {
    let mut r = [5u8; 48];
    r[40..].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    r
}

fn header<'a>(headers: &'a [(String, String)], name: &str) -> &'a str {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
        .unwrap_or_else(|| panic!("no {name} in {headers:?}"))
}

/// RFC 8291's decryption, as the browser does it.
fn decrypt(body: &[u8], secret: &SecretKey, auth: &[u8]) -> Vec<u8> {
    let salt = &body[..16];
    assert_eq!(&body[16..20], &4096u32.to_be_bytes());
    assert_eq!(body[20], 65);
    let sender = PublicKey::from_sec1_bytes(&body[21..86]).unwrap();
    let shared = p256::ecdh::diffie_hellman(secret.to_nonzero_scalar(), sender.as_affine());
    let mut info = b"WebPush: info\0".to_vec();
    info.extend(public(secret));
    info.extend(&body[21..86]);
    let mut ikm = [0; 32];
    Hkdf::<Sha256>::new(Some(auth), shared.raw_secret_bytes())
        .expand(&info, &mut ikm)
        .unwrap();
    let prk = Hkdf::<Sha256>::new(Some(salt), &ikm);
    let (mut cek, mut nonce) = ([0; 16], [0; 12]);
    prk.expand(b"Content-Encoding: aes128gcm\0", &mut cek)
        .unwrap();
    prk.expand(b"Content-Encoding: nonce\0", &mut nonce)
        .unwrap();
    let mut plain = Aes128Gcm::new_from_slice(&cek)
        .unwrap()
        .decrypt(Nonce::from_slice(&nonce), &body[86..])
        .unwrap();
    assert_eq!(plain.pop(), Some(2), "the last record's padding delimiter");
    plain
}

#[test]
fn the_browser_can_read_it() {
    // req:web.live/web-push-request@81a115ff
    let req = web_push(
        &subscription("https://push.example/s/1"),
        br#"{"title":"t"}"#,
        &vapid(),
        NOW,
        random(),
    )
    .unwrap();
    assert_eq!(req.url, "https://push.example/s/1");
    let (secret, auth) = browser();
    assert_eq!(decrypt(&req.body, &secret, &auth), br#"{"title":"t"}"#);
    assert_eq!(header(&req.headers, "Content-Encoding"), "aes128gcm");
    assert_eq!(
        header(&req.headers, "Content-Type"),
        "application/octet-stream"
    );
    assert_eq!(header(&req.headers, "TTL"), "86400");
    assert_eq!(header(&req.headers, "Urgency"), "normal");

    let auth_header = header(&req.headers, "Authorization");
    let rest = auth_header.strip_prefix("vapid t=").unwrap();
    let (jwt, k) = rest.split_once(", k=").unwrap();
    assert_eq!(k, vapid().public_key);
    let (signed, sig) = jwt.rsplit_once('.').unwrap();
    let key = VerifyingKey::from_sec1_bytes(&unb64(k)).unwrap();
    let sig = Signature::from_slice(&unb64(sig)).unwrap();
    key.verify(signed.as_bytes(), &sig).unwrap();
    let (head, claims) = signed.split_once('.').unwrap();
    let head: Value = serde_json::from_slice(&unb64(head)).unwrap();
    assert_eq!(head["alg"], "ES256");
    let claims: Value = serde_json::from_slice(&unb64(claims)).unwrap();
    assert_eq!(claims["aud"], "https://push.example");
    assert_eq!(claims["sub"], "https://clm-app.dev");
    let exp = claims["exp"].as_u64().unwrap();
    assert!(exp > NOW && exp <= NOW + 12 * 3600, "{exp}");
}

#[test]
fn rfc_8291_example() {
    // req:web.live/web-push-request@81a115ff
    let mut random = [0; 48];
    random[..32].copy_from_slice(&unb64("yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw"));
    random[32..].copy_from_slice(&unb64("DGv6ra1nlYgDCS1FRnbzlw"));
    let sub = Subscription {
        endpoint: "https://push.example.net/push/JzLQ3raZJfFBR0aqvOMsLrt54w4rJUsV".into(),
        p256dh: "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4"
            .into(),
        auth: "BTBZMqHH6r4Tts7J_aSIgg".into(),
    };
    let plain = unb64("V2hlbiBJIGdyb3cgdXAsIEkgd2FudCB0byBiZSBhIHdhdGVybWVsb24");
    let req = web_push(&sub, &plain, &vapid(), NOW, random).unwrap();
    let message = unb64(
        "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27ml
         mlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A_yl95bQpu6cVPT
         pK4Mqgkf1CXztLVBSt2Ks3oZwbuwXPXLWyouBWLVWGNWQexSgSxsj_Qulcy4a-fN",
    );
    assert_eq!(req.body, message);
}

fn error(sub: Subscription, vapid: Vapid, payload: &[u8]) -> PushError {
    web_push(&sub, payload, &vapid, NOW, random()).unwrap_err()
}

#[test]
fn a_bad_key() {
    // req:web.live/web-push-request@81a115ff
    let mut sub = subscription("https://push.example/s/1");
    sub.auth = b64(&[1; 15]);
    let err = error(sub, vapid(), b"{}");
    assert_eq!(err, PushError::Auth);
    assert!(err.to_string().contains("auth"), "{err}");
}

#[test]
fn bad_inputs_are_named() {
    // req:web.live/web-push-request@81a115ff
    let mut sub = subscription("https://push.example/s/1");
    sub.p256dh = b64(&[4; 65]); // 65 bytes, but not a point on the curve
    assert_eq!(error(sub, vapid(), b"{}"), PushError::P256dh);
    let mut sub = subscription("https://push.example/s/1");
    let (secret, _) = browser();
    let compressed = secret.public_key().to_encoded_point(true);
    sub.p256dh = b64(compressed.as_bytes());
    assert_eq!(error(sub, vapid(), b"{}"), PushError::P256dh);
    for endpoint in [
        "http://push.example/s/1",
        "https:///s/1",
        "push.example/s/1",
        "https://user@push.example/s/1",
    ] {
        let err = error(subscription(endpoint), vapid(), b"{}");
        assert_eq!(err, PushError::Endpoint, "{endpoint}");
        assert!(err.to_string().contains("endpoint"), "{err}");
    }
    let mut bad = vapid();
    bad.private_key = b64(&[0; 32]);
    assert_eq!(
        error(subscription("https://push.example/s/1"), bad, b"{}"),
        PushError::Vapid
    );
    let mut bad = vapid();
    bad.public_key = "not base64!".into();
    assert_eq!(
        error(subscription("https://push.example/s/1"), bad, b"{}"),
        PushError::Vapid
    );
}

#[test]
fn a_payload_too_long_for_one_record() {
    // req:web.live/web-push-request@81a115ff
    let sub = || subscription("https://push.example/s/1");
    // Push services take at most 4096 bytes of body: the 86-byte header, the payload, its
    // delimiter and the 16-byte tag.
    let req = web_push(&sub(), &[b'a'; 3993], &vapid(), NOW, random()).unwrap();
    assert_eq!(req.body.len(), 4096);
    assert_eq!(error(sub(), vapid(), &[b'a'; 3994]), PushError::Payload);
}

/// The JWT's claims, from a request's `Authorization` header.
fn claims(headers: &[(String, String)]) -> Value {
    let auth = header(headers, "Authorization");
    let jwt = auth
        .strip_prefix("vapid t=")
        .unwrap()
        .split(", k=")
        .next()
        .unwrap();
    serde_json::from_slice(&unb64(jwt.split('.').nth(1).unwrap())).unwrap()
}

#[test]
fn the_audience_is_the_normalised_origin() {
    // req:web.live/web-push-request@81a115ff
    for (endpoint, aud) in [
        ("https://Push.Example:443/s/1", "https://push.example"),
        (
            "https://push.example:8443/s?x=1",
            "https://push.example:8443",
        ),
        ("https://push.example", "https://push.example"),
    ] {
        let req = web_push(&subscription(endpoint), b"{}", &vapid(), NOW, random()).unwrap();
        assert_eq!(claims(&req.headers)["aud"], aud, "{endpoint}");
        assert_eq!(req.url, endpoint);
    }
}
