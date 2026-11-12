//! `unowned-files`: once the root sets `outside`, every code file is in some node's `paths`
//! or one of its `outside` globs, and every `outside` glob still matches such a file
//! (`incremental-adoption`, see specs/check/SPEC.md).

use std::path::{Path, PathBuf};

use clm_model::{Graph, Violation};

use crate::matches;

/// The spec paths, which no node owns: the same ones `gate` names.
const SPEC_DIRS: [&str; 4] = ["specs/", "changes/", "backlog/", "docs/"];
const SPEC_FILES: [&str; 4] = [
    "ARCHITECTURE.md",
    "CAPABILITIES.md",
    "CLAUDE.md",
    "AGENTS.md",
];

/// None when the root has no `outside` key. `tracked` is the listing `run` or `run_listed` reads.
pub fn violations(
    root: &Path,
    graph: &Graph,
    tracked: &Result<Vec<PathBuf>, clm_git::Error>,
) -> Vec<Violation> {
    let Some(outside) = graph.outside() else {
        return Vec::new();
    };
    let files = match files(root, tracked) {
        Ok(Some(files)) => files,
        Ok(None) => return Vec::new(),
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

/// The listing's files; none at all (`None`) outside a work tree without a listing, where the
/// rule reports nothing. `Err` when listing failed inside a work tree, so untracked files are
/// never read there.
fn files(
    root: &Path,
    tracked: &Result<Vec<PathBuf>, clm_git::Error>,
) -> Result<Option<Vec<PathBuf>>, String> {
    match tracked {
        Ok(files) => Ok(Some(files.clone())),
        Err(e) if in_work_tree(root) => Err(e.to_string()),
        Err(_) => Ok(None),
    }
}

fn in_work_tree(root: &Path) -> bool {
    clm_git::git(root, &["rev-parse", "--is-inside-work-tree"]).is_ok_and(|out| out == "true")
}
