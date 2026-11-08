//! The hooks that are not the guard, run as the binary: session start, the defect rule and
//! autosync.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{PoisonError, RwLock};

use tempfile::TempDir;

/// A child forked while another test thread writes an executable inherits the write fd
/// until it execs, and exec'ing that file meanwhile fails with ETXTBSY. So writers of
/// executables hold this lock exclusively, and every spawn shares it.
static FORKS: RwLock<()> = RwLock::new(());

fn spawn(command: &mut Command) -> Child {
    let _shared = FORKS.read().unwrap_or_else(PoisonError::into_inner);
    command.spawn().unwrap()
}

fn hook(name: &str, event: &str) -> Output {
    hook_env(name, event, &[])
}

fn hook_env(name: &str, event: &str, env: &[(&str, &str)]) -> Output {
    // The developer's own settings must not leak in either.
    let config = TempDir::new().unwrap();
    let mut child = spawn(
        Command::new(env!("CARGO_BIN_EXE_clm"))
            .args(["hook", name])
            .env("XDG_CONFIG_HOME", config.path())
            .env_remove("CLM_AUTOSYNC")
            .env_remove("CLM_DEFECTS")
            // The developer's own git config (signing, hooks, helpers) must not leak in.
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .envs(env.iter().copied())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
    );
    child
        .stdin
        .take()
        .unwrap()
        .write_all(event.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn session_start_command() {
    // req:plugin.hooks/session-status@1e909501
    // A repository of its own, not this one: a doc-only PR skips the tests
    // (ci/light-checks).
    let repo = clm_repo(None);
    let event = serde_json::json!({ "cwd": repo.path().canonicalize().unwrap() }).to_string();
    let out = hook("session-start", &event);
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("\nCLM status\n")
            && text.contains("\nchanges\n")
            && text.contains("\nclaims\n"),
        "{text}"
    );
    let elsewhere = TempDir::new().unwrap();
    let event = serde_json::json!({ "cwd": elsewhere.path() }).to_string();
    let out = hook("session-start", &event);
    assert_eq!((out.status.code(), out.stdout.len()), (Some(0), 0));
}

/// A git repository with a valid `specs/` and, when given, a `clm.toml`.
fn clm_repo(settings: Option<&str>) -> TempDir {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"]);
    fs::create_dir_all(dir.path().join("specs")).unwrap();
    fs::write(
        dir.path().join("specs/SPEC.md"),
        "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n",
    )
    .unwrap();
    // An up-to-date project allows the unattended `clm` rules (`upgrade/migration-unattended-allow-rules`).
    let rules: Vec<String> = [
        "apply",
        "branch",
        "check",
        "claim",
        "defect",
        "gate",
        "land",
        "pr",
        "rebase",
        "verify",
        "workaround",
    ]
    .iter()
    .flat_map(|s| [format!("Bash(clm {s})"), format!("Bash(clm {s} *)")])
    .collect();
    fs::create_dir_all(dir.path().join(".claude")).unwrap();
    fs::write(
        dir.path().join(".claude/settings.json"),
        serde_json::json!({ "permissions": { "allow": rules } }).to_string(),
    )
    .unwrap();
    if let Some(settings) = settings {
        fs::write(dir.path().join("clm.toml"), settings).unwrap();
    }
    dir
}

#[test]
fn session_start_reminds_to_upgrade() {
    // req:plugin.hooks/session-upgrade-reminder@5a5a1830
    let start = |repo: &TempDir| {
        let event = serde_json::json!({ "cwd": repo.path() }).to_string();
        let out = hook("session-start", &event);
        assert_eq!(out.status.code(), Some(0), "{out:?}");
        String::from_utf8(out.stdout).unwrap()
    };
    // Nothing pending: no line.
    let text = start(&clm_repo(None));
    assert!(!text.contains("/clm:upgrade"), "{text}");
    // A project a newer `clm` upgraded: the line, after the status.
    let repo = clm_repo(Some("upgraded = \"20990101-from-the-future\"\n"));
    let text = start(&repo);
    let claims = text.find("\nclaims\n").expect("claims section");
    let line = text.find("needs a newer `clm`").expect("reminder");
    assert!(line > claims, "{text}");
    // Even when `clm status` fails.
    fs::write(repo.path().join("specs/SPEC.md"), "not a spec").unwrap();
    let text = start(&repo);
    assert!(text.contains("needs a newer `clm`"), "{text}");
    assert!(!text.contains("CLM status"), "{text}");
}

#[test]
fn session_start_states_the_defect_rule() {
    // req:plugin.hooks/defect-rule@c40c6863
    let start = |repo: &TempDir| {
        let event = serde_json::json!({ "cwd": repo.path() }).to_string();
        let out = hook("session-start", &event);
        assert_eq!(out.status.code(), Some(0), "{out:?}");
        String::from_utf8(out.stdout).unwrap()
    };
    // asking: no settings, so `defects` is `ask`.
    let text = start(&clm_repo(None));
    let claims = text.find("\nclaims\n").expect("claims section");
    let rule = &text[claims..];
    // Where to report, what counts and what doesn't, and that a report is process only.
    for needle in [
        "clm defect",
        "## CLM defects",
        "CLI, CI, gate rules, plugin, skills or method",
        "is not one",
        "never includes the project's files, code, specs or output",
    ] {
        assert!(rule.contains(needle), "{needle} after claims: {text}");
    }
    // With `ask`, the report goes in a `<<'EOF'` heredoc, and nothing of the fix-first rule.
    assert!(rule.contains("<<'EOF'"), "{text}");
    assert!(
        rule.contains("the one form the guard shows the human"),
        "{text}"
    );
    assert!(!text.contains("--fix"), "{text}");
    // backlog: where reports go, and fix-first.
    let text = start(&clm_repo(Some("defects = \"backlog\"\n")));
    assert!(!text.contains("<<'EOF'"), "{text}");
    for needle in [
        "clm defect",
        "backlog/",
        "kind: bug",
        "fold-small-defer-large",
        "--component",
        "clm claim <component> --fix backlog/<slug>",
        "landed",
        "spec change",
        "code-fix-resolves-bug",
        "large amount of work",
        "unattended session folds what fits and reports the rest",
    ] {
        assert!(text.contains(needle), "{needle}: {text}");
    }
    // off
    let text = start(&clm_repo(Some("defects = \"off\"\n")));
    assert!(text.contains("\nclaims\n"), "{text}");
    assert!(!text.contains("clm defect"), "{text}");
    // bad settings
    let text = start(&clm_repo(Some("defects = \"always\"\n")));
    assert!(
        text.contains("clm.toml") && text.contains("defects"),
        "{text}"
    );
    assert!(!text.contains("clm defect"), "{text}");
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap().trim().to_owned()
}

/// A CLM repository on `spec/x` with an uncommitted delta, cloned from a bare `origin`.
fn spec_branch() -> (TempDir, PathBuf, PathBuf) {
    let dir = TempDir::new().unwrap();
    let (origin, work) = (dir.path().join("origin"), dir.path().join("work"));
    fs::create_dir_all(&origin).unwrap();
    git(&origin, &["init", "-q", "--bare", "-b", "main"]);
    git(
        dir.path(),
        &["clone", "-q", origin.to_str().unwrap(), "work"],
    );
    git(&work, &["config", "user.name", "t"]);
    git(&work, &["config", "user.email", "t@t"]);
    fs::create_dir_all(work.join("specs")).unwrap();
    fs::write(
        work.join("specs/SPEC.md"),
        "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n",
    )
    .unwrap();
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-qm", "base"]);
    git(&work, &["push", "-q", "origin", "HEAD:main"]);
    git(&work, &["checkout", "-q", "-b", "spec/x"]);
    fs::create_dir_all(work.join("changes/x/deltas")).unwrap();
    fs::write(
        work.join("changes/x/deltas/a.md"),
        "---\ncomponent: a\n---\n",
    )
    .unwrap();
    (dir, origin, work)
}

fn autosynced(origin: &Path) -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "--verify", "-q", "refs/heads/autosync/spec/x"])
        .current_dir(origin)
        .output()
        .unwrap();
    out.status
        .success()
        .then(|| String::from_utf8(out.stdout).unwrap().trim().to_owned())
}

#[test]
fn autosync_hook_pushes_spec_work_silently() {
    // req:plugin.hooks/autosync-hook@3a75312d
    let (_dir, origin, work) = spec_branch();
    // From a subdirectory: the hook finds the repository as `session-start` does.
    let cwd = work.join("changes/x");
    let event = serde_json::json!({ "hook_event_name": "Stop", "cwd": cwd }).to_string();
    let out = hook("autosync", &event);
    assert_eq!(
        (out.status.code(), out.stdout.len(), out.stderr.len()),
        (Some(0), 0, 0),
        "{out:?}"
    );
    let tip = autosynced(&origin).expect("pushed");
    let files = git(&origin, &["ls-tree", "-r", "--name-only", &tip]);
    assert!(
        files.lines().any(|f| f == "changes/x/deltas/a.md"),
        "{files}"
    );
}

#[test]
fn autosync_hook_turned_off() {
    // req:plugin.hooks/autosync-hook@3a75312d
    let (_dir, origin, work) = spec_branch();
    let event = serde_json::json!({ "cwd": work }).to_string();
    for off in ["0", "false"] {
        let out = hook_env("autosync", &event, &[("CLM_AUTOSYNC", off)]);
        assert_eq!(out.status.code(), Some(0));
        assert_eq!(autosynced(&origin), None, "CLM_AUTOSYNC={off}");
    }
    // A value no setting accepts: the settings can't load, so autosync stays off.
    let out = hook_env("autosync", &event, &[("CLM_AUTOSYNC", "maybe")]);
    assert_eq!(
        (out.status.code(), out.stdout.len(), out.stderr.len()),
        (Some(0), 0, 0),
        "{out:?}"
    );
    assert_eq!(autosynced(&origin), None, "CLM_AUTOSYNC=maybe");
}

#[test]
fn autosync_hook_turned_off_in_the_repository() {
    // req:plugin.hooks/autosync-hook@3a75312d
    let (_dir, origin, work) = spec_branch();
    fs::write(work.join("clm.toml"), "autosync = false\n").unwrap();
    let event = serde_json::json!({ "cwd": work }).to_string();
    let out = hook("autosync", &event);
    assert_eq!(
        (out.status.code(), out.stdout.len(), out.stderr.len()),
        (Some(0), 0, 0),
        "{out:?}"
    );
    assert_eq!(autosynced(&origin), None);
    // The repository's file loses to the variable.
    let out = hook_env("autosync", &event, &[("CLM_AUTOSYNC", "1")]);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        autosynced(&origin).is_some(),
        "CLM_AUTOSYNC=1 overrides clm.toml"
    );
}

#[test]
fn autosync_hook_stays_silent_when_it_cannot_push() {
    // req:plugin.hooks/autosync-hook@3a75312d
    let (dir, _origin, work) = spec_branch();
    let gone = dir.path().join("gone");
    git(
        &work,
        &["remote", "set-url", "origin", gone.to_str().unwrap()],
    );
    let event = serde_json::json!({ "cwd": work }).to_string();
    let elsewhere = TempDir::new().unwrap();
    for event in [
        event,
        "garbage".to_string(),
        serde_json::json!({ "cwd": elsewhere.path() }).to_string(),
    ] {
        let out = hook("autosync", &event);
        assert_eq!(
            (out.status.code(), out.stdout.len(), out.stderr.len()),
            (Some(0), 0, 0),
            "{event}: {out:?}"
        );
    }
}
