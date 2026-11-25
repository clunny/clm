//! A web write validated against the replica and appended with the requester's token
//! (`writes-through-object`).

mod common;

use clm_web_platform::fake::{self, Fake, FakeRepo, FileMap};
use common::*;
use futures::executor::block_on;
use serde_json::{Value, json};

const CAPABILITIES: &str = "## Capabilities\n\n### Capability: x\nA person can x.\n\n#### Why\nx why\n\n#### Scenario: s\n- ok\n";

const C_SPEC: &str = "---\ntitle: C\nstatus: active\nowners: []\nserves: [capability/x]\n---\n\n## Contract\nc\n\n## Requirements\n\n### Requirement: r\nIt SHALL r.\n\n#### Scenario: s\n- ok\n";

fn repo(capabilities: &str) -> clm_web_platform::fake::FileMap {
    repo_files(&[
        ("CAPABILITIES.md", capabilities),
        ("specs/c/SPEC.md", C_SPEC),
    ])
}

/// The capability `x`'s entry of the capabilities view.
fn entry(fake: &clm_web_platform::fake::Fake, object: &mut Obj) -> Value {
    let v = view(fake, object, "capabilities");
    v["view"]["report"]["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["slug"] == "x")
        .unwrap_or_else(|| panic!("{v}"))
        .clone()
}

fn decide(version: &str) -> Value {
    json!({"login": "carol", "token": "tok", "kind": "decide", "id": "capability/x",
        "version": version, "value": "high", "decision": "keep"})
}

#[test]
fn a_decision_shows_at_once() {
    // req:web.worker.replica/writes-through-object@43885d86
    let fake = fake_with("111", repo(CAPABILITIES));
    fake.gh.grant("tok", "o/a");
    let (mut object, _mem, _slot) = built(&fake);
    let before = entry(&fake, &mut object);
    assert!(
        before["decisions"].as_array().unwrap().is_empty(),
        "{before}"
    );
    let version = before["version"].as_str().unwrap().to_string();
    let answer = block_on(object.write(&fake, &decide(&version)));
    assert_eq!(answer.status, 200, "{}", answer.body);
    let after = entry(&fake, &mut object);
    assert_eq!(after["decisions"].as_array().unwrap().len(), 1, "{after}");
    let writes = fake.gh.writes();
    assert!(!writes.is_empty(), "the record was appended");
    assert!(
        writes
            .iter()
            .all(|w| matches!(&w.auth, clm_web_platform::platform::Auth::User(t) if t == "tok")),
        "with the requester's token"
    );
}

#[test]
fn reworded_since_the_page_was_drawn() {
    // req:web.worker.replica/writes-through-object@43885d86
    let fake = fake_with("111", repo(CAPABILITIES));
    fake.gh.grant("tok", "o/a");
    let (mut object, _mem, _slot) = built(&fake);
    let version = entry(&fake, &mut object)["version"]
        .as_str()
        .unwrap()
        .to_string();
    // GitHub's default branch holds a rewording the object hasn't been told of.
    let reworded = CAPABILITIES.replace("A person can x.", "A person can x, better.");
    commit(&fake, "222", repo(&reworded));
    let answer = block_on(object.write(&fake, &decide(&version)));
    assert_eq!(answer.status, 409, "{}", answer.body);
    assert!(fake.gh.writes().is_empty(), "nothing was appended");
    // The push was applied: a fresh read, which recomputes what was stale, shows it.
    block_on(object.view(&fake, "capabilities", true));
    let now = entry(&fake, &mut object);
    assert_ne!(now["version"].as_str().unwrap(), version);
}

#[test]
fn a_proposal_shows_at_once() {
    // req:web.worker.replica/writes-through-object@43885d86
    let fake = fake_with("111", repo(CAPABILITIES));
    fake.gh.grant("tok", "o/a");
    let (mut object, _mem, _slot) = built(&fake);
    let propose = json!({"login": "carol", "token": "tok", "kind": "propose",
        "title": "Export CSV", "body": "Please.", "nodes": ["a"]});
    let answer = block_on(object.write(&fake, &propose));
    assert_eq!(answer.status, 200, "{}", answer.body);
    let id = serde_json::from_str::<Value>(&answer.body).unwrap()["id"].clone();
    assert!(id.is_string());
    // The recompute ran before the answer.
    let seen = view(&fake, &mut object, "implementation");
    assert!(seen["computed_at"].as_u64().unwrap() >= fake.now.get());
    // The same title is now taken.
    let again = block_on(object.write(&fake, &propose));
    assert_eq!(again.status, 409);
    // A withdrawal decides from the requester's own ledger.
    let withdraw = json!({"login": "carol", "token": "tok", "kind": "withdraw", "id": id});
    assert_eq!(block_on(object.write(&fake, &withdraw)).status, 200);
    let other = json!({"login": "carol", "token": "tok", "kind": "withdraw", "id": "nope"});
    assert_eq!(block_on(object.write(&fake, &other)).status, 404);
}

#[test]
fn the_tip_cant_be_read() {
    // req:web.worker.replica/writes-through-object@43885d86
    let fake = fake_with("111", repo(CAPABILITIES));
    fake.gh.grant("tok", "o/a");
    let (mut object, _mem, _slot) = built(&fake);
    let version = entry(&fake, &mut object)["version"]
        .as_str()
        .unwrap()
        .to_string();
    fake.gh
        .repos
        .borrow_mut()
        .get_mut("o/a")
        .unwrap()
        .ref_fails
        .insert("main".into());
    let propose = json!({"login": "carol", "token": "tok", "kind": "propose",
        "title": "Export CSV", "body": "Please.", "nodes": []});
    let withdraw = json!({"login": "carol", "token": "tok", "kind": "withdraw", "id": "x"});
    for write in [decide(&version), propose, withdraw] {
        let answer = block_on(object.write(&fake, &write));
        assert_eq!(answer.status, 502, "{}", answer.body);
    }
    assert!(fake.gh.writes().is_empty(), "nothing was appended");
}

#[test]
fn still_preparing() {
    // req:web.worker.replica/writes-through-object@43885d86
    let fake = fake_with("111", repo(CAPABILITIES));
    fake.gh.grant("tok", "o/a");
    let (mut object, _mem, slot) = object();
    let propose = json!({"login": "carol", "token": "tok", "kind": "propose",
        "title": "Export CSV", "body": "Please.", "nodes": []});
    let before = fake.gh.calls.borrow().len();
    let answer = block_on(object.write(&fake, &propose));
    assert_eq!(answer.status, 503);
    assert_eq!(
        serde_json::from_str::<Value>(&answer.body).unwrap(),
        json!({"preparing": 0})
    );
    assert!(slot.0.get().is_some(), "the alarm is set");
    assert_eq!(fake.gh.calls.borrow().len(), before, "no ref was read");
    assert!(fake.gh.writes().is_empty(), "nothing was appended");
}

#[test]
fn an_answer_read_back() {
    // req:web.worker.replica/writes-through-object@43885d86
    let item = "---\ntitle: Q\nkind: question\nchoices: [yes, no]\n---\nQ?\n";
    let fake = fake_with("111", repo_files(&[("backlog/q.md", item)]));
    fake.gh.grant("tok", "o/a");
    let (mut object, _mem, _slot) = built(&fake);
    let answer = |item: &str, answer: &str| json!({"login": "carol", "token": "tok", "kind": "answer", "item": item, "answer": answer});
    let unknown = block_on(object.write(&fake, &answer("nope", "yes")));
    assert_eq!(unknown.status, 404);
    assert_eq!(
        serde_json::from_str::<Value>(&unknown.body).unwrap(),
        json!({"error": "unknown"})
    );
    let invalid = block_on(object.write(&fake, &answer("q", "maybe")));
    assert_eq!(invalid.status, 422);
    assert_eq!(
        serde_json::from_str::<Value>(&invalid.body).unwrap(),
        json!({"error": "answer"})
    );
    assert!(fake.gh.writes().is_empty(), "nothing was appended");
    let ok = block_on(object.write(&fake, &answer("q", "yes")));
    assert_eq!(ok.status, 200, "{}", ok.body);
    let id = serde_json::from_str::<Value>(&ok.body).unwrap()["id"].clone();
    assert!(id.is_string(), "{}", ok.body);
    assert!(
        fake.gh
            .writes()
            .iter()
            .all(|w| matches!(&w.auth, clm_web_platform::platform::Auth::User(t) if t == "tok")),
        "with the requester's token"
    );
    let seen = view(&fake, &mut object, "implementation");
    let answers = seen["view"]["answers"]
        .as_array()
        .unwrap_or_else(|| panic!("{seen}"));
    assert_eq!(answers.len(), 1, "{seen}");
    assert_eq!(answers[0]["item"], "q");
    assert_eq!(answers[0]["answer"], "yes");
    assert_eq!(answers[0]["id"], id);
}

// ---- the refusals (`write-refusals`)

const ARCHITECTURE: &str = "## Constraints\n\n### Constraint: e\nGit SHALL hold it.\n\n#### Scenario: s\n- ok\n\n#### Enforced by\n- c/r\n";

/// A built object over `o/a` holding `files`, with `tok` able to read the repo.
fn built_over(files: FileMap) -> (Fake, Obj) {
    built_from(FakeRepo::new("111", files))
}

fn built_from(repo: FakeRepo) -> (Fake, Obj) {
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo);
    fake.gh.grant("tok", "o/a");
    let (object, _mem, _slot) = built(&fake);
    (fake, object)
}

/// A decision by `carol` on `id`.
fn decision(id: &str, version: &str, value: &str) -> Value {
    json!({"login": "carol", "token": "tok", "kind": "decide", "id": id,
        "version": version, "value": value, "decision": "keep"})
}

fn proposal(title: &str, nodes: &[&str]) -> Value {
    json!({"login": "carol", "token": "tok", "kind": "propose",
        "title": title, "body": "", "nodes": nodes})
}

/// The answer's status and body, the body parsed (null for none).
fn sent(fake: &Fake, object: &mut Obj, write: &Value) -> (u16, Value) {
    let answer = block_on(object.write(fake, write));
    let body = if answer.body.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&answer.body).unwrap()
    };
    (answer.status, body)
}

fn x_version(fake: &Fake, object: &mut Obj) -> String {
    entry(fake, object)["version"].as_str().unwrap().to_string()
}

#[test]
fn signed() {
    // req:web.worker.replica/write-refusals@ad7750a1
    let mut files = repo(CAPABILITIES);
    files.extend(fake::files(&[
        ("ARCHITECTURE.md", ARCHITECTURE),
        (
            "docs/allowed_signers",
            "carol@example.com ssh-ed25519 AAAA\n",
        ),
    ]));
    let (fake, mut object) = built_over(files);
    let capability = sent(
        &fake,
        &mut object,
        &decision("capability/x", "stale", "high"),
    );
    assert_eq!(
        capability,
        (
            422,
            json!({"error": "signed",
                "command": "clm capabilities decide x --value high --decision keep"})
        )
    );
    let architecture = sent(
        &fake,
        &mut object,
        &decision("architecture/e", "stale", "high"),
    );
    assert_eq!(
        architecture,
        (
            422,
            json!({"error": "signed",
                "command": "clm architecture decide e --value high --decision keep"})
        )
    );
    // An entry the tip doesn't hold is refused before the signing is.
    let nope = sent(
        &fake,
        &mut object,
        &decision("capability/nope", "v1", "high"),
    );
    assert_eq!(nope, (422, Value::Null));
    assert!(fake.gh.writes().is_empty(), "nothing was appended");
}

#[test]
fn reworded_since() {
    // req:web.worker.replica/write-refusals@ad7750a1
    let (fake, mut object) = built_over(repo(CAPABILITIES));
    let now = x_version(&fake, &mut object);
    let old = "00000000";
    assert_ne!(old, now);
    let answer = sent(&fake, &mut object, &decision("capability/x", old, "high"));
    assert_eq!(answer, (409, Value::Null));
    assert!(fake.gh.writes().is_empty(), "nothing was appended");
}

#[test]
fn unknown_capability() {
    // req:web.worker.replica/write-refusals@ad7750a1
    let (fake, mut object) = built_over(repo(CAPABILITIES));
    let version = x_version(&fake, &mut object);
    let nope = sent(
        &fake,
        &mut object,
        &decision("capability/nope", "v1", "high"),
    );
    assert_eq!(nope, (422, Value::Null));
    let huge = sent(
        &fake,
        &mut object,
        &decision("capability/x", &version, "huge"),
    );
    assert_eq!(huge, (422, Value::Null));
    assert!(fake.gh.writes().is_empty(), "nothing was appended");

    // A second run: `specs/c/SPEC.md` doesn't parse.
    let mut files = repo(CAPABILITIES);
    files.extend(fake::files(&[(
        "specs/c/SPEC.md",
        "---\ntitle: [C\n---\n\n## Contract\nc\n",
    )]));
    let (fake, mut object) = built_over(files);
    let broken = sent(
        &fake,
        &mut object,
        &decision("capability/x", &version, "high"),
    );
    assert_eq!(broken, (422, Value::Null));
    // A proposal against specs that don't load is refused alike.
    let proposed = sent(&fake, &mut object, &proposal("Dark mode", &[]));
    assert_eq!(proposed, (422, Value::Null));
    assert!(fake.gh.writes().is_empty(), "nothing was appended");
}

#[test]
fn not_an_entry() {
    // req:web.worker.replica/write-refusals@ad7750a1
    let mut files = repo(CAPABILITIES);
    files.extend(fake::files(&[("ARCHITECTURE.md", ARCHITECTURE)]));
    let (fake, mut object) = built_over(files);
    let nope = sent(
        &fake,
        &mut object,
        &decision("architecture/nope", "v1", "high"),
    );
    assert_eq!(nope, (422, Value::Null));
    let stale = sent(
        &fake,
        &mut object,
        &decision("architecture/e", "v1", "high"),
    );
    assert_eq!(stale, (409, Value::Null));
    assert!(fake.gh.writes().is_empty(), "nothing was appended");
}

#[test]
fn refused() {
    // req:web.worker.replica/write-refusals@ad7750a1
    let mut files = repo(CAPABILITIES);
    files.extend(fake::files(&[(
        "backlog/export-csv.md",
        "---\ntitle: Export CSV\nkind: idea\n---\n\nLet me export.\n",
    )]));
    let (fake, mut object) = built_over(files);
    let taken = sent(&fake, &mut object, &proposal("Export CSV", &["a"]));
    assert_eq!(taken, (409, json!({"error": "taken"})));
    let nodes = sent(&fake, &mut object, &proposal("Dark mode", &["nope"]));
    assert_eq!(nodes, (422, json!({"error": "nodes"})));
    assert!(fake.gh.writes().is_empty(), "nothing was appended");
}

#[test]
fn taken_by_a_change() {
    // req:web.worker.replica/write-refusals@ad7750a1
    let mut files = repo(CAPABILITIES);
    files.extend(fake::files(&[(
        "changes/20270101-x/change.md",
        "---\nstatus: applied\nresolves:\n  - export-csv\n---\n\n# X\n",
    )]));
    let (fake, mut object) = built_over(files);
    assert_eq!(
        view(&fake, &mut object, "snapshot")["view"]["resolved"],
        json!(["export-csv"])
    );
    let taken = sent(&fake, &mut object, &proposal("Export CSV", &[]));
    assert_eq!(taken, (409, json!({"error": "taken"})));
    assert!(fake.gh.writes().is_empty(), "nothing was appended");

    // A second run: bob's open proposal holds the slug.
    let mut held = FakeRepo::new("111", repo(CAPABILITIES));
    let open = json!({"id": "p0", "writer": "bob/web", "at": "2027-01-15T07:00:00.000Z",
        "kind": "proposed", "slug": "export-csv", "title": "Export CSV", "body": "",
        "nodes": []});
    fake::ledger(&mut held, "bob/web", "LB", &[open]);
    let (fake, mut object) = built_from(held);
    let taken = sent(&fake, &mut object, &proposal("Export CSV", &[]));
    assert_eq!(taken, (409, json!({"error": "taken"})));
    assert!(fake.gh.writes().is_empty(), "nothing was appended");
}

#[test]
fn withdrawn() {
    // req:web.worker.replica/write-refusals@ad7750a1
    let proposed = |id: &str, writer: &str, slug: &str| {
        json!({"id": id, "writer": writer, "at": "2027-01-15T07:00:00.000Z",
            "kind": "proposed", "slug": slug, "title": slug, "body": "", "nodes": []})
    };
    let mut held = FakeRepo::new("111", repo(CAPABILITIES));
    fake::ledger(
        &mut held,
        "alice/web",
        "LA",
        &[proposed("p1", "alice/web", "s1")],
    );
    fake::ledger(
        &mut held,
        "bob/web",
        "LB",
        &[proposed("p2", "bob/web", "s2")],
    );
    let (fake, mut object) = built_from(held);
    let withdraw =
        |id: &str| json!({"login": "alice", "token": "tok", "kind": "withdraw", "id": id});
    assert_eq!(
        sent(&fake, &mut object, &withdraw("p2")),
        (404, Value::Null),
        "bob's"
    );
    assert!(fake.gh.writes().is_empty(), "nothing was appended");
    assert_eq!(
        sent(&fake, &mut object, &withdraw("p1")),
        (200, Value::Null)
    );
    let records: Vec<Value> = {
        let repos = fake.gh.repos.borrow();
        let repo = &repos["o/a"];
        let tip = &repo.branches["clm/ledger/alice/web"];
        std::str::from_utf8(&repo.commits[tip]["records.jsonl"])
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    };
    assert_eq!(records.len(), 2, "{records:?}");
    assert_eq!(records[1]["kind"], "withdrawn");
    assert_eq!(records[1]["proposal"], "p1");
    let writes = fake.gh.writes().len();
    assert_eq!(
        sent(&fake, &mut object, &withdraw("p1")),
        (404, Value::Null),
        "already withdrawn"
    );
    assert_eq!(fake.gh.writes().len(), writes, "nothing more was appended");
}

#[test]
fn bodies_of_the_wrong_shape() {
    // req:web.worker.replica/write-refusals@ad7750a1
    let (fake, mut object) = built_over(repo(CAPABILITIES));
    let version = x_version(&fake, &mut object);
    let who = |mut v: Value| {
        v["login"] = json!("carol");
        v["token"] = json!("tok");
        v
    };
    for body in [
        json!({"kind": "decide", "version": version, "value": "high", "decision": "keep"}),
        json!({"kind": "decide", "id": "capability/x", "value": "high", "decision": "keep"}),
        json!({"kind": "decide", "id": "capability/x", "version": version, "decision": "keep"}),
        json!({"kind": "decide", "id": "capability/x", "version": version, "value": "high"}),
        json!({"kind": "decide", "id": "capability/x", "version": version, "value": "high",
            "decision": "maybe"}),
        json!({"kind": "decide", "id": 3, "version": version, "value": "high",
            "decision": "keep"}),
        json!({"kind": "propose", "body": "", "nodes": []}),
        json!({"kind": "propose", "title": "Dark mode", "nodes": []}),
        json!({"kind": "withdraw"}),
        json!({"kind": "withdraw", "id": 1}),
    ] {
        let body = who(body);
        assert_eq!(
            sent(&fake, &mut object, &body),
            (422, Value::Null),
            "{body}"
        );
    }
    assert!(fake.gh.writes().is_empty(), "nothing was appended");
}
