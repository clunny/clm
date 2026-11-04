//! Dead-code and unused-dependency detectors the repository names (see specs/check/SPEC.md).

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use clm_model::Violation;

/// A command whose non-zero exit is a violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detector {
    pub name: String,
    pub command: Vec<String>,
    /// How to install the tool, shown when the command can't start.
    pub hint: Option<String>,
    /// Seconds it may run before it is stopped (`detector-timeouts`).
    pub timeout: u64,
}

/// Runs each detector in order from `root`, without a shell, copying its output to stderr
/// as it arrives.
pub fn detectors(root: &Path, list: &[Detector]) -> Vec<Violation> {
    list.iter().filter_map(|d| run_one(root, d)).collect()
}

fn run_one(root: &Path, d: &Detector) -> Option<Violation> {
    let message = match spawn(root, &d.command) {
        Err(e) => {
            let hint = d
                .hint
                .as_ref()
                .map(|h| format!("; install with: {h}"))
                .unwrap_or_default();
            format!("{} could not run: {e}{hint}", d.name)
        }
        Ok(mut child) => {
            let copies = [
                child.stdout.take().map(copy_to_stderr),
                child.stderr.take().map(copy_to_stderr),
            ];
            let copies: Vec<_> = copies.into_iter().flatten().collect();
            let finished = wait_within(&mut child, &copies, Duration::from_secs(d.timeout));
            for copy in copies {
                let _ = copy.join();
            }
            match finished {
                Err(e) => format!("{} could not run: {e}", d.name),
                Ok(None) => format!("{} timed out after {}s", d.name, d.timeout),
                Ok(Some(status)) => match status.code() {
                    Some(0) => return None,
                    Some(code) => format!("{} failed (exit {code})", d.name),
                    None => format!("{} failed (killed by a signal)", d.name),
                },
            }
        }
    };
    Some(Violation {
        path: PathBuf::from("clm.toml"),
        rule: "check/detectors",
        message,
    })
}

/// Starts `command` in `root` with its output piped, in a process group of its own on Unix.
fn spawn(root: &Path, command: &[String]) -> std::io::Result<Child> {
    let Some((program, args)) = command.split_first() else {
        return Err(std::io::Error::other("empty command"));
    };
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
    cmd.spawn()
}

/// Copies `stream` to our stderr line by line, on a thread of its own.
fn copy_to_stderr(stream: impl Read + Send + 'static) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stream);
        let mut line = Vec::new();
        while matches!(reader.read_until(b'\n', &mut line), Ok(n) if n > 0) {
            let _ = std::io::stderr().lock().write_all(&line);
            line.clear();
        }
    })
}

/// Waits for `child` to exit and its output to be copied until `limit` has passed, then
/// kills it, with every process in its group on Unix. `None` when it was still running.
/// A detector that exited but left a process holding its output is reported by its exit
/// status, once that process is stopped.
fn wait_within(
    child: &mut Child,
    copies: &[std::thread::JoinHandle<()>],
    limit: Duration,
) -> std::io::Result<Option<std::process::ExitStatus>> {
    let deadline = Instant::now() + limit;
    let mut status = None;
    loop {
        if status.is_none() {
            status = child.try_wait().inspect_err(|_| kill(child))?;
        }
        if status.is_some() && copies.iter().all(|c| c.is_finished()) {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            kill(child);
            if status.is_none() {
                child.wait()?;
            }
            return Ok(status);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(unix)]
fn kill(child: &mut Child) {
    // The child leads its own group (`process_group(0)`), so its ID is the group's.
    if let Ok(pgid) = libc::pid_t::try_from(child.id()) {
        // SAFETY: `killpg` takes plain integers and touches no memory of ours.
        unsafe {
            libc::killpg(pgid, libc::SIGKILL);
        }
    }
}

#[cfg(not(unix))]
fn kill(child: &mut Child) {
    let _ = child.kill();
}
