//! `install-and-release`: a person installs CLM as a plugin on a machine with Claude Code and
//! no Rust toolchain, and its first session runs the release's `clm` binary of the same
//! version, whether the plugin is installed in Claude Code or in Codex.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

/// A fake `curl`, the GitHub release beyond the boundary: `curl -fsSL -o <file> <url>` for the
/// release's asset writes a tarball holding a `clm` that says which version it is, and for
/// `<asset>.sha256` its checksum.
const FAKE_CURL: &str = r##"#!/bin/sh
echo "$*" >> "$FAKE_CURL_LOG"
out=
while [ $# -gt 1 ]; do
  [ "$1" = -o ] && out=$2
  shift
done
asset=$(basename "$1")
dir=$(dirname "$out")
case "$asset" in
  *.sha256) ;;
  *)
    work=$(mktemp -d)
    printf '#!/bin/sh\necho "clm release %s: $*"\n' "$FAKE_VERSION" > "$work/clm"
    chmod +x "$work/clm"
    tar czf "$dir/$asset" -C "$work" clm
    ;;
esac
case "$asset" in
  *.sha256) a=${asset%.sha256}
    (cd "$dir" && { sha256sum "$a" 2>/dev/null || shasum -a 256 "$a"; } > "$out") ;;
esac
"##;

/// The plugin as a person's agent harness installs it, copied from the repository's `plugin/`
/// into `dir`; returns the version its `manifest` declares.
fn install_plugin(dir: &Path, manifest: &str) -> String {
    let plugin = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugin");
    let status = Command::new("cp")
        .arg("-R")
        .arg(&plugin)
        .arg(dir)
        .status()
        .unwrap();
    assert!(status.success());
    let manifest = fs::read_to_string(dir.join(manifest)).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    manifest["version"].as_str().unwrap().to_string()
}

/// Installs the plugin by the manifest of one agent harness, runs its first session on a
/// machine with no Rust toolchain, and returns the install's directory (the temporary
/// directory is kept alive by the caller's `tmp`).
fn first_session_runs_release(tmp: &TempDir, manifest: &str) -> std::path::PathBuf {
    let top = tmp.path().canonicalize().unwrap();
    let installed = top.join("installed");
    let version = install_plugin(&installed, manifest);

    // A machine with Claude Code and `curl`, and no Rust toolchain: `PATH` holds only the
    // system's tools and the fake `curl`.
    let bin = top.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let curl = bin.join("curl");
    fs::write(&curl, FAKE_CURL).unwrap();
    fs::set_permissions(&curl, fs::Permissions::from_mode(0o755)).unwrap();
    let log = top.join("curl.log");
    let cache = top.join("cache");
    let session = |args: &[&str]| {
        Command::new(installed.join("bin/clm"))
            .args(args)
            .env_clear()
            .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
            .env("HOME", &top)
            .env("XDG_CACHE_HOME", &cache)
            .env("FAKE_CURL_LOG", &log)
            .env("FAKE_VERSION", &version)
            .current_dir(&top)
            .output()
            .unwrap()
    };

    // The first session runs the release's binary of the plugin's version.
    let first = session(&["status"]);
    let stdout = String::from_utf8_lossy(&first.stdout).into_owned();
    assert!(
        first.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(stdout.trim(), format!("clm release {version}: status"));
    let downloads = fs::read_to_string(&log).unwrap();
    assert!(
        downloads.contains(&format!("/releases/download/v{version}/clm-")),
        "{downloads}"
    );

    // It is cached: the next session downloads nothing.
    let second = session(&["status"]);
    assert!(second.status.success());
    assert_eq!(fs::read_to_string(&log).unwrap(), downloads);
    installed
}

#[test]
fn install() {
    // req:capability/install-and-release@2778e2e3
    let tmp = TempDir::new().unwrap();
    first_session_runs_release(&tmp, ".claude-plugin/plugin.json");
}

#[test]
fn install_on_codex() {
    // req:capability/install-and-release@2778e2e3
    let tmp = TempDir::new().unwrap();
    let installed = first_session_runs_release(&tmp, ".codex-plugin/plugin.json");

    // Its skills are listed: the manifest names a skills directory that holds each skill with
    // a description.
    let manifest = fs::read_to_string(installed.join(".codex-plugin/plugin.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    let skills = installed.join(manifest["skills"].as_str().unwrap());
    let mut listed = Vec::new();
    for entry in fs::read_dir(&skills).unwrap() {
        let skill = entry.unwrap().path().join("SKILL.md");
        let text = fs::read_to_string(&skill).unwrap();
        assert!(text.contains("\ndescription: "), "{}", skill.display());
        listed.push(skill);
    }
    assert!(listed.len() >= 10, "{listed:?}");
}
