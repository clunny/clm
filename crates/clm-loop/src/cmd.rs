//! The subprocesses the loop drives through the other commands' Contracts.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clm_git::{Failure, now};

use crate::Options;

/// A `clm` subprocess's outcome.
pub(crate) struct Ran {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Ran {
    /// `stderr` as one line, for an error message.
    pub fn reason(&self) -> String {
        self.stderr
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// Runs `clm <args>` in the loop's repository with nothing on standard input.
pub(crate) fn clm(options: &Options, args: &[&str]) -> Result<Ran, Failure> {
    clm_in(options, &options.root, args)
}

/// Runs `clm <args>` in `dir` with nothing on standard input.
pub(crate) fn clm_in(
    options: &Options,
    dir: &std::path::Path,
    args: &[&str],
) -> Result<Ran, Failure> {
    let program = resolve(&options.clm);
    let out = Command::new(&program)
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| Failure::Git(format!("{}: {e}", program.display())))?;
    Ok(Ran {
        // Killed by a signal: no code, so a failure.
        code: out.status.code().unwrap_or(2),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    })
}

/// The `clm` to run now for `clm`, the path `clm build` started from. On Linux, once that file
/// is replaced or deleted, `current_exe` gives it with ` (deleted)` appended: run the file now at
/// the path without the suffix, or else the running image. Any other path runs as given.
#[cfg(target_os = "linux")]
fn resolve(clm: &Path) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    if clm.exists() {
        return clm.to_path_buf();
    }
    let Some(path) = clm.as_os_str().as_bytes().strip_suffix(b" (deleted)") else {
        return clm.to_path_buf();
    };
    let now = PathBuf::from(std::ffi::OsStr::from_bytes(path));
    if now.exists() {
        now
    } else {
        PathBuf::from("/proc/self/exe")
    }
}

/// Elsewhere no replaced binary's path carries a suffix: every path runs as given.
#[cfg(not(target_os = "linux"))]
fn resolve(clm: &Path) -> PathBuf {
    clm.to_path_buf()
}

/// `clm <args>` that must exit 0: its stdout, or a failure carrying its reason.
pub(crate) fn clm_ok(options: &Options, args: &[&str]) -> Result<String, Failure> {
    let ran = clm(options, args)?;
    match ran.code {
        0 => Ok(ran.stdout),
        1 => Err(Failure::Refused(format!(
            "clm {}: {}",
            args.join(" "),
            ran.reason()
        ))),
        _ => Err(Failure::Git(format!(
            "clm {}: {}",
            args.join(" "),
            ran.reason()
        ))),
    }
}

/// A `say` action naming a file that holds `text`.
pub(crate) fn say(options: &Options, text: &str) -> Result<String, Failure> {
    let root = std::path::absolute(&options.root)
        .map_err(|e| Failure::Git(format!("{}: {e}", options.root.display())))?;
    let dir = root.join(".clm/prompts/loop");
    fs::create_dir_all(&dir).map_err(|e| Failure::Git(format!("{}: {e}", dir.display())))?;
    let path = dir.join(format!("say-{}-{}.md", now(), std::process::id()));
    let path = unique(path);
    fs::write(&path, format!("{text}\n"))
        .map_err(|e| Failure::Git(format!("{}: {e}", path.display())))?;
    Ok(format!("say {}", path.display()))
}

/// `path`, or the first of `path` with `-2`, `-3`, ... before its extension that doesn't exist.
fn unique(path: std::path::PathBuf) -> std::path::PathBuf {
    if !path.exists() {
        return path;
    }
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    (2..)
        .map(|n| path.with_file_name(format!("{stem}-{n}.md")))
        .find(|p| !p.exists())
        .unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deleted(path: &Path) -> PathBuf {
        PathBuf::from(format!("{} (deleted)", path.display()))
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn the_clm_it_started_from_was_deleted() {
        // req:orchestrator.loop/runs-the-clm-now-at-its-path@1471695a
        let tmp = tempfile::tempdir().unwrap();
        let gone = deleted(&tmp.path().join("clm"));
        assert_eq!(resolve(&gone), PathBuf::from("/proc/self/exe"));
    }

    #[test]
    fn a_path_that_names_a_file() {
        // req:orchestrator.loop/runs-the-clm-now-at-its-path@1471695a
        let tmp = tempfile::tempdir().unwrap();
        let plain = tmp.path().join("clm");
        fs::write(&plain, "").unwrap();
        assert_eq!(resolve(&plain), plain);
        // A file whose name ends in the suffix is run as named, not the one without it.
        let suffixed = deleted(&plain);
        fs::write(&suffixed, "").unwrap();
        assert_eq!(resolve(&suffixed), suffixed);
        // A path that names no file and has no suffix is run as given.
        let missing = tmp.path().join("missing");
        assert_eq!(resolve(&missing), missing);
    }
}
