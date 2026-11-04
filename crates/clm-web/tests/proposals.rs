//! Proposals for the backlog (`web-proposals-as-ledger-records`): a `proposed` record in
//! the requester's web ledger, its withdrawal, and the open proposals the implementation
//! response lists. The snapshot and implementation views are the `Replica` object's, which
//! the tests script.

mod common;

use clm_web_platform::http::Request;
use clm_web_platform::platform::Auth;
use common::*;
use serde_json::{Value, json};

const GIT: &str = "https://api.github.com/repos/o/a/git";

/// `o/a`, whose graph holds `a` and `b`, with `backlog/export-csv.md`, and alice signed in,
/// with `extra` done to the repository first. With `real`, a real `Replica` object is built
/// from it and answers the writes (`writes-to-object`); without, the views are scripted.
fn setup_as(real: bool, extra: impl FnOnce(&mut FakeRepo)) -> (Fake, String) {
    let mut files = clm_repo();
    files.insert(
        "specs/a/SPEC.md".into(),
        b"---\ntitle: A\nstatus: active\nowners: []\n---\n\n## Contract\na\n\n## Requirements\n\n### Requirement: p\nIt SHALL p.\n\n#### Scenario: s\n- ok\n".to_vec(),
    );
    files.insert(
        "specs/b/SPEC.md".into(),
        b"---\ntitle: B\nstatus: active\nowners: []\n---\n\n## Contract\nb\n".to_vec(),
    );
    files.insert(
        "backlog/export-csv.md".into(),
        b"---\ntitle: \"Export CSV\"\nkind: idea\ncomponents: [a]\n---\n\nWhy.\n".to_vec(),
    );
    let fake = Fake::default();
    fake.assets.borrow_mut().insert(
        "/_clm/invited.json".into(),
        ("application/json".into(), br#"["alice", "bob"]"#.to_vec()),
    );
    let mut repo = FakeRepo::new("111", files);
    extra(&mut repo);
    fake.gh.add_repo("o/a", repo);
    fake.gh
        .add_repo("o/private", FakeRepo::new("111", clm_repo()));
    fake.gh.grant("u", "o/a");
    if real {
        fake.real_object("o/a");
    } else {
        fake.set_view("o/a", "implementation", json!({"sha": "111"}));
    }
    let cookie = fake.cookie_as("u", "alice");
    (fake, cookie)
}

fn setup() -> (Fake, String) {
    setup_as(true, |_| {})
}

fn propose(fake: &Fake, cookie: &str, body: &Value) -> clm_web_platform::http::Response {
    fake.handle(post(
        "/api/repos/o/a/proposals",
        cookie,
        body.to_string().as_bytes(),
    ))
}

fn delete(path: &str, cookie: &str) -> Request {
    let mut req = post(path, cookie, b"");
    req.method = "DELETE".into();
    req
}

/// The records of `o/a`'s `clm/ledger/<writer>`, at its tip.
fn records(fake: &Fake, writer: &str) -> Vec<Value> {
    let repos = fake.gh.repos.borrow();
    let repo = &repos["o/a"];
    let Some(tip) = repo.branches.get(&format!("clm/ledger/{writer}")) else {
        return Vec::new();
    };
    std::str::from_utf8(&repo.commits[tip]["records.jsonl"])
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn proposed(id: &str, writer: &str, at: &str, slug: &str) -> Value {
    json!({
        "id": id, "writer": writer, "at": at, "kind": "proposed",
        "slug": slug, "title": slug, "body": "", "nodes": [],
    })
}

#[test]
fn a_proposal_is_appended() {
    // req:web.worker/proposals@e608c36e
    let (fake, cookie) = setup();
    let res = propose(
        &fake,
        &cookie,
        &json!({"title": " Export to JSON ", "body": "Let me export.\n", "nodes": ["a", "a"]}),
    );
    assert_eq!(res.status, 200, "{res:?}");
    let id = body_json(&res)["id"].clone();
    let writes: Vec<(String, String)> = fake
        .gh
        .writes()
        .iter()
        .map(|w| (w.method.to_string(), w.url.clone()))
        .collect();
    assert_eq!(
        writes,
        [
            ("POST".to_string(), format!("{GIT}/blobs")),
            ("POST".into(), format!("{GIT}/trees")),
            ("POST".into(), format!("{GIT}/commits")),
            ("POST".into(), format!("{GIT}/refs")),
        ]
    );
    assert!(
        fake.gh
            .writes()
            .iter()
            .all(|w| w.auth == Auth::User("u".into()))
    );
    let records = records(&fake, "alice/web");
    let last = records.last().unwrap();
    assert_eq!(last["id"], id);
    assert_eq!(last["kind"], "proposed");
    assert_eq!(last["writer"], "alice/web");
    assert_eq!(last["slug"], "export-to-json");
    assert_eq!(last["title"], "Export to JSON", "stored trimmed");
    assert_eq!(last["body"], "Let me export.");
    assert_eq!(last["nodes"], json!(["a"]), "deduplicated");
    let asked = fake.replicas.asked("o/a");
    assert_eq!(asked.len(), 1, "one post to the object");
    assert_eq!(asked[0].path, "/write");
}

#[test]
fn refused() {
    // req:web.worker/proposals@e608c36e
    let (fake, cookie) = setup();
    let res = propose(
        &fake,
        &cookie,
        &json!({"title": "Export CSV", "body": "Let me export.", "nodes": ["a"]}),
    );
    assert_eq!(res.status, 409, "the backlog has the slug");
    assert_eq!(body_json(&res), json!({"error": "taken"}));
    let res = propose(
        &fake,
        &cookie,
        &json!({"title": "Dark mode", "body": "", "nodes": ["nope"]}),
    );
    assert_eq!(res.status, 422);
    assert_eq!(body_json(&res), json!({"error": "nodes"}));
    assert!(fake.gh.writes().is_empty());
}
#[test]
fn taken_by_a_change() {
    // req:web.worker/proposals@e608c36e
    // A slug the replica's `resolved` holds is spent though no backlog item has it.
    let (fake, cookie) = setup_as(true, |repo| {
        repo.commits
            .get_mut("111")
            .unwrap()
            .remove("backlog/export-csv.md");
        repo.commits.get_mut("111").unwrap().insert(
            "changes/20270101-x/change.md".into(),
            b"---\nstatus: applied\nresolves:\n  - export-csv\n---\n\n# X\n".to_vec(),
        );
    });
    let res = propose(
        &fake,
        &cookie,
        &json!({"title": "Export CSV", "body": "", "nodes": []}),
    );
    assert_eq!(res.status, 409, "{res:?}");
    assert_eq!(body_json(&res), json!({"error": "taken"}));
    assert!(fake.gh.writes().is_empty(), "nothing written");
}

#[test]
fn an_open_proposal_takes_the_slug() {
    // req:web.worker/proposals@e608c36e
    // The replica's fold holds bob's open proposal, which takes the slug.
    let (fake, cookie) = setup_as(true, |repo| {
        let open = proposed("p0", "bob/web", "2027-01-15T07:00:00.000Z", "dark-mode");
        ledger(repo, "bob/web", "LB", &[open]);
    });
    let res = propose(
        &fake,
        &cookie,
        &json!({"title": "Dark Mode", "body": "", "nodes": []}),
    );
    assert_eq!(res.status, 409);
    assert_eq!(body_json(&res), json!({"error": "taken"}));
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn refused_before_asking_github() {
    // req:web.worker/proposals@e608c36e
    let (fake, cookie) = setup();
    let long_body = "x".repeat(8_001);
    let eleven: Vec<String> = (0..11).map(|i| format!("n{i}")).collect();
    for (body, field) in [
        (json!({"title": "", "body": "", "nodes": []}), "title"),
        (json!({"title": "   ", "body": "", "nodes": []}), "title"),
        (
            json!({"title": "a".repeat(121), "body": "", "nodes": []}),
            "title",
        ),
        (
            json!({"title": "Two\nlines", "body": "", "nodes": []}),
            "title",
        ),
        (json!({"body": "", "nodes": []}), "title"),
        (
            json!({"title": "Ok", "body": long_body, "nodes": []}),
            "body",
        ),
        (json!({"title": "Ok", "nodes": []}), "body"),
        (json!({"title": "Ok", "body": "", "nodes": eleven}), "nodes"),
        (json!({"title": "Ok", "body": "", "nodes": [1]}), "nodes"),
        (json!({"title": "Ok", "body": ""}), "nodes"),
        (json!({"title": "!!!", "body": "", "nodes": []}), "slug"),
    ] {
        let calls = fake.gh.calls.borrow().len();
        let res = propose(&fake, &cookie, &body);
        assert_eq!(res.status, 422, "{body}");
        assert_eq!(body_json(&res), json!({"error": field}), "{body}");
        assert_eq!(fake.gh.calls.borrow().len(), calls, "{body}: asked GitHub");
        assert!(
            fake.replicas.asked("o/a").is_empty(),
            "{body}: asked the object"
        );
    }
    // The longest title and body are accepted, and an empty node list.
    let res = propose(
        &fake,
        &cookie,
        &json!({"title": "é".repeat(120), "body": "é".repeat(8_000), "nodes": []}),
    );
    assert_eq!(res.status, 422, "a title of no ASCII word has no slug");
    let res = propose(
        &fake,
        &cookie,
        &json!({"title": "a".repeat(120), "body": "é".repeat(8_000), "nodes": []}),
    );
    assert_eq!(res.status, 200);
}

#[test]
fn no_access() {
    // req:web.worker/proposals@e608c36e
    let (fake, cookie) = setup();
    let body = json!({"title": "Dark mode", "body": "", "nodes": []}).to_string();
    let res = fake.handle(post(
        "/api/repos/o/private/proposals",
        &cookie,
        body.as_bytes(),
    ));
    assert_eq!(res.status, 404);
    let res = fake.handle(delete("/api/repos/o/private/proposals/p1", &cookie));
    assert_eq!(res.status, 404);
    assert!(fake.gh.writes().is_empty());
    assert!(
        fake.replicas.calls.borrow().is_empty(),
        "the object is asked nothing"
    );
}

#[test]
fn withdrawn() {
    // req:web.worker/proposals@e608c36e
    let (fake, cookie) = setup_as(true, |repo| {
        let p1 = proposed("p1", "alice/web", "2027-01-15T07:00:00.000Z", "s1");
        ledger(repo, "alice/web", "LA", &[p1]);
        let p2 = proposed("p2", "bob/web", "2027-01-15T07:00:00.000Z", "s2");
        ledger(repo, "bob/web", "LB", &[p2]);
    });
    let res = fake.handle(delete("/api/repos/o/a/proposals/p2", &cookie));
    assert_eq!(res.status, 404, "bob's");
    assert!(fake.gh.writes().is_empty());
    let res = fake.handle(delete("/api/repos/o/a/proposals/p1", &cookie));
    assert_eq!(res.status, 200);
    let records = records(&fake, "alice/web");
    assert_eq!(records.len(), 2);
    assert_eq!(records[1]["kind"], "withdrawn");
    assert_eq!(records[1]["proposal"], "p1");
    let writes = fake.gh.writes().len();
    let res = fake.handle(delete("/api/repos/o/a/proposals/p1", &cookie));
    assert_eq!(res.status, 404, "already withdrawn");
    assert_eq!(fake.gh.writes().len(), writes);
}

#[test]
fn open_proposals() {
    // req:web.worker/proposals@e608c36e
    let (fake, cookie) = setup_as(false, |_| {});
    let listed = [
        proposed("p2", "bob/web", "2027-01-15T07:10:00.000Z", "s2"),
        proposed("p1", "alice/web", "2027-01-15T07:00:00.000Z", "s1"),
    ];
    fake.set_view(
        "o/a",
        "implementation",
        json!({"sha": "111", "proposals": listed}),
    );
    let view = body_json(&fake.handle(get("/api/repos/o/a/implementation", &cookie)));
    let got: Vec<(&str, bool)> = view["proposals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["id"].as_str().unwrap(), p["mine"].as_bool().unwrap()))
        .collect();
    assert_eq!(got, [("p2", false), ("p1", true)], "the view's order");
    for (i, mine) in [false, true].into_iter().enumerate() {
        let mut expected = listed[i].clone();
        expected["mine"] = json!(mine);
        assert_eq!(view["proposals"][i], expected, "the other fields as given");
    }
}

#[test]
fn the_slug() {
    // req:web.worker/proposals@e608c36e
    let (fake, cookie) = setup();
    let titles = [
        "  Dark mode: on/off!  ".to_string(),
        "a".repeat(70),
        format!("{} bb", "a".repeat(62)),
    ];
    for title in &titles {
        let res = propose(
            &fake,
            &cookie,
            &json!({"title": title, "body": "", "nodes": []}),
        );
        assert_eq!(res.status, 200, "{title:?}: {res:?}");
    }
    let slugs: Vec<String> = records(&fake, "alice/web")
        .iter()
        .map(|r| r["slug"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        slugs,
        [
            "dark-mode-on-off".to_string(),
            "a".repeat(64),
            "a".repeat(62)
        ]
    );
    let (calls, asked) = (
        fake.gh.calls.borrow().len(),
        fake.replicas.asked("o/a").len(),
    );
    let res = propose(
        &fake,
        &cookie,
        &json!({"title": "!!!", "body": "", "nodes": []}),
    );
    assert_eq!(res.status, 422);
    assert_eq!(body_json(&res), json!({"error": "slug"}));
    assert_eq!(fake.gh.calls.borrow().len(), calls, "asked GitHub");
    assert_eq!(fake.replicas.asked("o/a").len(), asked, "asked the object");
}
