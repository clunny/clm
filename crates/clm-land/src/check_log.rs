//! A land's `[land] check` output (`land/check-log`): stdout and stderr through one pipe, in
//! the order the check wrote them, copied to `clm land`'s stderr and to a log, with the last
//! lines kept in memory for the refusal, and the step the check had open as it ended.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, ExitStatus};

use crate::Error;

/// How many of the check's last lines a failing land repeats.
pub(crate) const TAIL: usize = 40;

/// A step the check marked with GitHub Actions' group lines (`land/check-log`): its name and
/// the first and last line numbers, counted from 1 in the whole output, of the lines it spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub name: String,
    pub first: usize,
    pub last: usize,
}

/// What a logged run of the check came to.
pub(crate) struct Logged {
    pub(crate) status: ExitStatus,
    /// The last `TAIL` lines of its output.
    pub(crate) tail: Vec<String>,
    /// Whether the log was created, so holds the whole output.
    pub(crate) logged: bool,
    /// The step open as the output ended, if any.
    pub(crate) step: Option<Step>,
}

/// Runs `command` with its stdout and stderr on one pipe, copying what arrives to stderr and
/// to `log`, created with its directories and truncated first. When the log can't be
/// created, or a write to it fails, warns naming it and copies to stderr alone. Reads until
/// the pipe closes, so a check must end what it starts.
pub(crate) fn run(mut command: Command, log: Option<&Path>) -> Result<Logged, Error> {
    let warn = |log: &Path, e: std::io::Error| {
        eprintln!(
            "clm land: warning: can't write the check's log at {}: {e}; its output goes to \
             stderr alone",
            log.display()
        );
    };
    let mut file = log.and_then(|log| {
        log.parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| File::create(log))
            .map_err(|e| warn(log, e))
            .ok()
    });
    let (mut reader, writer) = std::io::pipe()?;
    command.stdout(writer.try_clone()?).stderr(writer);
    let mut child = command.spawn()?;
    // The command holds the pipe's writing ends; the pipe closes only once they are gone.
    drop(command);
    let mut scan = Scan::default();
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
        if let (Some(f), Some(log)) = (&mut file, log)
            && let Err(e) = f.write_all(&buf[..n])
        {
            // A log that misses output isn't named as holding it.
            warn(log, e);
            file = None;
        }
        scan.push(&buf[..n]);
    }
    let status = child.wait()?;
    let (tail, step) = scan.finish();
    Ok(Logged {
        status,
        tail,
        logged: file.is_some(),
        step,
    })
}

/// What a check's output holds that a refusal needs: its last `TAIL` lines, split on `\n`,
/// and the step open at its end, found by numbering the lines and following the check's
/// group markers. Only the open step's name and first line are kept, so steps cost no memory
/// beyond the tail.
#[derive(Default)]
struct Scan {
    lines: VecDeque<String>,
    partial: Vec<u8>,
    /// Lines seen whole so far.
    count: usize,
    /// The step open now: its name and the line that started it.
    open: Option<(String, usize)>,
}

impl Scan {
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
        self.count += 1;
        let text = String::from_utf8_lossy(&line).into_owned();
        let marker = text.trim_end();
        if let Some(name) = marker.strip_prefix("::group::") {
            self.open = Some((name.trim().to_string(), self.count));
        } else if marker == "::endgroup::" {
            self.open = None;
        }
        if self.lines.len() == TAIL {
            self.lines.pop_front();
        }
        self.lines.push_back(text);
    }

    /// The tail and the open step, a final unterminated line counting as one.
    fn finish(mut self) -> (Vec<String>, Option<Step>) {
        if !self.partial.is_empty() {
            let line = std::mem::take(&mut self.partial);
            self.line(line);
        }
        let last = self.count;
        let step = self.open.map(|(name, first)| Step { name, first, last });
        (self.lines.into(), step)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(text: &str, chunk: usize) -> (Vec<String>, Option<Step>) {
        let mut scan = Scan::default();
        for chunk in text.as_bytes().chunks(chunk) {
            scan.push(chunk);
        }
        scan.finish()
    }

    #[test]
    fn tail_keeps_the_last_lines_across_chunks() {
        // req:land/check-log@6fefb1a1
        let mut text: String = (1..=50).map(|i| format!("{i}\n")).collect();
        text.push_str("last");
        let (lines, step) = scan(&text, 3);
        assert_eq!(lines.len(), TAIL);
        assert_eq!(lines[0], "12");
        assert_eq!(lines[38], "50");
        assert_eq!(lines[39], "last");
        assert_eq!(step, None);
    }

    #[test]
    fn the_open_step_spans_its_marker_to_the_last_line() {
        // req:land/check-log@6fefb1a1
        let text = "::group::unit\r\nx\n::endgroup::\n::group:: e2e \nboom\nlast";
        let step = scan(text, 2).1.unwrap();
        assert_eq!(
            step,
            Step {
                name: "e2e".into(),
                first: 4,
                last: 6
            }
        );
        // A step that ended isn't open, and neither is a marker in the middle of a line.
        assert_eq!(scan("::group::a\nx\n::endgroup::\t\nboom\n", 5).1, None);
        assert_eq!(scan("say ::group::a\n", 5).1, None);
    }
}
