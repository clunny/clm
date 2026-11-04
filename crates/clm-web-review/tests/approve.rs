//! Approve appends an `approved` record with the branch's fingerprint to the user's web
//! ledger, `clm/ledger/<login>/web`, through the Git Data API with the user's own token,
//! and makes no other write.

mod common;

use clm_web_platform::platform::Auth;
use common::*;
use serde_json::{Value, json};

/// `o/a` with branches `spec/x` (a spec change, with change `x`), `spec/arch` (only
/// `ARCHITECTURE.md`, a spec path to the gate), `spec/code` and `impl/a` (only `crates/`).
fn repo() -> FakeRepo {
    let base = clm_repo();
    let mut spec = base.clone();
    spec.insert(
        "specs/a/SPEC.md".into(),
        node("", &["p", "x"], "active").into_bytes(),
    );
    spec.insert(
        "changes/x/change.md".into(),
        b"---\nstatus: proposed\n---\n\n## Intent\nAdd x.\n".to_vec(),
    );
    let mut code = base.clone();
    code.insert("crates/x/src/lib.rs".into(), b"fn x() {}\n".to_vec());
    let mut arch = base.clone();
    arch.insert(
        "ARCHITECTURE.md".into(),
        b"# Arch\n\nA new invariant.\n".to_vec(),
    );
    // Archives change `x` and adds change `z`.
    let mut archive = spec.clone();
    archive.remove("changes/x/change.md");
    archive.insert("changes/archive/x/change.md".into(), b"x\n".to_vec());
    archive.insert("changes/z/change.md".into(), b"z\n".to_vec());
    let mut repo = FakeRepo::new("111", base);
    repo.commits.insert("main".into(), spec.clone());
    for (branch, sha, files) in [
        ("spec/x", "spec", spec),
        ("spec/code", "code", code.clone()),
        ("impl/a", "code", code),
        ("spec/arch", "arch", arch),
        ("spec/z", "archive", archive),
    ] {
        repo.commits.insert(sha.into(), files);
        repo.branches.insert(branch.into(), sha.into());
    }
    repo
}

fn setup() -> (Fake, String) {
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo());
    let cookie = fake.user("u", &["o/a"]);
    (fake, cookie)
}

/// Approves `branch` naming its fingerprint now, as the app does with the review it drew.
fn approve(fake: &Fake, cookie: &str, branch: &str) -> u16 {
    let named = current(fake, branch).unwrap_or_else(|| "f".into());
    approve_with(
        fake,
        cookie,
        branch,
        &json!({"fingerprint": named}).to_string(),
    )
}

fn approve_with(fake: &Fake, cookie: &str, branch: &str, body: &str) -> u16 {
    fake.handle(post(
        &format!("/api/repos/o/a/branches/{branch}/approve"),
        cookie,
        body.as_bytes(),
    ))
    .status
}

/// `clm fingerprint` of `branch` against `main` as they are now, `None` when either is
/// missing.
fn current(fake: &Fake, branch: &str) -> Option<String> {
    let repos = fake.gh.repos.borrow();
    let repo = &repos["o/a"];
    let commit = |b: &str| repo.branches.get(b).and_then(|sha| repo.commits.get(sha));
    Some(fingerprint_of(commit("main")?, commit(branch)?))
}

/// `clm fingerprint` of the commit `sha` against `main`.
fn fingerprint(fake: &Fake, sha: &str) -> String {
    let repos = fake.gh.repos.borrow();
    let repo = &repos["o/a"];
    fingerprint_of(&repo.commits["111"], &repo.commits[sha])
}

/// The records on `branch`'s tip, one JSON value per line.
fn records(fake: &Fake, branch: &str) -> Vec<Value> {
    let repos = fake.gh.repos.borrow();
    let repo = &repos["o/a"];
    let tip = &repo.branches[branch];
    let text = std::str::from_utf8(&repo.commits[tip]["records.jsonl"]).unwrap();
    assert!(text.ends_with('\n'), "{text:?}");
    text.lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn write_urls(fake: &Fake) -> Vec<(String, String)> {
    fake.gh
        .writes()
        .iter()
        .map(|w| (w.method.to_string(), w.url.clone()))
        .collect()
}

const GIT: &str = "https://api.github.com/repos/o/a/git";

#[test]
fn approve_spec_branch() {
    // req:web.worker.review/approve@4584ce53
    let (fake, cookie) = setup();
    assert_eq!(approve(&fake, &cookie, "spec/x"), 200);
    assert_eq!(
        write_urls(&fake),
        [
            ("POST".to_string(), format!("{GIT}/blobs")),
            ("POST".into(), format!("{GIT}/trees")),
            ("POST".into(), format!("{GIT}/commits")),
            ("POST".into(), format!("{GIT}/refs")),
        ],
        "a blob, a tree, a commit and the ref, created on first use"
    );
    assert!(
        fake.gh
            .writes()
            .iter()
            .all(|w| w.auth == Auth::User("u".into()))
    );
    let records = records(&fake, "clm/ledger/clunny/web");
    assert_eq!(records.len(), 1);
    let r = &records[0];
    assert_eq!(r["kind"], "approved");
    assert_eq!(r["fingerprint"], fingerprint(&fake, "spec").as_str());
    assert_eq!(r["branch"], "spec/x");
    assert_eq!(r["change"], "x");
    assert_eq!(r["writer"], "clunny/web");
    assert_eq!(r["at"], "2027-01-15T08:00:00.000Z");
    assert_eq!(r["id"].as_str().unwrap().len(), 32);
    let tip = fake.gh.repos.borrow()["o/a"].branches["clm/ledger/clunny/web"].clone();
    assert_eq!(
        fake.gh.repos.borrow()["o/a"].parents[&tip],
        Vec::<String>::new()
    );
    // The review finds the record as `clm land` would, once its fold is fetched again.
    fake.now.set(NOW + 61);
    let review = body_json(&fake.handle(get("/api/repos/o/a/branches/spec/x/review", &cookie)));
    assert_eq!(review["approvals"][0]["writer"], "clunny/web");
}

#[test]
fn approve_appends() {
    // req:web.worker.review/approve@4584ce53
    // A ledger that exists gains one line on a commit whose parent is its tip, and the ref
    // moves without force; a last line without its newline still ends before the new one.
    let mut repo = repo();
    let old = r#"{"id":"0","writer":"bob/web","at":"2026-10-26T10:00:00.000Z","kind":"approved","fingerprint":"f0","branch":"spec/y","change":null}"#;
    repo.commits
        .insert("L1".into(), files(&[("records.jsonl", old)]));
    repo.branches
        .insert("clm/ledger/bob/web".into(), "L1".into());
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo);
    fake.gh.grant("v", "o/a");
    fake.assets.borrow_mut().insert(
        "/_clm/invited.json".into(),
        ("application/json".into(), br#"["clunny", "bob"]"#.to_vec()),
    );
    let cookie = fake.cookie_as("v", "bob");
    assert_eq!(approve(&fake, &cookie, "spec/arch"), 200);
    let writes = fake.gh.writes();
    assert_eq!(writes.len(), 4, "{writes:?}");
    assert_eq!(writes[3].method, "PATCH");
    assert_eq!(
        writes[3].url,
        format!("{GIT}/refs/heads/clm/ledger/bob/web")
    );
    let body: Value = serde_json::from_str(writes[3].body.as_deref().unwrap()).unwrap();
    assert_eq!(body["force"], false);
    let records = records(&fake, "clm/ledger/bob/web");
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["id"], "0");
    assert_eq!(records[1]["branch"], "spec/arch");
    assert_eq!(records[1]["writer"], "bob/web");
    assert_eq!(records[1]["change"], Value::Null, "no change is touched");
    let tip = fake.gh.repos.borrow()["o/a"].branches["clm/ledger/bob/web"].clone();
    assert_eq!(fake.gh.repos.borrow()["o/a"].parents[&tip], ["L1"]);
}

#[test]
fn change_as_the_cli_names_it() {
    // req:web.worker.review/approve@4584ce53
    let (fake, cookie) = setup();
    fake.gh
        .repos
        .borrow_mut()
        .get_mut("o/a")
        .unwrap()
        .branches
        .insert("main".into(), "main".into());
    assert_eq!(approve(&fake, &cookie, "spec/z"), 200);
    assert_eq!(records(&fake, "clm/ledger/clunny/web")[0]["change"], "z");
}

#[test]
fn code_branch() {
    // req:web.worker.review/approve@4584ce53
    let (fake, cookie) = setup();
    assert_eq!(approve(&fake, &cookie, "impl/a"), 422);
    assert_eq!(approve(&fake, &cookie, "spec/code"), 422);
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn approve_refuses_a_missing_or_odd_branch() {
    // req:web.worker.review/approve@4584ce53
    let (fake, cookie) = setup();
    assert_eq!(approve(&fake, &cookie, "spec/missing"), 422);
    assert_eq!(approve(&fake, &cookie, "spec%2F..%2Fmain"), 422);
    assert_eq!(approve(&fake, &cookie, "main"), 422);
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn truncated_listing() {
    // req:web.worker.review/approve@4584ce53
    // req:web.worker.review/fingerprint-from-trees@24bcb92e
    // With the merge base's listing truncated there's no fingerprint to approve.
    let mut repo = repo();
    repo.truncated_trees.insert("111".into());
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo);
    let cookie = fake.user("u", &["o/a"]);
    assert_eq!(approve(&fake, &cookie, "spec/x"), 422);
    assert!(fake.gh.writes().is_empty(), "nothing appended");
}

#[test]
fn pushed_after_the_review() {
    // req:web.worker.review/approve@4584ce53
    // The review was drawn at `spec`'s fingerprint; the author then pushed a commit that
    // changes the change's own `change.md`, an authored file, so the branch's fingerprint
    // now differs.
    let (fake, cookie) = setup();
    let shown = fingerprint(&fake, "spec");
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        let mut pushed = repo.commits["spec"].clone();
        pushed.insert(
            "changes/x/change.md".into(),
            b"---\nstatus: proposed\n---\n\n## Intent\nAdd x and y.\n".to_vec(),
        );
        repo.commits.insert("pushed".into(), pushed);
        repo.branches.insert("spec/x".into(), "pushed".into());
    }
    let body = json!({"fingerprint": shown}).to_string();
    assert_eq!(approve_with(&fake, &cookie, "spec/x", &body), 409);
    assert!(fake.gh.writes().is_empty(), "no write");
    // What the branch is now is approved once the person was shown it.
    assert_eq!(approve(&fake, &cookie, "spec/x"), 200);
}

#[test]
fn nothing_named() {
    // req:web.worker.review/approve@4584ce53
    // A body that names no fingerprint is refused before any request to GitHub.
    let (fake, cookie) = setup();
    for body in ["", "not json", "{}", r#"{"fingerprint": 7}"#, r#"["f"]"#] {
        let before = fake.gh.calls.borrow().len();
        assert_eq!(
            approve_with(&fake, &cookie, "spec/x", body),
            422,
            "{body:?}"
        );
        let after = fake.gh.calls.borrow().len();
        assert_eq!(after, before, "{body:?}: no GitHub request");
    }
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn refusals_before_a_moved_fingerprint() {
    // req:web.worker.review/approve@4584ce53
    // A branch that can't be approved at all gets 422 whatever fingerprint it names.
    let (fake, cookie) = setup();
    let stale = json!({"fingerprint": "stale"}).to_string();
    for branch in ["spec/missing", "spec/code", "impl/a", "main"] {
        assert_eq!(
            approve_with(&fake, &cookie, branch, &stale),
            422,
            "{branch}"
        );
    }
    assert!(fake.gh.writes().is_empty());
    // A truncated listing gives no fingerprint: 422, not 409.
    let mut repo = repo();
    repo.truncated_trees.insert("111".into());
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo);
    let cookie = fake.user("u", &["o/a"]);
    assert_eq!(approve_with(&fake, &cookie, "spec/x", &stale), 422);
    assert!(fake.gh.writes().is_empty());
}

/// `docs/allowed_signers` added to every commit, `main`'s tip among them, so no branch's
/// diff changes.
fn sign_main(fake: &Fake) {
    let mut repos = fake.gh.repos.borrow_mut();
    let repo = repos.get_mut("o/a").unwrap();
    for files in repo.commits.values_mut() {
        files.insert(
            "docs/allowed_signers".into(),
            b"dana ssh-ed25519 AAAA\n".to_vec(),
        );
    }
}

fn approve_res(
    fake: &Fake,
    cookie: &str,
    branch: &str,
    body: &str,
) -> clm_web_platform::http::Response {
    fake.handle(post(
        &format!("/api/repos/o/a/branches/{branch}/approve"),
        cookie,
        body.as_bytes(),
    ))
}

#[test]
fn signed_repository() {
    // req:web.worker.review/approve@4584ce53
    // req:web.worker.review/signed-repository@a9873234
    let (fake, cookie) = setup();
    sign_main(&fake);
    let named = current(&fake, "spec/x").unwrap();
    let body = json!({"fingerprint": named}).to_string();
    let res = approve_res(&fake, &cookie, "spec/x", &body);
    assert_eq!(res.status, 422);
    assert_eq!(
        body_json(&res),
        json!({"error": "signed", "command": "clm approve spec/x"})
    );
    assert!(fake.gh.writes().is_empty(), "no write");
}

#[test]
fn signed_and_pushed_after_the_review() {
    // req:web.worker.review/approve@4584ce53
    // req:web.worker.review/signed-repository@a9873234
    let (fake, cookie) = setup();
    sign_main(&fake);
    let stale = json!({"fingerprint": "f"}).to_string();
    let res = approve_res(&fake, &cookie, "spec/x", &stale);
    assert_eq!(res.status, 422, "422, not 409");
    assert_eq!(body_json(&res)["error"], "signed");
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn signed_refusals_come_after_the_branch_checks() {
    // req:web.worker.review/approve@4584ce53
    // req:web.worker.review/signed-repository@a9873234
    // A branch that can't be approved at all gets an empty 422, signed repository or not.
    let (fake, cookie) = setup();
    sign_main(&fake);
    let stale = json!({"fingerprint": "f"}).to_string();
    for branch in ["spec/missing", "spec/code", "impl/a"] {
        let res = approve_res(&fake, &cookie, branch, &stale);
        assert_eq!(res.status, 422, "{branch}");
        assert!(res.body.is_empty(), "{branch}");
    }
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn signed_lookup_fails() {
    // req:web.worker.review/approve@4584ce53
    // req:web.worker.review/signed-repository@a9873234
    let mut repo = repo();
    repo.signers_fail = true;
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo);
    let cookie = fake.user("u", &["o/a"]);
    let named = current(&fake, "spec/x").unwrap();
    let body = json!({"fingerprint": named}).to_string();
    let res = approve_res(&fake, &cookie, "spec/x", &body);
    assert_eq!(res.status, 502);
    assert!(fake.gh.writes().is_empty());
}
