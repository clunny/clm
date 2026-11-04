//! A decision on an architectural constraint, from the browser: a `decided` record appended to
//! the user's web ledger as a capability's is.

mod common;

use std::path::PathBuf;

use clm_web_platform::platform::Auth;
use common::*;
use serde_json::{Value, json};

const ARCHITECTURE: &str = "## Constraints\n\n### Constraint: e\nGit SHALL hold it.\n\n#### Scenario: s\n- ok\n\n#### Enforced by\n- c/r\n";

/// `clm_repo` with `ARCHITECTURE.md` holding `e`, enforced by `c/r`, which has a friction flag.
fn files_now() -> FileMap {
    let mut f = clm_repo();
    f.extend(files(&[
        ("ARCHITECTURE.md", ARCHITECTURE),
        (
            "specs/c/SPEC.md",
            "---\ntitle: C\nstatus: active\nowners: []\n---\n\n## Contract\nc\n\n## Requirements\n\n### Requirement: r\nIt SHALL r.\n\n#### Scenario: s\n- ok\n\n#### Serves\n- architecture/e\n",
        ),
    ]));
    let version = graph(&f).requirement("c/r").unwrap().version().to_string();
    f.insert(
        "backlog/fr.md".into(),
        format!(
            "---\ntitle: R is hard\nkind: friction\ncomponents: [c]\nrequirement: c/r@{version}\n---\n\nIt needed a fake.\n"
        )
        .into_bytes(),
    );
    f
}

fn graph(files: &FileMap) -> clm_model::Graph {
    let files: clm_model::Files = files
        .iter()
        .map(|(path, bytes)| (PathBuf::from(path), bytes.clone()))
        .collect();
    clm_model::load_files(&files).0
}

fn setup_with(repo: FakeRepo) -> (Fake, String) {
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo);
    let cookie = fake.user("u", &["o/a"]);
    (fake, cookie)
}

fn setup() -> (Fake, String) {
    setup_with(FakeRepo::new("111", files_now()))
}

fn version() -> String {
    graph(&files_now())
        .entry("architecture/e")
        .unwrap()
        .version()
        .to_string()
}

fn decide(fake: &Fake, cookie: &str, slug: &str, body: &str) -> clm_web_platform::http::Response {
    fake.handle(post(
        &format!("/api/repos/o/a/architecture/{slug}/decide"),
        cookie,
        body.as_bytes(),
    ))
}

fn body(version: &str, value: &str, decision: &str) -> String {
    json!({"version": version, "value": value, "decision": decision}).to_string()
}

fn push(fake: &Fake, sha: &str, files: FileMap) {
    let mut repos = fake.gh.repos.borrow_mut();
    let repo = repos.get_mut("o/a").unwrap();
    repo.commits.insert(sha.into(), files);
    repo.branches.insert("main".into(), sha.into());
}

fn records(fake: &Fake, branch: &str) -> Vec<Value> {
    let repos = fake.gh.repos.borrow();
    let repo = &repos["o/a"];
    let tip = &repo.branches[branch];
    let text = std::str::from_utf8(&repo.commits[tip]["records.jsonl"]).unwrap();
    text.lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

const GIT: &str = "https://api.github.com/repos/o/a/git";

#[test]
fn decide_appends_decided() {
    // req:web.worker/architecture-decide@4b2aecfa
    let (fake, cookie) = setup();
    let res = decide(&fake, &cookie, "e", &body(&version(), "medium", "redesign"));
    assert_eq!(res.status, 200, "{res:?}");
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
    let records = records(&fake, "clm/ledger/clunny/web");
    let last = records.last().unwrap();
    assert_eq!(last["kind"], "decided");
    assert_eq!(last["capability"], "architecture/e");
    assert_eq!(last["writer"], "clunny/web");
    assert_eq!(last["value"], "medium");
    assert_eq!(last["decision"], "redesign");
    assert_eq!(last["settles"], json!(["fr"]));
    assert_eq!(&body_json(&res), last, "the answer is the record");
    // Settled now: a second decision settles nothing.
    let res = decide(&fake, &cookie, "e", &body(&version(), "high", "keep"));
    assert_eq!(res.status, 200);
    assert_eq!(body_json(&res)["settles"], json!([]));
}

#[test]
fn decide_percent_decodes_the_slug() {
    // req:web.worker/architecture-decide@4b2aecfa
    let (fake, cookie) = setup();
    assert_eq!(
        decide(&fake, &cookie, "%65", &body(&version(), "low", "cut")).status,
        200
    );
}

#[test]
fn not_an_entry_or_moved_version() {
    // req:web.worker/architecture-decide@4b2aecfa
    let (fake, cookie) = setup();
    let res = decide(&fake, &cookie, "nope", &body(&version(), "high", "keep"));
    assert_eq!(res.status, 422);
    assert!(res.body.is_empty());
    // A capability's slug isn't an architecture entry.
    let res = decide(&fake, &cookie, "x", &body(&version(), "high", "keep"));
    assert_eq!(res.status, 422);
    assert_eq!(
        decide(&fake, &cookie, "e", &body("0123abcd", "high", "keep")).status,
        409
    );
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn bad_body_and_broken_specs_422() {
    // req:web.worker/architecture-decide@4b2aecfa
    let (fake, cookie) = setup();
    let v = version();
    for bad in [
        String::new(),
        body("0123", "low", "cut"),
        body(&v, "huge", "cut"),
        body(&v, "low", "maybe"),
    ] {
        let before = fake.gh.calls.borrow().len();
        assert_eq!(decide(&fake, &cookie, "e", &bad).status, 422, "{bad:?}");
        assert_eq!(fake.gh.calls.borrow().len(), before, "no GitHub request");
    }
    let mut broken = files_now();
    broken.insert("specs/c/SPEC.md".into(), b"no frontmatter\n".to_vec());
    push(&fake, "444", broken);
    assert_eq!(
        decide(&fake, &cookie, "e", &body(&v, "low", "cut")).status,
        422
    );
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn signed_422() {
    // req:web.worker/architecture-decide@4b2aecfa
    let (fake, cookie) = setup();
    let mut signed = files_now();
    signed.insert(
        "docs/allowed_signers".into(),
        b"dana ssh-ed25519 AAAA\n".to_vec(),
    );
    push(&fake, "222", signed);
    let res = decide(&fake, &cookie, "e", &body("0123abcd", "high", "keep"));
    assert_eq!(res.status, 422, "422, not 409");
    assert_eq!(
        body_json(&res),
        json!({"error": "signed", "command": "clm architecture decide e --value high --decision keep"})
    );
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn github_fails_502() {
    // req:web.worker/architecture-decide@4b2aecfa
    let mut repo = FakeRepo::new("111", files_now());
    repo.signers_fail = true;
    let (fake, cookie) = setup_with(repo);
    assert_eq!(
        decide(&fake, &cookie, "e", &body(&version(), "low", "cut")).status,
        502
    );
    assert!(fake.gh.writes().is_empty());
}
