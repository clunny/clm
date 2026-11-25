//! The paths the newer calls replace are gone from this crate's sources (`old-paths-removed`).

use std::fs;
use std::path::{Path, PathBuf};

/// Every `.rs` file under this crate's `src/`, with its text.
fn sources() -> Vec<(PathBuf, String)> {
    let mut dirs = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
    let mut files = Vec::new();
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = fs::read_to_string(&path).unwrap();
                files.push((path, text));
            }
        }
    }
    files
}

/// What each line declares that `old-paths-removed` says no source declares.
fn withdrawn(line: &str) -> Option<&'static str> {
    let line = line.trim_start();
    let public = ["pub fn ", "pub async fn "];
    let declares = |name: &str| {
        public.iter().any(|p| {
            line.strip_prefix(p)
                .and_then(|rest| rest.strip_prefix(name))
                .is_some_and(|rest| rest.starts_with(['(', '<']))
        })
    };
    if line.starts_with("pub trait Kv ") || line.starts_with("pub trait Kv{") {
        Some("a public `Kv` trait")
    } else if ["kv:", "pub kv:", "kv_writes:", "pub kv_writes:"]
        .iter()
        .any(|f| line.starts_with(f))
    {
        Some("a `kv` or `kv_writes` field")
    } else if declares("check_direct") {
        Some("`check_direct`")
    } else if declares("is_spec_path") {
        Some("`is_spec_path`")
    } else if declares("unix_time") {
        Some("`unix_time`")
    } else {
        None
    }
}

#[test]
fn the_old_paths_are_gone() {
    // req:web.worker.platform/old-paths-removed@3fb7a67f
    let marker = concat!("clm", ":remove-by 20261124-wave-6-worker-by-the-rule");
    let files = sources();
    assert!(
        files.iter().any(|(p, _)| p.ends_with("fake.rs")),
        "the sources were not read"
    );
    let mut found = Vec::new();
    for (path, text) in &files {
        for (n, line) in text.lines().enumerate() {
            let what = withdrawn(line).or(line.contains(marker).then_some("the change's marker"));
            if let Some(what) = what {
                found.push(format!("{}:{}: {what}", path.display(), n + 1));
            }
        }
    }
    assert!(found.is_empty(), "withdrawn paths remain: {found:#?}");

    // The reading itself sees each declaration it looks for.
    for line in [
        "pub trait Kv {",
        "    pub kv: RefCell<BTreeMap<String, String>>,",
        "            kv_writes: RefCell::default(),",
        "pub async fn check_direct<P: GitHub>(",
        "pub fn is_spec_path(path: &str) -> bool {",
        "pub fn unix_time(iso: &str) -> Option<u64> {",
    ] {
        assert!(withdrawn(line).is_some(), "not seen: {line}");
    }
    assert!(withdrawn("pub async fn checked_direct<P: GitHub>(").is_none());
}
