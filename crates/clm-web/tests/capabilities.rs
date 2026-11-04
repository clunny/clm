//! What each capability costs, and a decision on one, from the browser: the report `clm
//! capabilities` gives, without history, and a `decided` record appended to the user's web
//! ledger as approve appends.

mod common;

use std::path::PathBuf;

use clm_web_platform::platform::Auth;
use common::*;
use serde_json::{Value, json};

const CAPABILITIES: &str = "## Capabilities\n\n### Capability: x\nA person can x.\n\n#### Why\nx why\n\n#### Scenario: s\n- ok\n";

/// `clm_repo` with `CAPABILITIES.md` holding `x`, a node `c` serving it with requirement
/// `r`, and a friction flag `fr` on `c/r` at its version now.
fn files_now() -> FileMap {
    let mut f = clm_repo();
    f.extend(files(&[
        ("CAPABILITIES.md", CAPABILITIES),
        (
            "specs/c/SPEC.md",
            "---\ntitle: C\nstatus: active\nowners: []\nserves: [capability/x]\n---\n\n## Contract\nc\n\n## Requirements\n\n### Requirement: r\nIt SHALL r.\n\n#### Scenario: s\n- ok\n",
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

/// An `effort` record on `impl/c` worth a million Opus 5.5 input tokens.
fn effort() -> Value {
    json!({
        "id": "e1", "kind": "effort", "writer": "ann/1", "at": "2026-10-30T00:00:00.000Z",
        "branch": "impl/c", "session": "s1",
        "models": {"claude-opus-5-5": {"input": 1_000_000, "output": 0, "cache_read": 0, "cache_write": 0}},
        "active_seconds": 3600,
    })
}

fn repo() -> FakeRepo {
    let mut repo = FakeRepo::new("111", files_now());
    ledger(&mut repo, "ann/1", "L1", &[effort()]);
    repo
}

fn setup_with(repo: FakeRepo) -> (Fake, String) {
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo);
    let cookie = fake.user("u", &["o/a"]);
    (fake, cookie)
}

/// `x`'s version in `CAPABILITIES.md` now.
fn version() -> String {
    graph(&files_now())
        .entry("capability/x")
        .unwrap()
        .version()
        .to_string()
}

fn decide(fake: &Fake, cookie: &str, slug: &str, body: &str) -> clm_web_platform::http::Response {
    fake.handle(post(
        &format!("/api/repos/o/a/capabilities/{slug}/decide"),
        cookie,
        body.as_bytes(),
    ))
}

fn body(version: &str, value: &str, decision: &str) -> String {
    json!({"version": version, "value": value, "decision": decision}).to_string()
}

/// Moves `main` to a commit `222` that adds `docs/allowed_signers`.
fn sign_main(fake: &Fake) {
    let mut repos = fake.gh.repos.borrow_mut();
    let repo = repos.get_mut("o/a").unwrap();
    let mut signed = files_now();
    signed.insert(
        "docs/allowed_signers".into(),
        b"dana ssh-ed25519 AAAA\n".to_vec(),
    );
    repo.commits.insert("222".into(), signed);
    repo.branches.insert("main".into(), "222".into());
}

/// The records on `branch`'s tip, one JSON value per line.
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
fn the_objects_report() {
    // req:web.worker/capabilities@9e23bf9c
    let (fake, cookie) = setup_with(repo());
    let view = json!({"report": {"capabilities": [{"slug": "x"}]}, "signed": true});
    fake.set_view("o/a", "capabilities", view.clone());
    let res = fake.handle(get("/api/repos/o/a/capabilities", &cookie));
    assert_eq!(res.status, 200, "{res:?}");
    assert_eq!(body_json(&res), view, "the object's view, as it gave it");
    let asked = fake.replicas.asked("o/a");
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].path, "/view/capabilities?fresh=0");
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn decide_appends_decided() {
    // req:web.worker/decide@bfd28177
    let (fake, cookie) = setup_with(repo());
    let res = decide(&fake, &cookie, "x", &body(&version(), "low", "cut"));
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
    assert_eq!(last["capability"], "capability/x");
    assert_eq!(last["writer"], "clunny/web");
    assert_eq!(last["value"], "low");
    assert_eq!(last["decision"], "cut");
    assert_eq!(last["settles"], json!(["fr"]));
    assert_eq!(&body_json(&res), last, "the answer is the record");
    // The object is told the ledger moved, as GitHub's webhook will, so a read with `fresh`
    // right after shows the decision.
    let told = fake.replicas.asked("o/a");
    assert_eq!(told.len(), 1);
    let sent = told[0].body.as_ref().unwrap();
    assert_eq!(told[0].path, "/event");
    assert_eq!(sent["body"]["ref"], "refs/heads/clm/ledger/clunny/web");
    // Settled now: a second decision settles nothing.
    let res = decide(&fake, &cookie, "x", &body(&version(), "high", "keep"));
    assert_eq!(res.status, 200);
    assert_eq!(body_json(&res)["settles"], json!([]));
}

/// The files with `x` reworded.
fn reworded_files() -> FileMap {
    let mut reworded = files_now();
    reworded.insert(
        "CAPABILITIES.md".into(),
        CAPABILITIES
            .replace("A person can x.", "A person can x, better.")
            .into_bytes(),
    );
    reworded
}

/// Moves `main` to a commit `sha` holding `files`.
fn push(fake: &Fake, sha: &str, files: FileMap) {
    let mut repos = fake.gh.repos.borrow_mut();
    let repo = repos.get_mut("o/a").unwrap();
    repo.commits.insert(sha.into(), files);
    repo.branches.insert("main".into(), sha.into());
}

#[test]
fn decide_moved_version_409() {
    // req:web.worker/decide@bfd28177
    let (fake, cookie) = setup_with(repo());
    let seen = version();
    // `x` is reworded on main after the page was drawn.
    push(&fake, "333", reworded_files());
    assert_eq!(
        decide(&fake, &cookie, "x", &body(&seen, "low", "cut")).status,
        409
    );
    assert!(fake.gh.writes().is_empty());
    // The version decide compares is the reworded one, as soon as main holds it.
    let now = graph(&reworded_files())
        .entry("capability/x")
        .unwrap()
        .version()
        .to_string();
    assert_ne!(now, seen);
    assert_eq!(
        decide(&fake, &cookie, "x", &body(&now, "low", "cut")).status,
        200
    );
}

#[test]
fn decide_percent_decodes_the_slug() {
    // req:web.worker/decide@bfd28177
    let (fake, cookie) = setup_with(repo());
    assert_eq!(
        decide(&fake, &cookie, "%78", &body(&version(), "low", "cut")).status,
        200
    );
}

#[test]
fn decide_refuses_specs_that_dont_load() {
    // req:web.worker/decide@bfd28177
    let (fake, cookie) = setup_with(repo());
    let mut broken = files_now();
    broken.insert(
        "specs/c/SPEC.md".into(),
        b"no frontmatter
"
        .to_vec(),
    );
    push(&fake, "444", broken);
    assert_eq!(
        decide(&fake, &cookie, "x", &body(&version(), "low", "cut")).status,
        422
    );
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn github_fails_502() {
    // req:web.worker/decide@bfd28177
    let mut repo = repo();
    repo.signers_fail = true;
    let (fake, cookie) = setup_with(repo);
    assert_eq!(
        decide(&fake, &cookie, "x", &body(&version(), "low", "cut")).status,
        502
    );
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn decide_append_fails_502() {
    // req:web.worker/decide@bfd28177
    let mut repo = repo();
    // The user's ledger branch exists but its `records.jsonl` can't be read, so no append.
    repo.branches
        .insert("clm/ledger/clunny/web".into(), "999".into());
    repo.commits.insert("999".into(), FileMap::new());
    let (fake, cookie) = setup_with(repo);
    let res = decide(&fake, &cookie, "x", &body(&version(), "low", "cut"));
    assert_eq!(res.status, 502);
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn decide_bad_body_422_no_request() {
    // req:web.worker/decide@bfd28177
    let (fake, cookie) = setup_with(repo());
    let v = version();
    for bad in [
        String::new(),
        "not json".into(),
        "{}".into(),
        body("0123", "low", "cut"),
        body("0123abcg", "low", "cut"),
        body(&v.to_uppercase(), "low", "cut"),
        body(&v, "huge", "cut"),
        body(&v, "low", "maybe"),
        json!({"version": 7, "value": "low", "decision": "cut"}).to_string(),
    ] {
        let before = fake.gh.calls.borrow().len();
        assert_eq!(decide(&fake, &cookie, "x", &bad).status, 422, "{bad:?}");
        assert_eq!(
            fake.gh.calls.borrow().len(),
            before,
            "{bad:?}: no GitHub request"
        );
    }
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn decide_signed_422() {
    // req:web.worker/decide@bfd28177
    let (fake, cookie) = setup_with(repo());
    sign_main(&fake);
    let res = decide(&fake, &cookie, "x", &body("0123abcd", "high", "keep"));
    assert_eq!(res.status, 422, "422, not 409");
    assert_eq!(
        body_json(&res),
        json!({"error": "signed", "command": "clm capabilities decide x --value high --decision keep"})
    );
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn decide_unknown_capability_422() {
    // req:web.worker/decide@bfd28177
    let (fake, cookie) = setup_with(repo());
    let res = decide(&fake, &cookie, "nope", &body(&version(), "high", "keep"));
    assert_eq!(res.status, 422);
    assert!(res.body.is_empty());
    assert!(fake.gh.writes().is_empty());
}
