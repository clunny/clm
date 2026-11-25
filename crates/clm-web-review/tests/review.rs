//! A spec branch's review: per touched node, every changed element with its text before
//! and after, plus the change, decisions, backlog edits, the branch's fingerprint, the
//! ledger's approvals of it, whether its head's code was checked, and its page.

mod common;

use common::*;
use serde_json::{Value, json};

fn approved(writer: &str, fingerprint: &str, at: &str) -> Value {
    json!({
        "id": format!("{writer}-{at}"), "writer": writer, "at": at, "kind": "approved",
        "fingerprint": fingerprint, "branch": "spec/c2", "change": "c2",
    })
}

const URL: &str = "/api/repos/o/a/branches/spec/c2/review";

fn spec_pr() -> FakeRepo {
    let mut base = clm_repo();
    base.insert(
        "specs/n/SPEC.md".into(),
        node("", &["r", "s", "gone"], "active").into_bytes(),
    );
    base.extend(files(&[
        ("specs/m/SPEC.md", &node("", &["x"], "active")),
        ("specs/m/assets/old.html", "<p>old</p>\n"),
        ("specs/m/assets/keep.html", "<p>v1</p>\n"),
    ]));
    let mut head = base.clone();
    head.remove("specs/m/assets/old.html");
    head.extend(files(&[
        (
            "specs/m/SPEC.md",
            &node("", &["x"], "active")
                .replace("title: T", "title: M")
                .replace("## Contract\nc\n", "## Contract\nc, wider\n"),
        ),
        ("specs/m/assets/keep.html", "<p>v2</p>\n"),
    ]));
    let changed = node("", &["r", "s", "new"], "active")
        .replace("It SHALL r.", "It SHALL r, faster.")
        .replace("## Design\nd\n", "## Design\nd, redesigned\n");
    head.insert("specs/n/SPEC.md".into(), changed.into_bytes());
    head.extend(files(&[
        ("specs/n/assets/m.html", "<p>mockup</p>\n"),
        (
            "docs/decisions/d.md",
            "---\ntitle: D\ndate: 2026-10-01\nstatus: accepted\n---\nWhy.\n",
        ),
        (
            "changes/c2/change.md",
            "---\nstatus: applied\nfrom: new\nresolves: [old]\n---\n\n## Intent\nMake r faster.\n\n## Other\nMore.\n",
        ),
        ("backlog/new.md", "---\ntitle: New\nkind: idea\ncomponents: [n]\n---\n"),
    ]));
    let mut repo = FakeRepo::new("111", base);
    repo.commits.insert("222".into(), head);
    repo.branches.insert("spec/c2".into(), "222".into());
    repo.pulls.push(FakePull {
        number: 5,
        state: "open".into(),
        title: "Make r faster".into(),
        base_ref: "main".into(),
        base_sha: "111".into(),
        head_ref: "spec/c2".into(),
        head_sha: "222".into(),
        ..FakePull::default()
    });
    repo
}

/// What `clm fingerprint` prints for `spec/c2` at `222`.
fn fingerprint(repo: &FakeRepo) -> String {
    fingerprint_of(&repo.commits["111"], &repo.commits["222"])
}

fn review() -> Value {
    review_of(spec_pr())
}

fn review_of(repo: FakeRepo) -> Value {
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo);
    let cookie = fake.user("u", &["o/a"]);
    let res = fake.handle(get(URL, &cookie));
    assert_eq!(res.status, 200, "{res:?}");
    body_json(&res)
}

#[test]
fn modified_requirement() {
    // req:web.worker.review/pr-review@5982714a
    let v = review();
    let nodes = v["nodes"].as_array().unwrap();
    let ids: Vec<&str> = nodes.iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["m", "n"], "only m and n changed");
    let n = &nodes[1];
    let modified = n["requirements"]["modified"].as_array().unwrap();
    assert_eq!(modified.len(), 1);
    assert_eq!(modified[0]["id"], "n/r");
    assert!(
        modified[0]["before"]
            .as_str()
            .unwrap()
            .contains("It SHALL r.")
    );
    assert!(
        modified[0]["after"]
            .as_str()
            .unwrap()
            .contains("It SHALL r, faster.")
    );
    assert_eq!(n["requirements"]["added"][0]["id"], "n/new");
    assert!(n["requirements"]["added"][0]["before"].is_null());
    assert_eq!(n["requirements"]["removed"][0]["id"], "n/gone");
    assert!(n["requirements"]["removed"][0]["after"].is_null());
    assert!(
        n["design"]["after"]
            .as_str()
            .unwrap()
            .contains("redesigned")
    );
    assert!(n["contract"].is_null(), "the Contract didn't change");
    assert!(n["frontmatter"].is_null());
    assert_eq!(n["assets"]["added"][0]["path"], "m.html");
    assert_eq!(n["assets"]["added"][0]["after"], "<p>mockup</p>\n");

    assert_eq!(v["decisions"][0]["name"], "d");
    assert_eq!(v["decisions"][0]["status"], "added");
    assert_eq!(v["backlog"][0]["slug"], "new");
    assert_eq!(v["backlog"][0]["status"], "added");
    assert_eq!(v["branch"], "spec/c2");
    assert_eq!(v["head_sha"], "222");
    assert_eq!(v["base_sha"], "111");
}

#[test]
fn approval_state() {
    // req:web.worker.review/pr-review@5982714a
    // req:web.worker.review/fingerprint-from-trees@e49d6717
    let mut repo = spec_pr();
    let fingerprint = fingerprint(&repo);
    ledger(
        &mut repo,
        "clunny/web",
        "L1",
        &[
            approved("clunny/web", &fingerprint, "2026-10-26T10:00:00.000Z"),
            approved("clunny/web", "f0f0", "2026-10-26T09:00:00.000Z"),
        ],
    );
    ledger(&mut repo, "bob/laptop", "L2", &[]);
    let v = review_of(repo);
    assert_eq!(
        v["fingerprint"],
        fingerprint.as_str(),
        "as `clm fingerprint`"
    );
    assert_eq!(
        v["approvals"],
        json!([{"writer": "clunny/web", "at": "2026-10-26T10:00:00.000Z", "signed": false}]),
        "approved by clunny/web, and only for this diff"
    );
}

#[test]
fn no_ledger() {
    // req:web.worker.review/pr-review@5982714a
    // `spec_pr` opens #5 from `spec/c2`, which is that branch's page: take it away, so
    // the branch has no pull request and so no page.
    let mut repo = spec_pr();
    repo.pulls.clear();
    let v = review_of(repo);
    assert_eq!(v["approvals"], json!([]));
    assert_eq!(v["checked"], false);
    assert_eq!(v["page"], Value::Null, "no page without a PR");
}

#[test]
fn approval_of_an_older_diff() {
    // req:web.worker.review/pr-review@5982714a
    // An approval is pinned to the diff it was given for: a push that changes the diff
    // leaves it behind. The older diff differs in a decision, an authored file: the branch
    // adds a change, so its `specs/` don't count (`gate/fingerprint`).
    let mut repo = spec_pr();
    let mut older = repo.commits["222"].clone();
    older.insert(
        "docs/decisions/d.md".into(),
        b"---\ntitle: D\ndate: 2026-10-01\nstatus: accepted\n---\nDraft.\n".to_vec(),
    );
    let stale = fingerprint_of(&repo.commits["111"], &older);
    ledger(
        &mut repo,
        "clunny/web",
        "L1",
        &[approved("clunny/web", &stale, "2026-10-26T10:00:00.000Z")],
    );
    assert_eq!(review_of(repo)["approvals"], json!([]));
}

#[test]
fn checked_head() {
    // req:web.worker.review/pr-review@5982714a
    let mut repo = spec_pr();
    let tree: Vec<(String, String)> = repo.commits["222"]
        .iter()
        .map(|(path, bytes)| (path.clone(), blob_sha(bytes)))
        .collect();
    let key = clm_web_platform::keys::code_key(tree.iter().map(|(path, sha)| {
        clm_web_platform::keys::TreeLine {
            mode: "100644",
            kind: "blob",
            sha,
            path,
        }
    }));
    let checked = json!({
        "id": "c1", "writer": "bob/laptop", "at": "2026-10-26T10:00:00.000Z",
        "kind": "checked", "code_key": key, "command": "ci-check.sh",
    });
    ledger(
        &mut repo,
        "bob/laptop",
        "L2",
        std::slice::from_ref(&checked),
    );
    assert_eq!(review_of(repo.clone())["checked"], true);

    let mut other = checked;
    other["code_key"] = json!("0000");
    ledger(&mut repo, "bob/laptop", "L2", &[other]);
    assert_eq!(review_of(repo)["checked"], false, "another tree's check");
}

#[test]
fn checked_needs_the_whole_tree() {
    // req:web.worker.review/pr-review@5982714a
    let mut repo = spec_pr();
    let tree: Vec<(String, String)> = repo.commits["222"]
        .iter()
        .map(|(path, bytes)| (path.clone(), blob_sha(bytes)))
        .collect();
    let key = clm_web_platform::keys::code_key(tree.iter().map(|(path, sha)| {
        clm_web_platform::keys::TreeLine {
            mode: "100644",
            kind: "blob",
            sha,
            path,
        }
    }));
    let checked = json!({
        "id": "c1", "writer": "bob/laptop", "at": "2026-10-26T10:00:00.000Z",
        "kind": "checked", "code_key": key, "command": "ci-check.sh",
    });
    ledger(&mut repo, "bob/laptop", "L2", &[checked]);
    repo.truncated_trees.insert("222".into());
    assert_eq!(
        review_of(repo)["checked"],
        false,
        "a truncated listing has no code key, so no check names it"
    );
}

#[test]
fn page_url() {
    // req:web.worker.review/pr-review@5982714a
    let mut repo = spec_pr();
    repo.pages
        .push(("spec/c2".into(), 5, "https://github.com/o/a/pull/5".into()));
    repo.pages.push((
        "spec/other".into(),
        6,
        "https://github.com/o/a/pull/6".into(),
    ));
    let v = review_of(repo);
    assert_eq!(
        v["page"],
        json!({"number": 5, "url": "https://github.com/o/a/pull/5"})
    );
}

#[test]
fn only_spec_branches() {
    // req:web.worker.review/pr-review@5982714a
    let mut repo = spec_pr();
    repo.branches.insert("impl/n".into(), "222".into());
    // `spec/x`'s ref lookup fails; `spec/y` names a commit GitHub can't compare.
    repo.branches.insert("spec/x".into(), "222".into());
    repo.ref_fails.insert("spec/x".into());
    repo.branches.insert("spec/y".into(), "999".into());
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo);
    let cookie = fake.user("u", &["o/a"]);
    for path in [
        "/api/repos/o/a/branches/impl/n/review",
        "/api/repos/o/a/branches/spec/missing/review",
        "/api/repos/o/a/branches/spec/../main/review",
        "/api/repos/o/a/branches/spec/review",
    ] {
        assert_eq!(fake.handle(get(path, &cookie)).status, 404, "{path}");
    }
    for path in [
        "/api/repos/o/a/branches/spec/x/review",
        "/api/repos/o/a/branches/spec/y/review",
    ] {
        assert_eq!(fake.handle(get(path, &cookie)).status, 502, "{path}");
    }
}

#[test]
fn what_the_branch_changes_whole() {
    // req:web.worker.review/pr-review@5982714a
    let caps = "## Capabilities\n\n### Capability: k\nOne can k.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n";
    let wider = caps.replace("One can k.", "One can k, fast.");
    let (before, after) = (node("", &["r"], "active"), node("", &["r", "s"], "active"));
    let mut repo = branch_from(
        &[
            ("specs/n/SPEC.md", &before),
            ("CAPABILITIES.md", caps),
            ("CLAUDE.md", "Read me.\n"),
            ("docs/old.md", "Gone.\n"),
            (".gitattributes", "*.lock linguist-generated\n"),
        ],
        &["docs/old.md"],
        &[
            ("specs/n/SPEC.md", &after),
            ("CAPABILITIES.md", &wider),
            ("CLAUDE.md", "Read me first.\n"),
            ("docs/allowed_signers", "alice ssh-ed25519 AAAA\n"),
            (".gitattributes", "Cargo.lock linguist-generated\n"),
        ],
    );
    // A binary file: GitHub gives no text for it.
    repo.commits.get_mut("222").unwrap().insert(
        "docs/img.png".into(),
        vec![0x89, b'P', b'N', b'G', 0xff, 0xfe],
    );
    let v = review_of(repo);
    assert_eq!(
        v["documents"],
        json!([
            {"path": "CAPABILITIES.md", "before": caps, "after": wider},
            {"path": "CLAUDE.md", "before": "Read me.\n", "after": "Read me first.\n"},
            {"path": "docs/allowed_signers", "before": null, "after": "alice ssh-ed25519 AAAA\n"},
            {"path": "docs/img.png", "before": null, "after": ""},
            {"path": "docs/old.md", "before": "Gone.\n", "after": null},
            {"path": "specs/n/SPEC.md", "before": before, "after": after},
        ]),
        "no .gitattributes"
    );
}

#[test]
fn a_binary_asset() {
    // req:web.worker.review/pr-review@5982714a
    let mut repo = branch_from(
        &[("specs/n/SPEC.md", &node("", &["r"], "active"))],
        &[],
        &[],
    );
    let base = repo.commits.get_mut("111").unwrap();
    base.insert("specs/n/assets/a.bin".into(), vec![0xff, 0x00, 0x01]);
    let head = repo.commits.get_mut("222").unwrap();
    head.insert("specs/n/assets/a.bin".into(), vec![0xff, 0x00, 0x02]);
    head.insert(
        "specs/n/assets/logo.png".into(),
        vec![0x89, b'P', b'N', b'G', 0xff, 0xfe],
    );
    let v = review_of(repo);
    let n = &v["nodes"][0];
    assert_eq!(n["id"], "n", "{v}");
    assert_eq!(
        n["assets"],
        json!({
            "added": [{"path": "logo.png", "before": null, "after": ""}],
            "modified": [{"path": "a.bin", "before": "", "after": ""}],
            "removed": [],
        })
    );
}

#[test]
fn the_page() {
    // req:web.worker.review/pr-review@5982714a
    // `spec_pr` opens #5 from `spec/c2` on `main`; a fork opens #6 from its own `spec/c2`.
    let mut repo = spec_pr();
    repo.pulls.push(FakePull {
        number: 6,
        state: "open".into(),
        title: "Fork's c2".into(),
        base_ref: "main".into(),
        base_sha: "111".into(),
        head_ref: "spec/c2".into(),
        head_sha: "333".into(),
        fork: true,
    });
    assert_eq!(
        review_of(repo.clone())["page"],
        json!({"number": 5, "url": "https://github.com/o/a/pull/5"})
    );
    repo.fail_paths.push("pulls".into());
    assert_eq!(
        review_of(repo)["page"],
        Value::Null,
        "the review still answers"
    );
}

#[test]
fn a_pull_request_onto_another_base() {
    // req:web.worker.review/pr-review@5982714a
    // `spec/c3`'s only open pull request, #8, is onto `base`; the default branch is `default`.
    let page = |default: &str, base: &str| {
        let mut repo = spec_pr();
        repo.default_branch = default.into();
        repo.branches.insert(default.into(), "111".into());
        repo.branches.insert("spec/c3".into(), "222".into());
        repo.pulls.push(FakePull {
            number: 8,
            state: "open".into(),
            title: "Onto another base".into(),
            base_ref: base.into(),
            base_sha: "111".into(),
            head_ref: "spec/c3".into(),
            head_sha: "222".into(),
            fork: false,
        });
        let fake = Fake::default();
        fake.gh.add_repo("o/a", repo);
        let cookie = fake.user("u", &["o/a"]);
        let res = fake.handle(get("/api/repos/o/a/branches/spec/c3/review", &cookie));
        assert_eq!(res.status, 200, "{res:?}");
        body_json(&res)["page"].clone()
    };
    assert_eq!(page("main", "release"), Value::Null);
    // The default branch is the repository's, not always `main`.
    assert_eq!(page("trunk", "main"), Value::Null);
    assert_eq!(page("trunk", "trunk")["number"], 8);
}

#[test]
fn contract_frontmatter_and_assets() {
    // req:web.worker.review/pr-review@5982714a
    let v = review();
    let m = &v["nodes"][0];
    assert_eq!(m["id"], "m");
    assert_eq!(m["status"], "modified");
    assert_eq!(m["contract"]["before"].as_str().unwrap().trim(), "c");
    assert_eq!(m["contract"]["after"].as_str().unwrap().trim(), "c, wider");
    assert!(
        m["frontmatter"]["before"]
            .as_str()
            .unwrap()
            .contains("title: T")
    );
    assert!(
        m["frontmatter"]["after"]
            .as_str()
            .unwrap()
            .contains("title: M")
    );
    assert!(m["design"].is_null());
    assert!(m["requirements"]["modified"].as_array().unwrap().is_empty());
    let modified = &m["assets"]["modified"][0];
    assert_eq!(modified["path"], "keep.html");
    assert_eq!(modified["before"], "<p>v1</p>\n");
    assert_eq!(modified["after"], "<p>v2</p>\n");
    let removed = &m["assets"]["removed"][0];
    assert_eq!(removed["path"], "old.html");
    assert_eq!(removed["before"], "<p>old</p>\n");
    assert!(removed["after"].is_null());
}

#[test]
fn capabilities_edit_is_a_document() {
    // req:web.worker.review/pr-review@5982714a
    let mut repo = spec_pr();
    let caps = "## Capabilities\n\n### Capability: k\nOne can k.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n";
    repo.commits
        .get_mut("222")
        .unwrap()
        .insert("CAPABILITIES.md".into(), caps.as_bytes().to_vec());
    let v = review_of(repo);
    let docs = v["documents"].as_array().unwrap();
    let doc = docs.iter().find(|d| d["path"] == "CAPABILITIES.md");
    assert_eq!(doc.expect("a document")["after"], caps, "{docs:?}");
}

#[test]
fn change_intent_and_documents() {
    // req:web.worker.review/pr-review@5982714a
    let v = review();
    let c = &v["change"];
    assert_eq!(c["id"], "c2");
    assert!(c.get("review").is_none(), "{c}");
    assert_eq!(v["review"], false, "no burden and no human layer");
    assert_eq!(c["from"], "new");
    assert_eq!(c["resolves"][0], "old");
    assert_eq!(c["intent_section"], "Make r faster.");
    assert!(c.get("significant").is_none(), "{c}");
    assert!(c.get("gate").is_none(), "{c}");
    assert!(c["intent"].as_str().unwrap().contains("## Other"));
    // Each changed file, whole, before and after, for the full-document view.
    let docs = v["documents"].as_array().unwrap();
    let spec = docs
        .iter()
        .find(|d| d["path"] == "specs/n/SPEC.md")
        .unwrap();
    assert!(spec["before"].as_str().unwrap().contains("It SHALL r."));
    assert!(
        spec["after"]
            .as_str()
            .unwrap()
            .contains("It SHALL r, faster.")
    );
}

/// A repository at `111` and the spec branch `spec/c2` at `222`, whose head is `111`'s
/// files with `edits`.
fn branch_with(edits: &[(&str, &str)]) -> FakeRepo {
    let base = clm_repo();
    let mut head = base.clone();
    head.extend(files(edits));
    let mut repo = FakeRepo::new("111", base);
    repo.commits.insert("222".into(), head);
    repo.branches.insert("spec/c2".into(), "222".into());
    repo
}

/// `branch_with(edits)` whose merge base `111` also holds `base`, and whose head holds it
/// too but for the paths `removed`.
fn branch_from(base: &[(&str, &str)], removed: &[&str], edits: &[(&str, &str)]) -> FakeRepo {
    let mut repo = branch_with(&[]);
    for sha in ["111", "222"] {
        repo.commits.get_mut(sha).unwrap().extend(files(base));
    }
    let head = repo.commits.get_mut("222").unwrap();
    for path in removed {
        head.remove(*path);
    }
    head.extend(files(edits));
    repo
}

fn cuts(v: &Value) -> Vec<&str> {
    v["cuts"]
        .as_array()
        .unwrap_or_else(|| panic!("no cuts: {v}"))
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect()
}

#[test]
fn a_removed_requirement() {
    // req:web.worker.review/gate-answer@d38075fd
    let v = review_of(branch_from(
        &[("specs/n/SPEC.md", &node("", &["r", "s"], "active"))],
        &[],
        &[("specs/n/SPEC.md", &node("", &["s"], "active"))],
    ));
    assert_eq!(v["review"], true, "{v}");
    assert_eq!(
        cuts(&v),
        ["specs/n/SPEC.md: gate/cut: requirement n/r is gone"]
    );
}

#[test]
fn a_shortened_requirement() {
    // req:web.worker.review/gate-answer@d38075fd
    // Every cut the gate lists, in its order: a requirement stating fewer sentences too.
    let two = node("", &["r", "s"], "active").replace("It SHALL r.", "It SHALL r. It SHALL q.");
    let v = review_of(branch_from(
        &[("specs/n/SPEC.md", &two)],
        &[],
        &[("specs/n/SPEC.md", &node("", &["r", "s"], "active"))],
    ));
    assert_eq!(v["review"], true, "{v}");
    assert_eq!(
        cuts(&v),
        ["specs/n/SPEC.md: gate/cut: n/r states 1 sentences where it stated 2"]
    );
}

#[test]
fn cuts_in_the_gates_order() {
    // req:web.worker.review/gate-answer@d38075fd
    // Several cuts come as `clm gate` prints them: gone requirements, then shortened ones,
    // then the tests the lists add.
    let two =
        node("", &["r", "s", "t"], "active").replace("It SHALL s.", "It SHALL s. It SHALL q.");
    let v = review_of(branch_from(
        &[("specs/n/SPEC.md", &two)],
        &[],
        &[
            ("specs/n/SPEC.md", &node("", &["s", "t"], "active")),
            (
                "changes/x/change.md",
                "---\nstatus: applied\n---\n\n## Intent\nx\n",
            ),
            ("changes/x/deleted-tests.txt", "DELETE tests/a.rs::t\n"),
        ],
    ));
    assert_eq!(
        cuts(&v),
        [
            "specs/n/SPEC.md: gate/cut: requirement n/r is gone",
            "specs/n/SPEC.md: gate/cut: n/s states 1 sentences where it stated 2",
            "changes/x/deleted-tests.txt: gate/cut: test tests/a.rs::t is to be deleted",
        ],
        "{v}"
    );
}

#[test]
fn a_widened_exemption() {
    // req:web.worker.review/gate-answer@d38075fd
    let exempt = node("", &["r"], "active").replace(
        "depends_on: []\n",
        "depends_on: []\nexemptions:\n  - rule: skipped-tests\n    files: [\"src/n/**\"]\n    until: backlog/x\n",
    );
    assert_ne!(exempt, node("", &["r"], "active"));
    let v = review_of(branch_from(
        &[("specs/n/SPEC.md", &node("", &["r"], "active"))],
        &[],
        &[("specs/n/SPEC.md", &exempt)],
    ));
    assert_eq!(v["review"], true, "{v}");
    assert_eq!(cuts(&v), Vec::<&str>::new());
}

#[test]
fn specs_that_dont_load() {
    // req:web.worker.review/gate-answer@d38075fd
    // The head's specs don't load, so the requirements can't be compared: the gate's one
    // line, naming the head, and review.
    let unscenarioed = node("", &["r"], "active").replace("\n#### Scenario: s\n- ok\n", "");
    let v = review_of(branch_from(
        &[("specs/n/SPEC.md", &node("", &["r"], "active"))],
        &[],
        &[("specs/n/SPEC.md", &unscenarioed)],
    ));
    assert_eq!(v["review"], true, "{v}");
    assert_eq!(
        cuts(&v),
        ["specs: gate/cut: the specs at 222 don't load, so what the branch cuts isn't listed"]
    );
}

#[test]
fn base_specs_that_dont_load() {
    // req:web.worker.review/gate-answer@d38075fd
    // The merge base's specs don't load: the one line names the merge base.
    let unscenarioed = node("", &["r"], "active").replace("\n#### Scenario: s\n- ok\n", "");
    let v = review_of(branch_from(
        &[("specs/n/SPEC.md", &unscenarioed)],
        &[],
        &[("specs/n/SPEC.md", &node("", &["r"], "active"))],
    ));
    assert_eq!(
        cuts(&v),
        ["specs: gate/cut: the specs at 111 don't load, so what the branch cuts isn't listed"]
    );
}

#[test]
fn a_broken_live_change() {
    // req:web.worker.review/gate-answer@d38075fd
    // The gate loads the live `change.md` files, so one the model rejects at the head keeps
    // the specs there from loading.
    let v = review_of(branch_with(&[
        ("specs/a/SPEC.md", &node("", &["p", "added"], "active")),
        ("changes/x/change.md", "---\n: [\n---\n"),
    ]));
    assert_eq!(
        cuts(&v),
        ["specs: gate/cut: the specs at 222 don't load, so what the branch cuts isn't listed"]
    );
}

#[test]
fn a_rule_serving_a_capability() {
    // req:web.worker.review/gate-answer@d38075fd
    // The gate loads the human layers, so a rule serving a capability resolves: a removed
    // requirement is listed as gone, not hidden behind specs that seem not to load.
    let caps = "## Capabilities\n\n### Capability: k\nOne can k.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n";
    let serving = |reqs: &[&str]| {
        node("", reqs, "active").replace(
            "depends_on: []\n",
            "depends_on: []\nserves: [capability/k]\n",
        )
    };
    let v = review_of(branch_from(
        &[
            ("CAPABILITIES.md", caps),
            ("specs/n/SPEC.md", &serving(&["r", "s"])),
        ],
        &[],
        &[("specs/n/SPEC.md", &serving(&["s"]))],
    ));
    assert_eq!(
        cuts(&v),
        ["specs/n/SPEC.md: gate/cut: requirement n/r is gone"],
        "{v}"
    );
}

#[test]
fn a_spec_github_withholds() {
    // req:web.worker.review/gate-answer@d38075fd
    // A SPEC.md whose text GitHub gives as null is one the gate can't load either: its one
    // line, not a cut for each requirement the file seems to lose.
    let mut repo = branch_from(
        &[("specs/n/SPEC.md", &node("", &["r", "s"], "active"))],
        &[],
        &[],
    );
    let mut bytes = node("", &["r", "s"], "active").into_bytes();
    bytes.push(0xff);
    repo.commits
        .get_mut("222")
        .unwrap()
        .insert("specs/n/SPEC.md".into(), bytes);
    let v = review_of(repo);
    assert_eq!(v["review"], true, "{v}");
    assert_eq!(
        cuts(&v),
        ["specs: gate/cut: the specs at 222 don't load, so what the branch cuts isn't listed"]
    );
}

#[test]
fn a_broken_backlog_item() {
    // req:web.worker.review/gate-answer@d38075fd
    // The cuts are read from the files the gate loads, so a backlog item the model rejects,
    // which the gate never sees, doesn't make the specs fail to load.
    let bad = "---\ntitle: Bad\nkind: nonsense\ncomponents: [n]\n---\n";
    let repo = branch_from(
        &[("specs/n/SPEC.md", &node("", &["r"], "active"))],
        &[],
        &[
            ("specs/n/SPEC.md", &node("", &["r", "added"], "active")),
            ("backlog/bad.md", bad),
        ],
    );
    let head: clm_model::Files = repo.commits["222"]
        .iter()
        .map(|(p, b)| (p.into(), b.clone()))
        .collect();
    let (_, violations) = clm_model::load_files(&head);
    assert!(
        violations.iter().any(|v| v.path.starts_with("backlog")),
        "the model rejects the item: {violations:?}"
    );
    let v = review_of(repo);
    assert_eq!(cuts(&v), Vec::<&str>::new(), "{v}");
}

#[test]
fn a_broken_archived_change() {
    // req:web.worker.review/gate-answer@d38075fd
    // The gate loads only the live `change.md` files, so an archived one the model rejects
    // doesn't make the specs fail to load either.
    let repo = branch_from(
        &[
            ("specs/n/SPEC.md", &node("", &["r"], "active")),
            ("changes/archive/x/change.md", "---\n: [\n---\n"),
        ],
        &[],
        &[("specs/n/SPEC.md", &node("", &["r", "added"], "active"))],
    );
    let head: clm_model::Files = repo.commits["222"]
        .iter()
        .map(|(p, b)| (p.into(), b.clone()))
        .collect();
    let (_, violations) = clm_model::load_files(&head);
    assert!(
        violations
            .iter()
            .any(|v| v.path.starts_with("changes/archive")),
        "the model rejects the archived change: {violations:?}"
    );
    let v = review_of(repo);
    assert_eq!(cuts(&v), Vec::<&str>::new(), "{v}");
}

#[test]
fn untouched_specs_list_no_cuts() {
    // req:web.worker.review/gate-answer@d38075fd
    // The cuts are read only when a changed path is under `specs/` or a list: a branch that
    // edits a decision lists none, though the specs on both sides don't load.
    let unscenarioed = node("", &["r"], "active").replace("\n#### Scenario: s\n- ok\n", "");
    let v = review_of(branch_from(
        &[("specs/n/SPEC.md", &unscenarioed)],
        &[],
        &[(
            "docs/decisions/d.md",
            "---\ntitle: D\ndate: 2026-10-01\nstatus: accepted\n---\nWhy.\n",
        )],
    ));
    assert_eq!(cuts(&v), Vec::<&str>::new(), "{v}");
}

#[test]
fn a_list_github_withholds() {
    // req:web.worker.review/gate-answer@d38075fd
    // A list whose text GitHub gives as null can't be read, so the branch needs review, and
    // its cuts say so rather than list nothing for it.
    let mut repo = branch_with(&[(
        "changes/c/change.md",
        "---\nstatus: applied\n---\n\n## Intent\nc\n",
    )]);
    repo.commits.get_mut("222").unwrap().insert(
        "changes/c/deleted-tests.txt".into(),
        b"DELETE tests/a.rs::\xff\n".to_vec(),
    );
    let v = review_of(repo);
    assert_eq!(v["review"], true, "{v}");
    assert_eq!(
        cuts(&v),
        [
            "changes/c/deleted-tests.txt: gate/cut: GitHub withholds its text at 222, so the tests it lists aren't listed"
        ]
    );
}

#[test]
fn a_list_github_withholds_at_the_merge_base() {
    // req:web.worker.review/gate-answer@d38075fd
    // A list whose text is withheld at the merge base can't tell which of its head's lines
    // are new: none is listed, and one line names the list and the merge base.
    let change = "---\nstatus: applied\n---\n\n## Intent\nc\n";
    let mut repo = branch_from(
        &[("changes/c/change.md", change)],
        &[],
        &[(
            "changes/c/deleted-tests.txt",
            "DELETE tests/t.rs::one\nDELETE tests/t.rs::two\n",
        )],
    );
    repo.commits.get_mut("111").unwrap().insert(
        "changes/c/deleted-tests.txt".into(),
        b"DELETE tests/t.rs::\xff\n".to_vec(),
    );
    let v = review_of(repo);
    assert_eq!(v["review"], true, "{v}");
    assert_eq!(
        cuts(&v),
        [
            "changes/c/deleted-tests.txt: gate/cut: GitHub withholds its text at 111, so the tests it lists aren't listed"
        ]
    );

    // Withheld at both sides, the merge base is named, as the first side that withholds it.
    let mut repo = branch_from(&[("changes/c/change.md", change)], &[], &[]);
    for (sha, bytes) in [("111", &b"\xff1\n"[..]), ("222", &b"\xff2\n"[..])] {
        repo.commits
            .get_mut(sha)
            .unwrap()
            .insert("changes/c/deleted-tests.txt".into(), bytes.to_vec());
    }
    let v = review_of(repo);
    assert_eq!(
        cuts(&v),
        [
            "changes/c/deleted-tests.txt: gate/cut: GitHub withholds its text at 111, so the tests it lists aren't listed"
        ]
    );
}

#[test]
fn an_archived_list_github_withholds_at_the_merge_base() {
    // req:web.worker.review/gate-answer@d38075fd
    // An archived list the merge base doesn't hold reads its base text from the live list
    // there, as the gate does: when that text is withheld, the list's lines aren't listed.
    let change = "---\nstatus: applied\n---\n\n## Intent\nx\n";
    let mut repo = branch_from(
        &[("changes/x/change.md", change)],
        &["changes/x/change.md"],
        &[
            ("changes/archive/x/change.md", change),
            (
                "changes/archive/x/deleted-tests.txt",
                "DELETE tests/t.rs::one\n",
            ),
        ],
    );
    repo.commits.get_mut("111").unwrap().insert(
        "changes/x/deleted-tests.txt".into(),
        b"DELETE tests/t.rs::\xff\n".to_vec(),
    );
    let v = review_of(repo);
    assert_eq!(v["review"], true, "{v}");
    assert_eq!(
        cuts(&v),
        [
            "changes/archive/x/deleted-tests.txt: gate/cut: GitHub withholds its text at 111, so the tests it lists aren't listed"
        ]
    );
}

#[test]
fn withheld_lists_follow_the_gates_lines() {
    // req:web.worker.review/gate-answer@d38075fd
    // One line per withheld list, after the gate's own lines.
    let change = "---\nstatus: applied\n---\n\n## Intent\nc\n";
    let mut repo = branch_from(
        &[
            ("specs/n/SPEC.md", &node("", &["r", "s"], "active")),
            ("changes/a/change.md", change),
            ("changes/b/change.md", change),
        ],
        &[],
        &[("specs/n/SPEC.md", &node("", &["s"], "active"))],
    );
    let head = repo.commits.get_mut("222").unwrap();
    for id in ["a", "b"] {
        head.insert(
            format!("changes/{id}/deleted-tests.txt"),
            b"\xff\n".to_vec(),
        );
    }
    let v = review_of(repo);
    assert_eq!(
        cuts(&v),
        [
            "specs/n/SPEC.md: gate/cut: requirement n/r is gone",
            "changes/a/deleted-tests.txt: gate/cut: GitHub withholds its text at 222, so the tests it lists aren't listed",
            "changes/b/deleted-tests.txt: gate/cut: GitHub withholds its text at 222, so the tests it lists aren't listed",
        ]
    );
}

#[test]
fn a_withheld_list_in_a_truncated_listing() {
    // req:web.worker.review/gate-answer@d38075fd
    // A truncated listing lists no cuts, a withheld list's line included.
    let mut repo = branch_with(&[(
        "changes/c/change.md",
        "---\nstatus: applied\n---\n\n## Intent\nc\n",
    )]);
    repo.commits
        .get_mut("222")
        .unwrap()
        .insert("changes/c/deleted-tests.txt".into(), b"\xff\n".to_vec());
    repo.truncated_trees.insert("222".into());
    let v = review_of(repo);
    assert_eq!(v["review"], true, "{v}");
    assert_eq!(cuts(&v), Vec::<&str>::new(), "{v}");
}

#[test]
fn a_truncated_listing_lists_no_cuts() {
    // req:web.worker.review/gate-answer@d38075fd
    // Cuts read from part of the files would mislead: none are listed, and review holds.
    let mut repo = branch_from(
        &[("specs/n/SPEC.md", &node("", &["r", "s"], "active"))],
        &[],
        &[("specs/n/SPEC.md", &node("", &["s"], "active"))],
    );
    repo.truncated_trees.insert("222".into());
    let v = review_of(repo);
    assert_eq!(v["review"], true, "{v}");
    assert_eq!(cuts(&v), Vec::<&str>::new(), "{v}");
}

#[test]
fn a_test_list_cut() {
    // req:web.worker.review/gate-answer@d38075fd
    // A change listing a test to delete needs review, whether or not the test exists: that
    // is the gate's to check, as the Worker fetches no code.
    let change = "---\nstatus: applied\n---\n\n## Intent\nx\n";
    let v = review_of(branch_with(&[
        ("changes/x/change.md", change),
        (
            "changes/x/deleted-tests.txt",
            "# gone\nDELETE tests/a.rs::t\tbecause\n",
        ),
    ]));
    assert_eq!(v["review"], true, "{v}");
    assert_eq!(
        cuts(&v),
        ["changes/x/deleted-tests.txt: gate/cut: test tests/a.rs::t is to be deleted"]
    );

    // A list the base already held a line of adds only what's new.
    let v = review_of(branch_from(
        &[
            ("changes/x/change.md", change),
            ("changes/x/deleted-tests.txt", "DELETE tests/a.rs::t\n"),
        ],
        &[],
        &[(
            "changes/x/deleted-tests.txt",
            "DELETE tests/a.rs::t\nDELETE tests/a.rs::u\n",
        )],
    ));
    assert_eq!(
        cuts(&v),
        ["changes/x/deleted-tests.txt: gate/cut: test tests/a.rs::u is to be deleted"]
    );
}

#[test]
fn an_archived_test_list() {
    // req:web.worker.review/gate-answer@d38075fd
    // An archive move takes the live list's lines as its base, as the gate does: nothing is
    // cut anew.
    let change = "---\nstatus: applied\n---\n\n## Intent\nx\n";
    let list = "DELETE tests/a.rs::t\n";
    let v = review_of(branch_from(
        &[
            ("changes/x/change.md", change),
            ("changes/x/deleted-tests.txt", list),
        ],
        &["changes/x/change.md", "changes/x/deleted-tests.txt"],
        &[
            ("changes/archive/x/change.md", change),
            ("changes/archive/x/deleted-tests.txt", list),
        ],
    ));
    assert_eq!(cuts(&v), Vec::<&str>::new(), "{v}");
    assert_eq!(v["review"], false, "{v}");
}

#[test]
fn no_cuts() {
    // req:web.worker.review/gate-answer@d38075fd
    // `spec_pr` drops `n/gone`, but its scenario moves to `n/new`: nothing is cut.
    let v = review();
    assert_eq!(cuts(&v), Vec::<&str>::new(), "{v}");
}

#[test]
fn human_layer_without_a_change() {
    // req:web.worker.review/gate-answer@d38075fd
    let v = review_of(branch_with(&[("ARCHITECTURE.md", "# Arch\n\nMore.\n")]));
    assert_eq!(v["review"], true, "{v}");
    assert_eq!(v["change"], Value::Null);
}

#[test]
fn a_change_with_burdens() {
    // req:web.worker.review/gate-answer@d38075fd
    let burdened = review_of(branch_with(&[(
        "changes/x/change.md",
        "---\nstatus: applied\nhuman_burdens: [b]\n---\n\n## Intent\nx\n",
    )]));
    assert_eq!(burdened["review"], true, "{burdened}");
    assert_eq!(burdened["change"]["id"], "x");
    assert!(burdened["change"].get("review").is_none());

    let plain = review_of(branch_with(&[
        (
            "changes/y/change.md",
            "---\nstatus: applied\n---\n\n## Intent\ny\n",
        ),
        (
            "docs/decisions/d.md",
            "---\ntitle: D\ndate: 2026-10-01\nstatus: accepted\n---\nWhy.\n",
        ),
    ]));
    assert_eq!(plain["review"], false, "{plain}");
    assert_eq!(plain["change"]["id"], "y");
    assert!(plain["change"].get("review").is_none());
}

#[test]
fn a_change_whose_change_md_is_missing() {
    // req:web.worker.review/gate-answer@d38075fd
    // A change the branch touches whose `change.md` the head lacks, or can't be parsed,
    // fails closed, as the gate does.
    let delta = "---\nbase: {}\n---\n";
    let missing = review_of(branch_with(&[("changes/z/deltas/a.md", delta)]));
    assert_eq!(missing["review"], true, "{missing}");
    let broken = review_of(branch_with(&[("changes/z/change.md", "---\n: [\n---\n")]));
    assert_eq!(broken["review"], true, "{broken}");
}

#[test]
fn an_archived_change_with_burdens() {
    // req:web.worker.review/gate-answer@d38075fd
    let v = review_of(branch_with(&[(
        "changes/archive/x/change.md",
        "---\nstatus: applied\nhuman_burdens: [b]\n---\n\n## Intent\nx\n",
    )]));
    assert_eq!(v["review"], true, "{v}");
}

#[test]
fn an_archived_change_without_burdens() {
    // req:web.worker.review/gate-answer@d38075fd
    let v = review_of(branch_with(&[(
        "changes/archive/x/change.md",
        "---\nstatus: applied\n---\n\n## Intent\nx\n",
    )]));
    assert_eq!(v["review"], false, "{v}");
}

#[test]
fn the_approval_setting_needs_review() {
    // req:web.worker.review/gate-answer@d38075fd
    let changed = review_of(branch_with(&[("clm.toml", "approval = \"other\"\n")]));
    assert_eq!(changed["review"], true, "{changed}");
    let comment = review_of(branch_with(&[("clm.toml", "# a comment\n")]));
    assert_eq!(comment["review"], false, "{comment}");
}

#[test]
fn truncated_base_listing_needs_review() {
    // req:web.worker.review/gate-answer@d38075fd
    let mut repo = spec_pr();
    repo.truncated_trees.insert("111".into());
    assert_eq!(review_of(repo)["review"], true);
    // A branch that removes `n/r`, with only its merge base's listing truncated: no cuts.
    let mut repo = branch_from(
        &[("specs/n/SPEC.md", &node("", &["r", "s"], "active"))],
        &[],
        &[("specs/n/SPEC.md", &node("", &["s"], "active"))],
    );
    repo.truncated_trees.insert("111".into());
    let v = review_of(repo);
    assert_eq!(v["review"], true, "{v}");
    assert_eq!(cuts(&v), Vec::<&str>::new(), "{v}");
}

#[test]
fn an_old_archived_change_is_not_read() {
    // req:web.worker.review/gate-answer@d38075fd
    // A file added beside an archived change's `change.md` doesn't make it the branch's
    // change, as the gate reads only a newly archived one.
    let mut repo = branch_with(&[("changes/archive/x/notes.md", "Notes.\n")]);
    let burdened = "---\nstatus: applied\nhuman_burdens: [b]\n---\n\n## Intent\nx\n";
    for sha in ["111", "222"] {
        repo.commits.get_mut(sha).unwrap().insert(
            "changes/archive/x/change.md".into(),
            burdened.as_bytes().to_vec(),
        );
    }
    let v = review_of(repo);
    assert_eq!(v["review"], false, "{v}");
}

#[test]
fn a_removed_change_is_not_read() {
    // req:web.worker.review/gate-answer@d38075fd
    // Only the changes the head holds a touched file of count: a branch that deletes a
    // change with burdens is judged by its other paths.
    let mut repo = branch_with(&[]);
    let burdened = "---\nstatus: applied\nhuman_burdens: [b]\n---\n\n## Intent\nx\n";
    repo.commits
        .get_mut("111")
        .unwrap()
        .insert("changes/x/change.md".into(), burdened.as_bytes().to_vec());
    let v = review_of(repo);
    assert_eq!(v["review"], false, "{v}");
}

#[test]
fn truncated_listing_needs_review() {
    // req:web.worker.review/gate-answer@d38075fd
    let mut repo = spec_pr();
    repo.truncated_trees.insert("222".into());
    let v = review_of(repo);
    assert_eq!(v["review"], true, "{v}");
    assert_eq!(v["fingerprint"], Value::Null);
}

#[test]
fn the_review_errs_toward_asking() {
    // req:web.worker.review/gate-answer@d38075fd
    // Where `clm gate` reads more than two trees show (an archive move, a code file, a
    // replayed rename), the review asks: `clm gate` prints `spec`, `mixed` and `spec`.
    let burdened = "---\nstatus: applied\nhuman_burdens: [b]\n---\n\n## Intent\nx\n";
    let mut moved = branch_from(&[("changes/x/change.md", burdened)], &[], &[]);
    let head = moved.commits.get_mut("222").unwrap();
    head.remove("changes/x/change.md");
    head.insert(
        "changes/archive/x/change.md".into(),
        burdened.as_bytes().to_vec(),
    );
    let v = review_of(moved);
    assert_eq!(v["review"], true, "an archive move: {v}");

    let mixed = review_of(branch_from(
        &[("specs/n/SPEC.md", &node("", &["r", "s"], "active"))],
        &[],
        &[
            ("specs/n/SPEC.md", &node("", &["s"], "active")),
            ("crates/a/src/lib.rs", "fn a() {}\n"),
        ],
    ));
    assert_eq!(mixed["review"], true, "a code file too: {mixed}");

    // What `clm rename` commits for `n/r` -> `n/s`: the spec, and the architecture entry
    // its rule enforces, which `rename/replay` accepts and the two trees can't tell apart
    // from an edit of the human layer.
    let before = node("", &["q", "r"], "active");
    let after = before.replace("### Requirement: r\n", "### Requirement: s\n");
    let arch = |id: &str| {
        format!(
            "# Arch\n\n## Requirements\n\n### Requirement: law\nA law SHALL hold.\n\n#### Scenario: s\n- ok\n\n#### Enforced by\n- {id}\n"
        )
    };
    let renamed = review_of(branch_from(
        &[
            ("specs/n/SPEC.md", &before),
            ("ARCHITECTURE.md", &arch("n/r")),
        ],
        &[],
        &[
            ("specs/n/SPEC.md", &after),
            ("ARCHITECTURE.md", &arch("n/s")),
            (
                "changes/r/change.md",
                "---\nstatus: applied\nrenames:\n  - requirement: n/r\n    to: s\n---\n\n## Intent\nRename.\n",
            ),
        ],
    ));
    assert_eq!(renamed["review"], true, "a replayed rename: {renamed}");
    assert_eq!(cuts(&renamed), Vec::<&str>::new(), "both sides load");
}

#[test]
fn comparison_cached_before_review_moved() {
    // req:web.worker.review/gate-answer@d38075fd
    // A comparison kept under an earlier key isn't served, even one of today's shape: the
    // branch's `review`, which now counts what it cuts, is computed anew.
    let fake = Fake::default();
    fake.gh.add_repo(
        "o/a",
        branch_from(
            &[("specs/n/SPEC.md", &node("", &["r", "s"], "active"))],
            &[],
            &[("specs/n/SPEC.md", &node("", &["s"], "active"))],
        ),
    );
    let cookie = fake.user("u", &["o/a"]);
    let old = json!({"change": null, "nodes": [], "decisions": [], "backlog": [],
                     "documents": [], "fingerprint": "f0f0", "code_key": null,
                     "review": false, "cuts": []});
    block_on(async {
        use clm_web_platform::platform::Cache;
        for key in [
            "compare3:o/a@111:222",
            "compare4:o/a@111:222",
            "compare5:o/a@111:222",
            "compare6:o/a@111:222",
            "compare7:o/a@111:222",
        ] {
            fake.cache_put(key, &old.to_string(), 86_400).await;
        }
    });
    let v = body_json(&fake.handle(get(URL, &cookie)));
    assert_eq!(v["review"], true, "{v}");
    assert_ne!(v["fingerprint"], "f0f0");
    assert_eq!(
        v["cuts"],
        json!(["specs/n/SPEC.md: gate/cut: requirement n/r is gone"]),
        "{v}"
    );
}

#[test]
fn change_without_sections() {
    // req:web.worker.review/pr-review@5982714a
    let mut repo = spec_pr();
    repo.commits.get_mut("222").unwrap().insert(
        "changes/c2/change.md".into(),
        b"---\nstatus: applied\n---\n\n# Faster r\n\nJust prose.\n".to_vec(),
    );
    let c = &review_of(repo)["change"];
    assert_eq!(c.get("intent_section"), Some(&Value::Null), "{c}");
    assert_eq!(c["intent"], "# Faster r\n\nJust prose.");
}

#[test]
fn change_with_empty_sections() {
    // req:web.worker.review/pr-review@5982714a
    // A heading with nothing under it is no section: the view then falls back to the whole intent.
    let mut repo = spec_pr();
    repo.commits.get_mut("222").unwrap().insert(
        "changes/c2/change.md".into(),
        b"---\nstatus: applied\n---\n\n## Intent\n\n".to_vec(),
    );
    let c = &review_of(repo)["change"];
    assert_eq!(c.get("intent_section"), Some(&Value::Null), "{c}");
}

#[test]
fn comparison_cached_with_a_patch_id() {
    // req:web.worker.review/gate-answer@d38075fd
    // A comparison the Cache API kept from when fingerprints were patch-ids isn't served:
    // its fingerprint would match no approval made since. It holds every field today's
    // comparison does, so only its key keeps it from being served.
    let fake = Fake::default();
    let repo = spec_pr();
    let expected = fingerprint(&repo);
    fake.gh.add_repo("o/a", repo);
    let cookie = fake.user("u", &["o/a"]);
    let old = json!({"change": null, "nodes": [], "decisions": [], "backlog": [],
                     "documents": [], "fingerprint": "f0f0", "code_key": null,
                     "review": false, "cuts": []});
    block_on(async {
        use clm_web_platform::platform::Cache;
        for key in ["compare:", "compare2:"] {
            fake.cache_put(&format!("{key}o/a@111:222"), &old.to_string(), 86_400)
                .await;
        }
    });
    let v = body_json(&fake.handle(get(URL, &cookie)));
    assert_eq!(v["fingerprint"], expected.as_str());
    assert_eq!(v["nodes"][1]["id"], "n");
}

#[test]
fn truncated_listing() {
    // req:web.worker.review/pr-review@5982714a
    // req:web.worker.review/fingerprint-from-trees@e49d6717
    // GitHub truncating the merge base's listing leaves the review without a fingerprint,
    // so no approval shows, though one exists for the branch's real fingerprint.
    let mut repo = spec_pr();
    let real = fingerprint(&repo);
    ledger(
        &mut repo,
        "clunny/web",
        "L1",
        &[approved("clunny/web", &real, "2026-10-26T10:00:00.000Z")],
    );
    repo.truncated_trees.insert("111".into());
    let v = review_of(repo);
    assert_eq!(v["fingerprint"], Value::Null);
    assert_eq!(v["approvals"], json!([]));
    assert_eq!(v["nodes"][1]["id"], "n", "the review itself still shows");
}

fn move_branch(fake: &Fake, branch: &str, sha: &str) {
    let mut repos = fake.gh.repos.borrow_mut();
    let repo = repos.get_mut("o/a").unwrap();
    repo.branches.insert(branch.into(), sha.into());
}

/// Drops the spec files kept per commit, as the Cache API may at any time, so only the
/// review's own cache can answer without fetching them. Every version of the platform's
/// key is dropped (`files2:`, `files3:`, ...), so the test proves the review's cache
/// whichever version the platform writes.
fn evict_files(fake: &Fake) {
    fake.cache.borrow_mut().retain(|k, _| !is_files_key(k));
}

/// Whether `key` is the platform's spec-files key: `files`, an optional version number
/// and `:`.
fn is_files_key(key: &str) -> bool {
    key.strip_prefix("files")
        .map(|rest| rest.trim_start_matches(|c: char| c.is_ascii_digit()))
        .is_some_and(|rest| rest.starts_with(':'))
}

#[test]
fn unchanged_branch() {
    // req:web.worker.review/review-cached@5f4be262
    let fake = Fake::default();
    fake.gh.add_repo("o/a", spec_pr());
    let cookie = fake.user("u", &["o/a"]);
    let first = body_json(&fake.handle(get(URL, &cookie)));
    let fetched = fake.gh.fetched_paths().len();
    assert!(fetched > 0);
    evict_files(&fake);
    fake.now.set(NOW + 600);
    let again = body_json(&fake.handle(get(URL, &cookie)));
    assert_eq!(again, first);
    assert_eq!(
        fake.gh.fetched_paths().len(),
        fetched,
        "no spec files fetched"
    );
}

#[test]
fn truncated_listing_cached() {
    // req:web.worker.review/review-cached@5f4be262
    // A review without a fingerprint still comes from the Cache API: a commit's listing
    // never changes, so asking again would be truncated again.
    let mut repo = spec_pr();
    repo.truncated_trees.insert("111".into());
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo);
    let cookie = fake.user("u", &["o/a"]);
    let first = body_json(&fake.handle(get(URL, &cookie)));
    assert_eq!(first["fingerprint"], Value::Null);
    let fetched = fake.gh.fetched_paths().len();
    evict_files(&fake);
    fake.now.set(NOW + 600);
    let again = body_json(&fake.handle(get(URL, &cookie)));
    assert_eq!(again, first);
    assert_eq!(
        fake.gh.fetched_paths().len(),
        fetched,
        "no spec files fetched"
    );
}

#[test]
fn new_push_to_the_branch() {
    // req:web.worker.review/review-cached@5f4be262
    let mut repo = spec_pr();
    let mut moved = repo.commits["222"].clone();
    moved.insert(
        "specs/n/SPEC.md".into(),
        node("", &["r", "s", "new", "newer"], "active").into_bytes(),
    );
    repo.commits.insert("333".into(), moved);
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo);
    let cookie = fake.user("u", &["o/a"]);
    let first = body_json(&fake.handle(get(URL, &cookie)));
    assert_eq!(first["head_sha"], "222");
    move_branch(&fake, "spec/c2", "333");
    let after = body_json(&fake.handle(get(URL, &cookie)));
    assert_eq!(after["head_sha"], "333");
    let added: Vec<&str> = after["nodes"][1]["requirements"]["added"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    assert_eq!(added, ["n/new", "n/newer"], "built again from 333");
}

#[test]
fn default_branch_moved() {
    // req:web.worker.review/review-cached@5f4be262
    let mut repo = spec_pr();
    let mut moved = repo.commits["111"].clone();
    moved.insert(
        "specs/n/SPEC.md".into(),
        node("", &["r", "s", "gone", "base-only"], "active").into_bytes(),
    );
    repo.commits.insert("112".into(), moved);
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo);
    let cookie = fake.user("u", &["o/a"]);
    let removed = |v: &Value| -> Vec<String> {
        v["nodes"][1]["requirements"]["removed"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["id"].as_str().unwrap().to_string())
            .collect()
    };
    let first = body_json(&fake.handle(get(URL, &cookie)));
    assert_eq!(removed(&first), ["n/gone"]);
    move_branch(&fake, "main", "112");
    let after = body_json(&fake.handle(get(URL, &cookie)));
    assert_eq!(
        removed(&after),
        ["n/base-only", "n/gone"],
        "built again from base 112"
    );
}

/// `spec_pr` with a ledger approving its fingerprint twice: with a `sig` by `dana/9f9f9f9f`
/// and without one by `clunny/web`.
fn signed_approvals() -> FakeRepo {
    let mut repo = spec_pr();
    let fingerprint = fingerprint(&repo);
    let mut signed = approved("dana/9f9f9f9f", &fingerprint, "2026-10-26T09:00:00.000Z");
    signed["sig"] = json!("-----BEGIN SSH SIGNATURE-----\nabc\n-----END SSH SIGNATURE-----\n");
    let unsigned = approved("clunny/web", &fingerprint, "2026-10-26T10:00:00.000Z");
    ledger(&mut repo, "dana/9f9f9f9f", "L1", &[signed]);
    ledger(&mut repo, "clunny/web", "L2", &[unsigned]);
    repo
}

/// `docs/allowed_signers` added to the commit `sha`.
fn sign(repo: &mut FakeRepo, sha: &str) {
    let files = repo.commits.get_mut(sha).unwrap();
    files.insert(
        "docs/allowed_signers".into(),
        b"dana ssh-ed25519 AAAA\n".to_vec(),
    );
}

fn approvals_signed(v: &Value) -> Vec<(String, bool)> {
    v["approvals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            (
                a["writer"].as_str().unwrap().into(),
                a["signed"].as_bool().unwrap(),
            )
        })
        .collect()
}

#[test]
fn signed_repository() {
    // req:web.worker.review/signed-repository@e056a364
    let mut repo = signed_approvals();
    sign(&mut repo, "111");
    sign(&mut repo, "222");
    let v = review_of(repo);
    assert_eq!(v["signed"], true);
    assert_eq!(
        approvals_signed(&v),
        [
            ("dana/9f9f9f9f".to_string(), true),
            ("clunny/web".to_string(), false)
        ]
    );
}

#[test]
fn unsigned_repository() {
    // req:web.worker.review/signed-repository@e056a364
    let v = review_of(signed_approvals());
    assert_eq!(v["signed"], false);
    assert_eq!(
        approvals_signed(&v),
        [
            ("dana/9f9f9f9f".to_string(), true),
            ("clunny/web".to_string(), false)
        ]
    );
}

#[test]
fn empty_sig_is_unsigned() {
    // req:web.worker.review/signed-repository@e056a364
    let mut repo = spec_pr();
    let fingerprint = fingerprint(&repo);
    let mut empty = approved("dana/9f9f9f9f", &fingerprint, "2026-10-26T09:00:00.000Z");
    empty["sig"] = json!("");
    let mut number = approved("eve/1a1a1a1a", &fingerprint, "2026-10-26T09:30:00.000Z");
    number["sig"] = json!(7);
    ledger(&mut repo, "dana/9f9f9f9f", "L1", &[empty, number]);
    let v = review_of(repo);
    assert_eq!(
        approvals_signed(&v),
        [
            ("dana/9f9f9f9f".to_string(), false),
            ("eve/1a1a1a1a".to_string(), false)
        ]
    );
}

#[test]
fn signed_behind_main() {
    // req:web.worker.review/signed-repository@e056a364
    // `spec/c2` branched before `main` gained `docs/allowed_signers`: its merge base and
    // head lack the file, and `main`'s tip holds it.
    let mut repo = spec_pr();
    let mut tip = repo.commits["111"].clone();
    tip.insert(
        "docs/allowed_signers".into(),
        b"dana ssh-ed25519 AAAA\n".to_vec(),
    );
    repo.commits.insert("333".into(), tip);
    repo.branches.insert("main".into(), "333".into());
    // The compare API's merge base of `333` and `222` is `111`.
    repo.compare_base.insert("333...222".into(), "111".into());
    let v = review_of(repo);
    assert_eq!(v["signed"], true);
}

#[test]
fn signed_lookup_fails() {
    // req:web.worker.review/signed-repository@e056a364
    let mut repo = spec_pr();
    repo.signers_fail = true;
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo);
    let cookie = fake.user("u", &["o/a"]);
    assert_eq!(fake.handle(get(URL, &cookie)).status, 502);
}

#[test]
fn signed_lookup_null_repository() {
    // req:web.worker.review/signed-repository@e056a364
    // A token that can read the repo over REST but gets a null `repository` from GraphQL.
    let fake = Fake::default();
    fake.gh.add_repo("o/a", spec_pr());
    let cookie = fake.user("u", &["o/a"]);
    fake.gh.graphql_null.borrow_mut().insert("Signers".into());
    assert_eq!(fake.handle(get(URL, &cookie)).status, 502);
}

#[test]
fn signed_is_cached_by_tip() {
    // req:web.worker.review/signed-repository@e056a364
    let fake = Fake::default();
    let mut repo = spec_pr();
    sign(&mut repo, "111");
    fake.gh.add_repo("o/a", repo);
    let cookie = fake.user("u", &["o/a"]);
    assert_eq!(body_json(&fake.handle(get(URL, &cookie)))["signed"], true);
    let asked = || {
        fake.gh
            .calls
            .borrow()
            .iter()
            .filter(|c| {
                c.body
                    .as_deref()
                    .is_some_and(|b| b.contains("query Signers"))
            })
            .count()
    };
    assert_eq!(asked(), 1);
    let second = body_json(&fake.handle(get(URL, &cookie)));
    assert_eq!(second["signed"], true);
    assert_eq!(asked(), 1, "kept by tip");
    // The approve route shares the answer: it refuses as signed, asking nothing more.
    let pin = second["fingerprint"].as_str().unwrap();
    let body = json!({ "fingerprint": pin }).to_string();
    let res = fake.handle(post(
        "/api/repos/o/a/branches/spec/c2/approve",
        &cookie,
        body.as_bytes(),
    ));
    assert_eq!(res.status, 422);
    assert_eq!(body_json(&res)["error"], "signed");
    assert_eq!(asked(), 1, "the approve shares the answer");
}
