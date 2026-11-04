//! The runner's loop: look for work with `clm next`, prepare it with `clm run`, start a
//! session for each node prepared, and finish each with `clm run --finish` when it exits.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::Options;
use crate::session::{Launch, Session};

/// How often a wait checks for an exited session or a signal.
const POLL: Duration = Duration::from_millis(50);
/// How long a stopping runner waits for its sessions before killing their groups.
const GRACE: Duration = Duration::from_secs(30);

pub(crate) fn run(options: &Options) -> u8 {
    let Some(mode) = options.permission_mode.as_deref() else {
        eprintln!("clm runner: --permission-mode is required");
        return 1;
    };
    let Some(claude) = on_path(&options.claude) else {
        eprintln!(
            "clm runner: no `{}` on PATH",
            options.claude.to_string_lossy()
        );
        return 1;
    };
    let root = std::path::absolute(&options.root).unwrap_or_else(|_| options.root.clone());
    if let Err(why) = crate::guard::check(&claude, &root) {
        eprintln!("clm runner: {why}");
        return 1;
    }
    let stop = Arc::new(AtomicBool::new(false));
    for signal in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
        // A second signal, while stopping, ends the runner at once with exit 1.
        let escalate =
            signal_hook::flag::register_conditional_shutdown(signal, 1, Arc::clone(&stop));
        let registered =
            escalate.and_then(|_| signal_hook::flag::register(signal, Arc::clone(&stop)));
        if let Err(e) = registered {
            eprintln!("clm runner: catching signals: {e}");
            return 1;
        }
    }
    let mut runner = Runner {
        options,
        launch: Launch {
            root: &root,
            claude: &claude,
            mode,
        },
        stop,
        sessions: BTreeMap::new(),
    };
    runner.run()
}

/// `name` itself when it holds a `/`, otherwise the first executable file of that name in a
/// directory of `PATH`.
fn on_path(name: &OsStr) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let executable = |p: &Path| {
        p.metadata()
            .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    };
    if name.to_string_lossy().contains('/') {
        let path = PathBuf::from(name);
        return executable(&path).then_some(path);
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|p| executable(p))
}

struct Runner<'a> {
    options: &'a Options,
    launch: Launch<'a>,
    stop: Arc<AtomicBool>,
    sessions: BTreeMap<String, Session>,
}

/// A `clm` subprocess's outcome.
struct Ran {
    code: i32,
    stdout: String,
    stderr: String,
}

/// `clm run`'s stderr as one line: each line's leading `clm run: ` removed, joined by `; `.
fn reason(stderr: &str) -> String {
    stderr
        .lines()
        .map(|l| l.strip_prefix("clm run: ").unwrap_or(l).trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("; ")
}

impl Runner<'_> {
    fn run(&mut self) -> u8 {
        let mut first = true;
        loop {
            let next = self.clm(&["next"]);
            let started = match next.code {
                0 => self.start(&next.stdout),
                code if first => {
                    eprint!("{}", next.stderr);
                    return if code == 1 { 1 } else { 2 };
                }
                _ => {
                    eprintln!("clm next failed: {}", next.stderr.trim());
                    0
                }
            };
            first = false;
            if self.options.once && started == 0 && self.sessions.is_empty() {
                return 0;
            }
            if !self.wait() {
                self.shut_down();
                return 0;
            }
        }
    }

    fn stopping(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    /// Runs `clm <args> --expiry <h> --root <root>`.
    fn clm(&self, args: &[&str]) -> Ran {
        let out = Command::new(&self.options.clm)
            .args(args)
            .arg("--expiry")
            .arg(self.options.expiry.to_string())
            .arg("--root")
            .arg(self.launch.root)
            .stdin(Stdio::null())
            // Out of the runner's group, so a terminal's interrupt reaches only the runner,
            // which lets the command finish and then stops.
            .process_group(0)
            .output();
        match out {
            Ok(out) => Ran {
                // Killed by a signal: no code, so a failure.
                code: out.status.code().unwrap_or(2),
                stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            },
            Err(e) => Ran {
                code: 2,
                stdout: String::new(),
                stderr: format!("{}: {e}\n", self.options.clm.display()),
            },
        }
    }

    /// Prepares and starts the nodes `clm next` printed, returning how many it started.
    fn start(&mut self, nodes: &str) -> usize {
        let mut started = 0;
        for node in nodes.lines().map(str::trim).filter(|n| !n.is_empty()) {
            if self.stopping() {
                break;
            }
            if self.sessions.contains_key(node) {
                continue;
            }
            let ran = self.clm(&["run", node]);
            match ran.code {
                0 => {
                    let launcher = ran.stdout.trim_end_matches('\n');
                    match self.launch.start(node, launcher) {
                        Ok(session) => {
                            self.sessions.insert(node.to_string(), session);
                            started += 1;
                        }
                        Err(not) => {
                            println!("{}", not.0);
                            self.finish(node);
                        }
                    }
                }
                // No free place: nothing more until a session exits or the interval passes.
                3 => break,
                1 => println!("skipped {node}: {}", reason(&ran.stderr)),
                _ => {
                    println!("git failed: {}", reason(&ran.stderr));
                    break;
                }
            }
        }
        started
    }

    /// Waits until a session exits (finishing each that has), the interval passes, or a
    /// signal arrives. False when a signal arrived.
    fn wait(&mut self) -> bool {
        let deadline = Instant::now() + self.options.interval;
        loop {
            if self.stopping() {
                return false;
            }
            let exited: Vec<(String, ExitStatus)> = self
                .sessions
                .iter_mut()
                .filter_map(|(node, s)| s.exited().map(|status| (node.clone(), status)))
                .collect();
            for (node, status) in &exited {
                if let Some(session) = self.sessions.remove(node) {
                    self.ended(node, &session, *status);
                }
            }
            if !exited.is_empty() || Instant::now() >= deadline {
                return true;
            }
            std::thread::sleep(POLL);
        }
    }

    /// Runs `clm run --finish <node>`, passing its output through.
    fn finish(&self, node: &str) {
        let ran = self.clm(&["run", "--finish", node]);
        print!("{}", ran.stdout);
        if ran.code == 2 {
            println!("git failed: {}", reason(&ran.stderr));
        } else {
            eprint!("{}", ran.stderr);
        }
    }

    fn ended(&self, node: &str, session: &Session, status: ExitStatus) {
        self.finish(node);
        println!("finished {node}: {}", session.last_line(status));
    }

    /// Terminates every session's process group, kills those still running after the grace
    /// period, and finishes each node.
    fn shut_down(&mut self) {
        for session in self.sessions.values() {
            session.signal(libc::SIGTERM);
        }
        let deadline = Instant::now() + GRACE;
        let sessions = std::mem::take(&mut self.sessions);
        for (node, mut session) in sessions {
            let status = session.end_by(deadline);
            self.ended(&node, &session, status);
        }
    }
}
