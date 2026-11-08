//! Signed repositories: `clm approve` signs its record (signed-approve), and `clm land`
//! counts only signed approvals and signs its own record (signed-land).

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use clm_ledger::{Body, Record};
use common::{Fixture, refused};

/// A fresh ed25519 key pair at `dir/<name>`: the private key's path and the public key.
fn keygen(dir: &Path, name: &str) -> (PathBuf, String) {
    let key = dir.join(name);
    let ok = Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-C", name, "-f"])
        .arg(&key)
        .status()
        .unwrap()
        .success();
    assert!(ok, "ssh-keygen -t ed25519");
    let public = std::fs::read_to_string(key.with_extension("pub")).unwrap();
    (key, public.trim().to_string())
}

/// `repo` signs with `key` from now on.
fn use_key(f: &Fixture, repo: &Path, key: &Path) {
    f.git(repo, &["config", "gpg.format", "ssh"]);
    f.git(repo, &["config", "user.signingkey", key.to_str().unwrap()]);
}

/// A repository and its keys: `christopher` (the work clone's person) and `dana`.
struct Signed {
    f: Fixture,
    christopher: PathBuf,
    dana: PathBuf,
}

/// The env every call gets: the fixture's, with git's global config isolated from the
/// user's own.
fn env(f: &Fixture) -> impl Fn(&str) -> Option<String> + use<> {
    let global = f.path("gitconfig").display().to_string();
    f.env(&[("GIT_CONFIG_GLOBAL", &global)])
}

/// A fixture whose `origin/main` holds `docs/allowed_signers` listing `christopher-lunny` and
/// `dana`, unless `signers` is false; the work clone is at that `main`.
fn signed_repo(settings: &str, signers: bool) -> Signed {
    let f = Fixture::new(settings);
    let keys = f.path("keys");
    std::fs::create_dir_all(&keys).unwrap();
    let (christopher, christopher_pub) = keygen(&keys, "christopher");
    let (dana, dana_pub) = keygen(&keys, "dana");
    if signers {
        let list = format!("christopher-lunny {christopher_pub}\ndana {dana_pub}\n");
        f.advance_main("docs/allowed_signers", &list);
        let work = f.work();
        f.git(&work, &["fetch", "-q", "origin"]);
        f.git(&work, &["merge", "-q", "--ff-only", "origin/main"]);
    }
    Signed {
        f,
        christopher,
        dana,
    }
}

/// A `spec/x` that needs review,, authored as `author` (default: the work clone's `Christopher Lunny`).
fn hard_branch(f: &Fixture, author: Option<&str>) {
    f.write("gate-out", "spec review\n");
    f.write("needs-approval", "");
    let work = f.work();
    if let Some(author) = author {
        f.git(&work, &["config", "user.name", author]);
    }
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    f.git(&work, &["config", "user.name", "Christopher Lunny"]);
}

/// The line `fold` read for `record`.
fn line_of(f: &Fixture, record: &Record) -> serde_json::Map<String, serde_json::Value> {
    f.records();
    clm_ledger::fold(&f.work()).unwrap().lines[&record.id].clone()
}

/// `clm approve spec/x` from a clone whose person is `user`, signing with `key` when given.
fn approve_from(s: &Signed, user: &str, key: Option<&Path>) {
    let f = &s.f;
    let clone = f.clone_as(&format!("clone-{user}"), user);
    match key {
        Some(key) => {
            use_key(f, &clone, key);
            clm_land::approve(&clone, Some("spec/x"), &env(f)).unwrap();
        }
        // An unsigned approval, as one made where nothing signs.
        None => {
            f.append(
                &clone,
                Body::Approved {
                    fingerprint: f.fingerprint("spec/x"),
                    branch: "spec/x".into(),
                    change: Some("x".into()),
                },
            );
        }
    }
}

#[test]
fn unsigned_approval_refused() {
    // req:land.approval/signed-land@a7c79d9c
    let s = signed_repo("", true);
    let f = &s.f;
    use_key(f, &f.work(), &s.christopher);
    hard_branch(f, None);
    approve_from(&s, "Dana", None);
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("spec/x"), &env(f)));
    assert!(why.contains("gate/approval"), "{why}");
    assert!(why.contains("not signed"), "{why}");
    assert_eq!(f.remote("main"), old);
}

#[test]
fn forged_name_refused() {
    // req:land.approval/signed-land@a7c79d9c
    let s = signed_repo("approval = \"other\"", true);
    let f = &s.f;
    use_key(f, &f.work(), &s.christopher);
    hard_branch(f, None);
    // Written as `dana`, signed with `christopher-lunny`'s key.
    approve_from(&s, "Dana", Some(&s.christopher));
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("spec/x"), &env(f)));
    assert!(why.contains("not signed by its writer"), "{why}");
    assert_eq!(f.remote("main"), old);
}

#[test]
fn lander_approved_it_themselves() {
    // req:land.approval/signed-land@a7c79d9c
    let s = signed_repo("approval = \"other\"", true);
    let f = &s.f;
    use_key(f, &f.work(), &s.christopher);
    hard_branch(f, Some("Zed"));
    // The lander's own clone approves, signed by `christopher-lunny`.
    clm_land::approve(&f.work(), Some("spec/x"), &env(f)).unwrap();
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("spec/x"), &env(f)));
    assert!(why.contains("approval = \"other\""), "{why}");
    assert!(why.contains("lander"), "{why}");
    assert_eq!(f.remote("main"), old);
}

#[test]
fn signed_land() {
    // req:land.approval/signed-land@a7c79d9c
    let s = signed_repo("approval = \"other\"", true);
    let f = &s.f;
    use_key(f, &f.work(), &s.christopher);
    hard_branch(f, None);
    approve_from(&s, "Dana", Some(&s.dana));
    let out = clm_land::land(&f.work(), Some("spec/x"), &env(f)).unwrap();
    let main = f.remote("main");
    assert_eq!(out.lines[0], format!("landed {main} spec review"));
    let landed: Vec<Record> = f
        .records()
        .into_iter()
        .filter(|r| matches!(r.body, Body::Landed { .. }))
        .collect();
    let [record] = &landed[..] else {
        panic!("not one landed record")
    };
    assert!(matches!(&record.body, Body::Landed { commit, .. } if *commit == main));
    let line = line_of(f, record);
    let work = f.work();
    assert!(clm_ledger::signed_by(&work, "origin/main", &line, "clm-land").unwrap());
}

#[test]
fn lander_cant_sign() {
    // req:land.approval/signed-land@a7c79d9c
    let s = signed_repo("", true);
    let f = &s.f;
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &env(f)));
    assert!(why.contains("user.signingkey"), "{why}");
    assert!(f.lines("check.log").is_empty(), "the check ran");
    assert!(f.lines("gate.log").is_empty(), "the gate ran");
    assert_eq!(f.remote("main"), old);
    assert!(f.records().is_empty(), "{:?}", f.records());
    f.assert_no_scratch();
}

#[test]
fn unsigned_repository_lands_as_before() {
    // req:land.approval/signed-land@a7c79d9c
    let s = signed_repo("", false);
    let f = &s.f;
    f.branch("impl/a", &[("b.txt", "b\n")]);
    clm_land::land(&f.work(), Some("impl/a"), &env(f)).unwrap();
    let landed: Vec<Record> = f
        .records()
        .into_iter()
        .filter(|r| matches!(r.body, Body::Landed { .. }))
        .collect();
    let [record] = &landed[..] else {
        panic!("not one landed record")
    };
    assert!(!line_of(f, record).contains_key("sig"));
}

#[test]
fn signing_fails_pushes_nothing() {
    // req:land.approval/signed-land@a7c79d9c
    let s = signed_repo("", true);
    let f = &s.f;
    // A key file that can be read but isn't a key: the check before the land passes, and
    // signing the `landed` record fails.
    let bogus = f.path("keys/bogus");
    std::fs::write(&bogus, "not a key\n").unwrap();
    use_key(f, &f.work(), &bogus);
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &env(f)));
    assert!(why.contains("user.signingkey"), "{why}");
    assert_eq!(f.lines("check.log").len(), 1, "the check didn't run");
    assert_eq!(f.remote("main"), old);
    assert!(f.landed().is_empty(), "{:?}", f.landed());
}

#[test]
fn unsigned_request_holds_in_a_signed_repository() {
    // req:land.approval/changes-requested-holds@27e6c16b
    let s = signed_repo("", true);
    let f = &s.f;
    use_key(f, &f.work(), &s.christopher);
    hard_branch(f, None);
    approve_from(&s, "Dana", Some(&s.dana));
    let fingerprint = f.fingerprint("spec/x");
    f.web_ledger(
        "alice/web",
        &[serde_json::json!({"id": "r1", "writer": "alice/web",
            "at": "2099-01-01T00:00:00.000Z", "kind": "changes_requested",
            "branch": "spec/x", "fingerprint": fingerprint, "text": "Name the rule"})],
    );
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("spec/x"), &env(f)));
    assert!(why.contains("land/changes-requested: alice/web"), "{why}");
    assert_eq!(f.remote("main"), old);
    // Without the request, Dana's approval counts and it lands.
    f.web_ledger("alice/web", &[]);
    clm_land::land(&f.work(), Some("spec/x"), &env(f)).unwrap();
}
