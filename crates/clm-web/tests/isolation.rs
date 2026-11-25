//! Only this crate talks to the Cache API, so nothing else can read it: of the workspace's
//! crates, only it, `clm-web-live`, which holds the `Live` Durable Object, and
//! `clm-web-replica`, which holds the `Replica` one, depend on `worker`, those two name
//! none of the caches, and no crate uses a KV namespace. What the Worker and the object
//! keep is rebuilt from GitHub once emptied.

mod common;

use std::fs;
use std::path::Path;

use common::*;

#[test]
fn an_emptied_cache_shows_the_same_snapshot() {
    // req:web.worker/cache-is-disposable@3469815b
    let fake = Fake::default();
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    let cookie = fake.user("u", &["o/a"]);
    fake.real_object("o/a");
    let path = "/api/repos/o/a/snapshot";
    let first = fake.handle(get(path, &cookie));
    assert_eq!(first.status, 200, "{first:?}");
    let first = body_json(&first);
    assert_eq!(first["coverage"]["state"], "complete", "{first}");
    assert!(!fake.cache.borrow().is_empty(), "the view was cached");

    fake.cache.borrow_mut().clear();
    fake.empty_object("o/a");
    let mut answers = Vec::new();
    for _ in 0..20 {
        let res = fake.handle(get(path, &cookie));
        answers.push(res.status);
        if res.status == 200 {
            let body = body_json(&res);
            if body["coverage"]["state"] == "complete" {
                assert_eq!(body, first, "the rebuilt snapshot differs");
                assert_eq!(answers[0], 503, "the emptied object was rebuilt");
                return;
            }
        }
        fake.run_alarms();
    }
    panic!("the index never completed: {answers:?}");
}

#[test]
fn only_the_worker_crates_depend_on_worker() {
    // req:web.worker/cache-is-disposable@3469815b
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut users = Vec::new();
    for entry in fs::read_dir(&crates).unwrap().flatten() {
        let Ok(manifest) = fs::read_to_string(entry.path().join("Cargo.toml")) else {
            continue;
        };
        // `worker = ...`, `worker.workspace = true`, `worker={ ... }`, a
        // `[dependencies.worker]` table, or another name with `package = "worker"`.
        let depends = manifest.lines().any(|l| {
            let l = l.trim_start();
            l.strip_prefix("worker")
                .is_some_and(|rest| rest.starts_with([' ', '=', '.']))
                || (l.starts_with('[') && l.trim_end().ends_with(".worker]"))
                || l.replace(' ', "").contains("package=\"worker\"")
        });
        if depends {
            users.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    users.retain(|c| !["clm-web", "clm-web-live", "clm-web-replica"].contains(&c.as_str()));
    assert!(
        users.is_empty(),
        "these crates depend on `worker`: {users:?}"
    );
}

/// The identifiers in `text`: runs of ASCII letters, digits and `_`.
fn identifiers(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
}

#[test]
fn the_objects_name_no_cache() {
    // req:web.worker/cache-is-disposable@3469815b
    let mut dirs = Vec::new();
    for name in ["clm-web-live", "clm-web-replica"] {
        let src = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(name)
            .join("src");
        assert!(src.is_dir(), "no sources of {name}");
        dirs.push(src);
    }
    let mut found = Vec::new();
    let mut read = 0;
    let mut paths = Vec::new();
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                paths.push(path);
            }
        }
    }
    for path in paths {
        read += 1;
        let text = fs::read_to_string(&path).unwrap();
        for word in identifiers(&text).filter(|w| ["kv", "KvStore", "Cache"].contains(w)) {
            found.push(format!("{}: {word}", path.display()));
        }
    }
    assert!(read > 0, "no sources read");
    assert!(found.is_empty(), "the objects name the caches: {found:?}");
}

#[test]
fn no_crate_names_a_kv_store() {
    // req:web.worker/cache-is-disposable@3469815b
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut dirs: Vec<_> = fs::read_dir(&crates)
        .unwrap()
        .flatten()
        .map(|e| e.path().join("src"))
        .filter(|src| src.is_dir())
        .collect();
    assert!(dirs.len() > 1, "the workspace's crates were not found");
    let mut found = Vec::new();
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = fs::read_to_string(&path).unwrap();
                if identifiers(&text).any(|w| w == "KvStore") {
                    found.push(path.display().to_string());
                }
            }
        }
    }
    assert!(found.is_empty(), "these sources name `KvStore`: {found:?}");
}
