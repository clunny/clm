//! One build's session: its worktree, its `claude` process group and its log.

use std::fs::{self, File};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use crate::UNATTENDED;

/// Why a session wasn't started; the message the runner prints.
pub(crate) struct NotStarted(pub(crate) String);

/// What a session is started with.
pub(crate) struct Launch<'a> {
    pub(crate) root: &'a Path,
    pub(crate) claude: &'a Path,
    pub(crate) mode: &'a str,
}

/// A running `claude` session, the leader of its own process group.
pub(crate) struct Session {
    child: Child,
    log: PathBuf,
}

impl Launch<'_> {
    /// Prepares the node's worktree and starts `claude` in it with `launcher`.
    pub(crate) fn start(&self, node: &str, launcher: &str) -> Result<Session, NotStarted> {
        let cwd = self.worktree(node)?;
        let logs = self.root.join(".clm/logs");
        let log = logs.join(format!("{node}.log"));
        let opened = fs::create_dir_all(&logs).and_then(|()| File::create(&log));
        let out = opened.map_err(|e| NotStarted(format!("{}: {e}", log.display())))?;
        let err = out
            .try_clone()
            .map_err(|e| NotStarted(format!("{}: {e}", log.display())))?;
        let child = Command::new(self.claude)
            .arg("-p")
            .arg(format!("{UNATTENDED}\n\n{launcher}"))
            .args(["--model", "sonnet"])
            .arg("--permission-mode")
            .arg(self.mode)
            .arg("--add-dir")
            .arg(self.root.join(".clm/prompts"))
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(out)
            .stderr(err)
            .process_group(0)
            .spawn()
            .map_err(|e| NotStarted(format!("{} failed: {e}", self.claude.display())))?;
        Ok(Session { child, log })
    }

    /// Adds `<main>/.claude/worktrees/agent-runner-<node>` detached at `origin/main`, first
    /// removing a clean leftover there, and returns the directory the session runs in.
    fn worktree(&self, node: &str) -> Result<PathBuf, NotStarted> {
        let git_failed = |e: clm_git::Error| NotStarted(format!("git failed: {e}"));
        let listed = clm_git::worktrees(self.root).map_err(git_failed)?;
        let main = &listed
            .first()
            .ok_or_else(|| NotStarted("git failed: no worktree listed".into()))?
            .path;
        let parent = main.join(".claude/worktrees");
        let path = parent.join(format!("agent-runner-{node}"));
        if listed.iter().any(|w| w.path == path) {
            if path.exists() {
                let dirty = clm_git::git(&path, &["status", "--porcelain"]).map_err(git_failed)?;
                if !dirty.is_empty() {
                    return Err(NotStarted(format!(
                        "kept {}: uncommitted changes",
                        path.display()
                    )));
                }
                let remove = ["worktree", "remove", "--force", path_str(&path)?];
                clm_git::git(self.root, &remove).map_err(git_failed)?;
            } else {
                clm_git::git(self.root, &["worktree", "prune"]).map_err(git_failed)?;
            }
        }
        fs::create_dir_all(&parent)
            .map_err(|e| NotStarted(format!("git failed: {}: {e}", parent.display())))?;
        let add = [
            "worktree",
            "add",
            "--detach",
            path_str(&path)?,
            "origin/main",
        ];
        clm_git::git(self.root, &add).map_err(git_failed)?;
        let prefix =
            clm_git::git(self.root, &["rev-parse", "--show-prefix"]).map_err(git_failed)?;
        Ok(path.join(prefix))
    }
}

fn path_str(path: &Path) -> Result<&str, NotStarted> {
    path.to_str()
        .ok_or_else(|| NotStarted(format!("git failed: not UTF-8: {}", path.display())))
}

impl Session {
    /// The session's exit status once it has exited, without waiting.
    pub(crate) fn exited(&mut self) -> Option<ExitStatus> {
        // An error here means the child can't be waited on: it is gone.
        self.child
            .try_wait()
            .unwrap_or_else(|_| Some(ExitStatus::from_raw(0)))
    }

    /// Sends the session's process group `signal`.
    pub(crate) fn signal(&self, signal: libc::c_int) {
        let group = self.child.id() as libc::pid_t;
        // SAFETY: kill(2) takes no pointers; a negative pid names the process group.
        unsafe {
            libc::kill(-group, signal);
        }
    }

    /// Waits until `deadline` for the session to exit, then kills its group and reaps it.
    pub(crate) fn end_by(&mut self, deadline: Instant) -> ExitStatus {
        while Instant::now() < deadline {
            if let Some(status) = self.exited() {
                return status;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        self.signal(libc::SIGKILL);
        self.child
            .wait()
            .unwrap_or_else(|_| ExitStatus::from_raw(0))
    }

    /// `<line>` of `finished <node>: <line>`: the log's last non-empty line, or the exit.
    pub(crate) fn last_line(&self, status: ExitStatus) -> String {
        let log = fs::read(&self.log).unwrap_or_default();
        let log = String::from_utf8_lossy(&log);
        match log.lines().map(str::trim).rfind(|l| !l.is_empty()) {
            Some(line) => line.to_string(),
            None => match (status.code(), status.signal()) {
                (Some(code), _) => format!("exit {code}"),
                (None, Some(signal)) => format!("exit signal {signal}"),
                (None, None) => "exit".to_string(),
            },
        }
    }
}
