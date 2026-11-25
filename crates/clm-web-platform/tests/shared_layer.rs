//! A record appended to its writer's web ledger (`web-ledger-append`).

use clm_ledger::{Body, Record};
use clm_web_platform::access::{self, Access};
use clm_web_platform::fake::*;
use clm_web_platform::ledger;
use serde_json::json;

fn setup() -> (Fake, Access) {
    let fake = Fake::default();
    let mut repo = FakeRepo::new("111", clm_repo());
    let old = json!({"id": "r0", "kind": "approved", "writer": "alice/web", "at": iso(NOW),
        "fingerprint": "f0", "branch": "spec/x", "change": null});
    clm_web_platform::fake::ledger(&mut repo, "alice/web", "old", &[old]);
    fake.gh.add_repo("o/a", repo);
    fake.gh.grant("u", "o/a");
    let access = block_on(access::check(&fake, "u", "o", "a")).unwrap();
    (fake, access)
}

#[test]
fn the_append() {
    // req:web.worker.platform/web-ledger-append@debff849
    let (fake, access) = setup();
    let body = Body::approved("f1".into(), "spec/y".into(), None);
    let record: Record = ledger::record(&fake, "alice", body);
    assert_eq!(record.writer, "alice/web");
    block_on(ledger::append(&fake, &access, &record)).unwrap();
    let writes = fake.gh.writes();
    let urls: Vec<&str> = writes.iter().map(|w| w.url.as_str()).collect();
    let git = "https://api.github.com/repos/o/a/git";
    assert_eq!(
        urls,
        [
            format!("{git}/blobs"),
            format!("{git}/trees"),
            format!("{git}/commits"),
            format!("{git}/refs/heads/clm/ledger/alice/web"),
        ]
    );
    let body = |i: usize| -> serde_json::Value {
        serde_json::from_str(writes[i].body.as_deref().unwrap()).unwrap()
    };
    assert_eq!(body(2)["parents"], json!(["old"]), "the old tip");
    assert_eq!(writes[3].method, "PATCH");
    assert_eq!(body(3)["force"], json!(false));
    let repos = fake.gh.repos.borrow();
    let repo = &repos["o/a"];
    let tip = &repo.branches["clm/ledger/alice/web"];
    assert_ne!(tip, "old");
    let text = String::from_utf8(repo.commits[tip]["records.jsonl"].clone()).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("\"r0\""), "the old line first");
    assert_eq!(lines[1], serde_json::to_string(&record).unwrap());
}
