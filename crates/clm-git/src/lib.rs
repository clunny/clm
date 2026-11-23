//! The one place CLM runs `git` and `gh` (specs/vcs/SPEC.md).

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Mutex, OnceLock, mpsc};
use std::time::{Duration, Instant};

/// What `run` feeds the command besides its arguments.
#[derive(Default)]
pub struct Options<'a> {
    pub stdin: Option<&'a [u8]>,
    pub env: &'a [(&'a str, &'a OsStr)],
}

#[derive(Debug)]
pub enum Error {
    Failed {
        command: String,
        status: ExitStatus,
        stderr: String,
    },
    Spawn {
        command: String,
        source: io::Error,
    },
    TimedOut {
        command: String,
        limit: Duration,
    },
    Unreadable {
        command: String,
        why: String,
    },
    StillOnDisk {
        path: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Failed {
                command, stderr, ..
            } if !stderr.is_empty() => {
                write!(f, "{command}: {stderr}")
            }
            Error::Failed {
                command, status, ..
            } => write!(f, "{command}: {status}"),
            Error::Spawn { command, source } => write!(f, "{command}: {source}"),
            Error::TimedOut { command, limit } => {
                write!(f, "{command}: timed out after {limit:?}")
            }
            Error::Unreadable { command, why } => write!(f, "{command}: {why}"),
            Error::StillOnDisk { path } => write!(f, "{path}: still on disk"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Spawn { source, .. } => Some(source),
            Error::Failed { .. }
            | Error::TimedOut { .. }
            | Error::Unreadable { .. }
            | Error::StillOnDisk { .. } => None,
        }
    }
}

/// Runs `program` (`git`, `gh`, found on `PATH`) in `root` and returns its stdout
/// untrimmed, waiting however long it takes.
pub fn run(root: &Path, program: &str, args: &[&str], options: Options) -> Result<Vec<u8>, Error> {
    let unset = follow_clone(root, program, &options);
    run_bounded(root, program, args, options, None, &unset)
}

/// As [`run`], but a command still running once `limit` has passed is killed, reaped and
/// reported as [`Error::TimedOut`]. A zero `limit` starts nothing.
pub fn run_within(
    root: &Path,
    program: &str,
    args: &[&str],
    options: Options,
    limit: Duration,
) -> Result<Vec<u8>, Error> {
    let unset = follow_clone(root, program, &options);
    run_bounded(root, program, args, options, Some(limit), &unset)
}

fn run_bounded(
    root: &Path,
    program: &str,
    args: &[&str],
    options: Options,
    limit: Option<Duration>,
    unset: &[String],
) -> Result<Vec<u8>, Error> {
    let command = std::iter::once(program)
        .chain(args.iter().copied())
        .collect::<Vec<_>>()
        .join(" ");
    if let Some(limit @ Duration::ZERO) = limit {
        return Err(Error::TimedOut { command, limit });
    }
    let started = Instant::now();
    let spawn = |source| Error::Spawn {
        command: command.clone(),
        source,
    };
    let mut process = Command::new(program);
    process
        .args(args)
        .current_dir(root)
        .envs(options.env.iter().copied());
    for name in unset {
        process.env_remove(name);
    }
    let mut child = process
        .stdin(if options.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(spawn)?;
    if let (Some(input), Some(mut pipe)) = (options.stdin, child.stdin.take()) {
        // A writer thread, so a command that fills its stdout before reading all its stdin
        // can't deadlock us. A command that exits early closes the pipe; its exit status
        // then tells the story, so the write error is dropped. The thread owns its bytes
        // and is detached, since it may outlive a call that times out.
        let input = input.to_vec();
        std::thread::spawn(move || {
            let _ = pipe.write_all(&input);
        });
    }
    // Detached readers: a process the command started may hold the pipes open after the
    // command is killed, and the readers end only when it closes them.
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let status = match limit {
        None => child.wait().map_err(spawn)?,
        Some(limit) => loop {
            // The clock is read before the poll, so a command that exited by then is
            // never reported as timed out.
            let elapsed = started.elapsed();
            if let Some(status) = child.try_wait().map_err(spawn)? {
                break status;
            }
            if elapsed >= limit {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Error::TimedOut { command, limit });
            }
            std::thread::sleep((limit - elapsed).min(Duration::from_millis(5)));
        },
    };
    // A command that exited in time may have left a process holding its pipes: its own
    // bytes are in the pipes' buffers by now, so the streams get a moment to close and the
    // command's outcome is returned with what they sent by then.
    let until = limit.map(|_| Instant::now() + STREAMS_AFTER_EXIT);
    let stdout = collect(&stdout, until).map_err(spawn)?;
    let stderr = collect(&stderr, until).map_err(spawn)?;
    if status.success() {
        Ok(stdout)
    } else {
        Err(Error::Failed {
            command,
            status,
            stderr: String::from_utf8_lossy(&stderr).trim().to_string(),
        })
    }
}

/// How long a bounded run waits for the streams of a command that has exited to close.
const STREAMS_AFTER_EXIT: Duration = Duration::from_millis(100);

type Reader = mpsc::Receiver<io::Result<Vec<u8>>>;

/// Reads `pipe` on a detached thread, which sends each chunk as it reads it and ends,
/// dropping the sender, at the stream's end or after a read error, which it sends.
fn drain(pipe: Option<impl Read + Send + 'static>) -> Reader {
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        let Some(mut pipe) = pipe else {
            return;
        };
        // Once the call has returned, the bytes are read and dropped until the stream ends:
        // closing the read end early would kill a process still writing to it.
        let mut buf = vec![0; 64 * 1024];
        loop {
            match pipe.read(&mut buf) {
                Ok(0) => return,
                Ok(n) => {
                    let _ = send.send(Ok(buf[..n].to_vec()));
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => {
                    let _ = send.send(Err(e));
                    return;
                }
            }
        }
    });
    receive
}

/// What `reader` sent until its stream ended, or, with `until`, until then at the latest.
fn collect(reader: &Reader, until: Option<Instant>) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        let chunk = match until {
            None => reader.recv().ok(),
            Some(until) => reader
                .recv_timeout(until.saturating_duration_since(Instant::now()))
                .ok(),
        };
        match chunk {
            Some(chunk) => out.extend(chunk?),
            None => return Ok(out),
        }
    }
}

/// `git <args>` in `root`, its stdout with trailing whitespace trimmed.
pub fn git(root: &Path, args: &[&str]) -> Result<String, Error> {
    run(root, "git", args, Options::default()).map(trimmed)
}

/// `git <args>` in `dir` as though the environment named no repository: every variable
/// `git rev-parse --local-env-vars` lists is removed from the command's environment.
pub fn git_isolated(dir: &Path, args: &[&str]) -> Result<String, Error> {
    let unset = local_env_vars(dir)?;
    run_bounded(dir, "git", args, Options::default(), None, &unset).map(trimmed)
}

/// The variables `git rev-parse --local-env-vars` prints, which name a repository.
fn local_env_vars(dir: &Path) -> Result<Vec<String>, Error> {
    static LISTED: OnceLock<Vec<String>> = OnceLock::new();
    if let Some(listed) = LISTED.get() {
        return Ok(listed.clone());
    }
    let out = run_bounded(
        dir,
        "git",
        &["rev-parse", "--local-env-vars"],
        Options::default(),
        None,
        &[],
    )?;
    let listed: Vec<String> = String::from_utf8_lossy(&out)
        .split_whitespace()
        .map(str::to_string)
        .collect();
    Ok(LISTED.get_or_init(|| listed).clone())
}

/// The variables to remove from a `git` command run in `root` (`git-follows-its-clone`):
/// all of git's repository variables when `root` lies outside the work tree the process's
/// environment names, less any the caller sets itself; none otherwise, or for another program.
fn follow_clone(root: &Path, program: &str, options: &Options) -> Vec<String> {
    if program != "git" {
        return Vec::new();
    }
    let Some(work_tree) = environment_work_tree() else {
        return Vec::new();
    };
    if !lies_outside(root, &work_tree) {
        return Vec::new();
    }
    let Ok(mut unset) = local_env_vars(root) else {
        return Vec::new();
    };
    unset.retain(|name| !options.env.iter().any(|(set, _)| set == name));
    unset
}

/// The work tree `GIT_DIR` and `GIT_WORK_TREE` name, found with the environment as the
/// process has it and kept while neither value changes. `None` when neither is set or git
/// names no work tree.
fn environment_work_tree() -> Option<PathBuf> {
    type Key = (Option<OsString>, Option<OsString>);
    static FOUND: Mutex<Option<(Key, Option<PathBuf>)>> = Mutex::new(None);
    let key = (
        std::env::var_os("GIT_DIR"),
        std::env::var_os("GIT_WORK_TREE"),
    );
    if key.0.is_none() && key.1.is_none() {
        return None;
    }
    let mut found = FOUND.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((was, tree)) = found.as_ref()
        && *was == key
    {
        return tree.clone();
    }
    let tree = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim_end().to_string())
        .filter(|top| !top.is_empty())
        .map(|top| std::fs::canonicalize(&top).unwrap_or_else(|_| PathBuf::from(top)));
    *found = Some((key, tree.clone()));
    tree
}

/// Whether `root` is neither `work_tree` nor under it, or has a `.git` entry itself or in a
/// directory between it and `work_tree`: a linked worktree or another repository.
fn lies_outside(root: &Path, work_tree: &Path) -> bool {
    let Ok(root) = std::fs::canonicalize(root) else {
        return false;
    };
    if root == work_tree {
        return false;
    }
    if !root.starts_with(work_tree) {
        return true;
    }
    root.ancestors()
        .take_while(|dir| *dir != work_tree)
        .any(|dir| dir.join(".git").exists())
}

/// The clone's common git directory, absolute: the main checkout's `.git` from any worktree.
pub fn common_dir(root: &Path) -> Result<PathBuf, Error> {
    git(
        root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .map(PathBuf::from)
}

/// The blobs of `commit` whose path `keep` accepts, as stored, read from the local objects
/// with one `ls-tree` and one `cat-file --batch`. Gitlinks, symbolic links and paths that
/// aren't UTF-8 are skipped.
pub fn tree_blobs(
    root: &Path,
    commit: &str,
    keep: &dyn Fn(&str) -> bool,
) -> Result<Vec<(String, Vec<u8>)>, Error> {
    let env: &[(&str, &OsStr)] = &[("GIT_NO_LAZY_FETCH", OsStr::new("1"))];
    let listing = run(
        root,
        "git",
        &["ls-tree", "-r", "-z", "--full-tree", commit],
        Options { stdin: None, env },
    )?;
    let mut wanted: Vec<(String, String)> = Vec::new();
    for entry in listing.split(|b| *b == 0).filter(|e| !e.is_empty()) {
        // "<mode> SP <type> SP <oid> TAB <path>"
        let Some(tab) = entry.iter().position(|b| *b == b'\t') else {
            continue;
        };
        let (Ok(meta), Ok(path)) = (
            std::str::from_utf8(&entry[..tab]),
            std::str::from_utf8(&entry[tab + 1..]),
        ) else {
            continue;
        };
        let mut fields = meta.split(' ');
        let (Some(mode), Some("blob"), Some(oid)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        if mode == "120000" || !keep(path) {
            continue;
        }
        wanted.push((path.to_string(), oid.to_string()));
    }
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    let input: String = wanted.iter().map(|(_, oid)| format!("{oid}\n")).collect();
    let answer = run(
        root,
        "git",
        &["cat-file", "--batch"],
        Options {
            stdin: Some(input.as_bytes()),
            env,
        },
    )?;
    let unreadable = |path: &str, why: String| Error::Unreadable {
        command: "git cat-file --batch".to_string(),
        why: format!("{path}: {why}"),
    };
    let mut at = 0;
    let mut out = Vec::with_capacity(wanted.len());
    for (path, oid) in wanted {
        let rest = &answer[at..];
        let end = rest
            .iter()
            .position(|b| *b == b'\n')
            .ok_or_else(|| unreadable(&path, "no answer".into()))?;
        let header = String::from_utf8_lossy(&rest[..end]).into_owned();
        let mut fields = header.split(' ');
        let size = match (fields.next(), fields.next(), fields.next()) {
            (Some(got), Some("blob"), Some(size)) if got == oid => size.parse::<usize>().ok(),
            _ => None,
        };
        let Some(size) = size else {
            return Err(unreadable(&path, format!("unexpected answer `{header}`")));
        };
        let body = end + 1;
        let Some(bytes) = rest.get(body..body + size) else {
            return Err(unreadable(&path, format!("short answer `{header}`")));
        };
        out.push((path, bytes.to_vec()));
        at += body + size + 1;
    }
    Ok(out)
}

/// `gh <args>` in `root`, its stdout with trailing whitespace trimmed.
pub fn gh(root: &Path, args: &[&str]) -> Result<String, Error> {
    run(root, "gh", args, Options::default()).map(trimmed)
}

fn trimmed(stdout: Vec<u8>) -> String {
    String::from_utf8_lossy(&stdout).trim_end().to_string()
}

/// Every file the index holds under `root`, relative to it, each once, in git's order.
pub fn tracked_files(root: &Path) -> Result<Vec<PathBuf>, Error> {
    ls_files(root, &[])
}

/// The files of [`tracked_files`] that any of `globs` matches as `:(glob)<glob>`; none, and
/// no git run, when `globs` is empty.
pub fn tracked_matching(root: &Path, globs: &[String]) -> Result<Vec<PathBuf>, Error> {
    if globs.is_empty() {
        return Ok(Vec::new());
    }
    let pathspecs: Vec<String> = globs.iter().map(|g| format!(":(glob){g}")).collect();
    ls_files(root, &pathspecs)
}

/// Every file a `git add -A` in `root` would commit: the index's files and the untracked
/// ones git's standard excludes don't ignore, relative to `root`, each once, sorted.
pub fn files_to_commit(root: &Path) -> Result<Vec<PathBuf>, Error> {
    let args = [
        "ls-files",
        "-z",
        "--cached",
        "--others",
        "--exclude-standard",
        "--deduplicate",
    ];
    let out = run(root, "git", &args, Options::default())?;
    // git lists the index's files, then the untracked ones: merge them into one order.
    let mut names: Vec<&[u8]> = nul_separated(&out).collect();
    names.sort_unstable();
    Ok(names.into_iter().map(path_from_bytes).collect())
}

fn ls_files(root: &Path, pathspecs: &[String]) -> Result<Vec<PathBuf>, Error> {
    let mut args = vec!["ls-files", "-z", "--cached", "--deduplicate"];
    if !pathspecs.is_empty() {
        args.push("--");
        args.extend(pathspecs.iter().map(String::as_str));
    }
    let out = run(root, "git", &args, Options::default())?;
    Ok(nul_separated(&out).map(path_from_bytes).collect())
}

fn nul_separated(out: &[u8]) -> impl Iterator<Item = &[u8]> {
    out.split(|&b| b == 0).filter(|name| !name.is_empty())
}

#[cfg(unix)]
fn path_from_bytes(name: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(OsStr::from_bytes(name))
}

#[cfg(not(unix))]
fn path_from_bytes(name: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(name).into_owned())
}

/// One entry of `git worktree list --porcelain`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worktree {
    pub path: std::path::PathBuf,
    /// `None` when detached.
    pub branch: Option<String>,
    pub locked: bool,
    /// Whether the lock is held: its reason names `pid <n>` and that process runs
    /// (`held-lock`).
    pub held: bool,
}

/// The remote every command fetches from and pushes to.
pub const REMOTE: &str = "origin";

/// The current time in whole seconds since the Unix epoch, 0 before 1970.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The error of a command that either refused (exit 1) or failed in git (exit 2).
#[derive(Debug)]
pub enum Failure {
    Refused(String),
    Git(String),
}

impl Failure {
    pub fn exit_code(&self) -> u8 {
        match self {
            Failure::Refused(_) => 1,
            Failure::Git(_) => 2,
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Refused(why) | Failure::Git(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for Failure {}

impl From<Error> for Failure {
    fn from(e: Error) -> Self {
        Failure::Git(e.to_string())
    }
}

/// `git fetch` from [`REMOTE`], run again after a failure as [`fetch_retrying`] runs it; the
/// last run's error when all fail, with no `git ls-remote`.
pub fn fetch(root: &Path) -> Result<(), Error> {
    run_again(root, &["fetch", REMOTE], &FETCH_WAITS, std::thread::sleep).map(drop)
}

/// The waits between a fetch's three runs.
const FETCH_WAITS: [Duration; 2] = [Duration::from_secs(1), Duration::from_secs(2)];

/// The waits between a worktree listing's three runs (`worktrees-tries-again`).
const WORKTREE_WAITS: [Duration; 2] = [Duration::from_millis(100), Duration::from_millis(500)];

/// Why [`fetch_retrying`] gave up.
#[derive(Debug)]
pub enum FetchError {
    /// The fetch kept failing and `REMOTE` can't be reached either.
    Offline,
    /// The fetch kept failing though `REMOTE` answers: the last run's error.
    Git(Error),
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::Offline => write!(f, "{REMOTE} can't be reached"),
            FetchError::Git(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for FetchError {}

/// `git <args>` (a fetch naming [`REMOTE`]) in `root`, run again after a failure, at most twice
/// more, after 1 and then 2 seconds. A run `git` can't be started for ends it at once, as
/// [`FetchError::Git`] holding an [`Error::Spawn`] that names `root`. When the last run fails,
/// [`FetchError::Offline`] if `git ls-remote` of `REMOTE` fails too, else [`FetchError::Git`]
/// with the last run's error.
pub fn fetch_retrying(root: &Path, args: &[&str]) -> Result<(), FetchError> {
    fetch_waiting(root, args, std::thread::sleep)
}

/// [`fetch_retrying`] with its waits made through `wait`, so a test sees them without the
/// clock.
fn fetch_waiting(
    root: &Path,
    args: &[&str],
    mut wait: impl FnMut(Duration),
) -> Result<(), FetchError> {
    let mut result = git(root, args);
    for &pause in &FETCH_WAITS {
        if matches!(result, Ok(_) | Err(Error::Spawn { .. })) {
            break;
        }
        wait(pause);
        result = git(root, args);
    }
    let last = match result {
        Ok(_) => return Ok(()),
        Err(Error::Spawn { command, source }) => {
            let command = format!("{command} in {}", root.display());
            return Err(FetchError::Git(Error::Spawn { command, source }));
        }
        Err(last) => last,
    };
    match git(root, &["ls-remote", "--quiet", REMOTE, "refs/heads/main"]) {
        Err(_) => Err(FetchError::Offline),
        Ok(_) => Err(FetchError::Git(last)),
    }
}

/// `git <args>` in `root`, run again after a failure once per entry of `waits`, after
/// `wait`ing that long; the first successful run's output, or the last run's error.
fn run_again(
    root: &Path,
    args: &[&str],
    waits: &[Duration],
    mut wait: impl FnMut(Duration),
) -> Result<String, Error> {
    let mut result = git(root, args);
    for &pause in waits {
        if result.is_ok() {
            break;
        }
        wait(pause);
        result = git(root, args);
    }
    result
}

/// Whether `a` is an ancestor of `b` (`git merge-base --is-ancestor a b` exits 0).
pub fn is_ancestor(root: &Path, a: &str, b: &str) -> Result<bool, Error> {
    match git(root, &["merge-base", "--is-ancestor", a, b]) {
        Ok(_) => Ok(true),
        Err(Error::Failed { status, .. }) if status.code() == Some(1) => Ok(false),
        Err(e) => Err(e),
    }
}

/// The checked-out branch, or `None` on a detached `HEAD`.
pub fn current_branch(root: &Path) -> Result<Option<String>, Error> {
    match git(root, &["symbolic-ref", "--quiet", "--short", "HEAD"]) {
        Ok(branch) => Ok(Some(branch)),
        Err(Error::Failed { status, .. }) if status.code() == Some(1) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Every worktree of the repository, from `git worktree list --porcelain -z`, run again
/// after a failure, since git fails to list while another process adds or removes a
/// worktree (`worktrees-tries-again`).
pub fn worktrees(root: &Path) -> Result<Vec<Worktree>, Error> {
    worktrees_waiting(root, std::thread::sleep)
}

/// [`worktrees`] with its waits made through `wait`, so a test sees them without the clock.
fn worktrees_waiting(root: &Path, wait: impl FnMut(Duration)) -> Result<Vec<Worktree>, Error> {
    let list = run_again(
        root,
        &["worktree", "list", "--porcelain", "-z"],
        &WORKTREE_WAITS,
        wait,
    )?;
    let mut out = Vec::new();
    for field in list.split('\0') {
        if let Some(path) = field.strip_prefix("worktree ") {
            out.push(Worktree {
                path: path.into(),
                branch: None,
                locked: false,
                held: false,
            });
        } else if let Some(entry) = out.last_mut() {
            if let Some(branch) = field.strip_prefix("branch ") {
                let branch = branch.strip_prefix("refs/heads/").unwrap_or(branch);
                entry.branch = Some(branch.to_string());
            } else if field == "locked" || field.starts_with("locked ") {
                entry.locked = true;
                let reason = field.strip_prefix("locked ").unwrap_or("");
                entry.held = lock_pid(reason).is_some_and(runs);
            }
        }
    }
    Ok(out)
}

/// A process ID, as the platform types it.
#[cfg(unix)]
type Pid = libc::pid_t;
#[cfg(not(unix))]
type Pid = u32;

/// The process ID a lock's reason names (`held-lock`): the first `pid` at its start or after
/// a character that is neither a letter nor a digit, followed by one space and decimal
/// digits, when their value is above 0 and fits [`Pid`]. Claude Code writes
/// `claude agent agent-<id> (pid <n> start <t>)`.
fn lock_pid(reason: &str) -> Option<Pid> {
    let (at, _) = reason.match_indices("pid ").find(|&(i, _)| {
        let before = reason[..i].chars().next_back();
        before.is_none_or(|c| !c.is_alphabetic() && !c.is_ascii_digit())
            && reason[i + 4..].starts_with(|c: char| c.is_ascii_digit())
    })?;
    let digits = &reason[at + 4..];
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    digits[..end].parse::<Pid>().ok().filter(|&pid| pid > 0)
}

/// Whether a process with ID `pid` runs on this machine: a signal 0 is accepted, or refused
/// for want of permission.
#[cfg(unix)]
fn runs(pid: Pid) -> bool {
    // SAFETY: `kill` with signal 0 sends nothing; it only checks the process, and `pid` is
    // above 0, so it names one process, never a group.
    let accepted = unsafe { libc::kill(pid, 0) } == 0;
    accepted || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// Where CLM can't tell whether a process runs, a lock naming one is held.
#[cfg(not(unix))]
fn runs(_pid: Pid) -> bool {
    true
}

/// `rev`'s tree checked out into a temporary directory through a temporary index, leaving
/// the working tree and the repository's index alone. Files hold what `git checkout` of
/// `rev` writes (`extract-as-checkout`). A temporary directory that can't be made is
/// reported as a `Spawn` error of `extract_tree <rev>`.
pub fn extract_tree(root: &Path, rev: &str) -> Result<tempfile::TempDir, Error> {
    let extraction = Extraction::new(root, rev, format!("extract_tree {rev}"))?;
    extraction.git(&["checkout-index", "--all"], None)?;
    Ok(extraction.tree)
}

/// As [`extract_tree`], for only the files under `paths`: each a file or directory taken
/// literally and relative to the top of `rev`'s tree, a trailing `/` allowed. A path that is
/// empty, absolute, has a `.` or `..` component, or matches nothing is an error naming it.
pub fn extract_paths(root: &Path, rev: &str, paths: &[&str]) -> Result<tempfile::TempDir, Error> {
    let command = format!("extract_paths {rev}");
    let refuse = |path: &str, why: &str| Error::Spawn {
        command: command.clone(),
        source: io::Error::new(io::ErrorKind::InvalidInput, format!("`{path}`: {why}")),
    };
    let mut wanted = Vec::with_capacity(paths.len());
    for &path in paths {
        let trimmed = path.strip_suffix('/').unwrap_or(path);
        let malformed = path.starts_with('/')
            || trimmed
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..");
        if malformed {
            return Err(refuse(path, "not a path relative to the top of the tree"));
        }
        wanted.push((path, trimmed));
    }
    if wanted.is_empty() {
        return tempfile::tempdir().map_err(|source| Error::Spawn { command, source });
    }
    let extraction = Extraction::new(root, rev, command.clone())?;
    let mut args = vec!["ls-files", "-z", "--cached", "--full-name", "--"];
    let pathspecs: Vec<String> = wanted
        .iter()
        .map(|(_, p)| format!(":(top,literal){p}"))
        .collect();
    args.extend(pathspecs.iter().map(String::as_str));
    let listed = extraction.git(&args, None)?;
    for (path, trimmed) in &wanted {
        // As git matches a literal pathspec: the file itself, or anything in the directory.
        let under = |entry: &[u8]| match entry.strip_prefix(trimmed.as_bytes()) {
            Some([]) => trimmed == path,
            Some(rest) => rest[0] == b'/',
            None => false,
        };
        if !nul_separated(&listed).any(under) {
            return Err(refuse(path, &format!("matches nothing in {rev}")));
        }
    }
    extraction.git(&["checkout-index", "--stdin", "-z"], Some(&listed))?;
    Ok(extraction.tree)
}

/// A temporary directory to write `rev`'s files into, and a temporary index holding all of
/// `rev`, so a checkout reads `rev`'s attributes whichever paths it writes.
struct Extraction<'a> {
    root: &'a Path,
    tree: tempfile::TempDir,
    _scratch: tempfile::TempDir,
    index: PathBuf,
}

impl<'a> Extraction<'a> {
    fn new(root: &'a Path, rev: &str, command: String) -> Result<Self, Error> {
        let temp = |source| Error::Spawn {
            command: command.clone(),
            source,
        };
        let tree = tempfile::tempdir().map_err(temp)?;
        let scratch = tempfile::tempdir().map_err(temp)?;
        let extraction = Extraction {
            root,
            index: scratch.path().join("index"),
            tree,
            _scratch: scratch,
        };
        extraction.git(&["read-tree", rev], None)?;
        Ok(extraction)
    }

    /// `git <args>` against the temporary index, with the temporary directory as work tree.
    fn git(&self, args: &[&str], stdin: Option<&[u8]>) -> Result<Vec<u8>, Error> {
        let work_tree = self.tree.path().to_string_lossy();
        let args: Vec<&str> = ["--work-tree", &work_tree]
            .into_iter()
            .chain(args.iter().copied())
            .collect();
        let env = [("GIT_INDEX_FILE", self.index.as_os_str())];
        run(self.root, "git", &args, Options { stdin, env: &env })
    }
}

/// Removes the linked worktree at `path` whole, ignored files included. Claude Code locks
/// the worktrees it makes, so it unlocks first (failing for one that isn't locked is
/// fine). A build can leave a directory without its owner's write permission, which git
/// can't delete, so the owner is given access to `path` and every directory under it
/// before `git worktree remove` runs, without `--force`: a worktree git refuses stays.
/// What git leaves of a worktree it no longer lists is removed here, for up to 5 seconds.
pub fn remove_worktree(root: &Path, path: &Path) -> Result<(), Error> {
    let arg = path.to_string_lossy();
    let resolved = resolve(path);
    let listed_before = lists_worktree(root, &resolved) == Some(true);
    let _ = git(root, &["worktree", "unlock", &arg]);
    make_writable(path);
    let removal = git(root, &["worktree", "remove", &arg]);
    if exists(path) && listed_before && lists_worktree(root, &resolved) == Some(false) {
        let started = Instant::now();
        loop {
            make_writable(path);
            let _ = std::fs::remove_dir_all(path);
            if !exists(path) || started.elapsed() >= LEFTOVER_LIMIT {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        if !exists(path) {
            return Ok(());
        }
    }
    removal?;
    if exists(path) {
        return Err(Error::StillOnDisk {
            path: arg.into_owned(),
        });
    }
    Ok(())
}

/// How long [`remove_worktree`] keeps trying to remove what git left of a worktree.
const LEFTOVER_LIMIT: Duration = Duration::from_secs(5);

fn exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// `path` with the symbolic links in it resolved, its last part included only when it
/// exists.
fn resolve(path: &Path) -> PathBuf {
    if let Ok(real) = path.canonicalize() {
        return real;
    }
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => parent
            .canonicalize()
            .map_or_else(|_| path.to_path_buf(), |p| p.join(name)),
        _ => path.to_path_buf(),
    }
}

/// Whether git lists a worktree at `resolved`, comparing with symbolic links resolved.
fn lists_worktree(root: &Path, resolved: &Path) -> Option<bool> {
    git(root, &["worktree", "list", "--porcelain"])
        .ok()
        .map(|out| {
            out.lines()
                .filter_map(|l| l.strip_prefix("worktree "))
                .any(|listed| resolve(Path::new(listed)) == resolved)
        })
}

/// Gives the owner read, write and execute permission on `dir` and every directory under
/// it, before reading each one's entries so a directory its owner can't list is still
/// walked. Never follows or changes a symbolic link, so nothing outside `dir` changes.
/// Best effort: what can't be changed is left for git to report.
fn make_writable(dir: &Path) {
    let Ok(meta) = std::fs::symlink_metadata(dir) else {
        return;
    };
    if !meta.is_dir() {
        return;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = meta.permissions().mode();
        if mode & 0o700 != 0o700 {
            let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(mode | 0o700));
        }
    }
    #[cfg(not(unix))]
    {
        let mut perms = meta.permissions();
        if perms.readonly() {
            perms.set_readonly(false);
            let _ = std::fs::set_permissions(dir, perms);
        }
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        make_writable(&entry.path());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failing_fetch_waits_1_then_2_seconds() {
        // req:vcs/fetch-retries@f4f74e93
        let dir = tempfile::tempdir().unwrap();
        git(dir.path(), &["init", "-q"]).unwrap();
        let missing = dir.path().join("missing");
        git(
            dir.path(),
            &["remote", "add", REMOTE, missing.to_str().unwrap()],
        )
        .unwrap();
        let mut waits = Vec::new();
        let err = fetch_waiting(dir.path(), &["fetch", REMOTE], |d| waits.push(d)).unwrap_err();
        assert!(matches!(err, FetchError::Offline), "{err}");
        assert_eq!(waits, [Duration::from_secs(1), Duration::from_secs(2)]);
    }

    #[test]
    fn a_root_git_cant_start_in_ends_the_fetch_at_once() {
        // req:vcs/fetch-retries@f4f74e93
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file");
        std::fs::write(&file, "").unwrap();
        for root in [dir.path().join("missing"), file] {
            let mut waits = Vec::new();
            let err = fetch_waiting(&root, &["fetch", REMOTE], |d| waits.push(d)).unwrap_err();
            assert!(waits.is_empty(), "{root:?}: {waits:?}");
            let FetchError::Git(Error::Spawn { command, .. }) = &err else {
                panic!("{root:?}: {err}");
            };
            assert_eq!(*command, format!("git fetch origin in {}", root.display()));
        }
    }

    #[test]
    fn a_failing_worktree_listing_waits_100_then_500_milliseconds() {
        // req:vcs/worktrees-tries-again@c983d508
        // A directory that doesn't exist fails every run, wherever the temporary directory
        // lies, even inside some repository's work tree.
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing");
        let mut waits = Vec::new();
        let err = worktrees_waiting(&missing, |d| waits.push(d)).unwrap_err();
        assert!(err.to_string().contains("worktree list"), "{err}");
        assert_eq!(
            waits,
            [Duration::from_millis(100), Duration::from_millis(500)]
        );
    }

    #[test]
    fn a_lock_reason_names_the_first_pid_standing_as_a_word() {
        // req:vcs/held-lock@c8d3683a
        assert_eq!(lock_pid("claude agent agent-1 (pid 42 start 1)"), Some(42));
        assert_eq!(lock_pid("pid 7"), Some(7));
        assert_eq!(lock_pid("x-pid 7"), Some(7));
        assert_eq!(lock_pid("pid 7x"), Some(7));
        assert_eq!(lock_pid("rapid 1 pid 3"), Some(3));
        assert_eq!(lock_pid("pid x1 pid 3"), Some(3));
        assert_eq!(lock_pid("pid 0 pid 3"), None);
        assert_eq!(lock_pid("pid 5 pid 3"), Some(5));
        assert_eq!(lock_pid("pid  5"), None);
        assert_eq!(lock_pid("2pid 5"), None);
        assert_eq!(lock_pid("épid 5"), None);
        assert_eq!(lock_pid("²pid 5"), Some(5));
        assert_eq!(lock_pid("rapid 1"), None);
        assert_eq!(lock_pid("pid 0"), None);
        assert_eq!(lock_pid("pid 99999999999999999999"), None);
        assert_eq!(lock_pid(&format!("pid {}", Pid::MAX)), Some(Pid::MAX));
        assert_eq!(lock_pid(""), None);
    }
}
