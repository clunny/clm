//! The App's installation tokens, kept until shortly before they expire
//! (`installation-token`).

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use clm_web_platform::fake::*;
use clm_web_platform::install;
use clm_web_platform::platform::Auth;
use serde_json::{Value, json};

fn mints(fake: &Fake) -> usize {
    let calls = fake.gh.calls.borrow();
    calls
        .iter()
        .filter(|c| c.url.ends_with("/app/installations/7/access_tokens"))
        .count()
}

/// Each JWT the App sent, its claims.
fn claims(fake: &Fake) -> Vec<Value> {
    let calls = fake.gh.calls.borrow();
    calls
        .iter()
        .filter_map(|c| match &c.auth {
            Auth::App(jwt) => {
                let part = jwt.split('.').nth(1).unwrap();
                Some(serde_json::from_slice(&URL_SAFE_NO_PAD.decode(part).unwrap()).unwrap())
            }
            _ => None,
        })
        .collect()
}

fn twice(fake: &Fake) -> [Option<Auth>; 2] {
    [(); 2].map(|_| block_on(install::token(fake, 7)))
}

#[test]
fn a_token_near_its_expiry() {
    // req:web.worker.platform/installation-token@e5f2d889
    let hour = json!("2027-01-15T09:00:00Z");
    let four_minutes = json!("2027-01-15T08:04:00+00:00");
    for (expires_at, minted) in [
        (hour, 1),
        (four_minutes, 2),
        (Value::Null, 2),
        (json!("soon"), 2),
    ] {
        let fake = Fake::default();
        *fake.gh.expires_at.borrow_mut() = Some(expires_at.clone());
        let tokens = twice(&fake);
        assert_eq!(
            tokens,
            [(); 2].map(|_| Some(Auth::Installation("inst-7".into())))
        );
        assert_eq!(mints(&fake), minted, "{expires_at}");
        for c in claims(&fake) {
            assert_eq!(c, json!({"iss": "1", "iat": NOW - 60, "exp": NOW + 540}));
        }
    }
    let fake = Fake::default();
    fake.gh.mint_status.set(Some(401));
    assert_eq!(twice(&fake), [None, None], "a 401");
    let jwt = json!({"iss": "1", "iat": NOW - 60, "exp": NOW + 540});
    assert!(claims(&fake).iter().all(|c| *c == jwt) && !claims(&fake).is_empty());
    assert_eq!(block_on(install::token_direct(&fake, 7)), None);
    let mut fake = Fake::default();
    fake.secrets.remove("GITHUB_APP_ID");
    assert_eq!(twice(&fake), [None, None], "no App ID");
    assert_eq!(block_on(install::token_direct(&fake, 7)), None);
    assert_eq!(block_on(install::installation_id(&fake, "o/a")), None);
    assert!(fake.gh.calls.borrow().is_empty(), "asked GitHub nothing");

    let fake = Fake::default();
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    assert_eq!(block_on(install::installation_id(&fake, "o/a")), Some(9));
    assert_eq!(block_on(install::installation_id(&fake, "o/none")), None);
    assert_eq!(claims(&fake), [jwt.clone(), jwt]);
    assert!(
        fake.gh
            .calls
            .borrow()
            .iter()
            .all(|c| matches!(c.auth, Auth::App(_)))
    );
}
