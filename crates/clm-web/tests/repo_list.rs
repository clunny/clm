//! The repo list holds the installations' repos that have `specs/SPEC.md`.

mod common;

use common::*;
use serde_json::json;

#[test]
fn non_clm_repo() {
    // req:web.worker/repo-list@b7761292
    let fake = Fake::default();
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    fake.gh.add_repo(
        "o/b",
        FakeRepo::new("222", files(&[("README.md", "# not CLM\n")])),
    );
    fake.gh
        .installations
        .borrow_mut()
        .insert("u".into(), vec![(1, vec!["o/a".into(), "o/b".into()])]);
    let cookie = fake.user("u", &["o/a", "o/b"]);
    let res = fake.handle(get("/api/repos", &cookie));
    assert_eq!(res.status, 200);
    assert_eq!(
        body_json(&res),
        json!([{"owner": "o", "name": "a", "default_branch": "main"}])
    );
}

#[test]
fn many_installations_stay_within_budget() {
    // req:web.worker/repo-list@b7761292
    let fake = Fake::default();
    let mut installs = Vec::new();
    for i in 0..60 {
        let name = format!("org{i}/r");
        fake.gh.add_repo(&name, FakeRepo::new("111", clm_repo()));
        installs.push((i, vec![name]));
    }
    let names: Vec<String> = installs.iter().map(|(_, r)| r[0].clone()).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    fake.gh
        .installations
        .borrow_mut()
        .insert("u".into(), installs);
    let cookie = fake.user("u", &refs);
    // `handle` asserts at most 50 subrequests.
    let res = fake.handle(get("/api/repos", &cookie));
    assert_eq!(res.status, 200);
    assert!(!body_json(&res).as_array().unwrap().is_empty());
}

/// A fake whose user `u` has one installation covering `o/a`, a CLM repo.
fn installed() -> (Fake, String) {
    let fake = Fake::default();
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    fake.gh
        .installations
        .borrow_mut()
        .insert("u".into(), vec![(1, vec!["o/a".into()])]);
    let cookie = fake.user("u", &["o/a"]);
    (fake, cookie)
}

fn list(fake: &Fake, cookie: &str) -> serde_json::Value {
    let res = fake.handle(get("/api/repos", cookie));
    assert_eq!(res.status, 200);
    body_json(&res)
}

#[test]
fn listed_again() {
    // req:web.worker/repo-list@b7761292
    let (fake, cookie) = installed();
    let first = list(&fake, &cookie);
    assert_eq!(first[0]["name"], "a");
    let calls = fake.gh.calls.borrow().len();
    fake.now.set(NOW + 60);
    assert_eq!(list(&fake, &cookie), first);
    assert_eq!(fake.gh.calls.borrow().len(), calls, "no GitHub request");
    // Five minutes on, GitHub is asked again.
    fake.now.set(NOW + 301);
    assert_eq!(list(&fake, &cookie), first);
    assert!(fake.gh.calls.borrow().len() > calls);
}

#[test]
fn another_user() {
    // req:web.worker/repo-list@b7761292
    let (fake, cookie_a) = installed();
    list(&fake, &cookie_a);
    // B's installations cover another CLM repo.
    fake.gh.add_repo("p/b", FakeRepo::new("111", clm_repo()));
    fake.gh
        .installations
        .borrow_mut()
        .insert("v".into(), vec![(2, vec!["p/b".into()])]);
    let cookie_b = fake.user("v", &["p/b"]);
    fake.now.set(NOW + 60);
    let calls = fake.gh.calls.borrow().len();
    let b = list(&fake, &cookie_b);
    assert_eq!(
        b,
        json!([{"owner": "p", "name": "b", "default_branch": "main"}])
    );
    let with_b = fake.gh.calls.borrow()[calls..]
        .iter()
        .filter(|c| c.auth == clm_web_platform::platform::Auth::User("v".into()))
        .count();
    assert!(with_b > 0, "B's list comes from GitHub with B's token");
}

#[test]
fn empty_list() {
    // req:web.worker/repo-list@b7761292
    let fake = Fake::default();
    let cookie = fake.user("u", &[]);
    assert_eq!(list(&fake, &cookie), json!([]));
    // Since then the App was installed on a CLM repo.
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    fake.gh
        .installations
        .borrow_mut()
        .insert("u".into(), vec![(1, vec!["o/a".into()])]);
    fake.gh.grant("u", "o/a");
    fake.now.set(NOW + 60);
    let again = list(&fake, &cookie);
    assert_eq!(again[0]["name"], "a", "an empty list isn't memoised");
}
