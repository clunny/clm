//! Where a land writes what it prints on stderr while it runs: its waiting lines, its warnings
//! and the output of the commands it runs. The process's own stderr, or the stream a caller of
//! `land_command` passes (`land/commands-in-crate`). A write that fails is no reason to end a
//! run, so its result is dropped (`land/shared-check-runner`).

use std::io::Write;

/// A land's stderr.
#[derive(Clone, Copy)]
pub(crate) struct Stderr<'a>(&'a dyn Fn(&[u8]));

impl<'a> Stderr<'a> {
    /// The process's own stderr.
    pub(crate) const PROCESS: Stderr<'static> = Stderr(&process);

    /// The stream `write` writes to.
    pub(crate) fn to(write: &'a dyn Fn(&[u8])) -> Self {
        Stderr(write)
    }

    /// Writes `line` and a line break.
    pub(crate) fn line(self, line: &str) {
        (self.0)(format!("{line}\n").as_bytes());
    }

    /// Writes `bytes` as they are.
    pub(crate) fn bytes(self, bytes: &[u8]) {
        (self.0)(bytes);
    }
}

fn process(bytes: &[u8]) {
    let _ = std::io::stderr().write_all(bytes);
}
