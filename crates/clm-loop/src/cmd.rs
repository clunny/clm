//! The subprocesses the loop drives through the other commands' Contracts.

use std::process::{Command, Stdio};

use clm_git::Failure;

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
    let out = Command::new(&options.clm)
        .args(args)
        .current_dir(&options.root)
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
