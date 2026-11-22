//! Whether a base is signed, the key check and signing a record
//! (land.approval/key-and-signing).

mod common;

use clm_ledger::{Body, LandKind};
use common::{Fixture, keygen, refused};

#[test]
fn signed_and_not() {
    // req:land.approval/key-and-signing@3487dc91
    let f = Fixture::new("");
    let work = f.work();
    let unsigned = f.git(&work, &["rev-parse", "origin/main"]);
    f.advance_main(
        "docs/allowed_signers",
        "christopher-lunny ssh-ed25519 AAAA\n",
    );
    f.git(&work, &["fetch", "-q", "origin"]);
    let signed = f.git(&work, &["rev-parse", "origin/main"]);
    f.with_repo(|repo| {
        assert!(repo.signed(&signed).unwrap());
        assert!(!repo.signed(&unsigned).unwrap());
    });
}

#[test]
fn no_key() {
    // req:land.approval/key-and-signing@3487dc91
    let f = Fixture::new("");
    let work = f.work();
    // SSH signing, but no user.signingkey.
    f.git(&work, &["config", "gpg.format", "ssh"]);
    let why = refused(f.with_repo(|repo| repo.require_key()));
    assert!(why.contains("user.signingkey"), "{why}");
    assert!(f.records().is_empty());
    // A key set, but not for SSH, is refused alike.
    f.git(&work, &["config", "--unset", "gpg.format"]);
    let (key, _) = keygen(&f.path("keys"), "christopher");
    f.git(&work, &["config", "user.signingkey", key.to_str().unwrap()]);
    let why = refused(f.with_repo(|repo| repo.require_key()));
    assert!(why.contains("user.signingkey"), "{why}");
    f.git(&work, &["config", "gpg.format", "ssh"]);
    f.with_repo(|repo| repo.require_key()).unwrap();
}

#[test]
fn a_key_that_cant_be_read() {
    // req:land.approval/key-and-signing@3487dc91
    let f = Fixture::new("");
    let work = f.work();
    let missing = f.path("keys/gone");
    f.git(&work, &["config", "gpg.format", "ssh"]);
    f.git(
        &work,
        &["config", "user.signingkey", missing.to_str().unwrap()],
    );
    let why = refused(f.with_repo(|repo| repo.require_key()));
    assert!(why.contains("user.signingkey"), "{why}");
    assert!(why.contains("keys/gone"), "{why}");
    // A relative path is taken from the top of the repository.
    keygen(&work.join("k"), "key");
    f.git(&work, &["config", "user.signingkey", "k/key"]);
    f.with_repo(|repo| repo.require_key()).unwrap();
}

#[test]
fn a_lands_record() {
    // req:land.approval/key-and-signing@3487dc91
    let f = Fixture::new("");
    let work = f.work();
    let (key, public) = keygen(&f.path("keys"), "christopher");
    f.advance_main(
        "docs/allowed_signers",
        &format!("christopher-lunny {public}\n"),
    );
    f.git(&work, &["fetch", "-q", "origin"]);
    let base = f.git(&work, &["rev-parse", "origin/main"]);
    f.git(&work, &["config", "gpg.format", "ssh"]);
    f.git(&work, &["config", "user.signingkey", key.to_str().unwrap()]);
    let landed = Body::landed(
        base.clone(),
        "impl/x".into(),
        LandKind::Code,
        false,
        "g".into(),
    );
    let signed = f
        .with_repo(|repo| repo.sign(landed, clm_approval::LAND))
        .unwrap();
    let line = signed.line();
    assert!(clm_ledger::signed_by(&work, &base, &line, "clm-land").unwrap());
    assert!(!clm_ledger::signed_by(&work, &base, &line, "clm-approve").unwrap());
    assert_eq!(clm_approval::LAND, "clm-land");
    assert_eq!(clm_approval::APPROVE, "clm-approve");
    // Nothing was appended: signing isn't appending.
    assert!(f.records().is_empty());
}

#[test]
fn signing_without_a_usable_key() {
    // req:land.approval/key-and-signing@3487dc91
    let f = Fixture::new("");
    let body = Body::approved("g".into(), "spec/x".into(), None);
    let why = refused(f.with_repo(|repo| repo.sign(body, clm_approval::APPROVE)));
    assert!(why.contains("user.signingkey"), "{why}");
}
