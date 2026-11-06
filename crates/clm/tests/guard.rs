//! `clm hook guard` run as the binary: the reasons it prints and exits with, the `gh` and
//! `git` it asks through, and the time it takes.

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::sync::{PoisonError, RwLock};

use serde_json::Value;
use tempfile::TempDir;

/// A child forked while another test thread writes an executable inherits the write fd
/// until it execs, and exec'ing that file meanwhile fails with ETXTBSY. So writers of
/// executables hold this lock exclusively, and every spawn shares it.
static FORKS: RwLock<()> = RwLock::new(());

fn spawn(command: &mut Command) -> Child {
    let _shared = FORKS.read().unwrap_or_else(PoisonError::into_inner);
    command.spawn().unwrap()
}

fn executable(path: &Path, script: &str) {
    let _exclusive = FORKS.write().unwrap_or_else(PoisonError::into_inner);
    fs::write(path, script).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// A git repository with a valid `specs/` and, when given, a `clm.toml`.
fn clm_repo(settings: Option<&str>) -> TempDir {
    let dir = TempDir::new().unwrap();
    let init = Command::new("git")
        .args(["init", "-q", "-b", "main"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(init.status.success(), "{init:?}");
    fs::create_dir_all(dir.path().join("specs")).unwrap();
    fs::write(
        dir.path().join("specs/SPEC.md"),
        "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n",
    )
    .unwrap();
    if let Some(settings) = settings {
        fs::write(dir.path().join("clm.toml"), settings).unwrap();
    }
    dir
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
fn guard_command_asks_with_the_reason() {
    // req:plugin.hooks.guard/approve-guard@0d27c2be
    let event = |c: &str| {
        serde_json::json!({ "tool_name": "Bash", "tool_input": { "command": c } }).to_string()
    };
    for command in [
        "clm approve spec/x",
        "cargo run -q -p clm -- approve spec/x",
    ] {
        let out = hook("guard", &event(command));
        assert_eq!(
            (out.status.code(), out.stderr.len()),
            (Some(0), 0),
            "{command}: {out:?}"
        );
        let answer: Value = serde_json::from_slice(&out.stdout).expect("ask JSON on stdout");
        let decision = &answer["hookSpecificOutput"];
        assert_eq!(decision["hookEventName"], "PreToolUse", "{answer}");
        assert_eq!(decision["permissionDecision"], "ask", "{answer}");
        let reason = decision["permissionDecisionReason"].as_str().unwrap();
        assert!(reason.contains("humans-review-intent"), "{reason}");
    }
    // approval combined with a push to main: the block wins, on stderr, exit 2.
    let out = hook(
        "guard",
        &event("clm approve spec/x && git push origin main"),
    );
    assert_eq!(
        (out.status.code(), out.stdout.len()),
        (Some(2), 0),
        "{out:?}"
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("guard-blocks-push-to-main"));
    for command in ["gh pr edit 30 --add-label spec-approved", "clm review x"] {
        let out = hook("guard", &event(command));
        assert_eq!(
            (out.status.code(), out.stdout.len(), out.stderr.len()),
            (Some(0), 0, 0),
            "{command}"
        );
    }
    let out = hook("guard", "garbage");
    assert_eq!((out.status.code(), out.stdout.len()), (Some(0), 0));
}

#[test]
fn guard_command_blocks_dispatching_a_release() {
    // req:plugin.hooks.guard/release-guard@e9278dcb
    let event = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": { "command": "gh workflow run release.yml -f version=0.2.0" },
    })
    .to_string();
    let out = hook("guard", &event);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("humans-cut-releases"));
    // dispatching the site
    for command in [
        "gh workflow run site.yml",
        "gh workflow run .github/workflows/Site.YAML",
        "gh workflow run SITE",
    ] {
        let event = serde_json::json!({
            "tool_name": "Bash",
            "tool_input": { "command": command },
        })
        .to_string();
        let out = hook("guard", &event);
        assert_eq!(out.status.code(), Some(2), "{command}: {out:?}");
        assert!(String::from_utf8_lossy(&out.stderr).contains("humans-cut-releases"));
    }
}

/// A `PATH` whose `gh` is `script`, held in `dir`, ahead of the system's.
fn path_with_gh(dir: &Path, script: &str) -> String {
    executable(&dir.join("gh"), script);
    format!("{}:{}", dir.display(), std::env::var("PATH").unwrap())
}

/// A fake `gh` that logs each call's arguments, directory, `GH_REPO` and `GH_HOST` to
/// `calls`; answers `repo view --json url` with `https://ghe.example.com/o/r` when its
/// `GH_REPO` is `o/r` and with `url` otherwise; fails on anything else but `api` as `gh` 2.45
/// fails on `run view --json path`; and answers the REST lookups of runs and jobs by the
/// path's last part.
fn fake_rest_gh(dir: &Path, calls: &Path, url: &str) -> String {
    path_with_gh(
        dir,
        &format!(
            "#!/bin/sh\nprintf '%s|%s|%s|%s\\n' \"$*\" \"$(pwd)\" \"$GH_REPO\" \"$GH_HOST\" >> {calls}\n\
             if [ \"$*\" = 'repo view --json url' ]; then\n\
             if [ \"$GH_REPO\" = o/r ]; then echo '{{\"url\":\"https://ghe.example.com/o/r\"}}'; \
             else echo '{{\"url\":\"{url}\"}}'; fi; exit 0; fi\n\
             if [ \"$1\" != api ]; then echo 'Unknown JSON field: \"path\"' >&2; exit 1; fi\n\
             for last; do :; done\ncase \"$last\" in\n\
             */actions/runs/77) echo '{{\"name\":\"Release\",\"path\":\".github/workflows/release.yml\"}}' ;;\n\
             */actions/runs/76) echo '{{\"name\":\"Publish\",\"path\":\".github/workflows/release.yaml\"}}' ;;\n\
             */actions/runs/78) echo '{{\"name\":\"release\",\"path\":\".github/workflows/publish.yml\"}}' ;;\n\
             */actions/runs/79) echo '{{\"name\":\"Publish\",\"path\":\".github/workflows/release.yml@refs/heads/main\"}}' ;;\n\
             */actions/runs/80) echo '{{\"name\":\"Publish\",\"path\":\".github/workflows/Site.YML\"}}' ;;\n\
             */actions/runs/81) echo '{{\"name\":\"SITE\",\"path\":\".github/workflows/pages.yml\"}}' ;;\n\
             */actions/runs/82) echo '{{\"name\":\"Publish\",\"path\":\".github/workflows/site.yaml@refs/heads/main\"}}' ;;\n\
             */actions/runs/0) echo '{{\"name\":\"CI\",\"path\":\".github/workflows/ci.yml\"}}' ;;\n\
             */actions/runs/42) echo '{{\"name\":\"CI\",\"path\":\".github/workflows/ci.yml\"}}' ;;\n\
             */actions/runs/43) echo '{{\"name\":\"CI\"}}' ;;\n\
             */actions/runs/44) echo '[]' ;;\n\
             */actions/jobs/900) echo '{{\"run_id\":77}}' ;;\n\
             */actions/jobs/901) echo '{{\"run_id\":42}}' ;;\n\
             */actions/jobs/902) echo '{{\"run_id\":\"x\"}}' ;;\n\
             */actions/jobs/903) echo 'not json' ;;\n\
             */actions/jobs/904) echo '{{\"run_id\":0}}' ;;\n\
             *) echo 'HTTP 404: Not Found' >&2; exit 1 ;;\nesac\n",
            calls = calls.display()
        ),
    )
}

/// The calls the fake `gh` logged since the last one, clearing the log.
fn take_calls(calls: &Path) -> Vec<String> {
    let log = fs::read_to_string(calls).unwrap_or_default();
    let _ = fs::remove_file(calls);
    log.lines().map(String::from).collect()
}

#[test]
fn guard_blocks_rerunning_a_release_run() {
    // req:plugin.hooks.guard/release-guard@e9278dcb
    let fake = TempDir::new().unwrap();
    let calls = fake.path().join("calls");
    let path = fake_rest_gh(fake.path(), &calls, "https://github.com/o/r");
    let cwd = TempDir::new().unwrap();
    let env = [("PATH", path.as_str())];
    let guard = |command: &str| guard_in(cwd.path(), command, &env);
    // re-running a release run, and a release workflow by another file or name
    for command in [
        "gh run rerun 77 --failed",
        "gh run rerun 76",
        "gh run rerun 78",
        "gh run rerun 79",
        // a site run is a release run
        "gh run rerun 80",
        "gh run rerun 81",
        "gh run rerun 82",
        // a body a shell runs
        "bash <<'EOF'\ngh run rerun 77\nEOF",
    ] {
        let (code, why) = guard(command);
        assert_eq!(code, Some(2), "{command}: {why}");
        assert!(why.contains("humans-cut-releases"), "{command}: {why}");
    }
    // re-running a CI run, on a `gh` without `run view`'s `path`, in the event's `cwd`
    take_calls(&calls);
    assert_eq!(guard("gh run rerun 42 --failed"), (Some(0), String::new()));
    let dir = cwd.path().canonicalize().unwrap();
    let dir = dir.to_string_lossy();
    let expected = [
        format!("repo view --json url|{dir}||"),
        format!("api --hostname github.com repos/o/r/actions/runs/42|{dir}||"),
    ];
    assert_eq!(take_calls(&calls), expected);
    // re-running a release run's job, in each spelling, and beside a CI run
    for command in [
        "gh run rerun --job 900",
        "gh run rerun -j 900",
        "gh run rerun -j900",
        "gh run rerun --job=900",
        "gh run rerun 42 --job 900",
    ] {
        let (code, why) = guard(command);
        assert_eq!(code, Some(2), "{command}: {why}");
        assert!(why.contains("humans-cut-releases"), "{command}: {why}");
    }
    // re-running a CI run's job: the job names its run
    take_calls(&calls);
    assert_eq!(guard("gh run rerun --job 901"), (Some(0), String::new()));
    let asked: Vec<String> = take_calls(&calls)
        .iter()
        .map(|c| c.split('|').next().unwrap().to_string())
        .collect();
    assert_eq!(
        asked,
        [
            "repo view --json url",
            "api --hostname github.com repos/o/r/actions/jobs/901",
            "api --hostname github.com repos/o/r/actions/runs/42",
        ],
    );
    // the guard stops looking up at the first release run, after resolving the repository
    // once per segment
    for command in [
        "gh run rerun 77 --job 901",
        "gh run rerun 77; gh run rerun 42",
    ] {
        assert_eq!(guard(command).0, Some(2), "{command}");
        assert_eq!(take_calls(&calls).len(), 2, "{command}");
    }
    assert_eq!(guard("gh run rerun 42 --job 901").0, Some(0));
    let made = take_calls(&calls);
    let views = made.iter().filter(|c| c.starts_with("repo view")).count();
    assert_eq!((views, made.len()), (1, 4), "{made:?}");
    // run can't be looked up: an error, a run without `path`, something that isn't a run
    // or a job, a job whose `run_id` isn't a positive integer, a job ID that isn't one
    for command in [
        "gh run rerun 5",
        "gh run rerun 43",
        "gh run rerun 44",
        "gh run rerun --job 905",
        "gh run rerun --job 902",
        "gh run rerun --job 903",
        "gh run rerun --job 904",
        "gh run rerun --job $JOB",
        "gh run rerun $RUN",
        "gh run rerun `cat id`",
    ] {
        let (code, why) = guard(command);
        assert_eq!(code, Some(2), "{command}: {why}");
    }
    let no_gh = TempDir::new().unwrap();
    let path = no_gh.path().to_str().unwrap();
    for command in ["gh run rerun 42", "gh run rerun --job 901"] {
        let (code, why) = guard_in(cwd.path(), command, &[("PATH", path)]);
        assert_eq!(code, Some(2), "{command}: {why}");
    }
    // no run or job ID, or no rerun: nothing to look up, so prose that names a rerun isn't one
    take_calls(&calls);
    for command in [
        "gh run rerun",
        "gh run rerun --failed",
        "gh run list",
        "gh run view 77",
        "git commit -m \"Document gh run rerun failures\"",
        "git commit -m \"Explain gh run rerun -j usage\"",
    ] {
        assert_eq!(guard(command), (Some(0), String::new()), "{command}");
    }
    assert!(!calls.exists(), "gh was called");
}

#[test]
fn guard_looks_a_rerun_up_where_it_runs() {
    // req:plugin.hooks.guard/release-guard@e9278dcb
    let fake = TempDir::new().unwrap();
    let calls = fake.path().join("calls");
    let path = fake_rest_gh(fake.path(), &calls, "https://github.com/p/q");
    let cwd = TempDir::new().unwrap();
    fs::create_dir(cwd.path().join("other")).unwrap();
    let env = [("PATH", path.as_str())];
    let top = cwd.path().canonicalize().unwrap();
    let top = top.to_string_lossy();
    let other = format!("{top}/other");
    // the re-run's repository and host
    for (command, call) in [
        (
            "cd other && gh run rerun 42",
            format!(
                "repo view --json url|{other}||\n\
                 api --hostname github.com repos/p/q/actions/runs/42|{other}||"
            ),
        ),
        (
            "gh run rerun 42 -R o/r",
            format!("api repos/o/r/actions/runs/42|{top}||"),
        ),
        (
            "gh run rerun 42 --repo ghe.example.com/o/r",
            format!("api --hostname ghe.example.com repos/o/r/actions/runs/42|{top}||"),
        ),
        (
            "GH_REPO=o/r gh run rerun 42 -R x/y",
            format!("api repos/x/y/actions/runs/42|{top}|o/r|"),
        ),
        (
            "GH_HOST=ghe.example.com GH_REPO=o/r gh run rerun 42",
            format!(
                "repo view --json url|{top}|o/r|ghe.example.com\n\
                 api --hostname ghe.example.com repos/o/r/actions/runs/42|{top}|o/r|ghe.example.com"
            ),
        ),
        // glued flags, a `cd` it can't follow, and a job looked up on the same host
        (
            "gh run rerun -Ro/r 42",
            format!("api repos/o/r/actions/runs/42|{top}||"),
        ),
        (
            "gh run rerun --repo=o/r 42",
            format!("api repos/o/r/actions/runs/42|{top}||"),
        ),
        (
            "gh run rerun 42 -R x/y -R o/r",
            format!("api repos/o/r/actions/runs/42|{top}||"),
        ),
        (
            "cd other; cd $W && gh run rerun 42",
            format!(
                "repo view --json url|{top}||\n\
                 api --hostname github.com repos/p/q/actions/runs/42|{top}||"
            ),
        ),
        (
            "cd nonesuch && gh run rerun 42",
            format!(
                "repo view --json url|{top}||\n\
                 api --hostname github.com repos/p/q/actions/runs/42|{top}||"
            ),
        ),
        (
            "gh run rerun -j 901 -R h.example/o/r",
            format!(
                "api --hostname h.example repos/o/r/actions/jobs/901|{top}||\n\
                 api --hostname h.example repos/o/r/actions/runs/42|{top}||"
            ),
        ),
    ] {
        take_calls(&calls);
        assert_eq!(
            guard_in(cwd.path(), command, &env),
            (Some(0), String::new()),
            "{command}"
        );
        assert_eq!(take_calls(&calls).join("\n"), call, "{command}");
    }
    // a repository that can't be read
    for command in [
        "gh run rerun 42 -R https://github.com/o/r",
        "gh run rerun 42 -R r",
        "gh run rerun 42 -R o//r",
    ] {
        let (code, why) = guard_in(cwd.path(), command, &env);
        assert_eq!(code, Some(2), "{command}: {why}");
    }
}

#[test]
fn guard_blocks_a_rerun_whose_repository_cant_be_resolved() {
    // req:plugin.hooks.guard/release-guard@e9278dcb
    // a repository that can't be resolved: `repo view` fails, or its `url` isn't
    // `<scheme>://<host>/<owner>/<repo>`
    let api_ci = "if [ \"$1\" = api ]; then \
                  echo '{\"name\":\"CI\",\"path\":\".github/workflows/ci.yml\"}'; exit 0; fi\n";
    let cwd = TempDir::new().unwrap();
    for view in [
        "echo 'no git remotes found' >&2; exit 1",
        "echo '{\"url\":\"o/r\"}'",
        "echo '{\"url\":\"https://github.com/o/r/\"}'",
        "echo '{\"url\":\"https://github.com/o\"}'",
        "echo '{\"url\":\"https:///o/r\"}'",
        "echo '{}'",
    ] {
        let fake = TempDir::new().unwrap();
        let path = path_with_gh(fake.path(), &format!("#!/bin/sh\n{api_ci}{view}\n"));
        let (code, why) = guard_in(cwd.path(), "gh run rerun 42", &[("PATH", &path)]);
        assert_eq!(code, Some(2), "{view}: {why}");
        assert!(why.contains("humans-cut-releases"), "{view}: {why}");
    }
    // The same fake, answering `repo view` well, lets the CI run's re-run through.
    let fake = TempDir::new().unwrap();
    let view = "echo '{\"url\":\"https://github.com/o/r\"}'";
    let path = path_with_gh(fake.path(), &format!("#!/bin/sh\n{api_ci}{view}\n"));
    assert_eq!(
        guard_in(cwd.path(), "gh run rerun 42", &[("PATH", &path)]),
        (Some(0), String::new())
    );
}

#[test]
fn guard_answers_a_hanging_lookup_within_its_budget() {
    // req:plugin.hooks.guard/release-guard@e9278dcb
    // One never answers `repo view`, one answers it but never `api`. Each writes its process
    // ID to `pids` and then `exec`s its sleep, so the sleep has that ID.
    let hanging = TempDir::new().unwrap();
    let pids = hanging.path().join("pids");
    let path = path_with_gh(
        hanging.path(),
        &format!(
            "#!/bin/sh\necho $$ >> '{}'\nexec sleep 60\n",
            pids.display()
        ),
    );
    let hanging_api = TempDir::new().unwrap();
    let api_pids = hanging_api.path().join("pids");
    let api_path = path_with_gh(
        hanging_api.path(),
        &format!(
            "#!/bin/sh\nif [ \"$1\" = repo ]; then \
             echo '{{\"url\":\"https://github.com/o/r\"}}'; exit 0; fi\n\
             echo $$ >> '{}'\nexec sleep 60\n",
            api_pids.display()
        ),
    );
    // Each lookup answers a CI job or run within 20 seconds, but two don't fit in one
    // shared budget.
    let slow = TempDir::new().unwrap();
    let slow_path = path_with_gh(
        slow.path(),
        "#!/bin/sh\nif [ \"$1\" = repo ]; then \
         echo '{\"url\":\"https://github.com/o/r\"}'; exit 0; fi\n\
         sleep 20\nfor last; do :; done\ncase \"$last\" in\n\
         */jobs/*) echo '{\"run_id\":42}' ;;\n\
         *) echo '{\"name\":\"CI\",\"path\":\".github/workflows/ci.yml\"}' ;;\nesac\n",
    );
    let cwd = TempDir::new().unwrap();
    let started = std::time::Instant::now();
    std::thread::scope(|s| {
        let runs: Vec<_> = [
            ("gh run rerun 42", path.as_str()),
            ("gh run rerun --job 900", path.as_str()),
            ("gh run rerun 42", api_path.as_str()),
            ("gh run rerun --job 900", api_path.as_str()),
            ("gh run rerun --job 901", slow_path.as_str()),
        ]
        .into_iter()
        .map(|(command, path)| {
            let cwd = cwd.path();
            s.spawn(move || (command, guard_in(cwd, command, &[("PATH", path)])))
        })
        .collect();
        for run in runs {
            let (command, (code, why)) = run.join().unwrap();
            assert_eq!(code, Some(2), "{command}: {why}");
            assert!(why.contains("humans-cut-releases"), "{command}: {why}");
        }
    });
    assert!(started.elapsed().as_secs() < 35, "{:?}", started.elapsed());
    // Each hung lookup was killed, not left running after its guard exited.
    for pids in [&pids, &api_pids] {
        let pids = fs::read_to_string(pids).unwrap();
        let pids: Vec<&str> = pids.split_whitespace().collect();
        assert_eq!(pids.len(), 2, "{pids:?}");
        for pid in pids {
            assert!(
                gone(pid),
                "gh lookup {pid} still running after the guard exited"
            );
        }
    }
}

#[test]
fn guard_blocks_a_push_git_does_not_answer() {
    // req:plugin.hooks.guard/push-guard@c75aa114
    // a push git doesn't answer: the fake `git` writes its process ID to `pids` and then
    // `exec`s its sleep, so the sleep has that ID.
    let fake = TempDir::new().unwrap();
    let pids = fake.path().join("pids");
    executable(
        &fake.path().join("git"),
        &format!(
            "#!/bin/sh\necho $$ >> '{}'\nexec sleep 60\n",
            pids.display()
        ),
    );
    let path = format!(
        "{}:{}",
        fake.path().display(),
        std::env::var("PATH").unwrap()
    );
    let cwd = TempDir::new().unwrap();
    let started = std::time::Instant::now();
    // After a `cd` the guard couldn't follow, the reason doesn't change.
    let (code, why) = guard_in(cwd.path(), "cd nowhere && git push", &[("PATH", &path)]);
    assert_eq!(code, Some(2), "{why}");
    assert!(started.elapsed().as_secs() < 25, "{:?}", started.elapsed());
    assert!(
        why.contains("guard-blocks-push-to-main")
            && why.contains("didn't answer in time")
            && why.contains("HEAD:<branch>")
            && !why.contains("couldn't follow"),
        "{why}"
    );
    let ran = fs::read_to_string(&pids).unwrap();
    for pid in ran.split_whitespace() {
        assert!(
            gone(pid),
            "git query {pid} still running after the guard exited"
        );
    }
    // An explicit branch needs no query.
    fs::remove_file(&pids).unwrap();
    let (code, why) = guard_in(cwd.path(), "git push origin impl/x/b", &[("PATH", &path)]);
    assert_eq!(code, Some(0), "{why}");
    assert!(!pids.exists(), "the fake git ran");
}

#[test]
fn guard_push_queries_share_one_budget() {
    // req:plugin.hooks.guard/push-guard@c75aa114
    // Each query takes 4 seconds and answers nothing: a segment's three fit in 20 seconds,
    // two segments' six don't. The fifth query is killed at 20 seconds, and the sixth is
    // never started.
    let fake = TempDir::new().unwrap();
    let calls = fake.path().join("calls");
    executable(
        &fake.path().join("git"),
        &format!(
            "#!/bin/sh\necho \"$*\" >> '{}'\nsleep 4\nexit 1\n",
            calls.display()
        ),
    );
    let path = format!(
        "{}:{}",
        fake.path().display(),
        std::env::var("PATH").unwrap()
    );
    let cwd = TempDir::new().unwrap();
    let started = std::time::Instant::now();
    let (code, why) = guard_in(cwd.path(), "git push; git push", &[("PATH", &path)]);
    assert_eq!(code, Some(2), "{why}");
    assert!(why.contains("didn't answer in time"), "{why}");
    assert!(started.elapsed().as_secs() < 25, "{:?}", started.elapsed());
    let calls = fs::read_to_string(&calls).unwrap();
    assert_eq!(calls.lines().count(), 5, "{calls}");
}

/// Whether no live process `pid` exists (a zombie counts as gone), waiting up to two seconds
/// for one that is exiting.
fn gone(pid: &str) -> bool {
    (0..20).any(|_| {
        let alive = Command::new("sh")
            .args(["-c", &format!("ps -o stat= -p {pid} | grep -qv Z")])
            .status()
            .unwrap()
            .success();
        if alive {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        !alive
    })
}

#[test]
fn guard_command_blocks_pushing_to_main() {
    // req:plugin.hooks.guard/push-guard@c75aa114
    let event = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": { "command": "git push origin HEAD:main" },
    })
    .to_string();
    let out = hook("guard", &event);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("guard-blocks-push-to-main"));
}

/// The guard's answer to `command` run in `cwd` with `env`: its exit code, and the `ask`
/// reason or the block's stderr.
fn guard_in(cwd: &Path, command: &str, env: &[(&str, &str)]) -> (Option<i32>, String) {
    let event = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": { "command": command },
        "cwd": cwd,
    })
    .to_string();
    let out = hook_env("guard", &event, env);
    if out.stdout.is_empty() {
        let stderr = String::from_utf8(out.stderr).unwrap();
        return (out.status.code(), stderr);
    }
    let answer: Value = serde_json::from_slice(&out.stdout).expect("ask JSON on stdout");
    assert_eq!(answer["hookSpecificOutput"]["permissionDecision"], "ask");
    let reason = answer["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .unwrap()
        .to_owned();
    (out.status.code(), reason)
}

#[test]
fn defect_ask_says_pull_request() {
    // req:plugin.hooks.guard/defect-ask-says-pull-request@c62dad6a
    let nowhere = TempDir::new().unwrap();
    let command = "clm defect --title x <<'EOF'\nThe gate misfired.\nEOF";
    let (code, reason) = guard_in(nowhere.path(), command, &[]);
    assert_eq!(code, Some(0));
    assert!(reason.contains("## CLM defects"), "{reason}");
    assert!(reason.contains("pull request body"), "{reason}");
    assert!(!reason.contains("page"), "{reason}");
}

#[test]
fn guard_asks_before_filing_a_defect() {
    // req:plugin.hooks.guard/defect-approval@bf60a27a
    let nowhere = TempDir::new().unwrap();
    let report = "clm defect --title \"x\" <<'EOF'\nThe gate misfired.\nEOF";
    // asking: no settings, so `defects` is `ask`.
    let (code, reason) = guard_in(nowhere.path(), report, &[]);
    assert_eq!(code, Some(0));
    assert!(reason.contains("## CLM defects"), "{reason}");
    assert!(reason.contains("public issue"), "{reason}");
    assert!(!reason.contains("humans-review-intent"), "{reason}");
    // Settings that can't be loaded ask too, and so does a `clm` called by path or after
    // assignments.
    let (code, reason) = guard_in(nowhere.path(), report, &[("CLM_DEFECTS", "always")]);
    assert_eq!(code, Some(0));
    assert!(reason.contains("## CLM defects"), "{reason}");
    for command in [
        "/home/u/.claude/plugins/clm/bin/clm defect --title x <<'EOF'\nr\nEOF",
        "env X=1 clm defect --title x <<'EOF'\nr\nEOF",
        // asking through cargo run
        "cargo run -q -p clm -- defect --title \"x\" <<'EOF'\nThe gate misfired.\nEOF",
        "cargo run --bin clm defect --title x <<'EOF'\nr\nEOF",
        // whichever package it runs (`guard-asks-any-cargo-run`)
        "cargo run -p clm-web -- defect --title x <<'EOF'\nr\nEOF",
        "cargo run -- defect --title x <<'EOF'\nr\nEOF",
        "cargo r --manifest-path crates/clm/Cargo.toml defect --title x <<'EOF'\nr\nEOF",
    ] {
        let (code, reason) = guard_in(nowhere.path(), command, &[]);
        assert_eq!(code, Some(0), "{command}");
        assert!(reason.contains("## CLM defects"), "{command}: {reason}");
    }
    // backlog, and off, from the repository's settings
    let repo = clm_repo(Some("defects = \"backlog\"\n"));
    assert_eq!(guard_in(repo.path(), report, &[]), (Some(0), String::new()));
    let off = clm_repo(Some("defects = \"off\"\n"));
    assert_eq!(guard_in(off.path(), report, &[]), (Some(0), String::new()));
    let cargo = "cargo run -q -p clm -- defect --title \"x\" <<'EOF'\nreport\nEOF";
    assert_eq!(guard_in(repo.path(), cargo, &[]), (Some(0), String::new()));
    let unshown = "clm defect --title x < report.md";
    assert_eq!(
        guard_in(repo.path(), unshown, &[]),
        (Some(0), String::new())
    );
    // The command's own `CLM_DEFECTS` is the one `clm defect` reads.
    for (set, asks) in [
        ("ask", true),
        ("always", true),
        ("backlog", false),
        ("off", false),
    ] {
        let command = format!("CLM_DEFECTS={set} {report}");
        let (code, reason) = guard_in(repo.path(), &command, &[]);
        assert_eq!(code, Some(0), "{set}");
        assert_eq!(reason.contains("## CLM defects"), asks, "{set}: {reason}");
        let command = format!("env CLM_DEFECTS={set} {report}");
        let (_, reason) = guard_in(repo.path(), &command, &[]);
        assert_eq!(
            reason.contains("## CLM defects"),
            asks,
            "env {set}: {reason}"
        );
    }
    // Other `clm` commands, and prose naming `clm defect`, aren't filing.
    for command in [
        "clm check",
        "echo run clm defect later",
        "clm defects",
        "cargo run -p clm -- defects",
        "cargo test -p clm -- defect",
        "cargo build -p clm defect",
    ] {
        assert_eq!(
            guard_in(nowhere.path(), command, &[]),
            (Some(0), String::new()),
            "{command}"
        );
    }
}

#[test]
fn guard_ignores_a_defect_report_body() {
    // req:plugin.hooks.guard/defect-approval@bf60a27a
    let nowhere = TempDir::new().unwrap();
    let quoting = "clm approve spec/x\ngit push origin main";
    // quoting a blocked command, under each form of the heredoc operator.
    for (open, close) in [
        ("<<'EOF'", "EOF"),
        ("<<\"EOF\"", "EOF"),
        ("<<EOF", "EOF"),
        ("<< EOF", "EOF"),
        ("<<-EOF", "\t\tEOF"),
        // A redirection's `&` before the operator doesn't end the segment.
        ("2>&1 <<'EOF'", "EOF"),
        ("&>log <<'EOF'", "EOF"),
    ] {
        let command = format!("clm defect --title \"x\" {open}\n{quoting}\n{close}");
        let (code, reason) = guard_in(nowhere.path(), &command, &[]);
        assert_eq!(code, Some(0), "{open}: {reason}");
        assert!(reason.contains("## CLM defects"), "{open}: {reason}");
        assert!(!reason.contains("humans-review-intent"), "{open}: {reason}");
        // With `defects = "backlog"` the body still matches no rule.
        let repo = clm_repo(Some("defects = \"backlog\"\n"));
        assert_eq!(
            guard_in(repo.path(), &command, &[]),
            (Some(0), String::new()),
            "{open}"
        );
    }
    // No heredoc after all: a quoted `<<`, `<<<`, or no terminator line.
    for command in [
        "clm defect --title \"a <<EOF\"\ngit push origin main\nEOF",
        "clm defect --title x <<<EOF\ngit push origin main\nEOF",
        "clm defect --title x <<EOF\ngit push origin main",
    ] {
        let (code, reason) = guard_in(nowhere.path(), command, &[]);
        assert_eq!(code, Some(2), "{command}: {reason}");
        assert!(reason.contains("guard-blocks-push-to-main"), "{reason}");
    }
    // Another command's heredoc isn't a report, but its body belongs to it, which isn't `gh`
    // or a shell, so it matches no rule either (`guard-heredocs`).
    let (code, _) = guard_in(
        nowhere.path(),
        "clm check <<EOF\ngit push origin main\nEOF",
        &[],
    );
    assert_eq!(code, Some(0));
}

#[test]
fn guard_block_wins_over_a_defect_ask() {
    // req:plugin.hooks.guard/defect-approval@bf60a27a
    let nowhere = TempDir::new().unwrap();
    // block wins: the push guard's block wins over the defect's ask.
    let (code, reason) = guard_in(
        nowhere.path(),
        "clm defect --title x <<'EOF'\nreport\nEOF\ngit push origin main",
        &[],
    );
    assert_eq!(code, Some(2), "{reason}");
    assert!(reason.contains("guard-blocks-push-to-main"), "{reason}");
    // Two asks make one decision giving both reasons.
    let (code, reason) = guard_in(
        nowhere.path(),
        "clm defect --title x <<'EOF'\nreport\nEOF\nclm approve spec/x",
        &[],
    );
    assert_eq!(code, Some(0));
    assert!(
        reason.contains("## CLM defects") && reason.contains("humans-review-intent"),
        "{reason}"
    );
}

#[test]
fn guard_quotes_the_defect_report_it_asks_about() {
    // req:plugin.hooks.guard/defect-approval@bf60a27a
    let nowhere = TempDir::new().unwrap();
    let command = "clm defect --title \"x\" --component gate <<'EOF'\nThe gate misfired.\nEOF";
    let (code, reason) = guard_in(nowhere.path(), command, &[]);
    assert_eq!(code, Some(0), "{reason}");
    assert!(reason.contains("## CLM defects"), "{reason}");
    assert!(
        reason.contains("clm defect --title \"x\" --component gate"),
        "{reason}"
    );
    assert!(reason.contains("The gate misfired."), "{reason}");
    // An unquoted heredoc with nothing to expand is sent as written.
    let (code, reason) = guard_in(
        nowhere.path(),
        "clm defect --title x <<EOF\nplain words\nEOF",
        &[],
    );
    assert_eq!(code, Some(0), "{reason}");
    assert!(reason.contains("plain words"), "{reason}");
    // A cargo run's report is quoted, a multi-line body whole, and the body once.
    let (_, reason) = guard_in(
        nowhere.path(),
        "cargo run -q -p clm -- defect --title x <<'EOF'\nThe gate misfired.\nsecond line\nEOF",
        &[],
    );
    assert!(
        reason.contains("The gate misfired.\nsecond line"),
        "{reason}"
    );
    assert_eq!(reason.matches("The gate misfired.").count(), 1, "{reason}");
    // Each segment's report is quoted, in order.
    let (_, reason) = guard_in(
        nowhere.path(),
        "clm defect --title a <<'EOF'\nfirst\nEOF\nclm defect --title b <<'EOF'\nsecond\nEOF",
        &[],
    );
    let (a, b) = (reason.find("first"), reason.find("second"));
    assert!(a.is_some() && b.is_some() && a < b, "{reason}");
}

#[test]
fn guard_refuses_a_defect_report_it_cannot_show() {
    // req:plugin.hooks.guard/defect-approval@bf60a27a
    let nowhere = TempDir::new().unwrap();
    for command in [
        "clm defect --title x < report.md",
        "cat report.md | clm defect --title x",
        "clm defect --title x <<< \"$R\"",
        "clm defect --title x",
        "clm defect --title x <<EOF\ncost $HOME\nEOF",
        "clm defect --title x <<EOF\nno end",
    ] {
        let (code, reason) = guard_in(nowhere.path(), command, &[]);
        assert_eq!(code, Some(2), "{command}: {reason}");
        assert!(reason.contains("defect-ask-shows-report"), "{reason}");
        assert!(reason.contains("<<'EOF'"), "{reason}");
    }
    // A refusal wins over any ask, and one unshown report refuses the lot.
    let (code, reason) = guard_in(
        nowhere.path(),
        "clm defect --title a <<'EOF'\nr\nEOF\nclm defect --title b\nclm approve spec/x",
        &[],
    );
    assert_eq!(code, Some(2), "{reason}");
    // Help files nothing, and neither does a segment under `backlog`.
    for command in [
        "clm defect --help",
        "clm defect -h",
        "cargo run -- defect -h",
    ] {
        assert_eq!(
            guard_in(nowhere.path(), command, &[]),
            (Some(0), String::new()),
            "{command}"
        );
    }
    let repo = clm_repo(Some("defects = \"backlog\"\n"));
    assert_eq!(
        guard_in(repo.path(), "clm defect --title x < report.md", &[]),
        (Some(0), String::new())
    );
}
