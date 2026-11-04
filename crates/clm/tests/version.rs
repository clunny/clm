//! `clm --version` names the commit its source was built from (clm/build-commit).

use std::path::Path;
use std::process::Command;

#[test]
fn version_names_the_commit_built_from() {
    // req:clm/build-commit@b9775322
    let krate = Path::new(env!("CARGO_MANIFEST_DIR"));
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(krate)
        .output()
        .unwrap();
    assert!(head.status.success(), "git rev-parse HEAD failed");
    let head = String::from_utf8(head.stdout).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim_end(),
        format!("clm {} ({})", env!("CARGO_PKG_VERSION"), &head.trim()[..12])
    );
}
