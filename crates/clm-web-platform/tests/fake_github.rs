//! The fake GitHub answers as GitHub does where the Worker's crates' tests depend on it.

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
    repo.pulls[4].head_ref = "spec/c".into();
    repo.pulls[5] = FakePull {
        fork: true,
        ..pull(6, "spec/c2")
    };
    repo.pulls[6].head_ref = "spec/c2".into();
    repo.pulls[6].base_ref = "dev".into();
    // A closed pull request from `spec/c`, which no listing gives.
    repo.pulls.insert(
        3,
        FakePull {
            state: "closed".into(),
            ..pull(151, "spec/c")
        },
    );
    repo.pages.push((
        "spec/page".into(),
        200,
        "https://github.com/o/a/pull/200".into(),
    ));
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
    // req:web.worker.platform/fake-github@8915d6cf
    let fake = setup();
    let first = pulls_page(&fake, Value::Null).unwrap();
    assert_eq!(numbers(&first["nodes"]), (1..=100).collect::<Vec<_>>());
    assert_eq!(first["pageInfo"]["hasNextPage"], true);
    let second = pulls_page(&fake, first["pageInfo"]["endCursor"].clone()).unwrap();
    assert_eq!(numbers(&second["nodes"]), (101..=150).collect::<Vec<_>>());
    assert_eq!(second["pageInfo"]["hasNextPage"], false);
    // A smaller `first`, given as a variable.
    let query = PULLS.replace("first: 100", "first: $first");
    let vars = json!({"owner": "o", "name": "a", "after": "140", "first": 7});
    let data = block_on(gh::graphql(&fake, &auth(), &query, vars)).unwrap();
    let page = &data["repository"]["pullRequests"];
    assert_eq!(numbers(&page["nodes"]), (141..=147).collect::<Vec<_>>());
    assert_eq!(page["pageInfo"]["hasNextPage"], true);
    assert_eq!(page["pageInfo"]["endCursor"], "147");
}

#[test]
fn modes_and_a_failed_blob() {
    // req:web.worker.platform/fake-github@8915d6cf
    let fake = setup();
    let query = "query Blobs($owner: String!, $name: String!, $e0: String!, $t0: String!, $t1: String!) { repository(owner: $owner, name: $name) { f0: object(expression: $e0) { ... on Blob { text isBinary isTruncated oid } } t0: object(expression: $t0) { ... on Tree { entries { name mode } } } t1: object(expression: $t1) { ... on Tree { entries { name mode } } } } }";
    let vars = json!({"owner": "o", "name": "a", "e0": "c1:a/b.md", "t0": "c1:a", "t1": "c1:"});
    let data = block_on(gh::graphql(&fake, &auth(), query, vars)).unwrap();
    let repo = &data["repository"];
    assert_eq!(repo["f0"]["text"], "bee");
    assert_eq!(repo["f0"]["isBinary"], false);
    assert_eq!(repo["f0"]["isTruncated"], false);
    assert_eq!(repo["f0"]["oid"], blob_sha(b"bee"));
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
fn the_rest_pulls_by_head() {
    // req:web.worker.platform/fake-github@8915d6cf
    let fake = setup();
    let (status, own) = rest(&fake, "pulls?head=o:spec/c2&state=open");
    assert_eq!((status, numbers(&own)), (200, vec![7]), "not the fork's #6");
    assert_eq!(own[0]["html_url"], "https://github.com/o/a/pull/7");
    assert_eq!(own[0]["title"], "t7");
    assert_eq!(own[0]["head"]["ref"], "spec/c2");
    assert_eq!(own[0]["head"]["sha"], "h7");
    assert_eq!(own[0]["head"]["repo"]["full_name"], "o/a");
    assert_eq!(own[0]["base"]["ref"], "dev", "the pull's own base");
    assert_eq!(own[0]["base"]["repo"]["full_name"], "o/a");
    let (_, c) = rest(&fake, "pulls?head=o%3Aspec%2Fc");
    assert_eq!(numbers(&c), vec![5], "not the closed #151");
    let (_, forked) = rest(&fake, "pulls?head=fork:spec/c2");
    assert_eq!(numbers(&forked), vec![6]);
    assert_eq!(forked[0]["html_url"], "https://github.com/o/a/pull/6");
    assert_eq!(forked[0]["title"], "t6");
    assert_eq!(forked[0]["head"]["ref"], "spec/c2");
    assert_eq!(forked[0]["head"]["sha"], "h6");
    assert_eq!(forked[0]["head"]["repo"]["full_name"], "fork/a");
    assert_eq!(forked[0]["base"]["repo"]["full_name"], "o/a");
    let (_, page) = rest(&fake, "pulls?head=o:spec/page");
    assert_eq!(numbers(&page), vec![200], "a page of `pages`");
    assert_eq!(page[0]["html_url"], "https://github.com/o/a/pull/200");
    assert_eq!(page[0]["head"]["ref"], "spec/page");
    assert_eq!(page[0]["head"]["repo"]["full_name"], "o/a");
    assert_eq!(page[0]["base"]["ref"], "main");
    assert_eq!(page[0]["base"]["repo"]["full_name"], "o/a");
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        // A page of a pull request `pulls` holds too is listed once.
        repo.pages
            .push(("spec/c".into(), 5, "https://github.com/o/a/pull/5".into()));
        repo.fork_owner = "f".into();
    }
    assert_eq!(numbers(&rest(&fake, "pulls?head=o:spec/c").1), vec![5]);
    let (_, forked) = rest(&fake, "pulls?head=f:spec/c2");
    assert_eq!(numbers(&forked), vec![6]);
    assert_eq!(forked[0]["head"]["repo"]["full_name"], "f/a");
    assert!(numbers(&rest(&fake, "pulls?head=fork:spec/c2").1).is_empty());
}

#[test]
fn failing_paths() {
    // req:web.worker.platform/fake-github@8915d6cf
    let fake = setup();
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

#[test]
fn the_base_a_listing_names() {
    // req:web.worker.platform/fake-github@8915d6cf
    let fake = Fake::default();
    let mut repo = FakeRepo::new("111", clm_repo());
    assert_eq!(repo.default_branch, "main");
    repo.pulls = vec![
        FakePull {
            base_ref: "release".into(),
            ..pull(8, "spec/c3")
        },
        pull(9, "spec/c3"),
    ];
    repo.pages
        .push(("spec/p".into(), 10, "https://github.com/o/a/pull/10".into()));
    fake.gh.add_repo("o/a", repo);
    fake.gh.grant("u", "o/a");
    let asked = |q: &str| numbers(&rest(&fake, &format!("pulls?{q}")).1);
    assert_eq!(asked("head=o:spec/c3&base=main"), vec![9]);
    assert_eq!(asked("head=o:spec/c3&base=release"), vec![8]);
    assert_eq!(asked("head=o:spec/c3"), vec![8, 9]);
    assert_eq!(asked("head=o:spec/p&base=main"), vec![10]);
    assert!(asked("head=o:spec/p&base=release").is_empty());
    // `<base>` is percent-decoded as `head=` is.
    assert_eq!(asked("head=o%3Aspec%2Fc3&base=rel%65ase"), vec![8]);
    // A page whose number is a pull request's is that pull request, with its base.
    fake.gh
        .repos
        .borrow_mut()
        .get_mut("o/a")
        .unwrap()
        .pages
        .push(("spec/c3".into(), 8, "https://github.com/o/a/pull/8".into()));
    assert_eq!(asked("head=o:spec/c3&base=main"), vec![9]);
    assert_eq!(asked("head=o:spec/c3&base=release"), vec![8]);
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        repo.pulls[0].state = "closed".into();
        repo.pages[1].0 = "spec/c4".into();
    }
    let (_, closed) = rest(&fake, "pulls?head=o:spec/c4&base=release");
    assert_eq!(
        numbers(&closed),
        vec![8],
        "its base, though not listed open"
    );
    assert_eq!(closed[0]["base"]["ref"], "release");
    assert!(asked("head=o:spec/c4&base=main").is_empty());
}
