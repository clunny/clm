//! The capabilities' black-box tests (`human-layer-proof`), one module per capability of
//! `CAPABILITIES.md`. Each runs the `clm` binary in a temporary git repository whose
//! `origin` is a bare repository beside it, and checks only what a person would see.

mod adopt_clm;
mod approve_intent;
mod build_from_spec;
mod catch_drift;
mod configure_clm;
mod contribute_from_outside;
mod git_host;
mod guard_sessions;
mod install_and_release;
mod keep_a_backlog;
mod know_health;
mod land_safely;
mod no_dead_weight;
mod pin_working_method;
mod propose_intent;
mod prove_by_tests;
mod report_defects;
mod right_model_per_task;
mod see_project_state;
mod slow_tests_after_land;
mod triage_unattended;
mod unattended_runner;
mod upgrade_clm;
mod work_concurrently;
mod wrap_up_session;

use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Arc;

use tempfile::TempDir;

/// The fixture's capabilities: `c`, which node `a`'s rules serve.
const CAPABILITIES: &str = "# Capabilities\n\n## Capabilities\n\n### Capability: c\nA person can c.\n\n#### Why\nw\n\n#### Scenario: s\n- GIVEN g\n- WHEN w\n- THEN t\n";

/// Node `a`, owning `src/**` and `tests/**`, with requirements `x` and `y`.
const NODE_A: &str = "---\ntitle: A\nstatus: active\nowners: []\npaths: [\"src/**\", \"tests/**\"]\nserves: [capability/c]\n---\n\n## Contract\nA does x and y.\n\n## Requirements\n\n### Requirement: x\nA SHALL x.\n\n#### Scenario: x\n- GIVEN g\n- WHEN w\n- THEN t\n\n### Requirement: y\nA SHALL y.\n\n#### Scenario: y\n- GIVEN g\n- WHEN w\n- THEN t\n";

/// A person's repository: a CLM repository on `main`, pushed to a bare `origin` beside it.
pub struct Repo {
    /// The temporary directory holding the work tree, `origin` and every other clone.
    dir: Arc<TempDir>,
    /// The work tree.
    pub root: PathBuf,
    /// An empty directory for `XDG_CONFIG_HOME`, so the developer's settings never leak in.
    config: PathBuf,
}

impl Repo {
    /// A repository with the root node, node `a` and `CAPABILITIES.md`, committed on `main`
    /// and pushed to `origin`.
    pub fn init() -> Repo {
        let repo = Repo::empty();
        repo.write(
            "specs/SPEC.md",
            "---\ntitle: R\nstatus: active\nowners: []\n---\n\n## Contract\nThe product.\n",
        );
        repo.write("specs/a/SPEC.md", NODE_A);
        repo.write("CAPABILITIES.md", CAPABILITIES);
        repo.write("clm.toml", "host = \"none\"\n");
        // Build worktrees and `.clm/` are ignored as `clm init` ignores them. What a land
        // writes in the work tree, its `target/` places, is ignored by hand (backlog/clm-land-
        // leaves-untracked-target-in-a-non-rust-adopter-s-work).
        repo.write(".gitignore", ".clm/\n.claude/worktrees/\ntarget/\n");
        repo.commit("specs");
        repo.git(&["push", "-q", "-u", "origin", "main"]);
        repo
    }

    /// An empty git repository on `main`, its `origin` an empty bare repository beside it.
    pub fn empty() -> Repo {
        let dir = TempDir::new().unwrap();
        // `clm` names paths as the system resolves them, symlinks followed.
        let top = dir.path().canonicalize().unwrap();
        let root = top.join("work");
        let origin = top.join("origin.git");
        let config = top.join("config");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&config).unwrap();
        run_git(&top, &["init", "-q", "--bare", "-b", "main", "origin.git"]);
        let repo = Repo {
            dir: Arc::new(dir),
            root,
            config,
        };
        repo.git(&["init", "-q", "-b", "main"]);
        repo.git(&["config", "user.name", "Ann"]);
        repo.git(&["config", "user.email", "ann@example.com"]);
        repo.git(&["remote", "add", "origin", origin.to_str().unwrap()]);
        repo
    }

    /// Another person's clone of `origin`, named `name`, beside this one.
    pub fn clone_as(&self, name: &str) -> Repo {
        let top = self.root.parent().unwrap();
        run_git(top, &["clone", "-q", "origin.git", name]);
        let other = Repo {
            dir: Arc::clone(&self.dir),
            root: top.join(name),
            config: self.config.clone(),
        };
        other.git(&["config", "user.name", name]);
        other.git(&["config", "user.email", &format!("{name}@example.com")]);
        other
    }

    /// The same person working in another work tree of the repository, `root`.
    pub fn at(&self, root: &Path) -> Repo {
        Repo {
            dir: Arc::clone(&self.dir),
            root: root.to_path_buf(),
            config: self.config.clone(),
        }
    }

    /// Writes `content` to `rel` in the work tree.
    pub fn write(&self, rel: &str, content: &str) {
        let path = self.root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    /// Runs `git` in the work tree, asserting it succeeds, and returns its stdout.
    pub fn git(&self, args: &[&str]) -> String {
        run_git(&self.root, args)
    }

    /// Commits everything in the work tree.
    pub fn commit(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", message]);
    }

    /// Makes `origin` a bare git server that lands push to, with `check` as the land's check
    /// command, and pushes that setting to `main`.
    pub fn land_on_origin(&self, check: &str) {
        self.write(
            "clm.toml",
            &format!("host = \"git\"\n\n[land]\ncheck = {check:?}\n"),
        );
        self.commit("land on origin");
        self.git(&["push", "-q", "origin", "main"]);
    }

    /// Commits everything on a new branch `branch` and pushes it to `origin`, leaving the
    /// work tree on `main`.
    pub fn push_branch(&self, branch: &str, message: &str) {
        self.git(&["switch", "-q", "-c", branch]);
        self.commit(message);
        self.git(&["push", "-q", "-u", "origin", branch]);
        self.git(&["switch", "-q", "main"]);
    }

    /// The commit `main` is at on `origin`.
    pub fn origin_main(&self) -> String {
        self.git(&["ls-remote", "origin", "refs/heads/main"])
            .split_whitespace()
            .next()
            .unwrap()
            .to_string()
    }

    /// The ledger as `clm ledger show` prints it: one record per line, split into words.
    pub fn ledger(&self) -> Vec<Vec<String>> {
        let out = self.clm(&["ledger", "show"]);
        assert!(out.status.success(), "{}", printed(&out));
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|l| l.split_whitespace().map(str::to_string).collect())
            .collect()
    }

    /// The hash `clm hashes <node>` prints for `element`, named as it prints it
    /// (`requirement:x`, `contract`, `capability:c`): a delta's base, and its first 8 hex
    /// digits a marker's version.
    pub fn hash(&self, node: &str, element: &str) -> String {
        let out = self.clm(&["hashes", node]);
        assert!(out.status.success(), "{}", printed(&out));
        let stdout = String::from_utf8_lossy(&out.stdout);
        let key = format!("{element} ");
        stdout
            .lines()
            .find_map(|l| l.strip_prefix(&key))
            .unwrap_or_else(|| panic!("no hash for {element}: {stdout}"))
            .trim()
            .to_string()
    }

    /// Runs `clm` with `args` in the work tree.
    pub fn clm(&self, args: &[&str]) -> Output {
        self.clm_stdin(args, "")
    }

    /// Runs `clm` with `args` in the work tree, `stdin` on its standard input.
    pub fn clm_stdin(&self, args: &[&str], stdin: &str) -> Output {
        self.clm_env(args, stdin, &[])
    }

    /// Runs `clm` with `args` in the work tree, `stdin` on its standard input and `env` set.
    pub fn clm_env(&self, args: &[&str], stdin: &str, env: &[(&str, &str)]) -> Output {
        let mut child = isolated(Command::new(env!("CARGO_BIN_EXE_clm")))
            .args(args)
            .envs(env.iter().copied())
            .current_dir(&self.root)
            .env("XDG_CONFIG_HOME", &self.config)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        // A command that exits before reading its input shows its own output, not a broken pipe.
        let written = child.stdin.take().unwrap().write_all(stdin.as_bytes());
        if let Err(e) = written {
            assert_eq!(e.kind(), ErrorKind::BrokenPipe, "{e}");
        }
        child.wait_with_output().unwrap()
    }
}

/// `command` with none of the environment's git or CLM settings, so only the fixture counts.
fn isolated(mut command: Command) -> Command {
    command
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1");
    for var in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_COMMON_DIR",
        "GIT_OBJECT_DIRECTORY",
        "GIT_AUTHOR_NAME",
        "GIT_AUTHOR_EMAIL",
        "GIT_COMMITTER_NAME",
        "GIT_COMMITTER_EMAIL",
        "CLM_AUTOSYNC",
        "CLM_DEFECTS",
        "CLM_WRITER",
        "CLM_MAX_BUILDS",
        "CLAUDECODE",
        clm_agent_harness::CODEX_MARKER,
    ] {
        command.env_remove(var);
    }
    command
}

fn run_git(dir: &Path, args: &[&str]) -> String {
    let out = isolated(Command::new("git"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The `PreToolUse` event Claude Code sends the guard for a Bash `command` run in `cwd`.
pub fn bash_event(command: &str, cwd: &Path) -> String {
    serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": { "command": command },
        "cwd": cwd,
    })
    .to_string()
}

/// What a command printed, stdout then stderr.
pub fn printed(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// The commit a `clm land` that succeeded names in its `landed <sha> <kind>` line.
pub fn landed(out: &Output) -> String {
    let text = printed(out);
    assert!(out.status.success(), "{text}");
    text.lines()
        .find_map(|l| l.strip_prefix("landed "))
        .and_then(|l| l.split_whitespace().next())
        .unwrap_or_else(|| panic!("no landed line: {text}"))
        .to_string()
}
