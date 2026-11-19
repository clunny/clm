//! Passes `clm --version` the commit the source was built from, as `CLM_VERSION`
//! (clm/build-commit). It runs `git` through `clm-git`, as the binary does.

use std::path::{Path, PathBuf};

#[path = "src/version.rs"]
mod version;

fn main() {
    let dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let commit = git(&dir, &["rev-parse", "HEAD"]).and_then(|head| {
        let head = head.trim();
        (head.len() >= 12 && head.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| head[..12].to_string())
    });
    if let Some(dirs) = git(&dir, &["rev-parse", "--git-dir", "--git-common-dir"]) {
        let mut lines = dirs.lines().map(|l| dir.join(l));
        if let (Some(git_dir), Some(common)) = (lines.next(), lines.next()) {
            // Only paths that exist: cargo reruns the script on every build for one that
            // doesn't, which would relink `clm` every time.
            for path in [
                git_dir.join("HEAD"),
                common.join("packed-refs"),
                common.join("refs/heads"),
            ] {
                if path.exists() {
                    println!("cargo:rerun-if-changed={}", path.display());
                }
            }
        }
    }
    let package = std::env::var("CARGO_PKG_VERSION").unwrap();
    println!(
        "cargo:rustc-env=CLM_VERSION={}",
        version::version_string(&package, commit.as_deref())
    );
}

/// `git <args>` in `dir`: its stdout, or `None` when it can't run or fails.
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    clm_git::git(dir, args).ok()
}
