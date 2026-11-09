//! A land's `[land] check` output (`land/check-log`): stdout and stderr through one pipe, in
//! the order the check wrote them, copied to `clm land`'s stderr and to a log, with the last
//! lines kept in memory for the refusal, and the step the check had open as it ended.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, ExitStatus};
use std::time::Instant;

use clm_ledger::TimedStep;

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
    /// The command's wall-clock seconds, from its spawn to its exit (`land/step-times`).
    pub(crate) seconds: u64,
    /// The steps the check marked, in order, with their seconds and selecting paths.
    pub(crate) steps: Vec<TimedStep>,
}

/// Runs `command` with its stdout and stderr on one pipe, copying what arrives to stderr and
/// to `log`, created with its directories and truncated first. When the log can't be
/// created, or a write to it fails, warns naming it and copies to stderr alone. Reads until
/// the pipe closes, so a check must end what it starts.
pub(crate) fn run(mut command: Command, log: Option<&Path>) -> Result<Logged, Error> {
    let warn = |log: &Path, e: std::io::Error| {
        crate::lock::tell(&format!(
            "clm land: warning: can't write the check's log at {}: {e}; its output goes to \
             stderr alone",
            log.display()
        ));
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
    let started = Instant::now();
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
        scan.push(&buf[..n], Instant::now());
    }
    let closed = Instant::now();
    let status = child.wait()?;
    let seconds = secs(started, Instant::now());
    let (tail, step, steps) = scan.finish(closed);
    Ok(Logged {
        status,
        tail,
        logged: file.is_some(),
        step,
        seconds,
        steps,
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
    open: Option<Open>,
    /// The steps that have ended.
    done: Vec<TimedStep>,
}

/// The step open now: its name, the line that started it, when that line was read and the
/// paths its `::clm-selected::` lines named (`land/step-times`).
struct Open {
    name: String,
    first: usize,
    at: Instant,
    selected_by: Vec<String>,
}

/// Whole seconds from `from` to `to`, rounded down.
fn secs(from: Instant, to: Instant) -> u64 {
    to.saturating_duration_since(from).as_secs()
}

impl Scan {
    /// Takes `bytes`, read at `at`: the lines they complete are read then.
    fn push(&mut self, bytes: &[u8], at: Instant) {
        for &b in bytes {
            if b == b'\n' {
                let line = std::mem::take(&mut self.partial);
                self.line(line, at);
            } else {
                self.partial.push(b);
            }
        }
    }

    /// Ends the open step, if any, at `at`.
    fn end(&mut self, at: Instant) {
        if let Some(open) = self.open.take() {
            self.done.push(TimedStep {
                name: open.name,
                seconds: secs(open.at, at),
                selected_by: open.selected_by,
            });
        }
    }

    fn line(&mut self, line: Vec<u8>, at: Instant) {
        self.count += 1;
        let text = String::from_utf8_lossy(&line).into_owned();
        let marker = text.trim_end();
        if let Some(name) = marker.strip_prefix("::group::") {
            self.end(at);
            self.open = Some(Open {
                name: name.trim().to_string(),
                first: self.count,
                at,
                selected_by: Vec::new(),
            });
        } else if marker == "::endgroup::" {
            self.end(at);
        } else if let Some(path) = marker.strip_prefix("::clm-selected::")
            && let Some(open) = &mut self.open
            && !path.is_empty()
            && !open.selected_by.iter().any(|p| p == path)
        {
            open.selected_by.push(path.to_string());
        }
        if self.lines.len() == TAIL {
            self.lines.pop_front();
        }
        self.lines.push_back(text);
    }

    /// The tail, the open step and every step with its time, a final unterminated line
    /// counting as one; the pipe closed at `closed`, which ends the open step's time.
    fn finish(mut self, closed: Instant) -> (Vec<String>, Option<Step>, Vec<TimedStep>) {
        if !self.partial.is_empty() {
            let line = std::mem::take(&mut self.partial);
            self.line(line, closed);
        }
        let last = self.count;
        let step = self.open.as_ref().map(|o| Step {
            name: o.name.clone(),
            first: o.first,
            last,
        });
        self.end(closed);
        (self.lines.into(), step, self.done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(text: &str, chunk: usize) -> (Vec<String>, Option<Step>) {
        let mut scan = Scan::default();
        let now = Instant::now();
        for chunk in text.as_bytes().chunks(chunk) {
            scan.push(chunk, now);
        }
        let (tail, step, _) = scan.finish(now);
        (tail, step)
    }

    #[test]
    fn steps_take_their_seconds_between_their_lines() {
        // req:land/step-times@d247dda8
        let t0 = Instant::now();
        let at = |s: u64| t0 + std::time::Duration::from_millis(s);
        let mut scan = Scan::default();
        scan.push(
            b"::clm-selected::stray
::group::a
",
            at(0),
        );
        scan.push(
            b"::clm-selected::p
::clm-selected::q 
",
            at(500),
        );
        scan.push(
            b"::clm-selected::p
::endgroup::
",
            at(2900),
        );
        scan.push(
            b"::group::b
::group::a
last",
            at(3000),
        );
        let (_, _, steps) = scan.finish(at(5999));
        let got: Vec<_> = steps
            .iter()
            .map(|s| (s.name.as_str(), s.seconds, s.selected_by.clone()))
            .collect();
        assert_eq!(
            got,
            [
                ("a", 2, vec!["p".to_string(), "q".to_string()]),
                ("b", 0, vec![]),
                ("a", 2, vec![]),
            ]
        );
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
