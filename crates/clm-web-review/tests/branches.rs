//! The spec-branch list: every `spec/*` branch the review accepts, by name, with its head
//! and the page of its open pull request on the default branch, from the live state.

mod common;

use clm_web_platform::access;
use clm_web_review::branches;
use common::*;
use serde_json::{Value, json};

const URL: &str = "/api/repos/o/a/branches";

fn pull(number: u64, title: &str, head: &str, base: &str, fork: bool) -> FakePull {
    FakePull {
        number,
        state: "open".into(),
        title: title.into(),
        base_ref: base.into(),
        base_sha: "000".into(),
        head_ref: head.into(),
        head_sha: "999".into(),
        fork,
    }
}

fn repo() -> FakeRepo {
    let mut repo = FakeRepo::new("000", clm_repo());
    for (branch, sha) in [
        ("spec/a", "111"),
        ("spec/b", "222"),
        ("impl/c", "333"),
        ("spec/.hidden", "444"),
        ("spec/a..b", "445"),
        ("spec/x.lock", "446"),
        ("specs/d", "555"),
    ] {
        repo.branches.insert(branch.into(), sha.into());
    }
    repo.pulls = vec![
        pull(7, "Add a", "spec/a", "main", false),
        // Neither is `spec/b`'s page: one is a fork's branch of the same name, the other
        // isn't on the default branch.
        pull(8, "Fork's b", "spec/b", "main", true),
        pull(9, "B on dev", "spec/b", "dev", false),
    ];
    repo
}

#[test]
fn with_and_without_a_page() {
    // req:web.worker.review/spec-branches@3a863b51
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo());
    let cookie = fake.user("u", &["o/a"]);
    let res = fake.handle(get(URL, &cookie));
    assert_eq!(res.status, 200, "{res:?}");
    assert_eq!(
        body_json(&res),
        json!([
            {
                "branch": "spec/a",
                "head_sha": "111",
                "page": {"number": 7, "url": "https://github.com/o/a/pull/7", "title": "Add a"},
            },
            {"branch": "spec/b", "head_sha": "222", "page": Value::Null},
        ])
    );
}

#[test]
fn answered_from_the_live_state() {
    // req:web.worker.review/spec-branches@3a863b51
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo());
    let cookie = fake.user("u", &["o/a"]);
    let first = body_json(&fake.handle(get(URL, &cookie)));
    fake.gh
        .repos
        .borrow_mut()
        .get_mut("o/a")
        .unwrap()
        .branches
        .insert("spec/c".into(), "666".into());
    let calls = fake.gh.calls.borrow().len();
    fake.now.set(NOW + 59);
    assert_eq!(body_json(&fake.handle(get(URL, &cookie))), first);
    assert_eq!(fake.gh.calls.borrow().len(), calls, "no GitHub request");
    fake.now.set(NOW + 61);
    let later = body_json(&fake.handle(get(URL, &cookie)));
    let names: Vec<&str> = later
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["branch"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["spec/a", "spec/b", "spec/c"]);
}

#[test]
fn github_fails_the_list() {
    // req:web.worker.review/spec-branches@3a863b51
    let list = |fake: &Fake| {
        block_on(async {
            let access = access::check(fake, "u", "o", "a").await.expect("access");
            branches::list(fake, &access).await
        })
    };
    let fake = Fake::default();
    let mut failing = repo();
    failing.fail_paths.push("git/matching-refs".into());
    fake.gh.add_repo("o/a", failing);
    fake.user("u", &["o/a"]);
    assert_eq!(list(&fake), None, "the refs can't be listed");

    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo());
    fake.user("u", &["o/a"]);
    fake.gh.graphql_null.borrow_mut().insert("Pulls".into());
    assert_eq!(list(&fake), None, "the pull requests can't be listed");
}
