//! The hooks that are not the guard, through `clm-hooks`' calls of `clm hook session-start`
//! and `clm hook autosync` (`commands-in-crate`): session start, the defect rule and autosync.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

/// What a call of a hook wrote and returned.
#[derive(Debug)]
struct Output {
    status: Status,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// The status a call returned, read as a process's.
#[derive(Debug)]
struct Status(u8);

impl Status {
    fn code(&self) -> Option<i32> {
        Some(i32::from(self.0))
    }
}

fn hook(name: &str, event: &str) -> Output {
    hook_env(name, event, &[])
}

/// Calls `clm hook <name>` with `event` as its standard input and `args` as its arguments,
/// no user settings, and `env` through the call's lookup.
fn hook_args(name: &str, args: &[&str], event: &str, env: &[(&str, &str)]) -> Output {
    // The developer's own settings must not leak in either.
    let config = TempDir::new().unwrap();
    let mut vars: Vec<(String, String)> = vec![(
        "XDG_CONFIG_HOME".into(),
        config.path().display().to_string(),
    )];
    vars.extend(env.iter().map(|(k, v)| (k.to_string(), v.to_string())));
    let lookup = |key: &str| -> Option<String> {
        vars.iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    };
    // The developer's own git config (signing, hooks, helpers) must not leak into the `git`
    // the hook runs, which inherits this process's environment.
    // Set once, to the same values, before any call: nextest runs each test in a process of
    // its own (`ci/nextest-runner`), and under `cargo test` no thread changes them after.
    static GIT_ISOLATED: std::sync::Once = std::sync::Once::new();
    GIT_ISOLATED.call_once(|| {
        // SAFETY: as above.
        unsafe {
            std::env::set_var("GIT_CONFIG_GLOBAL", "/dev/null");
            std::env::set_var("GIT_CONFIG_NOSYSTEM", "1");
        }
    });
    // The hook finds its repository from the event's `cwd`: the call runs in a directory
    // that is no CLM repository, so nothing but the event can lead it to one.
    let elsewhere = TempDir::new().unwrap();
    let cwd = elsewhere.path();
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let root = clm_hooks::clm_root(cwd);
    let code = match name {
        "session-start" => clm_hooks::session_start_command(
            args,
            root,
            cwd,
            event,
            &lookup,
            &mut stdout,
            &mut stderr,
        ),
        "autosync" => {
            clm_hooks::autosync_command(args, root, cwd, event, &lookup, &mut stdout, &mut stderr)
        }
        _ => panic!("no hook {name}"),
    };
    Output {
        status: Status(code),
        stdout,
        stderr,
    }
}

fn hook_env(name: &str, event: &str, env: &[(&str, &str)]) -> Output {
    hook_args(name, &[], event, env)
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
        "build",
        "check",
        "claim",
        "defect",
        "gate",
        "land",
        "pr",
        "rebase",
        "setup",
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
    // req:plugin.hooks/session-upgrade-reminder@c217417c
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
    // req:plugin.hooks/defect-rule@8551aeef
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
    assert!(rule.contains("clm defect"), "{text}");
    assert!(rule.contains("<<'EOF'"), "{text}");
    assert!(
        !rule.contains("--fix") && !rule.contains("backlog/"),
        "{text}"
    );
    // backlog: the backlog route.
    let text = start(&clm_repo(Some("defects = \"backlog\"\n")));
    let rule = &text[text.find("\nclaims\n").expect("claims section")..];
    for needle in [
        "clm defect",
        "backlog/",
        "clm claim <component> --fix backlog/<slug>",
    ] {
        assert!(rule.contains(needle), "{needle}: {text}");
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

#[test]
fn hook_calls_take_the_command_line() {
    // req:plugin.hooks/commands-in-crate@305d2b91
    let repo = clm_repo(None);
    let event = serde_json::json!({ "cwd": repo.path() }).to_string();
    // `--agent-harness codex`: the call writes Codex's form to its output stream.
    let out = hook_args("session-start", &["--agent-harness", "codex"], &event, &[]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ctx = v["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(ctx.contains("\nCLM status\n"), "{ctx}");
    assert!(out.stderr.is_empty(), "{out:?}");
    // The settings come through the call's lookup: `CLM_DEFECTS=off` drops the rule.
    let out = hook_env("session-start", &event, &[("CLM_DEFECTS", "off")]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("\nclaims\n") && !text.contains("clm defect"),
        "{text}"
    );
    // An agent harness CLM doesn't know: nothing to do, and no failure.
    for name in ["session-start", "autosync"] {
        let out = hook_args(name, &["--agent-harness", "nonesuch"], &event, &[]);
        assert_eq!(
            (out.status.code(), out.stdout.len(), out.stderr.len()),
            (Some(0), 0, 0),
            "{name}: {out:?}"
        );
    }
}

#[test]
fn hook_calls_refuse_an_option_they_do_not_have() {
    // req:plugin.hooks/commands-in-crate@305d2b91
    let repo = clm_repo(None);
    let event = serde_json::json!({ "cwd": repo.path() }).to_string();
    for (name, usage) in [
        ("session-start", "clm hook session-start"),
        ("autosync", "clm hook autosync"),
    ] {
        let out = hook_args(name, &["--nonesuch"], &event, &[]);
        assert_eq!(out.status.code(), Some(2), "{name}: {out:?}");
        assert!(out.stdout.is_empty(), "{name}: {out:?}");
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(err.contains("--nonesuch") && err.contains(usage), "{err}");
    }
}
