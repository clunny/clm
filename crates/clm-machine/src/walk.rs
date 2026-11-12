//! A worktree's size: the space its files take on disk (`build-measure`).

use std::collections::HashSet;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// The bytes `root`'s files take on disk: `st_blocks × 512`, a hard-linked file once, no
/// symbolic link followed and no other filesystem entered. What can't be read counts nothing.
pub fn size(root: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(root) else {
        return 0;
    };
    let mut seen = HashSet::new();
    let mut total = 0;
    let mut pending = vec![(root.to_path_buf(), meta)];
    let device = pending[0].1.dev();
    while let Some((path, meta)) = pending.pop() {
        if meta.dev() != device {
            continue;
        }
        if (meta.nlink() > 1 && !meta.is_dir()) && !seen.insert((meta.dev(), meta.ino())) {
            continue;
        }
        total += meta.blocks() * 512;
        if !meta.is_dir() {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.flatten() {
            if let Ok(meta) = std::fs::symlink_metadata(entry.path()) {
                pending.push((entry.path(), meta));
            }
        }
    }
    total
}
