//! `install-and-release`: a person installs CLM as a plugin on a machine with Claude Code and
//! no Rust toolchain, and its first session runs the release's `clm` binary of the same
//! version.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

/// A fake `gh`, the GitHub release beyond the boundary: `gh release download v<version>
/// --pattern <asset> --pattern <asset>.sha256 --dir <dir>` writes the release's asset, a
/// tarball holding a `clm` that says which version it is, and its checksum.
const FAKE_GH: &str = r##"#!/bin/sh
echo "$*" >> "$FAKE_GH_LOG"
dir=
asset=
while [ $# -gt 0 ]; do
  case "$1" in
    --dir) dir=$2; shift ;;
    --pattern) case "$2" in *.sha256) ;; *) asset=$2 ;; esac; shift ;;
  esac
  shift
done
work=$(mktemp -d)
printf '#!/bin/sh\necho "clm release %s: $*"\n' "$FAKE_VERSION" > "$work/clm"
chmod +x "$work/clm"
tar czf "$dir/$asset" -C "$work" clm
(cd "$dir" && { sha256sum "$asset" 2>/dev/null || shasum -a 256 "$asset"; } > "$asset.sha256")
"##;

/// The plugin as a person's Claude Code installs it, copied from the repository's `plugin/`
/// into `dir`; returns the plugin's version.
fn install_plugin(dir: &Path) -> String {
    let plugin = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugin");
    let status = Command::new("cp")
        .arg("-R")
        .arg(&plugin)
        .arg(dir)
        .status()
        .unwrap();
    assert!(status.success());
    let manifest = fs::read_to_string(dir.join(".claude-plugin/plugin.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    manifest["version"].as_str().unwrap().to_string()
}

#[test]
fn install() {
    // req:capability/install-and-release@6456c1e6
    let tmp = TempDir::new().unwrap();
    let top = tmp.path().canonicalize().unwrap();
    let installed = top.join("installed");
    let version = install_plugin(&installed);

    // A machine with Claude Code and `gh`, and no Rust toolchain: `PATH` holds only the
    // system's tools and the fake `gh`.
    let bin = top.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let gh = bin.join("gh");
    fs::write(&gh, FAKE_GH).unwrap();
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o755)).unwrap();
    let log = top.join("gh.log");
    let cache = top.join("cache");
    let session = |args: &[&str]| {
        Command::new(installed.join("bin/clm"))
            .args(args)
            .env_clear()
            .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
            .env("HOME", &top)
            .env("XDG_CACHE_HOME", &cache)
            .env("FAKE_GH_LOG", &log)
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
        downloads.contains(&format!("release download v{version} ")),
        "{downloads}"
    );

    // It is cached: the next session downloads nothing.
    let second = session(&["status"]);
    assert!(second.status.success());
    assert_eq!(fs::read_to_string(&log).unwrap(), downloads);
}
