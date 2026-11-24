//! The panel plugin's package under `plugin-panel/`.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn panel() -> PathBuf {
    repo().join("plugin-panel")
}

fn json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn workspace_version() -> String {
    let text = fs::read_to_string(repo().join("Cargo.toml")).unwrap();
    let mut in_package = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_package = line == "[workspace.package]";
        } else if in_package && let Some(v) = line.strip_prefix("version = ") {
            return v.trim_matches('"').to_owned();
        }
    }
    panic!("no [workspace.package] version");
}

/// A plugin's `plugin.json` names the workspace version, and a failure names both.
fn assert_workspace_version(manifest: &Value, plugin: &str) {
    let (version, workspace) = (&manifest["version"], workspace_version());
    assert!(
        version.as_str() == Some(workspace.as_str()),
        "{plugin}'s plugin.json version {version} differs from the workspace version {workspace}"
    );
}

#[test]
fn panel_package() {
    // req:plugin.panel/panel-package@80e45fc2
    let panel = panel();
    let manifest = json(&panel.join(".claude-plugin/plugin.json"));
    assert_eq!(manifest["name"], "clm-panel");
    assert_eq!(manifest["dependencies"], serde_json::json!(["clm"]));
    assert_eq!(manifest["author"]["name"], "Christopher Lunny");
    assert_eq!(manifest["license"], "MIT");
    assert_workspace_version(&manifest, "plugin-panel");
    let package = json(&panel.join("package.json"));
    // It runs the tests, after the type-check (`type-check`).
    let test = package["scripts"]["test"].as_str().unwrap_or_default();
    assert!(test.ends_with("vitest run"), "{package}");
    assert!(panel.join("package-lock.json").is_file());
    let npmrc = fs::read_to_string(panel.join(".npmrc")).unwrap();
    assert!(
        npmrc.lines().any(|l| l.trim() == "ignore-scripts=true"),
        "{npmrc}"
    );
}

#[test]
fn panel_hooks_module() {
    // req:plugin.panel/panel-package@80e45fc2
    let panel = panel();
    assert_eq!(
        json(&panel.join("hooks/hooks.json")),
        serde_json::json!({ "modules": ["./register.tsx"] })
    );
    assert!(panel.join("hooks/register.tsx").is_file());
}
