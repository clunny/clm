//! The repo files the model reads, as a map of repo-relative paths to contents, and
//! directory listings over its keys.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::ops::Bound;
use std::path::{Path, PathBuf};
use std::str::Utf8Error;

use crate::{Layer, Violation, v};

/// Repo-relative paths to file contents.
pub type Files = BTreeMap<PathBuf, Vec<u8>>;

/// The directories `load` reads.
const DIRS: [&str; 4] = ["specs", "changes", "backlog", "docs/decisions"];

/// Reads every file under `<root>/specs`, `changes`, `backlog` and `docs/decisions`, and
/// the root's `ARCHITECTURE.md` and `CAPABILITIES.md` when they exist. Links aren't
/// followed into directories. A file that can't be read is a violation.
pub(crate) fn read_files(root: &Path) -> (Files, Vec<Violation>) {
    fn walk(root: &Path, dir: &Path, files: &mut Files, violations: &mut Vec<Violation>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                walk(root, &path, files, violations);
                continue;
            }
            let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            match fs::read(&path) {
                Ok(bytes) => {
                    files.insert(rel, bytes);
                }
                Err(e) => {
                    violations.push(v(&rel, "model/load-from-files", format!("unreadable: {e}")))
                }
            }
        }
    }
    let mut files = Files::new();
    let mut violations = Vec::new();
    for dir in DIRS {
        walk(root, &root.join(dir), &mut files, &mut violations);
    }
    for name in Layer::ALL.map(Layer::file) {
        match fs::read(root.join(name)) {
            Ok(bytes) => {
                files.insert(PathBuf::from(name), bytes);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => violations.push(v(
                Path::new(name),
                "model/load-from-files",
                format!("unreadable: {e}"),
            )),
        }
    }
    (files, violations)
}

/// Keys below `dir`.
pub(crate) fn under<'a>(files: &'a Files, dir: &'a Path) -> impl Iterator<Item = &'a PathBuf> {
    files
        .range::<Path, _>((Bound::Included(dir), Bound::Unbounded))
        .map(|(k, _)| k)
        .take_while(move |k| k.starts_with(dir))
        .filter(move |k| k.as_path() != dir)
}

/// Files directly in `dir`, sorted.
pub(crate) fn children(files: &Files, dir: &Path) -> Vec<PathBuf> {
    under(files, dir)
        .filter(|k| k.parent() == Some(dir))
        .cloned()
        .collect()
}

/// Directories directly in `dir` (those holding at least one file), sorted.
pub(crate) fn subdirs(files: &Files, dir: &Path) -> Vec<PathBuf> {
    let dirs: BTreeSet<PathBuf> = under(files, dir)
        .filter_map(|k| {
            let mut rest = k.strip_prefix(dir).ok()?.components();
            let first = rest.next()?;
            rest.next()?;
            Some(dir.join(first))
        })
        .collect();
    dirs.into_iter().collect()
}

/// The text of `path`: `None` when absent, `Err` when not UTF-8.
pub(crate) fn text(files: &Files, path: &Path) -> Option<Result<String, Utf8Error>> {
    files
        .get(path)
        .map(|b| std::str::from_utf8(b).map(str::to_owned))
}
