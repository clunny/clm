//! Where a land writes what it prints on stderr while it runs: its waiting lines, its warnings
//! and the output of the commands it runs. The process's own stderr, or the stream a caller of
//! `land_command` passes (`land/commands-in-crate`). A write that fails is no reason to end a
//! run, so its result is dropped (`land/shared-check-runner`).

use std::io::{self, Read, Write};
use std::process::{Command, ExitStatus, Stdio};

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

/// Runs `command`, copying its stderr to `err` as it arrives, and its stdout too, through the
/// same pipe in the order written, when `stdout_too`; otherwise its stdout is returned. Reads
/// until the pipe closes, so the command must end what it starts.
pub(crate) fn run(
    mut command: Command,
    err: Stderr,
    stdout_too: bool,
) -> io::Result<(ExitStatus, Vec<u8>)> {
    let (mut reader, writer) = io::pipe()?;
    match stdout_too {
        true => command.stdout(writer.try_clone()?),
        false => command.stdout(Stdio::piped()),
    };
    command.stderr(writer);
    let mut child = command.spawn()?;
    // The command holds the pipe's writing ends; the pipe closes only once they are gone.
    drop(command);
    let stdout = child.stdout.take().map(|mut s| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            s.read_to_end(&mut bytes).map(|_| bytes)
        })
    });
    let mut buf = [0u8; 8192];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => err.bytes(&buf[..n]),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e);
            }
        }
    }
    let status = child.wait()?;
    let stdout = match stdout {
        Some(reading) => reading
            .join()
            .map_err(|_| io::Error::other("reading the command's stdout panicked"))??,
        None => Vec::new(),
    };
    Ok((status, stdout))
}

fn process(bytes: &[u8]) {
    let _ = std::io::stderr().write_all(bytes);
}
