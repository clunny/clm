//! `paths-under-the-root`: how `clm rebase` names the CLM root's files, a root that may be a
//! subdirectory of its git repository (`root-from-cwd`). A second copy of the gate's
//! naming (`gate/paths-under-the-root`), since `clm-sync` doesn't depend on `clm-gate`.

use std::path::Path;

use crate::{Error, git};

/// The CLM root's place in its repository, as `git rev-parse --show-prefix` gives it: empty
/// at the top, else a directory ending in `/`.
pub(crate) struct Root {
    prefix: String,
}

impl Root {
    /// The place of the CLM root at `root`, asked of git once per run.
    pub(crate) fn of(root: &Path) -> Result<Root, Error> {
        Ok(Root {
            prefix: git(root, &["rev-parse", "--show-prefix"])?,
        })
    }

    /// The prefix itself.
    pub(crate) fn prefix(&self) -> &str {
        &self.prefix
    }

    /// A root-relative path named from the repository's top, as `extract_paths` and a
    /// `<rev>:<path>` object read it.
    pub(crate) fn under(&self, path: &str) -> String {
        format!("{}{path}", self.prefix)
    }

    /// A path as git prints it from the repository's top, named relative to the root:
    /// under it, the prefix taken off; outside it, the shortest path from the root,
    /// climbing out with `../`.
    pub(crate) fn name(&self, path: &str) -> String {
        if let Some(rest) = path.strip_prefix(self.prefix.as_str()) {
            return rest.to_string();
        }
        let root: Vec<&str> = self.prefix.split('/').filter(|c| !c.is_empty()).collect();
        let parts: Vec<&str> = path.split('/').collect();
        let shared = root.iter().zip(&parts).take_while(|(a, b)| a == b).count();
        let mut out = "../".repeat(root.len() - shared);
        out.push_str(&parts[shared..].join("/"));
        out
    }
}

/// The object naming the root-relative `path` at `rev`, which git reads from the directory
/// it runs in.
pub(crate) fn object(rev: &str, path: &str) -> String {
    format!("{rev}:./{path}")
}
