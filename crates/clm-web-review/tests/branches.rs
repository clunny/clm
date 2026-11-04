//! The spec-branch list: every `spec/*` branch the review accepts, by name, with its head
//! and the page of its open pull request on the default branch, from the live state.

mod common;

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
    // req:web.worker.review/spec-branches@a0bae05a
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
    // req:web.worker.review/spec-branches@a0bae05a
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
