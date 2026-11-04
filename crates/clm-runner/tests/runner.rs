//! `clm runner` against a fake `clm` and a fake `claude` (orchestrator.runner).
//!
//! Each scenario runs the runner in a subprocess, this test binary re-run as `child`, so its
//! environment, its current directory and the signals it catches are its own.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

const CLM: &str = r#"#!/bin/sh
F="$FAKE"
echo "$*" >> "$F/clm.log"
case "$1" in
next)
  n=$(cat "$F/next.count" 2>/dev/null || echo 0); n=$((n+1)); echo $n > "$F/next.count"
  if [ -f "$F/next.$n" ]; then cat "$F/next.$n"; elif [ -f "$F/next" ]; then cat "$F/next"; fi
  if [ -f "$F/next.$n.code" ]; then cat "$F/next.err" >&2; exit "$(cat "$F/next.$n.code")"; fi
  if [ -f "$F/next.code" ]; then cat "$F/next.err" >&2; exit "$(cat "$F/next.code")"; fi
  exit 0;;
run)
  if [ "$2" = "--finish" ]; then echo "freed $3"; exit 0; fi
  if [ -f "$F/run.$2.code" ]; then cat "$F/run.$2.err" >&2; exit "$(cat "$F/run.$2.code")"; fi
  echo "Read the prompt for $2"; exit 0;;
esac
"#;

const CLAUDE: &str = r#"#!/bin/sh
F="$FAKE"
if [ "$1" = plugin ]; then
  echo "$* in $(pwd -P)" >> "$F/plugin.log"
  if [ -f "$F/plugins.code" ]; then cat "$F/plugins.err" >&2; exit "$(cat "$F/plugins.code")"; fi
  if [ -f "$F/plugins" ]; then cat "$F/plugins"; else echo '[{"id":"clm@clm","scope":"user","enabled":true}]'; fi
  exit 0
fi
n=$(basename "$PWD"); n=${n#agent-runner-}
printf '%s\0' "$@" > "$F/claude.$n.args"
pwd -P > "$F/claude.$n.pwd"
env > "$F/claude.$n.env"
if [ -f "$F/trap" ]; then
  trap 'echo term > "$F/claude.term"; exit 0' TERM
  sh -c 'trap "echo term > \"$FAKE/child.term\"; exit 0" TERM; while :; do sleep 0.05; done' &
  echo started > "$F/started.$n"
  wait
fi
while [ -f "$F/hold" ]; do sleep 0.05; done
if [ -f "$F/claude.out" ]; then cat "$F/claude.out"; fi
exit "$(cat "$F/claude.code" 2>/dev/null || echo 0)"
"#;

/// Run as a subprocess by every scenario: runs the runner with the options in the
/// environment and exits with its code.
#[test]
fn child() {
    let Ok(root) = std::env::var("CLM_RUNNER_ROOT") else {
        return;
    };
    let var = |k: &str| std::env::var(k).ok();
    let options = clm_runner::Options {
        clm: var("CLM_RUNNER_CLM").unwrap().into(),
        claude: "claude".into(),
        root: root.into(),
        permission_mode: var("CLM_RUNNER_MODE"),
        interval: Duration::from_millis(var("CLM_RUNNER_INTERVAL_MS").unwrap().parse().unwrap()),
        once: var("CLM_RUNNER_ONCE").is_some(),
        expiry: 4,
    };
    std::process::exit(clm_runner::runner(&options).into());
}

struct Fixture {
    _dir: tempfile::TempDir,
    /// The clone the runner builds, its main worktree.
    work: PathBuf,
    /// The fake's state directory (`$FAKE`).
    fake: PathBuf,
    /// The directory holding the fake `claude`.
    bin: PathBuf,
    home: PathBuf,
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

fn script(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().canonicalize().unwrap();
    let origin = base.join("origin.git");
    git(
        &base,
        &[
            "init",
            "-q",
            "--bare",
            "-b",
            "main",
            origin.to_str().unwrap(),
        ],
    );
    let work = base.join("work");
    git(&base, &["init", "-q", "-b", "main", work.to_str().unwrap()]);
    git(&work, &["config", "user.name", "t"]);
    git(&work, &["config", "user.email", "t@t"]);
    fs::write(work.join("README"), "x\n").unwrap();
    fs::write(work.join(".gitignore"), "/.clm/\n/.claude/\n").unwrap();
    git(&work, &["add", "."]);
    git(&work, &["commit", "-q", "-m", "init"]);
    git(
        &work,
        &["remote", "add", "origin", origin.to_str().unwrap()],
    );
    git(&work, &["push", "-q", "origin", "main"]);
    git(&work, &["fetch", "-q", "origin"]);
    let fake = base.join("fake");
    let bin = base.join("bin");
    let home = base.join("home");
    for d in [&fake, &bin, &home] {
        fs::create_dir(d).unwrap();
    }
    script(&base.join("clm"), CLM);
    script(&bin.join("claude"), CLAUDE);
    Fixture {
        _dir: dir,
        work,
        fake,
        bin,
        home,
    }
}

impl Fixture {
    fn set(&self, name: &str, text: &str) {
        fs::write(self.fake.join(name), text).unwrap();
    }

    /// How many times the runner has run `clm next`.
    fn looks(&self) -> usize {
        self.clm_calls()
            .iter()
            .filter(|c| c.starts_with("next "))
            .count()
    }

    fn clm_calls(&self) -> Vec<String> {
        fs::read_to_string(self.fake.join("clm.log"))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    /// The environment the runner runs in, which its sessions should inherit unchanged.
    fn env(&self, mode: Option<&str>, once: bool) -> BTreeMap<String, String> {
        let mut env = BTreeMap::from([
            (
                "PATH".into(),
                format!("{}:/usr/bin:/bin", self.bin.display()),
            ),
            ("HOME".into(), self.home.display().to_string()),
            ("FAKE".into(), self.fake.display().to_string()),
            ("CLM_RUNNER_ROOT".into(), self.work.display().to_string()),
            (
                "CLM_RUNNER_CLM".into(),
                self.fake
                    .parent()
                    .unwrap()
                    .join("clm")
                    .display()
                    .to_string(),
            ),
            ("CLM_RUNNER_INTERVAL_MS".into(), "100".into()),
        ]);
        if let Some(mode) = mode {
            env.insert("CLM_RUNNER_MODE".into(), mode.into());
        }
        if once {
            env.insert("CLM_RUNNER_ONCE".into(), "1".into());
        }
        env
    }

    fn spawn(&self, env: &BTreeMap<String, String>) -> Child {
        Command::new(std::env::current_exe().unwrap())
            .args(["child", "--exact", "--nocapture", "--test-threads=1", "-q"])
            .env_clear()
            .envs(env)
            .current_dir(&self.work)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    }

    /// Runs the runner to its end, failing the test after 30 seconds.
    fn run(&self, mode: Option<&str>, once: bool) -> Output {
        finish(self.spawn(&self.env(mode, once)))
    }

    fn wait_for(&self, what: impl Fn() -> bool) {
        let start = Instant::now();
        while !what() {
            assert!(start.elapsed() < Duration::from_secs(30), "timed out");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn worktree(&self, node: &str) -> PathBuf {
        self.work
            .join(".claude/worktrees")
            .join(format!("agent-runner-{node}"))
    }
}

fn finish(mut child: Child) -> Output {
    let start = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(30) {
            child.kill().unwrap();
            panic!(
                "runner still running after 30s: {:?}",
                child.wait_with_output()
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    child.wait_with_output().unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn scenario_no_permission_mode() {
    // req:orchestrator.runner/runner-prerequisites@fd6e7608
    let f = fixture();
    let out = f.run(None, true);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(text(&out.stderr).contains("--permission-mode"), "{out:?}");
    assert!(f.clm_calls().is_empty());
}

#[test]
fn scenario_no_claude() {
    // req:orchestrator.runner/runner-prerequisites@fd6e7608
    let f = fixture();
    fs::remove_file(f.bin.join("claude")).unwrap();
    let out = f.run(Some("acceptEdits"), true);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let err = text(&out.stderr);
    assert!(
        err.contains("claude") && !err.contains("--permission-mode"),
        "{out:?}"
    );
    assert!(f.clm_calls().is_empty());
}

#[test]
fn scenario_ready_node_launched() {
    // req:orchestrator.runner/runner-launch@8acf1778
    let f = fixture();
    f.set("next.1", "a\n");
    let env = f.env(Some("acceptEdits"), true);
    let out = finish(f.spawn(&env));
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let args = fs::read_to_string(f.fake.join("claude.a.args")).unwrap();
    let args: Vec<&str> = args.trim_end_matches('\0').split('\0').collect();
    let prompts = f.work.join(".clm/prompts").display().to_string();
    assert_eq!(args.len(), 8, "{args:?}");
    assert_eq!(args[0], "-p");
    assert!(args[1].starts_with(clm_runner::UNATTENDED), "{}", args[1]);
    for word in ["`human-task`", "`blocks`", "`clm land`"] {
        assert!(clm_runner::UNATTENDED.contains(word), "{word}");
    }
    assert_eq!(
        args[1],
        format!("{}\n\nRead the prompt for a", clm_runner::UNATTENDED)
    );
    assert_eq!(
        &args[2..],
        [
            "--model",
            "sonnet",
            "--permission-mode",
            "acceptEdits",
            "--add-dir",
            prompts.as_str()
        ]
    );
    let pwd = fs::read_to_string(f.fake.join("claude.a.pwd")).unwrap();
    assert_eq!(Path::new(pwd.trim()), f.worktree("a"));
    let seen: BTreeMap<String, String> = fs::read_to_string(f.fake.join("claude.a.env"))
        .unwrap()
        .lines()
        .filter_map(|l| l.split_once('='))
        .filter(|(k, _)| !["PWD", "OLDPWD", "SHLVL", "_"].contains(k))
        // Under coverage the runner's profiler runtime adds these to its own
        // environment, so its session inherits them unchanged too.
        .filter(|(k, _)| !k.starts_with("__LLVM_PROFILE_") && *k != "LLVM_PROFILE_FILE")
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    assert_eq!(seen, env);
    assert!(
        f.clm_calls()
            .contains(&format!("run a --expiry 4 --root {}", f.work.display()))
    );
    let _ = fs::read_to_string(f.work.join(".clm/logs/a.log")).unwrap();
}

#[test]
fn scenario_the_sessions_prompt() {
    // req:orchestrator.runner/paragraph-says-pull-request@0ed40483
    let f = fixture();
    f.set("next.1", "a\n");
    let env = f.env(Some("acceptEdits"), true);
    let out = finish(f.spawn(&env));
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let args = fs::read_to_string(f.fake.join("claude.a.args")).unwrap();
    let prompt = args.split('\0').nth(1).unwrap();
    let paragraph = prompt.split("\n\n").next().unwrap();
    assert!(paragraph.contains("## CLM defects"), "{paragraph}");
    assert!(paragraph.contains("pull request body"), "{paragraph}");
    assert!(!paragraph.contains("page"), "{paragraph}");
}

#[test]
fn session_output_goes_to_the_log() {
    // req:orchestrator.runner/runner-launch@8acf1778
    let f = fixture();
    f.set("next.1", "a\n");
    f.set("claude.out", "built it\nlanded 1a2b3c4\n");
    let out = f.run(Some("acceptEdits"), true);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let log = fs::read_to_string(f.work.join(".clm/logs/a.log")).unwrap();
    assert_eq!(log, "built it\nlanded 1a2b3c4\n");
    // Detached at origin/main.
    let wt = f.worktree("a");
    assert_eq!(
        git(&wt, &["rev-parse", "HEAD"]),
        git(&f.work, &["rev-parse", "origin/main"])
    );
    let on_branch = Command::new("git")
        .args(["symbolic-ref", "-q", "HEAD"])
        .current_dir(&wt)
        .status()
        .unwrap();
    assert!(!on_branch.success(), "not detached");
}

#[test]
fn scenario_running_node_not_prepared_again() {
    // req:orchestrator.runner/runner-launch@8acf1778
    let f = fixture();
    f.set("next.1", "a\n");
    f.set("next.2", "a\nb\n");
    f.set("next.3", "a\nb\n");
    f.set("hold", "");
    let child = f.spawn(&f.env(Some("acceptEdits"), true));
    f.wait_for(|| f.looks() >= 4);
    fs::remove_file(f.fake.join("hold")).unwrap();
    let out = finish(child);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let runs = |node: &str| {
        f.clm_calls()
            .iter()
            .filter(|c| c.starts_with(&format!("run {node} ")))
            .count()
    };
    assert_eq!((runs("a"), runs("b")), (1, 1), "{:?}", f.clm_calls());
    assert!(f.fake.join("claude.b.args").exists());
}

#[test]
fn scenario_no_free_place() {
    // req:orchestrator.runner/runner-refusals@76e90c84
    let f = fixture();
    f.set("next.1", "a\nb\n");
    f.set("run.a.code", "3");
    f.set("run.a.err", "clm run: no free place: 3 of 3 held\n");
    let out = f.run(Some("acceptEdits"), true);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let calls = f.clm_calls();
    assert!(!calls.iter().any(|c| c.starts_with("run b")), "{calls:?}");
    assert!(!f.fake.join("claude.a.args").exists());
}

#[test]
fn no_free_place_waits_for_the_interval() {
    // req:orchestrator.runner/runner-refusals@76e90c84
    let f = fixture();
    f.set("next", "a\n");
    f.set("run.a.code", "3");
    f.set("run.a.err", "clm run: no free place\n");
    let mut child = f.spawn(&f.env(Some("acceptEdits"), false));
    std::thread::sleep(Duration::from_millis(1000));
    child.kill().unwrap();
    child.wait().unwrap();
    // One look every 100 ms interval, never a busy loop, and never a session.
    let looks = f
        .clm_calls()
        .iter()
        .filter(|c| c.starts_with("next"))
        .count();
    assert!((2..=15).contains(&looks), "{looks}");
    assert!(!f.fake.join("claude.a.args").exists());
}

#[test]
fn scenario_refused_node_skipped() {
    // req:orchestrator.runner/runner-refusals@76e90c84
    let f = fixture();
    f.set("next.1", "a\nb\n");
    f.set("run.a.code", "1");
    f.set(
        "run.a.err",
        "clm run: unpushed local branch impl/a\nclm run: push it first\n",
    );
    let out = f.run(Some("acceptEdits"), true);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let all = text(&out.stdout) + &text(&out.stderr);
    assert!(
        all.contains("skipped a: unpushed local branch impl/a; push it first"),
        "{out:?}"
    );
    assert!(f.clm_calls().iter().any(|c| c.starts_with("run b ")));
    assert!(f.fake.join("claude.b.args").exists());
}

#[test]
fn run_git_failure_stops_the_look() {
    // req:orchestrator.runner/runner-refusals@76e90c84
    let f = fixture();
    f.set("next.1", "a\nb\n");
    f.set("run.a.code", "2");
    f.set("run.a.err", "clm run: git fetch: no network\n");
    let out = f.run(Some("acceptEdits"), true);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let all = text(&out.stdout) + &text(&out.stderr);
    assert!(all.contains("git failed: git fetch: no network"), "{out:?}");
    assert!(!f.clm_calls().iter().any(|c| c.starts_with("run b")));
}

#[test]
fn scenario_first_look_fails_in_git() {
    // req:orchestrator.runner/runner-refusals@76e90c84
    let f = fixture();
    f.set("next.code", "2");
    f.set("next.err", "clm next: git fetch: no network\n");
    let out = f.run(Some("acceptEdits"), false);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(
        text(&out.stderr).contains("clm next: git fetch: no network"),
        "{out:?}"
    );
    assert!(!f.clm_calls().iter().any(|c| c.starts_with("run")));
}

#[test]
fn first_look_refused_exits_1() {
    // req:orchestrator.runner/runner-refusals@76e90c84
    let f = fixture();
    f.set("next.code", "1");
    f.set("next.err", "clm next: settings: bad\n");
    let out = f.run(Some("acceptEdits"), false);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        text(&out.stderr).contains("clm next: settings: bad"),
        "{out:?}"
    );
}

#[test]
fn later_look_failure_goes_on() {
    // req:orchestrator.runner/runner-refusals@76e90c84
    let f = fixture();
    f.set("next.1", "a\n");
    f.set("next.2.code", "2");
    f.set("next.err", "clm next: git fetch: no network\n");
    // `a` runs until a look after the failed one has happened.
    f.set("hold", "");
    let child = f.spawn(&f.env(Some("acceptEdits"), true));
    f.wait_for(|| f.looks() >= 3);
    fs::remove_file(f.fake.join("hold")).unwrap();
    let out = finish(child);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let all = text(&out.stdout) + &text(&out.stderr);
    assert!(
        all.contains("clm next failed: clm next: git fetch: no network"),
        "{out:?}"
    );
    assert!(
        f.clm_calls()
            .iter()
            .filter(|c| c.starts_with("next"))
            .count()
            >= 3
    );
}

#[test]
fn scenario_landed_build_finished() {
    // req:orchestrator.runner/runner-finish@d19ab622
    let f = fixture();
    f.set("next.1", "a\n");
    f.set("claude.out", "landed 1a2b3c4\n");
    let out = f.run(Some("acceptEdits"), true);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let stdout = text(&out.stdout);
    assert!(stdout.contains("freed a\n"), "{out:?}");
    assert!(stdout.contains("finished a: landed 1a2b3c4\n"), "{out:?}");
    let calls = f.clm_calls();
    let finish = calls
        .iter()
        .position(|c| c.starts_with("run --finish a "))
        .unwrap();
    assert!(calls[finish + 1].starts_with("next "), "{calls:?}");
}

#[test]
fn finished_session_looks_again_without_waiting() {
    // req:orchestrator.runner/runner-finish@d19ab622
    let f = fixture();
    f.set("next.1", "a\n");
    let mut env = f.env(Some("acceptEdits"), true);
    env.insert("CLM_RUNNER_INTERVAL_MS".into(), "60000".into());
    let start = Instant::now();
    let out = finish(f.spawn(&env));
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        start.elapsed() < Duration::from_secs(20),
        "{:?}",
        start.elapsed()
    );
}

#[test]
fn scenario_silent_failure() {
    // req:orchestrator.runner/runner-finish@d19ab622
    let f = fixture();
    f.set("next.1", "a\n");
    f.set("claude.code", "1");
    let out = f.run(Some("acceptEdits"), true);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        f.clm_calls()
            .iter()
            .any(|c| c.starts_with("run --finish a "))
    );
    assert!(
        text(&out.stdout).contains("finished a: exit 1\n"),
        "{out:?}"
    );
}

#[test]
fn scenario_clean_leftover_replaced() {
    // req:orchestrator.runner/runner-worktree-leftover@9bb429a7
    let f = fixture();
    let wt = f.worktree("a");
    git(
        &f.work,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            wt.to_str().unwrap(),
            "HEAD",
        ],
    );
    // An ignored file doesn't make it dirty.
    fs::create_dir_all(wt.join(".clm")).unwrap();
    fs::write(wt.join(".clm/x.log"), "").unwrap();
    // A commit on main after the leftover was made, so the new worktree is new.
    fs::write(f.work.join("README"), "y\n").unwrap();
    git(&f.work, &["commit", "-q", "-am", "next"]);
    git(&f.work, &["push", "-q", "origin", "main"]);
    git(&f.work, &["fetch", "-q", "origin"]);
    f.set("next.1", "a\n");
    let out = f.run(Some("acceptEdits"), true);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(f.fake.join("claude.a.args").exists(), "{out:?}");
    assert_eq!(
        git(&wt, &["rev-parse", "HEAD"]),
        git(&f.work, &["rev-parse", "origin/main"])
    );
}

#[test]
fn scenario_dirty_leftover_kept() {
    // req:orchestrator.runner/runner-worktree-leftover@9bb429a7
    let f = fixture();
    let wt = f.worktree("a");
    git(
        &f.work,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            wt.to_str().unwrap(),
            "HEAD",
        ],
    );
    fs::write(wt.join("README"), "changed\n").unwrap();
    f.set("next.1", "a\n");
    let out = f.run(Some("acceptEdits"), true);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(!f.fake.join("claude.a.args").exists());
    let all = text(&out.stdout) + &text(&out.stderr);
    assert!(
        all.contains(&format!("kept {}: uncommitted changes", wt.display())),
        "{out:?}"
    );
    assert!(
        f.clm_calls()
            .iter()
            .any(|c| c.starts_with("run --finish a "))
    );
    assert_eq!(fs::read_to_string(wt.join("README")).unwrap(), "changed\n");
}

#[test]
fn untracked_leftover_kept() {
    // req:orchestrator.runner/runner-worktree-leftover@9bb429a7
    let f = fixture();
    let wt = f.worktree("a");
    git(
        &f.work,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            wt.to_str().unwrap(),
            "HEAD",
        ],
    );
    fs::write(wt.join("new-file"), "x\n").unwrap();
    f.set("next.1", "a\n");
    let out = f.run(Some("acceptEdits"), true);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(!f.fake.join("claude.a.args").exists());
    assert!(wt.join("new-file").exists());
}

#[test]
fn scenario_once_drains_the_queue() {
    // req:orchestrator.runner/runner-once-and-stop@c9603f1e
    let f = fixture();
    f.set("next.1", "a\n");
    let out = f.run(Some("acceptEdits"), true);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let calls = f.clm_calls();
    let runs = calls.iter().filter(|c| c.starts_with("run a ")).count();
    let finishes = calls
        .iter()
        .filter(|c| c.starts_with("run --finish a "))
        .count();
    assert_eq!((runs, finishes), (1, 1), "{calls:?}");
}

#[test]
fn scenario_stopped_by_a_signal() {
    // req:orchestrator.runner/runner-once-and-stop@c9603f1e
    let f = fixture();
    f.set("next", "a\n");
    f.set("trap", "");
    let child = f.spawn(&f.env(Some("acceptEdits"), false));
    f.wait_for(|| f.fake.join("started.a").exists());
    let runs_before = f
        .clm_calls()
        .iter()
        .filter(|c| c.starts_with("run a "))
        .count();
    // SAFETY: plain kill(2) on the child we spawned.
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGTERM) }, 0);
    let out = finish(child);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(f.fake.join("claude.term").exists(), "{out:?}");
    assert!(f.fake.join("child.term").exists(), "{out:?}");
    let calls = f.clm_calls();
    assert!(
        calls.last().unwrap().starts_with("run --finish a "),
        "{calls:?}"
    );
    let runs = calls.iter().filter(|c| c.starts_with("run a ")).count();
    assert_eq!(runs, runs_before);
}

#[test]
fn signal_ends_a_wait_at_once() {
    // req:orchestrator.runner/runner-once-and-stop@c9603f1e
    let f = fixture();
    let mut env = f.env(Some("acceptEdits"), false);
    env.insert("CLM_RUNNER_INTERVAL_MS".into(), "600000".into());
    let child = f.spawn(&env);
    f.wait_for(|| f.looks() >= 1);
    std::thread::sleep(Duration::from_millis(200));
    let start = Instant::now();
    // SAFETY: plain kill(2) on the child we spawned.
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGINT) }, 0);
    let out = finish(child);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "{:?}",
        start.elapsed()
    );
}

#[test]
fn mode_passed_unchecked() {
    // req:orchestrator.runner/runner-prerequisites@fd6e7608
    let f = fixture();
    f.set("next.1", "a\n");
    let out = f.run(Some("no-such-mode"), true);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let args = fs::read_to_string(f.fake.join("claude.a.args")).unwrap();
    assert!(
        args.contains("--permission-mode\0no-such-mode\0"),
        "{args:?}"
    );
}

impl Fixture {
    /// The `claude plugin` calls the runner made, each with the directory it ran in.
    fn plugin_calls(&self) -> Vec<String> {
        fs::read_to_string(self.fake.join("plugin.log"))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }
}

#[test]
fn scenario_plugin_missing() {
    // req:orchestrator.runner/runner-needs-guard@df4cfbe1
    let f = fixture();
    f.set(
        "plugins",
        r#"[{"id":"clm-panel@clm","scope":"user","enabled":true}]"#,
    );
    let out = f.run(Some("acceptEdits"), false);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let err = text(&out.stderr);
    assert!(
        err.contains("`clm` plugin") && err.contains("guard"),
        "{out:?}"
    );
    assert!(f.clm_calls().is_empty(), "{:?}", f.clm_calls());
}

#[test]
fn scenario_plugin_disabled() {
    // req:orchestrator.runner/runner-needs-guard@df4cfbe1
    let f = fixture();
    f.set(
        "plugins",
        r#"[{"id":"clm@clm","scope":"user","enabled":false}]"#,
    );
    let out = f.run(Some("acceptEdits"), false);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(text(&out.stderr).contains("`clm` plugin"), "{out:?}");
    assert!(f.clm_calls().is_empty(), "{:?}", f.clm_calls());
}

#[test]
fn scenario_plugin_enabled() {
    // req:orchestrator.runner/runner-needs-guard@df4cfbe1
    let f = fixture();
    f.set(
        "plugins",
        &format!(
            r#"[{{"id":"clm@clm","scope":"project","enabled":true,"projectPath":"{}","projectEnabled":true}}]"#,
            f.work.display()
        ),
    );
    let out = f.run(Some("acceptEdits"), true);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(f.looks(), 1, "{:?}", f.clm_calls());
    assert_eq!(
        f.plugin_calls(),
        vec![format!("plugin list --json in {}", f.work.display())]
    );
}

#[test]
fn plugin_list_runs_in_the_main_worktree() {
    // req:orchestrator.runner/runner-needs-guard@df4cfbe1
    let f = fixture();
    let linked = f.work.parent().unwrap().join("linked");
    git(
        &f.work,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            linked.to_str().unwrap(),
        ],
    );
    let mut env = f.env(Some("acceptEdits"), true);
    env.insert("CLM_RUNNER_ROOT".into(), linked.display().to_string());
    let out = finish(f.spawn(&env));
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        f.plugin_calls(),
        vec![format!("plugin list --json in {}", f.work.display())]
    );
    // From the linked worktree, a project setting for the main worktree is still the one
    // that counts.
    f.set(
        "plugins",
        &format!(
            r#"[{{"id":"clm@clm","scope":"project","enabled":true,"projectPath":"{}","projectEnabled":false}}]"#,
            f.work.display()
        ),
    );
    let out = finish(f.spawn(&env));
    assert_eq!(out.status.code(), Some(1), "{out:?}");
}

#[test]
fn project_disabled_in_the_main_worktree_refused() {
    // req:orchestrator.runner/runner-needs-guard@df4cfbe1
    let f = fixture();
    f.set(
        "plugins",
        &format!(
            r#"[{{"id":"clm@clm","scope":"project","enabled":true,"projectPath":"{}","projectEnabled":false}}]"#,
            f.work.display()
        ),
    );
    let out = f.run(Some("acceptEdits"), true);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(f.clm_calls().is_empty(), "{:?}", f.clm_calls());
}

#[test]
fn plugin_list_failure_printed() {
    // req:orchestrator.runner/runner-needs-guard@df4cfbe1
    let f = fixture();
    f.set("plugins.code", "3");
    f.set("plugins.err", "not signed in\n");
    let out = f.run(Some("acceptEdits"), true);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(text(&out.stderr).contains("not signed in"), "{out:?}");
    assert!(f.clm_calls().is_empty(), "{:?}", f.clm_calls());
}

#[test]
fn plugin_list_after_the_prerequisites() {
    // req:orchestrator.runner/runner-needs-guard@df4cfbe1
    let f = fixture();
    let out = f.run(None, true);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(f.plugin_calls().is_empty(), "{:?}", f.plugin_calls());
}
