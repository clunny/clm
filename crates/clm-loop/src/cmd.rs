//! The subprocesses the loop drives through the other commands' Contracts.

use std::fs;
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
pub(crate) fn clm_in(options: &Options, dir: &std::path::Path, args: &[&str]) -> Result<Ran, Failure> {
    let out = Command::new(&options.clm)
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| Failure::Git(format!("{}: {e}", options.clm.display())))?;
    Ok(Ran {
        // Killed by a signal: no code, so a failure.
        code: out.status.code().unwrap_or(2),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    })
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
