//! The log a failed run kept (verify/kept-log).

use std::path::{Path, PathBuf};

use crate::tree;

/// The shortest name a log may have: `git rev-parse --short`'s least length.
const MIN_NAME: usize = 7;

/// The path of the log a failed run of `commit` (a full commit ID) kept in the clone of `root`,
/// a CLM root in any of its worktrees, or none. The file is `<name>.log` in
/// `.clm/verify-logs/` of the main worktree's CLM root, `<name>` a prefix of `commit` of at
/// least 7 characters, the longest when several match. It reads that directory and nothing
/// else, and creates and removes nothing.
pub fn kept_log(root: &Path, commit: &str) -> Option<PathBuf> {
    let prefix = clm_git::git(root, &["rev-parse", "--show-prefix"]).ok()?;
    let dir = tree::places_of(root, &prefix).ok()?.join("verify-logs");
    std::fs::read_dir(&dir)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file = entry.file_name().into_string().ok()?;
            let name = file.strip_suffix(".log")?;
            (name.len() >= MIN_NAME && commit.starts_with(name)).then(|| (name.len(), entry.path()))
        })
        .max_by_key(|(len, _)| *len)
        .map(|(_, path)| path)
}
