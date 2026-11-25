//! An object whose stored replica another release saved fetches again what that release's
//! markers came from (`refetches-what-a-release-forgot`).

mod common;

use clm_web_platform::fake::{Fake, files};
use common::*;
use futures::executor::block_on;
use serde_json::{Value, json};

/// The marker `src/a.rs` holds, built so this file holds no marker of its own.
const MARKER: &str = concat!("// req", ":a/p\n#[test]\nfn p() {}\n");

/// `o/a` with `src/a.rs` citing `a/p` and `extra` more non-Markdown files.
fn fake_of(extra: usize) -> Fake {
    let mut all = repo_files(&[("src/a.rs", MARKER)]);
    for i in 0..extra {
        all.extend(files(&[(
            &format!("src/f{i}.rs"),
            &format!("fn f{i}() {{}}"),
        )]));
    }
    fake_with("111", all)
}

/// Rewrites the replica `mem` holds as release `release` saved it, leaving the object's own
/// record beside it as it is. The chunks are `replica`'s: a count under `replica`, then
/// `replica/<n>`.
fn saved_by(mem: &Mem, release: &str) {
    let mut map = mem.0.borrow_mut();
    let count: usize = String::from_utf8(map["replica"].clone())
        .unwrap()
        .parse()
        .unwrap();
    let mut bytes = Vec::new();
    for n in 0..count {
        bytes.extend(map.remove(&format!("replica/{n}")).unwrap());
    }
    let mut state: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(state["release"], env!("CARGO_PKG_VERSION"));
    state["release"] = json!(release);
    let bytes = serde_json::to_vec(&state).unwrap();
    let chunks: Vec<&[u8]> = bytes.chunks(1 << 20).collect();
    for (n, chunk) in chunks.iter().enumerate() {
        map.insert(format!("replica/{n}"), chunk.to_vec());
    }
    map.insert("replica".into(), chunks.len().to_string().into_bytes());
}

/// The markers the snapshot `seen` gives requirement `a/p`.
fn markers_of_p(seen: &Value) -> Value {
    seen["view"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "a")
        .unwrap()["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "a/p")
        .unwrap()["markers"]
        .clone()
}

fn cites_a(seen: &Value) -> bool {
    markers_of_p(seen)
        .as_array()
        .is_some_and(|ms| ms.iter().any(|m| m.as_str().unwrap().contains("src/a.rs")))
}

fn notices(fake: &Fake) -> usize {
    fake.live
        .calls
        .borrow()
        .iter()
        .filter(|c| c.path == "/notify")
        .count()
}

#[test]
fn a_newer_release() {
    // req:web.worker.replica/refetches-what-a-release-forgot@a63df1b7
    let fake = fake_of(6000);
    let (mut object, mem, _slot) = built(&fake);
    let stored = view(&fake, &mut object, "snapshot");
    assert!(cites_a(&stored), "{}", markers_of_p(&stored));
    drop(object);
    saved_by(&mem, "0.0.0");
    // Later, with no page open, the object wakes with no alarm set.
    fake.now.set(fake.now.get() + 3600);
    let slot = Slot::default();
    let mut object = Obj::load("o/a", mem.clone(), slot.clone());

    let first = block_on(object.view(&fake, "snapshot", false));
    assert_eq!(first.status, 200, "{}", first.body);
    let first: Value = serde_json::from_str(&first.body).unwrap();
    assert_eq!(first, stored, "the stored snapshot, until the recompute");
    assert_eq!(
        slot.0.get(),
        Some(fake.now.get()),
        "the alarm is set to now"
    );

    let told = notices(&fake);
    let from = fake.gh.calls.borrow().len();
    let alarms = run_alarms(&fake, &mut object, &slot);
    assert!(alarms.iter().all(|n| *n <= 50), "{alarms:?}");
    let fetching = alarms.iter().filter(|n| **n > 0).count();
    assert!(fetching > 1, "fetched in slices: {alarms:?}");
    assert!(
        graphql_calls(&fake, from)
            .iter()
            .any(|q| q.contains("111:src/a.rs")),
        "src/a.rs was fetched again"
    );
    let second = view(&fake, &mut object, "snapshot");
    assert_eq!(second["view"]["coverage"]["state"], "complete", "{second}");
    assert!(cites_a(&second), "{}", markers_of_p(&second));
    assert!(notices(&fake) > told, "a notice was posted");
}

#[test]
fn an_idle_alarm_already_set() {
    // req:web.worker.replica/refetches-what-a-release-forgot@a63df1b7
    let fake = fake_of(0);
    let (mut object, mem, _slot) = built(&fake);
    let stored = view(&fake, &mut object, "snapshot");
    drop(object);
    saved_by(&mem, "0.0.0");
    fake.now.set(fake.now.get() + 3600);
    let now = fake.now.get();
    // The store holds an alarm ten minutes ahead.
    let slot = Slot::default();
    slot.0.set(Some(now + 600));
    let mut object = Obj::load("o/a", mem.clone(), slot.clone());

    let read = block_on(object.view(&fake, "snapshot", false));
    assert_eq!(read.status, 200, "{}", read.body);
    assert_eq!(serde_json::from_str::<Value>(&read.body).unwrap(), stored);
    assert_eq!(
        slot.0.get(),
        Some(now),
        "the alarm is brought forward to now"
    );

    let from = fake.gh.calls.borrow().len();
    block_on(object.on_alarm(&fake));
    assert!(
        graphql_calls(&fake, from)
            .iter()
            .any(|q| q.contains("111:src/a.rs")),
        "the alarm fetches src/a.rs"
    );
}
