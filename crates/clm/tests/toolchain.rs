//! The repo's build environment: its pinned Rust toolchain,
//! `rust-toolchain.toml`, the workspace's build profiles, and the commands
//! `.claude/settings.json` lets unattended sessions run.

use std::fs;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Whether `channel` is an exact `MAJOR.MINOR.PATCH` version.
fn is_exact_version(channel: &str) -> bool {
    let parts: Vec<&str> = channel.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

fn get<'a>(table: &'a toml::Table, key: &str) -> &'a toml::Value {
    table
        .get(key)
        .unwrap_or_else(|| panic!("rust-toolchain.toml lacks `{key}`"))
}

fn strings(table: &toml::Table, key: &str) -> Vec<String> {
    get(table, key)
        .as_array()
        .unwrap_or_else(|| panic!("`{key}` is not an array"))
        .iter()
        .map(|v| {
            v.as_str()
                .unwrap_or_else(|| panic!("`{key}` holds a non-string"))
                .to_string()
        })
        .collect()
}

// req:clm/pinned-toolchain@26d87ab0
#[test]
fn pinned() {
    let text = fs::read_to_string(repo().join("rust-toolchain.toml")).unwrap();
    let file: toml::Table = text.parse().unwrap();
    let toolchain = get(&file, "toolchain")
        .as_table()
        .expect("`toolchain` is not a table");
    let channel = get(toolchain, "channel")
        .as_str()
        .expect("`channel` is not a string");
    assert!(
        is_exact_version(channel),
        "channel {channel:?} is not MAJOR.MINOR.PATCH"
    );
    let components = strings(toolchain, "components");
    for c in ["rustfmt", "clippy"] {
        assert!(components.iter().any(|x| x == c), "components lack {c}");
    }
    let targets = strings(toolchain, "targets");
    assert!(
        targets.iter().any(|x| x == "wasm32-unknown-unknown"),
        "targets lack wasm32-unknown-unknown"
    );
}

#[test]
fn channel_shape() {
    assert!(is_exact_version("1.99.1"));
    for bad in ["1.99", "stable", "1.99.1-beta", "1..1", "1.99.1.0", ""] {
        assert!(!is_exact_version(bad), "{bad:?} accepted");
    }
}

/// What's wrong with a workspace manifest's profiles: `profile.dev.debug`
/// must be the integer `0`, `[profile.release]` must be absent, and no
/// `[profile.test]` or per-package override of `dev` or `test` may set
/// `debug`, since `cargo test` builds with those.
fn profile_violations(manifest: &toml::Table) -> Vec<String> {
    let mut out = Vec::new();
    let profile = manifest.get("profile").and_then(|p| p.as_table());
    let section = |name: &str| profile.and_then(|p| p.get(name)).and_then(|s| s.as_table());
    let dev_debug = section("dev").and_then(|d| d.get("debug"));
    if dev_debug != Some(&toml::Value::Integer(0)) {
        out.push(format!("profile.dev.debug is {dev_debug:?}, not 0"));
    }
    if section("release").is_some() {
        out.push("profile.release is set".to_string());
    }
    if let Some(v) = section("test").and_then(|t| t.get("debug")) {
        out.push(format!("profile.test sets debug = {v}"));
    }
    for name in ["dev", "test"] {
        let packages = section(name)
            .and_then(|s| s.get("package"))
            .and_then(|p| p.as_table());
        for (package, table) in packages.into_iter().flatten() {
            if let Some(v) = table.get("debug") {
                out.push(format!("profile.{name}.package.{package} sets debug = {v}"));
            }
        }
    }
    out
}

// req:clm/dev-builds-without-debuginfo@0368368b
#[test]
fn dev_profile() {
    let text = fs::read_to_string(repo().join("Cargo.toml")).unwrap();
    let manifest: toml::Table = text.parse().unwrap();
    assert_eq!(profile_violations(&manifest), Vec::<String>::new());
}

// req:clm/dev-builds-without-debuginfo@0368368b
#[test]
fn wrong_profiles_reported() {
    for bad in [
        "",
        "[profile.dev]\ndebug = false\n",
        "[profile.dev]\ndebug = \"none\"\n",
        "[profile.dev]\ndebug = 1\n",
        "[profile.dev]\ndebug = 0\n[profile.release]\nlto = true\n",
        "[profile.dev]\ndebug = 0\n[profile.test]\ndebug = 2\n",
        "[profile.dev]\ndebug = 0\n[profile.dev.package.\"*\"]\ndebug = 2\n",
    ] {
        let manifest: toml::Table = bad.parse().unwrap();
        assert!(
            !profile_violations(&manifest).is_empty(),
            "{bad:?} accepted"
        );
    }
    let good: toml::Table = "[profile.dev]\ndebug = 0\nopt-level = 1\n".parse().unwrap();
    assert_eq!(profile_violations(&good), Vec::<String>::new());
}

/// The toolchain's Bash commands unattended sessions may run beyond `clm run`'s own; the `clm`
/// rules `clm init` seeds come with them.
const TOOLCHAIN: [&str; 8] = [
    "Bash(cargo *)",
    "Bash(npm ci *)",
    "Bash(npm install --ignore-scripts *)",
    "Bash(npm run *)",
    "Bash(npm test *)",
    "Bash(npx playwright test *)",
    "Bash(.github/scripts/ci-check.sh)",
    "Bash(.github/scripts/ci-check.sh *)",
];

fn allowed() -> Vec<&'static str> {
    TOOLCHAIN
        .into_iter()
        .chain(clm_init::UNATTENDED_RULES)
        .collect()
}

/// Every `permissions.allow` entry that isn't one of `allowed()`.
fn disallowed(allow: &[&str]) -> Vec<String> {
    let allowed = allowed();
    allow
        .iter()
        .filter(|a| !allowed.contains(a))
        .map(|a| a.to_string())
        .collect()
}

// req:clm/session-commands@7fb59303
#[test]
fn allowed_commands() {
    let text = fs::read_to_string(repo().join(".claude/settings.json")).unwrap();
    let file: serde_json::Value = serde_json::from_str(&text).unwrap();
    let allow: Vec<&str> = file
        .pointer("/permissions/allow")
        .unwrap_or_else(|| panic!(".claude/settings.json lacks `permissions.allow`"))
        .as_array()
        .unwrap_or_else(|| panic!("`permissions.allow` is not an array"))
        .iter()
        .map(|v| {
            v.as_str()
                .unwrap_or_else(|| panic!("`permissions.allow` holds a non-string"))
        })
        .collect();
    assert_eq!(disallowed(&allow), Vec::<String>::new());
    for a in allowed() {
        assert!(allow.contains(&a), "permissions.allow lacks {a}");
    }
}

// req:clm/session-commands@7fb59303
#[test]
fn bare_npx_refused() {
    assert_eq!(
        disallowed(&["Bash(cargo *)", "Bash(npx *)"]),
        ["Bash(npx *)"]
    );
    for bad in [
        "Bash(npm install *)",
        "Bash(npm *)",
        "Bash(rustup *)",
        "Bash(git push *)",
        "Bash(clm approve *)",
        "Bash(clm approve)",
        "Bash(.github/scripts/clm.sh *)",
    ] {
        assert_eq!(disallowed(&[bad]), [bad], "{bad} not reported");
    }
}
