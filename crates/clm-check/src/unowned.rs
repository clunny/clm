//! `unowned-files`: once the root sets `outside`, every code file is in some node's `paths`
//! or one of its `outside` globs, and every `outside` glob still matches such a file
//! (`incremental-adoption`, see specs/check/SPEC.md).

use std::fs;
use std::path::{Path, PathBuf};

use clm_model::{Graph, Violation};

use crate::matches;

/// The spec paths, which no node owns: the same ones `gate` names.
const SPEC_DIRS: [&str; 4] = ["specs/", "changes/", "backlog/", "docs/"];
const SPEC_FILES: [&str; 3] = ["ARCHITECTURE.md", "CAPABILITIES.md", "CLAUDE.md"];

/// None when the root has no `outside` key. `tracked` is `clm_git::tracked_files(root)`.
pub fn violations(
    root: &Path,
    graph: &Graph,
    tracked: &Result<Vec<PathBuf>, clm_git::Error>,
) -> Vec<Violation> {
    let Some(outside) = graph.outside() else {
        return Vec::new();
    };
    let files = match files(root, tracked) {
        Ok(files) => files,
        Err(e) => {
            return vec![Violation {
                path: root.to_path_buf(),
                rule: "check/unowned-files",
                message: e.to_string(),
            }];
        }
    };
    let unowned: Vec<PathBuf> = files
        .into_iter()
        .filter(|f| {
            let name = f.to_string_lossy();
            !is_spec_path(&name)
                && !graph
                    .nodes
                    .values()
                    .any(|n| n.frontmatter.paths.iter().any(|g| matches(g, &name)))
        })
        .collect();
    let in_outside = |f: &PathBuf, g: &str| matches(g, &f.to_string_lossy());
    let files = unowned
        .iter()
        .filter(|f| !outside.iter().any(|g| in_outside(f, g)))
        .map(|f| Violation {
            path: f.clone(),
            rule: "check/unowned-files",
            message: "in no component's paths; add it to a component's paths, or to outside in \
                      specs/SPEC.md until a change adopts it"
                .to_string(),
        });
    let globs = outside
        .iter()
        .filter(|g| !unowned.iter().any(|f| in_outside(f, g)))
        .map(|g| Violation {
            path: PathBuf::from("specs/SPEC.md"),
            rule: "check/unowned-files",
            message: format!("outside glob {g} matches no file outside every component"),
        });
    files.chain(globs).collect()
}

fn is_spec_path(name: &str) -> bool {
    SPEC_DIRS.iter().any(|d| name.starts_with(d)) || SPEC_FILES.contains(&name)
}

/// Every entry the index holds under `root`; outside a git work tree, every file under it
/// but hidden directories and `target/`, as the coverage scan walks. `Err` when listing
/// fails inside a work tree, so untracked files are never read there.
fn files(
    root: &Path,
    tracked: &Result<Vec<PathBuf>, clm_git::Error>,
) -> Result<Vec<PathBuf>, String> {
    match tracked {
        Ok(files) => Ok(files.clone()),
        Err(e) if in_work_tree(root) => Err(e.to_string()),
        Err(_) => {
            let mut files = Vec::new();
            walk(root, root, &mut files);
            files.sort();
            Ok(files)
        }
    }
}

fn in_work_tree(root: &Path) -> bool {
    clm_git::git(root, &["rev-parse", "--is-inside-work-tree"]).is_ok_and(|out| out == "true")
}

/// Symlinks are listed, not followed, as the index holds them.
fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type().is_ok_and(|t| t.is_dir()) {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with('.') && name != "target" {
                walk(root, &path, out);
            }
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_path_buf());
        }
    }
}
