//! The writer identity, `<person>/<clone>` (`ledger/writer-identity`).

use crate::slug;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::Error;

/// This clone's identity, `<person>/<clone>`, creating the clone part on first use. `env` is
/// the environment `config` reads, as for `clm_config::load`.
pub fn writer(root: &Path, env: &dyn Fn(&str) -> Option<String>) -> Result<String, Error> {
    let person = person(root, env)?;
    let file = writer_file(root)?;
    let clone = match read_clone(&file) {
        Some(clone) => clone,
        None => create_clone(&file)?,
    };
    Ok(format!("{person}/{clone}"))
}

/// `config`'s `writer`, or the repository's git `user.name` as a slug.
fn person(root: &Path, env: &dyn Fn(&str) -> Option<String>) -> Result<String, Error> {
    let settings = clm_config::load(Some(root), env)?;
    let person = match settings.writer.value {
        // A ref name part: only a slug, so it can't break or reshape `<person>/<clone>`.
        Some(person) if slug(&person) != person => {
            return Err(Error::Writer(format!(
                "`writer` is `{person}`, not a slug: use lowercase letters, digits and `-`, \
                 such as `{}`",
                slug(&person)
            )));
        }
        Some(person) => person,
        None => {
            let name = match clm_git::git(root, &["config", "user.name"]) {
                Ok(name) => name,
                // Exit 1: unset.
                Err(clm_git::Error::Failed { status, .. }) if status.code() == Some(1) => {
                    String::new()
                }
                Err(e) => return Err(e.into()),
            };
            slug(&name)
        }
    };
    if person.is_empty() {
        return Err(Error::Writer(
            "no writer: set `writer` (CLM_WRITER, or `writer = \"<name>\"` in config.toml), \
             since git's user.name gives an empty name"
                .into(),
        ));
    }
    Ok(person)
}

/// `clm/writer` in the common git directory, so every worktree of a clone shares it.
fn writer_file(root: &Path) -> Result<PathBuf, Error> {
    let common = clm_git::git(root, &["rev-parse", "--git-common-dir"])?;
    Ok(root.join(common).join("clm").join("writer"))
}

/// The clone part, when this clone has one yet.
pub(crate) fn clone_id(root: &Path) -> Result<Option<String>, Error> {
    Ok(read_clone(&writer_file(root)?))
}

fn read_clone(file: &Path) -> Option<String> {
    let text = std::fs::read_to_string(file).ok()?;
    let clone = text.trim();
    (!clone.is_empty()).then(|| clone.to_string())
}

/// Writes eight random hex digits to `file`, unless another session got there first, whose
/// digits then win.
fn create_clone(file: &Path) -> Result<String, Error> {
    let clone = crate::random_hex(4)?;
    std::fs::create_dir_all(file.parent().expect("a file in a directory"))?;
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file)
    {
        Ok(mut f) => {
            writeln!(f, "{clone}")?;
            Ok(clone)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // Spin briefly in case the winner hasn't written its digits yet.
            for _ in 0..50 {
                if let Some(theirs) = read_clone(file) {
                    return Ok(theirs);
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            // Left empty by a session that died before writing: take it over.
            std::fs::write(file, format!("{clone}\n"))?;
            Ok(clone)
        }
        Err(e) => Err(e.into()),
    }
}
