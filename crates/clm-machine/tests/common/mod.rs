#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use clm_machine::{Readings, Record, Report, Space};

pub const GIB: u64 = 1 << 30;
pub const MIB: u64 = 1 << 20;

pub fn space(total: u64, available: u64) -> Option<Space> {
    Some(Space { total, available })
}

/// Readings of an 8-core machine with plenty of everything, to override per test.
pub fn readings() -> Readings {
    Readings {
        cores: 8,
        load1: Some(1.0),
        load5: Some(1.0),
        memory: space(16 * GIB, 8 * GIB),
        disk_path: "/clone/.git".into(),
        disk: space(100 * GIB, 50 * GIB),
    }
}

pub fn judged(r: &Readings, record: &Record, counting: &[&str], max: u64) -> Report {
    let names: Vec<String> = counting.iter().map(|s| s.to_string()).collect();
    clm_machine::judge(r, record, &names, max)
}

/// A git repository with one commit, in a temporary directory.
pub fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    clm_git::git(dir.path(), &["init", "-q", "-b", "main"]).unwrap();
    commit(dir.path());
    dir
}

pub fn commit(dir: &Path) {
    clm_git::git(
        dir,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "x",
        ],
    )
    .unwrap();
}

/// A counting build place `name` of `repo`, whose worktree is a new directory on branch
/// `impl/<name>`: returns the worktree's path.
pub fn build(repo: &Path, name: &str) -> (tempfile::TempDir, PathBuf) {
    let holder = tempfile::tempdir().unwrap();
    let path = holder.path().join(name);
    let branch = format!("impl/{name}");
    clm_git::git(
        repo,
        &[
            "worktree",
            "add",
            "-q",
            path.to_str().unwrap(),
            "-b",
            &branch,
        ],
    )
    .unwrap();
    let placed = clm_orchestrator::take_place(
        repo,
        name,
        clm_orchestrator::PlaceKind::Build,
        &[branch],
        3,
        4,
    )
    .unwrap();
    assert!(!matches!(placed, clm_orchestrator::Placed::Full { .. }));
    (holder, path)
}

pub fn common(repo: &Path) -> PathBuf {
    clm_git::common_dir(repo).unwrap()
}

/// A file of `bytes` of data that takes real blocks.
pub fn fill(path: &Path, bytes: u64) {
    fs::write(path, vec![7u8; bytes as usize]).unwrap();
}
