//! Only this crate talks to KV and the Cache API, so nothing else can read them: of the
//! workspace's crates, only it, `clm-web-live`, which holds the `Live` Durable Object, and
//! `clm-web-replica`, which holds the `Replica` one, depend on `worker`, and those two
//! name none of the caches.

use std::fs;
use std::path::Path;

#[test]
fn only_the_worker_crates_depend_on_worker() {
    // req:web.worker/cache-is-disposable@58ffcb85
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut users = Vec::new();
    for entry in fs::read_dir(&crates).unwrap().flatten() {
        let Ok(manifest) = fs::read_to_string(entry.path().join("Cargo.toml")) else {
            continue;
        };
        // `worker = ...`, `worker.workspace = true`, `worker={ ... }`.
        let depends = manifest.lines().any(|l| {
            l.trim_start()
                .strip_prefix("worker")
                .is_some_and(|rest| rest.starts_with([' ', '=', '.']))
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
    // req:web.worker/cache-is-disposable@58ffcb85
    // `clm-web-replica`'s sources are read once the crate exists.
    let mut dirs = Vec::new();
    for name in ["clm-web-live", "clm-web-replica"] {
        let src = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(name)
            .join("src");
        if src.is_dir() {
            dirs.push(src);
        }
    }
    assert!(!dirs.is_empty(), "no sources of clm-web-live");
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
