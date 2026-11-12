//! The corpus: `make.sh` and the entries it wrote.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus")
}

/// The `clm` built from this tree, next to the test binaries' `deps` directory.
fn built_clm() -> PathBuf {
    let status = Command::new(env!("CARGO"))
        .args(["build", "-q", "-p", "clm"])
        .status()
        .unwrap();
    assert!(status.success(), "cargo build -p clm");
    let exe = std::env::current_exe().unwrap();
    let clm = exe.parent().unwrap().parent().unwrap().join("clm");
    assert!(clm.exists(), "{} is not built", clm.display());
    clm
}

/// A copy of `make.sh` and the sample in a directory of its own, so entries land there.
fn corpus_copy() -> TempDir {
    let dir = TempDir::new().unwrap();
    let status = Command::new("cp")
        .arg("-r")
        .arg(corpus().join("sample"))
        .arg(corpus().join("make.sh"))
        .arg(dir.path())
        .status()
        .unwrap();
    assert!(status.success());
    dir
}

fn make(dir: &Path, version: &str, clm: &Path) -> std::process::Output {
    Command::new("sh")
        .arg(dir.join("make.sh"))
        .arg(version)
        .arg(clm)
        .output()
        .unwrap()
}

fn unpack(tarball: &Path, into: &Path) -> Vec<PathBuf> {
    fs::create_dir_all(into).unwrap();
    let status = Command::new("tar")
        .arg("-xzf")
        .arg(tarball)
        .arg("-C")
        .arg(into)
        .status()
        .unwrap();
    assert!(status.success());
    let out = Command::new("tar")
        .arg("-tzf")
        .arg(tarball)
        .output()
        .unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(PathBuf::from)
        .collect()
}

// req:upgrade/corpus-entry-script@f42f9f3c
#[test]
fn the_script_writes_the_same_entry_every_time() {
    let clm = built_clm();
    let dir = corpus_copy();
    let first = make(dir.path(), "9.9.9", &clm);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let entry = dir.path().join("v9.9.9.tar.gz");
    let one = fs::read(&entry).unwrap();
    fs::remove_file(&entry).unwrap();
    let second = make(dir.path(), "9.9.9", &clm);
    assert!(second.status.success());
    assert_eq!(
        fs::read(&entry).unwrap(),
        one,
        "the entry differs between runs"
    );

    let out = TempDir::new().unwrap();
    let names = unpack(&entry, out.path());
    for want in [
        "clm.toml",
        "ARCHITECTURE.md",
        "CLAUDE.md",
        "specs/sample/SPEC.md",
        "tests/sample.rs",
        "backlog/sample-item.md",
        "changes/archive/20260101-sample/change.md",
    ] {
        assert!(names.iter().any(|n| n == Path::new(want)), "no {want}");
    }
    assert!(
        names.iter().all(|n| !n.starts_with(".git/")),
        "the entry holds .git"
    );
    // `init` ran with `--no-detectors --host none`.
    let settings = fs::read_to_string(out.path().join("clm.toml")).unwrap();
    assert!(settings.contains("host = \"none\""), "{settings}");
    assert!(!settings.contains("[[detectors]]"), "{settings}");
    // The sample's placeholder is a marker prefix, not left in the file.
    let test = fs::read_to_string(out.path().join("tests/sample.rs")).unwrap();
    assert!(!test.contains("@REQ@") && test.contains(concat!("req", ":sample/greets@")));
}

// req:upgrade/corpus-entry-script@f42f9f3c
#[test]
fn the_script_fails_and_writes_nothing_when_the_check_fails() {
    let dir = corpus_copy();
    let fake = dir.path().join("fake-clm");
    fs::write(&fake, "#!/bin/sh\n[ \"$1\" = check ] && exit 1\nexit 0\n").unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    let out = make(dir.path(), "9.9.9", &fake);
    assert!(!out.status.success());
    assert!(!dir.path().join("v9.9.9.tar.gz").exists());
}

// req:upgrade/migration-list@067d119c
#[test]
fn a_fresh_project_has_every_migration_done() {
    let clm = built_clm();
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    for args in [&["init", "-q"][..], &["config", "user.name", "t"]] {
        let status = Command::new("git")
            .args(args)
            .current_dir(&root)
            .status()
            .unwrap();
        assert!(status.success());
    }
    let status = Command::new(&clm)
        .args(["init", "--lang", "rust", "--no-detectors", "--host", "none"])
        .current_dir(&root)
        .status()
        .unwrap();
    assert!(status.success());
    // A note a later done migration supersedes is left out of `pending`, though its own
    // `done` stays false.
    let pending = clm_upgrade::pending(&root).unwrap();
    for m in clm_upgrade::migrations() {
        assert!(
            (m.done)(&root) || !pending.iter().any(|p| p.id == m.id),
            "{} is not done on a fresh project",
            m.id
        );
    }
    assert!(
        pending.is_empty(),
        "pending on a fresh project: {pending:?}"
    );
}

// req:upgrade/corpus-seeded@43044b86
#[test]
fn the_corpus_holds_an_entry_for_every_published_release() {
    for version in ["0.14.0", "0.15.0", "0.16.0", "0.17.0", "0.18.0"] {
        let entry = corpus().join(format!("v{version}.tar.gz"));
        assert!(entry.exists(), "no corpus entry for v{version}");
        let out = TempDir::new().unwrap();
        let names = unpack(&entry, out.path());
        assert!(
            names.iter().any(|n| n == Path::new("clm.toml")),
            "v{version} holds no clm.toml"
        );
    }
}
