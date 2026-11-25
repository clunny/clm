//! The paths the newer calls replace are gone from this crate's files (`old-paths-removed`).

use std::fs;
use std::path::{Path, PathBuf};

/// Every file of this crate outside `target/`, with its text when it is text.
fn files() -> Vec<(PathBuf, String)> {
    let mut dirs = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR"))];
    let mut files = Vec::new();
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                if !path.ends_with("target") {
                    dirs.push(path);
                }
            } else if let Ok(text) = fs::read_to_string(&path) {
                files.push((path, text));
            }
        }
    }
    files
}

/// Whether `rest` starts with `name` as a whole identifier.
fn names(rest: &str, name: &str) -> bool {
    rest.strip_prefix(name)
        .is_some_and(|after| !after.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
}

/// What a source line declares or re-exports that `old-paths-removed` says no source offers.
fn withdrawn(line: &str) -> Option<&'static str> {
    let line = line.trim_start();
    let public = line
        .strip_prefix("pub ")
        .or_else(|| line.strip_prefix("pub(crate) "))
        .map(str::trim_start);
    let field = public.unwrap_or(line);
    if public.is_some_and(|rest| {
        rest.strip_prefix("trait ")
            .is_some_and(|rest| names(rest, "Kv"))
    }) {
        return Some("a public `Kv` trait");
    }
    for name in ["kv", "kv_writes"] {
        if field
            .strip_prefix(name)
            .is_some_and(|after| after.trim_start().starts_with(':') && !after.starts_with("::"))
        {
            return Some("a `kv` or `kv_writes` field");
        }
    }
    for (name, what) in [
        ("check_direct", "`check_direct`"),
        ("is_spec_path", "`is_spec_path`"),
        ("unix_time", "`unix_time`"),
    ] {
        let Some(rest) = public else { continue };
        let declared = rest.split_once("fn ").is_some_and(|(before, after)| {
            before
                .split_whitespace()
                .all(|w| ["const", "async", "unsafe"].contains(&w))
                && names(after, name)
        });
        let exported = rest.strip_prefix("use ").is_some_and(|path| {
            path.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .any(|word| word == name)
        });
        if declared || exported {
            return Some(what);
        }
    }
    None
}

#[test]
fn the_old_paths_are_gone() {
    // req:web.worker.platform/old-paths-removed@3fb7a67f
    let marker = concat!("clm", ":remove-by 20261124-wave-6-worker-by-the-rule");
    let files = files();
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    assert!(
        files.iter().any(|(p, _)| p.ends_with("src/fake.rs"))
            && files.iter().any(|(p, _)| p.ends_with("Cargo.toml")),
        "the crate's files were not read"
    );
    let mut found = Vec::new();
    for (path, text) in &files {
        let source = path.starts_with(&src) && path.extension().is_some_and(|e| e == "rs");
        for (n, line) in text.lines().enumerate() {
            let what = source
                .then(|| withdrawn(line))
                .flatten()
                .or(line.contains(marker).then_some("the change's marker"));
            if let Some(what) = what {
                found.push(format!("{}:{}: {what}", path.display(), n + 1));
            }
        }
    }
    assert!(found.is_empty(), "withdrawn paths remain: {found:#?}");

    // The reading itself sees each declaration it looks for, in the forms it may take.
    for line in [
        "pub trait Kv {",
        "pub trait Kv: Cache {",
        "pub trait Kv<T> {",
        "    pub kv: RefCell<BTreeMap<String, String>>,",
        "    pub kv : RefCell<BTreeMap<String, String>>,",
        "    kv_writes: RefCell<Vec<String>>,",
        "pub async fn check_direct<P: GitHub>(",
        "pub(crate) fn is_spec_path(path: &str) -> bool {",
        "pub const fn unix_time(iso: &str) -> Option<u64> {",
        "pub use clm_ledger::unix_time;",
        "pub use crate::access::{check_direct, checked_direct};",
    ] {
        assert!(withdrawn(line).is_some(), "not seen: {line}");
    }
    for line in [
        "pub async fn checked_direct<P: GitHub>(",
        "pub trait Kvs {",
        "    .and_then(clm_ledger::unix_time)",
        "    clm_replica::keeps_text(path) || clm_gate::is_spec_path(path)",
        "use clm_ledger::unix_time;",
    ] {
        assert!(withdrawn(line).is_none(), "seen: {line}");
    }
}
