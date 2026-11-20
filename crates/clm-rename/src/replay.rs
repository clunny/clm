//! `replays`: whether a branch is exactly what `clm rename` produces (`replay`).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use clm_fingerprint::{Entry, parse_ls_tree};
use clm_git::{Failure, Options};

use crate::{build, record};

/// A tree's entry as the replay compares it: its mode and object ID, by path.
type Entries = BTreeMap<Vec<u8>, (String, String)>;

/// Whether `head`'s tree is exactly what `rename` produces from `base`'s with the change
/// `head` adds, both read as the trees of the CLM root `repo`. False, without extracting a
/// tree, when `head` adds no change that sets `renames`, adds more than one, or changes an
/// entry outside the root.
pub fn replays(repo: &Path, base: &str, head: &str) -> Result<bool, Failure> {
    let prefix = clm_git::git(repo, &["rev-parse", "--show-prefix"])?;
    let listing = |rev: &str| -> Result<(Vec<Entry>, Vec<Entry>), Failure> {
        let args = ["ls-tree", "-r", "-z", "--full-tree", rev];
        let all = parse_ls_tree(&clm_git::run(repo, "git", &args, Options::default())?);
        let (mut inside, mut outside) = (Vec::new(), Vec::new());
        for mut entry in all {
            match entry.path.strip_prefix(prefix.as_bytes()) {
                Some(rel) => {
                    entry.path = rel.to_vec();
                    inside.push(entry);
                }
                None => outside.push(entry),
            }
        }
        Ok((inside, outside))
    };
    let ((before, before_out), (after, after_out)) = (listing(base)?, listing(head)?);
    // The gate classes a replayed branch `spec` whatever it changes, and an approval covers
    // only the root's files, so nothing outside the root may change.
    if before_out != after_out {
        return Ok(false);
    }
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
    let blob = format!("{head}:./{path}");
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
    let work = tree.path().join(&prefix);
    let change = work.join("changes").join(id).join("change.md");
    let placed = fs::create_dir_all(change.parent().unwrap_or(&work))
        .and_then(|()| fs::write(&change, record::with_status(&source, "proposed")));
    if placed.is_err() || build::carry_out(&work, id).is_err() {
        return Ok(false);
    }
    let left = left(repo, &work)?;
    let known: BTreeSet<&[u8]> = before.iter().map(|e| e.path.as_slice()).collect();
    // A session files defect reports on any branch, so a new backlog item is no edit.
    let filed = |e: &Entry| {
        e.mode != LINK && e.path.starts_with(b"backlog/") && !known.contains(e.path.as_slice())
    };
    let wanted: Entries = after
        .iter()
        .filter(|e| e.kind == "blob" && !filed(e))
        .map(|e| (e.path.clone(), (e.mode.clone(), e.id.clone())))
        .collect();
    // The extracted tree holds a submodule only as an empty directory, and `rename` writes
    // none, so `head`'s are `base`'s.
    let submodules = |entries: &[Entry]| -> Vec<Entry> {
        entries
            .iter()
            .filter(|e| e.kind == "commit")
            .cloned()
            .collect()
    };
    Ok(left == wanted && submodules(&before) == submodules(&after))
}

const LINK: &str = "120000";

/// The regular files and symbolic links `rename` left under `dir`, none followed, by path
/// relative to it: a file's mode and blob ID, its bytes hashed with no filter, and a link's
/// mode and the blob ID of the path it names.
fn left(repo: &Path, dir: &Path) -> Result<Entries, Failure> {
    let unreadable =
        |path: &Path, e: std::io::Error| Failure::Git(format!("{}: {e}", path.display()));
    let rel = |path: &Path| -> Vec<u8> {
        let rel = path.strip_prefix(dir).unwrap_or(path);
        rel.as_os_str().as_encoded_bytes().to_vec()
    };
    let (mut files, mut entries) = (Vec::new(), Entries::new());
    let mut pending = vec![dir.to_path_buf()];
    while let Some(d) = pending.pop() {
        for entry in fs::read_dir(&d).map_err(|e| unreadable(&d, e))?.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_symlink() {
                let target = fs::read_link(&path).map_err(|e| unreadable(&path, e))?;
                let id = hash(repo, &["--stdin"], target.as_os_str().as_encoded_bytes())?;
                entries.insert(rel(&path), (LINK.to_string(), id));
            } else if kind.is_file() {
                let meta = entry.metadata().map_err(|e| unreadable(&path, e))?;
                files.push((path, mode(&meta)));
            }
        }
    }
    let input: String = files
        .iter()
        .map(|(f, _)| format!("{}\n", f.display()))
        .collect();
    let ids = hash(repo, &["--stdin-paths"], input.as_bytes())?;
    for ((path, mode), id) in files.iter().zip(ids.lines()) {
        entries.insert(rel(path), (mode.to_string(), id.to_string()));
    }
    Ok(entries)
}

/// `git hash-object --no-filters` with `how` and `stdin`.
fn hash(repo: &Path, how: &[&str], stdin: &[u8]) -> Result<String, Failure> {
    let mut args = vec!["hash-object", "--no-filters"];
    args.extend(how);
    let out = clm_git::run(
        repo,
        "git",
        &args,
        Options {
            stdin: Some(stdin),
            env: &[],
        },
    )?;
    Ok(String::from_utf8_lossy(&out).trim_end().to_string())
}

/// The mode git records for a regular file: `100755` when its owner may execute it, where
/// the platform has the bit, and `100644` otherwise.
fn mode(meta: &fs::Metadata) -> &'static str {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o100 != 0 {
            return "100755";
        }
    }
    #[cfg(not(unix))]
    let _ = meta;
    "100644"
}
