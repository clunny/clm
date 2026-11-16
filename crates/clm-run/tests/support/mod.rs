//! Fixtures shared by the tests that call `clm-run`'s `clm run` (`commands-in-crate`): a
//! fake `claude`, a superpowers copy holding the three skills, and the call with the
//! environment the command would have.
#![allow(dead_code)]

use std::cell::Cell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard};

#[path = "../../../clm-orchestrator/tests/common/mod.rs"]
pub mod common;

use common::*;
use tempfile::TempDir;

pub const SKILLS: [&str; 3] = [
    "writing-plans",
    "test-driven-development",
    "verification-before-completion",
];

/// A fake `claude` that records that it ran, and a superpowers copy holding the three
/// skills.
pub struct Fake {
    _dir: TempDir,
    pub bin: PathBuf,
    pub out: PathBuf,
    pub superpowers: PathBuf,
}

pub fn fake() -> Fake {
    let dir = TempDir::new().unwrap();
    let bin = dir.path().join("bin");
    let out = dir.path().join("out");
    let superpowers = dir.path().join("superpowers");
    for d in [&bin, &out] {
        fs::create_dir_all(d).unwrap();
    }
    for s in SKILLS {
        write(&superpowers, &format!("skills/{s}/SKILL.md"), "skill\n");
    }
    write_exec(&bin.join("claude"), "#!/bin/sh\ntouch \"$FAKE_OUT/ran\"\n");
    Fake {
        _dir: dir,
        bin,
        out,
        superpowers,
    }
}

/// What a `clm run` printed.
pub struct Ran {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// What a call wrote to each buffer and the status it returned, shaped as the
/// `std::process::Output` of the binary it replaces.
pub struct Output {
    pub status: Status,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// A call's status, read as a process's exit status is.
pub struct Status(u8);

impl From<u8> for Status {
    fn from(status: u8) -> Self {
        Status(status)
    }
}

impl Status {
    pub fn code(&self) -> Option<i32> {
        Some(i32::from(self.0))
    }

    pub fn success(&self) -> bool {
        self.0 == 0
    }
}

thread_local! {
    static INTO_PROCESS: Cell<bool> = const { Cell::new(false) };
}

/// The process's environment, which `process_env.rs` changes: each of its tests holds this
/// lock for its whole run, so no thread reads the environment while one changes it.
static PROCESS_ENV: Mutex<()> = Mutex::new(());

/// Holds the process's environment for the calling test, and from now on sets each of this
/// thread's calls' environment in the process too, for the length of the call: a fake `git`
/// or `claude` on `PATH`, and a variable such as `GIT_DIR` that git reads, reach the commands
/// a call runs only through the process's own environment. Only a binary whose every test
/// takes it may use it.
pub fn into_process() -> MutexGuard<'static, ()> {
    let held = PROCESS_ENV.lock().unwrap_or_else(|e| e.into_inner());
    INTO_PROCESS.with(|c| c.set(true));
    held
}

/// What `call` wrote to each buffer and the status it returned, given `env` as its lookup.
pub fn called(
    call: impl FnOnce(&dyn Fn(&str) -> Option<String>, &mut Vec<u8>, &mut Vec<u8>) -> u8,
    env: &HashMap<String, String>,
) -> Output {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let lookup = |k: &str| env.get(k).cloned();
    let saved: Vec<(&String, Option<std::ffi::OsString>)> = if INTO_PROCESS.with(Cell::get) {
        env.keys().map(|k| (k, std::env::var_os(k))).collect()
    } else {
        Vec::new()
    };
    for (k, _) in &saved {
        // SAFETY: `into_process` holds the lock every test of this binary holds, so no other
        // thread reads the environment.
        unsafe { std::env::set_var(k, &env[*k]) };
    }
    let status = call(&lookup, &mut out, &mut err);
    for (k, old) in saved {
        // SAFETY: as above.
        unsafe {
            match old {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
    }
    Output {
        status: Status(status),
        stdout: out,
        stderr: err,
    }
}

/// `called`, read as a `clm run`'s output.
pub fn ran_with(
    call: impl FnOnce(&dyn Fn(&str) -> Option<String>, &mut Vec<u8>, &mut Vec<u8>) -> u8,
    env: &HashMap<String, String>,
) -> Ran {
    let out = called(call, env);
    Ran {
        code: out.status.code(),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}

/// The environment `pairs` names, as a call's lookup reads it.
pub fn env_of<V: AsRef<std::ffi::OsStr> + ?Sized>(pairs: &[(&str, &V)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.as_ref().to_string_lossy().into_owned()))
        .collect()
}

/// `clm superpowers <args>` run in `cwd` with the environment `env` alone, so the host's
/// override never leaks in.
pub fn clm_superpowers_in(cwd: &Path, args: &[&str], env: &[(&str, &std::ffi::OsStr)]) -> Output {
    called(
        |env, out, err| clm_run::superpowers_command(args, cwd, cwd, env, out, err),
        &env_of(env),
    )
}

/// Runs `clm superpowers <skill>` with `env` on top of an environment without
/// `CLM_SUPERPOWERS_DIR`, so the host's override never leaks in.
pub fn clm_superpowers(skill: &str, env: &[(&str, &std::ffi::OsStr)]) -> Output {
    clm_superpowers_in(&std::env::current_dir().unwrap(), &[skill], env)
}

/// A directory holding `skills/brainstorming/SKILL.md`; returns that file's path.
pub fn with_skill(dir: &Path) -> PathBuf {
    let skill = dir.join("skills/brainstorming/SKILL.md");
    fs::create_dir_all(skill.parent().unwrap()).unwrap();
    fs::write(&skill, "# brainstorming\n").unwrap();
    skill
}

/// An `XDG_CACHE_HOME` holding the pinned copy; returns it and the skill's path.
pub fn pinned_cache() -> (TempDir, PathBuf) {
    let cache = TempDir::new().unwrap();
    let skill = with_skill(
        &cache
            .path()
            .join("clm/superpowers")
            .join(clm_run::SUPERPOWERS_COMMIT),
    );
    (cache, skill)
}

pub fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

pub fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// A CLM repository (a `specs/SPEC.md` and a `.git`) with `clm.toml` holding `toml`, and a
/// subdirectory `src/`.
pub fn setup_repo(toml: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join("specs")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("specs/SPEC.md"), "# spec\n").unwrap();
    fs::write(root.join("clm.toml"), toml).unwrap();
    dir
}

/// `clm setup` run in `cwd`, in the CLM repository `crates/clm` finds from it (the nearest
/// directory up holding `specs/SPEC.md`), with `env` as its environment.
pub fn clm_setup_with(cwd: &Path, env: &[(&str, &std::ffi::OsStr)]) -> Output {
    let root = cwd
        .ancestors()
        .find(|d| d.join("specs/SPEC.md").is_file())
        .unwrap_or(cwd);
    called(
        |env, out, err| clm_run::setup_command::<_, &str>([], root, cwd, env, out, err),
        &env_of(env),
    )
}

pub fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The environment a `clm run` of the fake gets: `PATH` with the fake first, `FAKE_OUT`,
/// `CLM_SUPERPOWERS_DIR` naming the fake's copy and an `XDG_CONFIG_HOME` that keeps the
/// user's settings out, then `env` over them. Nothing else of the test's own environment,
/// such as a `CARGO_TARGET_DIR`, is in it.
pub fn run_env(f: &Fake, env: &[(&str, &Path)]) -> HashMap<String, String> {
    let path = format!(
        "{}:{}",
        f.bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut vars: HashMap<String, String> = [
        ("PATH", path),
        ("FAKE_OUT", f.out.display().to_string()),
        ("CLM_SUPERPOWERS_DIR", f.superpowers.display().to_string()),
        ("XDG_CONFIG_HOME", f.out.display().to_string()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    for (k, v) in env {
        vars.insert(k.to_string(), v.display().to_string());
    }
    vars
}

pub fn clm_run(r: &Repo, f: &Fake, node: &str, env: &[(&str, &Path)]) -> Ran {
    clm_run_in(&r.work, &r.work, f, node, env)
}

pub fn clm_run_in(cwd: &Path, root: &Path, f: &Fake, node: &str, env: &[(&str, &Path)]) -> Ran {
    clm_run_with(cwd, root, f, &[node], env)
}

/// `clm run <args> --root <root>`, run in `cwd`.
pub fn clm_run_with(
    cwd: &Path,
    root: &Path,
    f: &Fake,
    args: &[&str],
    env: &[(&str, &Path)],
) -> Ran {
    let mut line: Vec<&str> = args.to_vec();
    let root = root.to_str().unwrap();
    line.extend(["--root", root]);
    ran_with(
        |env, out, err| clm_run::run_command(line, Path::new("."), cwd, env, out, err),
        &run_env(f, env),
    )
}

/// What a `clm run` that must succeed printed, the launcher, and the prompt it wrote to
/// `.clm/prompts/<node>.md` under `root`, which the launcher names by its absolute path.
pub fn launched_in(
    cwd: &Path,
    root: &Path,
    f: &Fake,
    node: &str,
    env: &[(&str, &Path)],
) -> (String, String) {
    let ran = clm_run_in(cwd, root, f, node, env);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let launcher = ran.stdout;
    assert!(launcher.lines().count() <= 5, "{launcher}");
    let file = cwd
        .join(root)
        .canonicalize()
        .unwrap()
        .join(format!(".clm/prompts/{node}.md"));
    assert!(file.is_absolute());
    assert!(
        launcher.contains(&file.display().to_string()),
        "{} missing from {launcher}",
        file.display()
    );
    assert!(
        launcher.contains(&format!("git switch impl/{node}")),
        "{launcher}"
    );
    (launcher, fs::read_to_string(&file).unwrap())
}

/// The prompt of a `clm run` that must succeed.
pub fn prompt(r: &Repo, f: &Fake, node: &str, env: &[(&str, &Path)]) -> String {
    launched_in(&r.work, &r.work, f, node, env).1
}

/// A `clm run` that must refuse with `code`, printing nothing on stdout.
pub fn refused(r: &Repo, f: &Fake, node: &str, env: &[(&str, &Path)], code: i32) -> String {
    let ran = clm_run(r, f, node, env);
    assert_eq!(ran.code, Some(code), "{}", ran.stderr);
    assert_eq!(ran.stdout, "", "nothing on stdout");
    ran.stderr
}

pub fn ran(f: &Fake) -> bool {
    f.out.join("ran").exists()
}

pub fn local_tip(r: &Repo, branch: &str) -> Option<String> {
    let out = output(
        Command::new("git")
            .args([
                "rev-parse",
                "--verify",
                "-q",
                &format!("refs/heads/{branch}"),
            ])
            .current_dir(&r.work),
    );
    out.status
        .success()
        .then(|| String::from_utf8(out.stdout).unwrap().trim().to_string())
}

/// Whether some worktree has `branch` checked out.
pub fn checked_out(r: &Repo, branch: &str) -> bool {
    git(&r.work, &["worktree", "list", "--porcelain"])
        .lines()
        .any(|l| l == format!("branch refs/heads/{branch}"))
}

/// The worktree a build of `b` would have had: `.claude/worktrees/agent-1` in the main
/// worktree, where Claude Code puts it, on the local `impl/b` an earlier `clm run b` left.
pub fn build_worktree(r: &Repo, f: &Fake) -> PathBuf {
    prompt(r, f, "b", &[]);
    let wt = r.work.join(".claude/worktrees/agent-1");
    git(
        &r.work,
        &["worktree", "add", "-q", wt.to_str().unwrap(), "impl/b"],
    );
    wt
}

/// The names of the clone's places.
pub fn place_names(r: &Repo) -> Vec<String> {
    clm_orchestrator::places(&r.work, 4)
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect()
}

/// What `clm run --finish <node>` printed.
pub fn clm_finish(r: &Repo, node: &str) -> Ran {
    clm_finish_in(r, &r.work, node)
}

/// `clm run --finish <node>` run in the worktree at `dir`.
pub fn clm_finish_in(r: &Repo, dir: &Path, node: &str) -> Ran {
    let env: HashMap<String, String> = [(
        "XDG_CONFIG_HOME".to_string(),
        r.work.join(".no-config").display().to_string(),
    )]
    .into();
    let line = ["--finish", node, "--root", dir.to_str().unwrap()];
    ran_with(
        |env, out, err| clm_run::run_command(line, Path::new("."), dir, env, out, err),
        &env,
    )
}

/// A `clm run --finish` that must exit 0; its stdout.
pub fn finished(r: &Repo, node: &str) -> String {
    let ran = clm_finish(r, node);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    ran.stdout
}

/// The landed build of "landed build tidied": a place `b`, a clean, locked worktree at
/// `.claude/worktrees/agent-1` on local `impl/b`, whose tip is the second parent of a merge
/// on `origin/main`, and no `origin/impl/b`. Returns the worktree's canonical path.
pub fn landed_build(r: &Repo, f: &Fake) -> PathBuf {
    let wt = build_worktree(r, f).canonicalize().unwrap();
    git(&r.work, &["worktree", "lock", wt.to_str().unwrap()]);
    merge_claim(&r.work, "b");
    git(&r.origin, &["branch", "-q", "-D", "impl/b"]);
    assert_eq!(place_names(r), ["b"]);
    wt
}
