//! `replays`: whether a branch is exactly what `clm rename` produces (`replay`).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use clm_fingerprint::{Entry, parse_ls_tree};
use clm_git::{Failure, Options};

use crate::{build, record};

/// Whether `head`'s tree is exactly what `rename` produces from `base`'s with the change
/// `head` adds. False, without extracting a tree, when `head` adds no change that sets
/// `renames`, or adds more than one.
pub fn replays(repo: &Path, base: &str, head: &str) -> Result<bool, Failure> {
    let listing = |rev: &str| -> Result<Vec<Entry>, Failure> {
        let args = ["ls-tree", "-r", "-z", "--full-tree", rev];
        Ok(parse_ls_tree(&clm_git::run(
            repo,
            "git",
            &args,
            Options::default(),
        )?))
    };
    let (before, after) = (listing(base)?, listing(head)?);
    let added = clm_fingerprint::added_changes(&before, &after);
    let [path] = added.as_slice() else {
        return Ok(false);
    };
    let path = String::from_utf8_lossy(path).into_owned();
    let Some(id) = path
        .strip_suffix("/change.md")
        .and_then(|p| p.rsplit('/').next())
    else {
        return Ok(false);
    };
    let blob = format!("{head}:{path}");
    let bytes = clm_git::run(
        repo,
        "git",
        &["cat-file", "blob", &blob],
        Options::default(),
    )?;
    let source = String::from_utf8_lossy(&bytes).into_owned();
    if !record::records_renames(&source) {
        return Ok(false);
    }
    let tree = clm_git::extract_tree(repo, base)?;
    let change = tree.path().join("changes").join(id).join("change.md");
    let placed = fs::create_dir_all(change.parent().unwrap_or(tree.path()))
        .and_then(|()| fs::write(&change, record::with_status(&source, "proposed")));
    if placed.is_err() || build::carry_out(tree.path(), id).is_err() {
        return Ok(false);
    }
    let made = hashes(repo, tree.path())?;
    let known: BTreeSet<&[u8]> = before.iter().map(|e| e.path.as_slice()).collect();
    // A session files defect reports on any branch, so a new backlog item is no edit.
    let kept =
        |e: &&Entry| !(e.path.starts_with(b"backlog/") && !known.contains(e.path.as_slice()));
    let wanted: BTreeMap<String, String> = after
        .iter()
        .filter(|e| e.kind == "blob" && e.mode.starts_with("100"))
        .filter(kept)
        .map(|e| (String::from_utf8_lossy(&e.path).into_owned(), e.id.clone()))
        .collect();
    Ok(made == wanted)
}

/// The blob ID of each regular file under `dir`, by path relative to it.
fn hashes(repo: &Path, dir: &Path) -> Result<BTreeMap<String, String>, Failure> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(d) = pending.pop() {
        let entries =
            fs::read_dir(&d).map_err(|e| Failure::Git(format!("{}: {e}", d.display())))?;
        for entry in entries.flatten() {
            match entry.file_type() {
                Ok(t) if t.is_dir() => pending.push(entry.path()),
                Ok(t) if t.is_file() => files.push(entry.path()),
                _ => {}
            }
        }
    }
    files.sort();
    let input: String = files.iter().map(|f| format!("{}\n", f.display())).collect();
    let out = clm_git::run(
        repo,
        "git",
        &["hash-object", "--no-filters", "--stdin-paths"],
        Options {
            stdin: Some(input.as_bytes()),
            env: &[],
        },
    )?;
    let ids = String::from_utf8_lossy(&out).into_owned();
    Ok(files
        .iter()
        .zip(ids.lines())
        .filter_map(|(f, id)| {
            let rel = f.strip_prefix(dir).ok()?;
            Some((rel.display().to_string(), id.to_string()))
        })
        .collect())
}
