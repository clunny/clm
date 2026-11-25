//! The fake GitHub answers as GitHub does where the Worker's crates' tests depend on it.
//! These tests cite no requirement: `fake-github` also states that `pulls?head=` lists
//! `FakeRepo.pulls`, which waits on `backlog/fake-github-pulls-head-breaks-a-review-test`.

use clm_web_platform::fake::*;
use clm_web_platform::gh;
use clm_web_platform::platform::Auth;
use serde_json::{Value, json};

fn pull(number: u64, head: &str) -> FakePull {
    FakePull {
        number,
        state: "open".into(),
        title: format!("t{number}"),
        base_ref: "main".into(),
        head_ref: head.into(),
        head_sha: format!("h{number}"),
        ..FakePull::default()
    }
}

fn setup() -> Fake {
    let fake = Fake::default();
    let mut repo = FakeRepo::new("111", clm_repo());
    repo.pulls = (1..=150).map(|n| pull(n, &format!("spec/p{n}"))).collect();
    repo.pulls.insert(3, FakePull::default());
    repo.pages
        .push(("spec/c".into(), 5, "https://github.com/o/a/pull/5".into()));
    let commit = files(&[
        ("a/link", "b.md"),
        ("a/b.md", "bee"),
        ("a/c/d.md", "dee"),
        ("top.md", "top"),
    ]);
    repo.modes.insert("a/link".into(), "120000".into());
    repo.blob_fails.insert(blob_sha(b"bee"));
    repo.commits.insert("c1".into(), commit);
    repo.branches.insert("spec/x".into(), "111".into());
    fake.gh.add_repo("o/a", repo);
    fake.gh.grant("u", "o/a");
    fake
}

fn auth() -> Auth {
    Auth::User("u".into())
}

const PULLS: &str = "query Pulls($owner: String!, $name: String!, $after: String) { repository(owner: $owner, name: $name) { pullRequests(states: OPEN, first: 100, after: $after) { nodes { number } pageInfo { hasNextPage endCursor } } } }";

fn pulls_page(fake: &Fake, after: Value) -> Option<Value> {
    let vars = json!({"owner": "o", "name": "a", "after": after});
    let data = block_on(gh::graphql(fake, &auth(), PULLS, vars))?;
    Some(data["repository"]["pullRequests"].clone())
}

fn rest(fake: &Fake, path: &str) -> (u16, Value) {
    block_on(gh::get(fake, &auth(), &format!("/repos/o/a/{path}")))
}

fn numbers(v: &Value) -> Vec<u64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|p| p["number"].as_u64().unwrap())
        .collect()
}

#[test]
fn pages_of_open_pull_requests() {
    let fake = setup();
    let first = pulls_page(&fake, Value::Null).unwrap();
    assert_eq!(numbers(&first["nodes"]), (1..=100).collect::<Vec<_>>());
    assert_eq!(first["pageInfo"]["hasNextPage"], true);
    let second = pulls_page(&fake, first["pageInfo"]["endCursor"].clone()).unwrap();
    assert_eq!(numbers(&second["nodes"]), (101..=150).collect::<Vec<_>>());
    assert_eq!(second["pageInfo"]["hasNextPage"], false);
}

#[test]
fn modes_and_a_failed_blob() {
    let fake = setup();
    let query = "query Blobs($owner: String!, $name: String!, $e0: String!, $t0: String!, $t1: String!) { repository(owner: $owner, name: $name) { f0: object(expression: $e0) { ... on Blob { text isBinary isTruncated oid } } t0: object(expression: $t0) { ... on Tree { entries { name mode } } } t1: object(expression: $t1) { ... on Tree { entries { name mode } } } } }";
    let vars = json!({"owner": "o", "name": "a", "e0": "c1:a/b.md", "t0": "c1:a", "t1": "c1:"});
    let data = block_on(gh::graphql(&fake, &auth(), query, vars)).unwrap();
    let repo = &data["repository"];
    assert_eq!(repo["f0"]["text"], "bee");
    let entries = |t: &str| -> Vec<(String, u64)> {
        repo[t]["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                (
                    e["name"].as_str().unwrap().into(),
                    e["mode"].as_u64().unwrap(),
                )
            })
            .collect()
    };
    let named = |pairs: &[(&str, u64)]| -> Vec<(String, u64)> {
        pairs.iter().map(|(n, m)| (n.to_string(), *m)).collect()
    };
    assert_eq!(
        entries("t0"),
        named(&[("b.md", 33188), ("c", 16384), ("link", 40960)])
    );
    assert_eq!(entries("t1"), named(&[("a", 16384), ("top.md", 33188)]));
    let path = format!("git/blobs/{}", blob_sha(b"bee"));
    assert_eq!(rest(&fake, &path).0, 502, "a blob in blob_fails");
    let path = format!("git/blobs/{}", blob_sha(b"dee"));
    assert_eq!(rest(&fake, &path).0, 200, "another blob");
}

#[test]
fn a_page_and_failing_paths() {
    let fake = setup();
    let (status, page) = rest(&fake, "pulls?head=o:spec/c");
    assert_eq!((status, numbers(&page)), (200, vec![5]));
    assert_eq!(page[0]["html_url"], "https://github.com/o/a/pull/5");
    assert_eq!(page[0]["head"]["repo"]["full_name"], "o/a");
    assert_eq!(page[0]["base"]["repo"]["full_name"], "o/a");
    for failing in ["pulls", "git/matching-refs/heads/spec/"] {
        fake.gh
            .repos
            .borrow_mut()
            .get_mut("o/a")
            .unwrap()
            .fail_paths = vec![failing.into()];
        let pulls = rest(&fake, "pulls?head=o:spec/c");
        let refs = rest(&fake, "git/matching-refs/heads/spec/");
        let page = pulls_page(&fake, Value::Null);
        assert_eq!(
            page.unwrap()["nodes"].as_array().unwrap().len(),
            100,
            "{failing}"
        );
        if failing == "pulls" {
            assert_eq!(pulls.0, 502);
            assert_eq!((refs.0, refs.1.as_array().unwrap().len()), (200, 1));
        } else {
            assert_eq!((pulls.0, numbers(&pulls.1)), (200, vec![5]));
            assert_eq!(refs.0, 502);
        }
    }
}
