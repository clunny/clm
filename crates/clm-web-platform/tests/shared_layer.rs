//! `clm-web-platform` holds the Worker's shared layer once (`shared-layer`).

use std::path::{Path, PathBuf};

use clm_ledger::{Body, Record};
use clm_web_platform::access::{self, Access};
use clm_web_platform::fake::*;
use clm_web_platform::ledger;
use serde_json::json;

const MODULES: [&str; 9] = [
    "platform.rs",
    "http.rs",
    "access.rs",
    "gh.rs",
    "files.rs",
    "keys.rs",
    "install.rs",
    "live.rs",
    "ledger.rs",
];

/// The file names in `crates/<name>/src`, none for a directory that doesn't exist.
fn listing(name: &str) -> Vec<String> {
    let dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(name)
        .join("src");
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

#[test]
fn one_copy() {
    // req:web.worker.platform/shared-layer@33f47bc2
    let platform = listing("clm-web-platform");
    for module in MODULES {
        assert!(
            platform.iter().any(|f| f == module),
            "platform lacks {module}"
        );
    }
    for other in ["clm-web", "clm-web-review"] {
        let held: Vec<String> = listing(other)
            .into_iter()
            .filter(|f| MODULES.contains(&f.as_str()))
            .collect();
        assert!(held.is_empty(), "{other} holds {held:?}");
    }
}

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
    // req:web.worker.platform/shared-layer@33f47bc2
    let (fake, access) = setup();
    let body = Body::Approved {
        fingerprint: "f1".into(),
        branch: "spec/y".into(),
        change: None,
    };
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
