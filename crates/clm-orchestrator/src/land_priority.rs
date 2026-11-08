//! The priority a land carries (`land-priority`): read from git objects alone.

use std::path::Path;

use clm_git::git;
use clm_model::{ARCHIVE_DIR, Priority, parse_backlog_item, parse_change_frontmatter};

use crate::Error;
use crate::queue::queue_from_refs;
use crate::repo::load;

/// The priority a land of `branch`, whose head is `head`, carries onto `base`: the highest
/// of its component's effective priority at `base` when `branch` is `impl/<component>`,
/// the priority of each change `head` adds, and that of each backlog item `head` deletes;
/// `P5` when none gives one.
pub fn land_priority(root: &Path, branch: &str, base: &str, head: &str) -> Result<Priority, Error> {
    let tree = clm_git::extract_tree(root, base)?;
    let graph = load(tree.path())?;
    let markers = clm_check::scan(tree.path()).req;
    let mut found = Vec::new();
    if let Some(component) = branch.strip_prefix(crate::repo::BRANCH_PREFIX) {
        let queue = queue_from_refs(&graph, &markers, &[], 0, 0);
        found.extend(queue.node(component).map(|n| n.priority));
    }
    for id in children(root, head, "changes/")? {
        let file = format!("changes/{id}/change.md");
        if id == ARCHIVE_DIR || holds(root, base, &file) {
            continue;
        }
        if let Ok(text) = git(root, &["show", &format!("{head}:{file}")]) {
            found.extend(parse_change_frontmatter(&text).map(|c| c.priority));
        }
    }
    for name in children(root, base, "backlog/")? {
        let file = format!("backlog/{name}");
        let Some(slug) = name.strip_suffix(".md") else {
            continue;
        };
        if holds(root, head, &file) {
            continue;
        }
        if let Ok(text) = git(root, &["show", &format!("{base}:{file}")])
            && let Ok(item) = parse_backlog_item(slug, &text)
        {
            found.push(item.frontmatter.priority);
        }
    }
    Ok(found.into_iter().min().unwrap_or(Priority::P5))
}

/// The names directly under `dir` (ending in `/`) at `rev`; none when it has no such directory.
fn children(root: &Path, rev: &str, dir: &str) -> Result<Vec<String>, Error> {
    Ok(git(root, &["ls-tree", "--name-only", rev, dir])?
        .lines()
        .filter_map(|p| Some(p.strip_prefix(dir)?.to_string()))
        .collect())
}

/// Whether `rev` holds the file `path`.
fn holds(root: &Path, rev: &str, path: &str) -> bool {
    git(root, &["cat-file", "-e", &format!("{rev}:{path}")]).is_ok()
}
