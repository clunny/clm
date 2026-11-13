//! A web write validated against the replica and appended with the requester's token
//! (`writes-through-object`).

mod common;

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
