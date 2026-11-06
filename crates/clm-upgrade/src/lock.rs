//! `migrations.lock`: the hash of each shipped migration's directory (**The lock**).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Every file under `dir`, as its path relative to `dir`, with `/` separators.
fn files(dir: &Path, prefix: &str, out: &mut Vec<String>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
        if entry.file_type()?.is_dir() {
            files(&entry.path(), &format!("{name}/"), out)?;
        } else {
            out.push(name);
        }
    }
    Ok(())
}

/// The SHA-256, in hex, of the concatenation, over every file under `dir` in byte order of its
/// relative path, of the path, a NUL byte, the file's bytes and a NUL byte.
pub(crate) fn dir_hash(dir: &Path) -> std::io::Result<String> {
    let mut names = Vec::new();
    files(dir, "", &mut names)?;
    names.sort_unstable_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    let mut hasher = Sha256::new();
    for name in names {
        hasher.update(name.as_bytes());
        hasher.update([0]);
        hasher.update(fs::read(dir.join(&name))?);
        hasher.update([0]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

fn lock_path(crate_dir: &Path) -> PathBuf {
    crate_dir.join("migrations.lock")
}

fn migration_dir(crate_dir: &Path, id: &str) -> PathBuf {
    crate_dir.join("src/migrations").join(id)
}

/// The lock's lines as `(id, version, hash)`; none when there is no lock.
fn read_lock(crate_dir: &Path) -> Result<Vec<(String, String, String)>, String> {
    let path = lock_path(crate_dir);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let mut parts = line.split_whitespace();
            match (parts.next(), parts.next(), parts.next(), parts.next()) {
                (Some(id), Some(version), Some(hash), None) => {
                    Ok((id.to_string(), version.to_string(), hash.to_string()))
                }
                _ => Err(format!(
                    "{}: `{line}` is not `<id> <version> <hash>`",
                    path.display()
                )),
            }
        })
        .collect()
}

/// Checks the lock of the crate at `crate_dir` against the migrations `ids` it holds: every locked
/// migration must still exist and hash as locked.
pub fn check_lock(crate_dir: &Path, ids: &[&str]) -> Result<(), String> {
    for (id, version, hash) in read_lock(crate_dir)? {
        if !ids.contains(&id.as_str()) {
            return Err(format!(
                "migration `{id}`, shipped in {version}, is gone; a shipped migration never changes"
            ));
        }
        let actual = dir_hash(&migration_dir(crate_dir, &id))
            .map_err(|e| format!("migration `{id}`: {e}"))?;
        if actual != hash {
            return Err(format!(
                "migration `{id}`, shipped in {version}, was edited; a shipped migration never changes, so fix it with a new migration"
            ));
        }
    }
    Ok(())
}

/// Appends to the lock one line naming `version` for each of `ids` it doesn't list, leaving
/// every existing line as it is, and returns the IDs it locked.
pub fn append_lock(crate_dir: &Path, version: &str, ids: &[&str]) -> Result<Vec<String>, String> {
    let locked = read_lock(crate_dir)?;
    let mut lines = String::new();
    let mut added = Vec::new();
    for id in ids
        .iter()
        .filter(|id| !locked.iter().any(|(l, ..)| l == *id))
    {
        let hash = dir_hash(&migration_dir(crate_dir, id))
            .map_err(|e| format!("migration `{id}`: {e}"))?;
        lines.push_str(&format!("{id} {version} {hash}\n"));
        added.push(id.to_string());
    }
    if lines.is_empty() {
        return Ok(added);
    }
    let path = lock_path(crate_dir);
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if !existing.is_empty() && !existing.ends_with('\n') {
        lines.insert(0, '\n');
    }
    file.write_all(lines.as_bytes())
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(added)
}
