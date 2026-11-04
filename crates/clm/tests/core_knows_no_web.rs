//! The root Design: the core has no knowledge of the web, which reuses it compiled to
//! WebAssembly. No core crate may depend on `clm-web` or on a Worker crate.

use std::fs;
use std::path::{Path, PathBuf};

const CORE: [&str; 4] = ["clm-model", "clm-check", "clm-context", "clm-orchestrator"];

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The names of every dependency a manifest declares, in any dependency table,
/// target-specific ones included.
fn dependencies(manifest: &str) -> Vec<String> {
    const TABLES: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];
    let file: toml::Table = manifest.parse().unwrap();
    let mut scopes = vec![&file];
    if let Some(targets) = file.get("target").and_then(toml::Value::as_table) {
        scopes.extend(targets.values().filter_map(toml::Value::as_table));
    }
    scopes
        .into_iter()
        .flat_map(|scope| TABLES.iter().filter_map(|t| scope.get(*t)))
        .filter_map(toml::Value::as_table)
        .flat_map(|deps| deps.keys().cloned())
        .collect()
}

/// The dependencies that would give a core crate knowledge of the web.
fn web_dependencies(manifest: &str) -> Vec<String> {
    dependencies(manifest)
        .into_iter()
        .filter(|d| d == "clm-web" || d.contains("worker"))
        .collect()
}

#[test]
fn core_crates_depend_on_no_web_crate() {
    for krate in CORE {
        let path = repo().join("crates").join(krate).join("Cargo.toml");
        let manifest = fs::read_to_string(&path).unwrap();
        assert_eq!(web_dependencies(&manifest), Vec::<String>::new(), "{krate}");
    }
}

#[test]
fn web_dependencies_are_found_in_every_table() {
    let manifest = r#"
        [dependencies]
        clm-web = { path = "../clm-web" }
        serde = "1"
        [dev-dependencies]
        worker = "0.4"
        [target.'cfg(target_arch = "wasm32")'.build-dependencies]
        worker-build = "0.1"
    "#;
    assert_eq!(
        web_dependencies(manifest),
        ["clm-web", "worker", "worker-build"]
    );
}
