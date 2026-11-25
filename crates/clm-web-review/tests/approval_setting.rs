//! A branch that changes `clm.toml`'s approval setting is a spec branch: the review lists
//! the file and approve accepts it, decided by `clm_gate::approval_changed`, the gate's own
//! rule (`approval-setting-branch`).

mod common;

use common::*;
use serde_json::{Value, json};

const MAIN: &str = "# settings\napproval = \"other\"\n";

/// `o/a` whose `main` sets `approval = "other"`, with branches that change only `clm.toml`:
/// `spec/policy` (the setting), `spec/comment` (a comment), `spec/broken` (no longer TOML),
/// `spec/binary` (bytes that aren't UTF-8), `spec/removed` (the file deleted) and
/// `spec/same` (a code change, `clm.toml` kept).
fn setup() -> (Fake, String) {
    let mut base = clm_repo();
    base.insert("clm.toml".into(), MAIN.as_bytes().to_vec());
    let mut repo = FakeRepo::new("111", base.clone());
    let with = |toml: &[u8]| {
        let mut files = base.clone();
        files.insert("clm.toml".into(), toml.to_vec());
        files
    };
    let mut removed = base.clone();
    removed.remove("clm.toml");
    let mut same = base.clone();
    same.insert("crates/x/src/lib.rs".into(), b"fn x() {}\n".to_vec());
    for (branch, files) in [
        ("spec/policy", with(b"# settings\napproval = \"any\"\n")),
        (
            "spec/comment",
            with(b"# the settings\napproval = \"other\"\n"),
        ),
        ("spec/broken", with(b"approval = \n")),
        ("spec/binary", with(b"approval = \"other\"\n\xff\xfe\n")),
        ("spec/removed", removed),
        ("spec/same", same),
    ] {
        let sha = branch.replace('/', "-");
        repo.commits.insert(sha.clone(), files);
        repo.branches.insert(branch.into(), sha);
    }
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo);
    let cookie = fake.user("u", &["o/a"]);
    (fake, cookie)
}

fn review(fake: &Fake, cookie: &str, branch: &str) -> Value {
    let res = fake.handle(get(
        &format!("/api/repos/o/a/branches/{branch}/review"),
        cookie,
    ));
    assert_eq!(res.status, 200, "{branch}");
    body_json(&res)
}

fn documents(review: &Value) -> Vec<&str> {
    review["documents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["path"].as_str().unwrap())
        .collect()
}

fn approve(fake: &Fake, cookie: &str, branch: &str, fingerprint: &str) -> u16 {
    let body = json!({ "fingerprint": fingerprint }).to_string();
    fake.handle(post(
        &format!("/api/repos/o/a/branches/{branch}/approve"),
        cookie,
        body.as_bytes(),
    ))
    .status
}

/// The fingerprint `clm fingerprint` gives `branch` against `main`.
fn fingerprint(fake: &Fake, branch: &str) -> String {
    let repos = fake.gh.repos.borrow();
    let repo = &repos["o/a"];
    let commit = |b: &str| &repo.commits[&repo.branches[b]];
    fingerprint_of(commit("main"), commit(branch))
}

/// The records on `clm/ledger/clunny/web`, or none.
fn ledger(fake: &Fake) -> Vec<Value> {
    let repos = fake.gh.repos.borrow();
    let repo = &repos["o/a"];
    let Some(tip) = repo.branches.get("clm/ledger/clunny/web") else {
        return Vec::new();
    };
    std::str::from_utf8(&repo.commits[tip]["records.jsonl"])
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[test]
fn only_the_policy_changes() {
    // req:web.worker.review/approval-setting-branch@7dd4d0b9
    let (fake, cookie) = setup();
    let review = review(&fake, &cookie, "spec/policy");
    assert_eq!(review["nodes"], json!([]));
    assert_eq!(review["change"], Value::Null);
    assert_eq!(documents(&review), ["clm.toml"]);
    let doc = &review["documents"][0];
    assert_eq!(doc["before"], MAIN);
    assert_eq!(doc["after"], "# settings\napproval = \"any\"\n");
    let f = fingerprint(&fake, "spec/policy");
    assert_eq!(review["fingerprint"], f.as_str());
    assert_eq!(approve(&fake, &cookie, "spec/policy", &f), 200);
    let records = ledger(&fake);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["kind"], "approved");
    assert_eq!(records[0]["fingerprint"], f.as_str());
    assert_eq!(records[0]["branch"], "spec/policy");
}

#[test]
fn a_comment_in_the_policy_file() {
    // req:web.worker.review/approval-setting-branch@7dd4d0b9
    let (fake, cookie) = setup();
    let review = review(&fake, &cookie, "spec/comment");
    assert!(documents(&review).is_empty(), "{review}");
    let f = fingerprint(&fake, "spec/comment");
    assert_eq!(approve(&fake, &cookie, "spec/comment", &f), 422);
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn the_file_no_longer_parses() {
    // req:web.worker.review/approval-setting-branch@7dd4d0b9
    let (fake, cookie) = setup();
    assert_eq!(
        documents(&review(&fake, &cookie, "spec/broken")),
        ["clm.toml"]
    );
}

#[test]
fn unreadable_text_counts_as_changed() {
    // req:web.worker.review/approval-setting-branch@7dd4d0b9
    // A side whose text GitHub withholds shows empty, not null as a missing file does.
    let (fake, cookie) = setup();
    let review = review(&fake, &cookie, "spec/binary");
    assert_eq!(documents(&review), ["clm.toml"]);
    assert_eq!(review["documents"][0]["before"], MAIN);
    assert_eq!(review["documents"][0]["after"], "");
    let f = fingerprint(&fake, "spec/binary");
    assert_eq!(approve(&fake, &cookie, "spec/binary", &f), 200);
    assert_eq!(ledger(&fake).len(), 1);
}

#[test]
fn withheld_on_both_sides() {
    // req:web.worker.review/approval-setting-branch@7dd4d0b9
    // GitHub withholds the file's text at the merge base and at the head: the setting
    // counts as changed, and the review lists the file, empty on both sides.
    let mut base = clm_repo();
    base.insert("clm.toml".into(), b"approval = \"other\"\n\xff\n".to_vec());
    let mut repo = FakeRepo::new("111", base.clone());
    let mut head = base;
    head.insert("clm.toml".into(), b"approval = \"any\"\n\xfe\n".to_vec());
    repo.commits.insert("spec-both".into(), head);
    repo.branches.insert("spec/both".into(), "spec-both".into());
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo);
    let cookie = fake.user("u", &["o/a"]);
    let review = review(&fake, &cookie, "spec/both");
    assert_eq!(review["review"], true);
    assert_eq!(documents(&review), ["clm.toml"]);
    assert_eq!(review["documents"][0]["before"], "");
    assert_eq!(review["documents"][0]["after"], "");
}

#[test]
fn an_unchanged_file_fetches_nothing() {
    // req:web.worker.review/approval-setting-branch@7dd4d0b9
    let (fake, cookie) = setup();
    assert!(documents(&review(&fake, &cookie, "spec/same")).is_empty());
    let fake = setup().0;
    let f = fingerprint(&fake, "spec/same");
    assert_eq!(approve(&fake, &cookie, "spec/same", &f), 422);
    assert!(fake.gh.writes().is_empty());
    assert!(
        fake.gh.fetched_paths().is_empty(),
        "approve fetched {:?}",
        fake.gh.fetched_paths()
    );
}

/// The `Blobs` queries made so far, each as the paths it names.
fn blob_queries(fake: &Fake) -> Vec<Vec<String>> {
    fake.gh
        .calls
        .borrow()
        .iter()
        .filter_map(|c| {
            let v: Value = serde_json::from_str(c.body.as_deref()?).ok()?;
            v["query"].as_str()?.contains("query Blobs").then(|| {
                v["variables"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .filter(|(k, _)| k.starts_with('e'))
                    .map(|(_, e)| e.as_str().unwrap().split_once(':').unwrap().1.into())
                    .collect()
            })
        })
        .collect()
}

#[test]
fn approve_fetches_the_two_texts_once() {
    // req:web.worker.review/approval-setting-branch@7dd4d0b9
    // The approve route lists trees but fetches no spec files: the file's two texts come in
    // one request, and nothing else is fetched.
    let (fake, cookie) = setup();
    let f = fingerprint(&fake, "spec/policy");
    assert_eq!(approve(&fake, &cookie, "spec/policy", &f), 200);
    assert_eq!(
        blob_queries(&fake),
        [vec!["clm.toml".to_string(), "clm.toml".to_string()]]
    );
}

#[test]
fn the_review_asks_nothing_more_for_the_file() {
    // req:web.worker.review/approval-setting-branch@7dd4d0b9
    // The review has both texts from the spec files it fetches, one per side, and makes no
    // request of its own for them.
    let (fake, cookie) = setup();
    review(&fake, &cookie, "spec/policy");
    let fetched = fake.gh.fetched_paths();
    let policy = fetched.iter().filter(|p| *p == "clm.toml").count();
    assert_eq!(policy, 2, "{fetched:?}");
    let queries = blob_queries(&fake);
    assert!(
        queries.iter().all(|q| q.len() > 1),
        "no query for clm.toml alone: {queries:?}"
    );
}

#[test]
fn a_side_without_the_file() {
    // req:web.worker.review/approval-setting-branch@7dd4d0b9
    // A head whose listing has no `clm.toml` counts as missing: the setting `main` sets is
    // gone, so the setting changed.
    let (fake, cookie) = setup();
    let review = review(&fake, &cookie, "spec/removed");
    assert_eq!(documents(&review), ["clm.toml"]);
    assert_eq!(review["documents"][0]["before"], MAIN);
    assert_eq!(review["documents"][0]["after"], Value::Null);
    let f = fingerprint(&fake, "spec/removed");
    assert_eq!(approve(&fake, &cookie, "spec/removed", &f), 200);
    assert_eq!(ledger(&fake).len(), 1);
}

#[test]
fn an_earlier_builds_review_isnt_served() {
    // req:web.worker.review/approval-setting-branch@7dd4d0b9
    // A comparison an earlier build kept for the branch, listing no documents, is not
    // served: the review is built again and lists `clm.toml`.
    let (fake, cookie) = setup();
    let old = json!({"change": null, "nodes": [], "decisions": [], "backlog": [],
                     "documents": [], "fingerprint": "f0f0", "code_key": null,
                     "review": false, "cuts": []});
    block_on(async {
        use clm_web_platform::platform::Cache;
        let keys = (1..=7).map(|v| format!("compare{v}:o/a@111:spec-policy"));
        for key in keys.chain(["compare:o/a@111:spec-policy".to_string()]) {
            fake.cache_put(&key, &old.to_string(), 86_400).await;
        }
    });
    let review = review(&fake, &cookie, "spec/policy");
    assert_eq!(documents(&review), ["clm.toml"]);
    assert_eq!(review["review"], true);
    assert_ne!(review["fingerprint"], "f0f0");
}
