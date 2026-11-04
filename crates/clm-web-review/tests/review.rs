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
    // req:web.worker.review/pr-review@a515c5d4
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
    // req:web.worker.review/pr-review@a515c5d4
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
    // req:web.worker.review/pr-review@a515c5d4
    let v = review();
    assert_eq!(v["approvals"], json!([]));
    assert_eq!(v["checked"], false);
    assert_eq!(v["page"], Value::Null, "no page without a PR");
}

#[test]
fn approval_of_an_older_diff() {
    // req:web.worker.review/pr-review@a515c5d4
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
    // req:web.worker.review/pr-review@a515c5d4
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
    // req:web.worker.review/pr-review@a515c5d4
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
    // req:web.worker.review/pr-review@a515c5d4
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
    // req:web.worker.review/pr-review@a515c5d4
    let mut repo = spec_pr();
    repo.branches.insert("impl/n".into(), "222".into());
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
}

#[test]
fn contract_frontmatter_and_assets() {
    // req:web.worker.review/pr-review@a515c5d4
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
    // req:web.worker.review/pr-review@a515c5d4
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
    // req:web.worker.review/pr-review@a515c5d4
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

#[test]
fn human_layer_without_a_change() {
    // req:web.worker.review/pr-review@a515c5d4
    let v = review_of(branch_with(&[("ARCHITECTURE.md", "# Arch\n\nMore.\n")]));
    assert_eq!(v["review"], true, "{v}");
    assert_eq!(v["change"], Value::Null);
}

#[test]
fn a_change_with_burdens() {
    // req:web.worker.review/pr-review@a515c5d4
    let burdened = review_of(branch_with(&[(
        "changes/x/change.md",
        "---\nstatus: applied\nburdens: [b]\n---\n\n## Intent\nx\n",
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
    // req:web.worker.review/pr-review@a515c5d4
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
    // req:web.worker.review/pr-review@a515c5d4
    let v = review_of(branch_with(&[(
        "changes/archive/x/change.md",
        "---\nstatus: applied\nburdens: [b]\n---\n\n## Intent\nx\n",
    )]));
    assert_eq!(v["review"], true, "{v}");
}

#[test]
fn an_archived_change_without_burdens() {
    // req:web.worker.review/pr-review@a515c5d4
    let v = review_of(branch_with(&[(
        "changes/archive/x/change.md",
        "---\nstatus: applied\n---\n\n## Intent\nx\n",
    )]));
    assert_eq!(v["review"], false, "{v}");
}

#[test]
fn the_approval_setting_needs_review() {
    // req:web.worker.review/pr-review@a515c5d4
    let changed = review_of(branch_with(&[("clm.toml", "approval = \"other\"\n")]));
    assert_eq!(changed["review"], true, "{changed}");
    let comment = review_of(branch_with(&[("clm.toml", "# a comment\n")]));
    assert_eq!(comment["review"], false, "{comment}");
}

#[test]
fn truncated_base_listing_needs_review() {
    // req:web.worker.review/pr-review@a515c5d4
    let mut repo = spec_pr();
    repo.truncated_trees.insert("111".into());
    assert_eq!(review_of(repo)["review"], true);
}

#[test]
fn an_old_archived_change_is_not_read() {
    // req:web.worker.review/pr-review@a515c5d4
    // A file added beside an archived change's `change.md` doesn't make it the branch's
    // change, as the gate reads only a newly archived one.
    let mut repo = branch_with(&[("changes/archive/x/notes.md", "Notes.\n")]);
    let burdened = "---\nstatus: applied\nburdens: [b]\n---\n\n## Intent\nx\n";
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
    // req:web.worker.review/pr-review@a515c5d4
    // Only the changes the head holds a touched file of count: a branch that deletes a
    // change with burdens is judged by its other paths.
    let mut repo = branch_with(&[]);
    let burdened = "---\nstatus: applied\nburdens: [b]\n---\n\n## Intent\nx\n";
    repo.commits
        .get_mut("111")
        .unwrap()
        .insert("changes/x/change.md".into(), burdened.as_bytes().to_vec());
    let v = review_of(repo);
    assert_eq!(v["review"], false, "{v}");
}

#[test]
fn truncated_listing_needs_review() {
    // req:web.worker.review/pr-review@a515c5d4
    let mut repo = spec_pr();
    repo.truncated_trees.insert("222".into());
    let v = review_of(repo);
    assert_eq!(v["review"], true, "{v}");
    assert_eq!(v["fingerprint"], Value::Null);
}

#[test]
fn comparison_cached_before_review_moved() {
    // req:web.worker.review/pr-review@a515c5d4
    // A comparison kept under the previous key, with the change's own `review`, isn't
    // served: the branch's `review` is computed anew.
    let fake = Fake::default();
    fake.gh.add_repo(
        "o/a",
        branch_with(&[("ARCHITECTURE.md", "# Arch\n\nMore.\n")]),
    );
    let cookie = fake.user("u", &["o/a"]);
    let old = json!({"change": null, "nodes": [], "decisions": [], "backlog": [],
                     "documents": [], "fingerprint": "f0f0", "code_key": null,
                     "review": false});
    block_on(async {
        use clm_web_platform::platform::Cache;
        fake.cache_put("compare3:o/a@111:222", &old.to_string(), 86_400)
            .await;
    });
    let v = body_json(&fake.handle(get(URL, &cookie)));
    assert_eq!(v["review"], true, "{v}");
    assert_ne!(v["fingerprint"], "f0f0");
}

#[test]
fn change_without_sections() {
    // req:web.worker.review/pr-review@a515c5d4
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
    // req:web.worker.review/pr-review@a515c5d4
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
    // req:web.worker.review/pr-review@a515c5d4
    // A comparison the Cache API kept from when fingerprints were patch-ids isn't served:
    // its fingerprint would match no approval made since.
    let fake = Fake::default();
    let repo = spec_pr();
    let expected = fingerprint(&repo);
    fake.gh.add_repo("o/a", repo);
    let cookie = fake.user("u", &["o/a"]);
    let old = json!({"change": null, "nodes": [], "decisions": [], "backlog": [],
                     "documents": [], "fingerprint": "f0f0", "code_key": null});
    block_on(async {
        use clm_web_platform::platform::Cache;
        fake.cache_put("compare:o/a@111:222", &old.to_string(), 86_400)
            .await;
    });
    let v = body_json(&fake.handle(get(URL, &cookie)));
    assert_eq!(v["fingerprint"], expected.as_str());
    assert_eq!(v["nodes"][1]["id"], "n");
}

#[test]
fn truncated_listing() {
    // req:web.worker.review/pr-review@a515c5d4
    // req:web.worker.review/fingerprint-from-trees@24bcb92e
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
/// review's own cache can answer without fetching them.
fn evict_files(fake: &Fake) {
    fake.cache
        .borrow_mut()
        .retain(|k, _| !k.starts_with("files2:"));
}

#[test]
fn unchanged_branch() {
    // req:web.worker.review/review-cached@4c4b8baa
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
    // req:web.worker.review/review-cached@4c4b8baa
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
    // req:web.worker.review/review-cached@4c4b8baa
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
    // req:web.worker.review/review-cached@4c4b8baa
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
    // req:web.worker.review/signed-repository@a9873234
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
    // req:web.worker.review/signed-repository@a9873234
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
    // req:web.worker.review/signed-repository@a9873234
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
    // req:web.worker.review/signed-repository@a9873234
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
    // req:web.worker.review/signed-repository@a9873234
    let mut repo = spec_pr();
    repo.signers_fail = true;
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo);
    let cookie = fake.user("u", &["o/a"]);
    assert_eq!(fake.handle(get(URL, &cookie)).status, 502);
}

#[test]
fn signed_lookup_null_repository() {
    // req:web.worker.review/signed-repository@a9873234
    // A token that can read the repo over REST but gets a null `repository` from GraphQL.
    let fake = Fake::default();
    fake.gh.add_repo("o/a", spec_pr());
    let cookie = fake.user("u", &["o/a"]);
    fake.gh.graphql_null.borrow_mut().insert("Signers".into());
    assert_eq!(fake.handle(get(URL, &cookie)).status, 502);
}

#[test]
fn signed_is_cached_by_tip() {
    // req:web.worker.review/signed-repository@a9873234
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
    assert_eq!(body_json(&fake.handle(get(URL, &cookie)))["signed"], true);
    assert_eq!(asked(), 1, "kept by tip");
}
