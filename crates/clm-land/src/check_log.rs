//! A land's `[land] check` output (`land/check-log`): stdout and stderr through one pipe, in
//! the order the check wrote them, copied to `clm land`'s stderr and to a log, with the last
//! lines kept in memory for the refusal.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, ExitStatus};

use crate::Error;

/// How many of the check's last lines a failing land repeats.
pub(crate) const TAIL: usize = 40;

/// What a logged run of the check came to.
pub(crate) struct Logged {
    pub(crate) status: ExitStatus,
    /// The last `TAIL` lines of its output.
    pub(crate) tail: Vec<String>,
    /// Whether the log was created, so holds the whole output.
    pub(crate) logged: bool,
}

/// Runs `command` with its stdout and stderr on one pipe, copying what arrives to stderr and
/// to `log`, created with its directories and truncated first. When the log can't be
/// created, or a write to it fails, warns naming it and copies to stderr alone. Reads until
/// the pipe closes, so a check must end what it starts.
pub(crate) fn run(mut command: Command, log: &Path) -> Result<Logged, Error> {
    let warn = |e: std::io::Error| {
        eprintln!(
            "clm land: warning: can't write the check's log at {}: {e}; its output goes to \
             stderr alone",
            log.display()
        );
    };
    let mut file = log
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| File::create(log))
        .map_err(warn)
        .ok();
    let (mut reader, writer) = std::io::pipe()?;
    command.stdout(writer.try_clone()?).stderr(writer);
    let mut child = command.spawn()?;
    // The command holds the pipe's writing ends; the pipe closes only once they are gone.
    drop(command);
    let mut tail = Tail::default();
    let mut buf = [0u8; 8192];
    let mut stderr = std::io::stderr();
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => {
                // Not left running in the land tree, under the lock.
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.into());
            }
        };
        let _ = stderr.write_all(&buf[..n]);
        if let Some(f) = &mut file
            && let Err(e) = f.write_all(&buf[..n])
        {
            // A log that misses output isn't named as holding it.
            warn(e);
            file = None;
        }
        tail.push(&buf[..n]);
    }
    let status = child.wait()?;
    Ok(Logged {
        status,
        tail: tail.finish(),
        logged: file.is_some(),
    })
}

/// The last `TAIL` lines of what passes, split on `\n`.
#[derive(Default)]
struct Tail {
    lines: VecDeque<String>,
    partial: Vec<u8>,
}

impl Tail {
    fn push(&mut self, bytes: &[u8]) {
        for &b in bytes {
            if b == b'\n' {
                let line = std::mem::take(&mut self.partial);
                self.line(line);
            } else {
                self.partial.push(b);
            }
        }
    }

    fn line(&mut self, line: Vec<u8>) {
        if self.lines.len() == TAIL {
            self.lines.pop_front();
        }
        self.lines
            .push_back(String::from_utf8_lossy(&line).into_owned());
    }

    /// The lines, a final unterminated one counting as one.
    fn finish(mut self) -> Vec<String> {
        if !self.partial.is_empty() {
            let line = std::mem::take(&mut self.partial);
            self.line(line);
        }
        self.lines.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_keeps_the_last_lines_across_chunks() {
        // req:land/check-log@153c9698
        let mut tail = Tail::default();
        let text: String = (1..=50).map(|i| format!("{i}\n")).collect();
        for chunk in text.as_bytes().chunks(3) {
            tail.push(chunk);
        }
        tail.push(b"last");
        let lines = tail.finish();
        assert_eq!(lines.len(), TAIL);
        assert_eq!(lines[0], "12");
        assert_eq!(lines[38], "50");
        assert_eq!(lines[39], "last");
    }
}
