//! `ci`'s own tests (specs/SPEC.md names this directory `ci`'s): how this repository runs its
//! Rust tests, read from `.config/nextest.toml`, the two workflows parsed, and runs of the
//! scripts under `.github/scripts/` with fakes first on `PATH`, `release-prepare.sh` against a
//! temporary repository whose `origin` is a local bare repository. The modules hold the tests
//! of the check scripts and the release workflow.

mod ci_check;
mod release_workflow;

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_yaml::Value;
use tempfile::TempDir;

#[path = "../../../clm-git/tests/exec/mod.rs"]
mod exec;
use exec::write_exec as executable;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The two workflows, each by its file name and parsed.
fn workflows() -> Vec<(&'static str, Value)> {
    ["release.yml", "site.yml"]
        .into_iter()
        .map(|name| {
            let path = repo().join(".github/workflows").join(name);
            let text =
                fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            (name, serde_yaml::from_str(&text).unwrap())
        })
        .collect()
}

/// Every step of the two workflows, with its workflow's file name and its job's.
fn workflow_steps() -> Vec<(&'static str, String, Value)> {
    let mut steps = Vec::new();
    for (name, wf) in workflows() {
        for (job, body) in wf["jobs"].as_mapping().unwrap() {
            for step in body["steps"].as_sequence().unwrap() {
                steps.push((name, job.as_str().unwrap().to_owned(), step.clone()));
            }
        }
    }
    steps
}

/// Whether `words`, a command's arguments, pass nextest a profile or a configuration file.
fn picks_a_profile(words: &[&str]) -> bool {
    words.iter().any(|w| {
        ["--profile", "-P", "--config-file", "--tool-config-file"]
            .iter()
            .any(|flag| w.starts_with(flag))
    })
}

/// `PATH` with a `cargo` first that appends `<NEXTEST_PROFILE>|<NEXTEST_CONFIG>|<arguments>`
/// to `log`, each variable `unset` when it is, and then runs `real` with the same arguments.
fn recording_cargo(dir: &Path, real: &Path, log: &Path, rest: &OsStr) -> OsString {
    let bin = dir.join("env-bin");
    fs::create_dir_all(&bin).unwrap();
    executable(
        &bin.join("cargo"),
        &format!(
            "#!/usr/bin/env bash\necho \"${{NEXTEST_PROFILE-unset}}|${{NEXTEST_CONFIG-unset}}|$*\" >> '{}'\nexec '{}' \"$@\"\n",
            log.display(),
            real.display()
        ),
    );
    std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(rest))).unwrap()
}

#[test]
fn every_nextest_run_ends_a_test_after_fifteen_minutes() {
    // req:ci/tests-time-bounded@30fea9a6
    let path = repo().join(".config/nextest.toml");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let config: toml::Table = toml::from_str(&text).unwrap();
    let profiles = config["profile"].as_table().unwrap();
    assert_eq!(
        profiles.keys().collect::<Vec<_>>(),
        ["default"],
        "only the default profile"
    );
    let slow = profiles["default"]["slow-timeout"].as_table().unwrap();
    assert_eq!(slow.len(), 2, "{slow:?}");
    assert_eq!(slow["period"].as_str(), Some("60s"), "{slow:?}");
    assert_eq!(slow["terminate-after"].as_integer(), Some(15), "{slow:?}");

    // Each script run with every test selected, a `cargo` first on `PATH` recording what
    // nextest would see: the scripts' own runs, `cargo llvm-cov nextest` and `cargo mutants`
    // included, pass nextest no profile or configuration file and set neither variable.
    let check = ci_check::Tree::new();
    let coverage = ci_check::coverage_tree();
    let mutate = ci_check::mutate_tree(ci_check::RUST_BLOCK);
    let release = TempDir::new().unwrap();
    let mut runs = Vec::new();
    for (name, tree) in [
        ("ci-check.sh", &check),
        ("coverage.sh", &coverage),
        ("mutate.sh", &mutate),
    ] {
        let log = tree.dir.path().join("env-log");
        let path = recording_cargo(
            tree.dir.path(),
            &tree.dir.path().join("bin/cargo"),
            &log,
            &tree.path(),
        );
        let out = if name == "mutate.sh" {
            tree.mutate_command(&[])
                .env("PATH", &path)
                .output()
                .unwrap()
        } else {
            tree.output_env(None, "", None, &path, &[])
        };
        assert!(out.status.success(), "{name}: {out:?}");
        runs.push((name, log));
    }
    let (work, bin) = release_tree(release.path());
    let log = release.path().join("env-log");
    let system = std::env::var_os("PATH").unwrap_or_default();
    let rest =
        std::env::join_paths(std::iter::once(bin.clone()).chain(std::env::split_paths(&system)))
            .unwrap();
    let path = recording_cargo(release.path(), &bin.join("cargo"), &log, &rest);
    let out = run_release(release.path(), &work, &path);
    assert!(out.status.success(), "release-prepare.sh: {out:?}");
    runs.push(("release-prepare.sh", log));

    for (name, log) in runs {
        let log = fs::read_to_string(&log).unwrap_or_default();
        let nextest: Vec<&str> = log
            .lines()
            .filter(|l| {
                let args = l.splitn(3, '|').nth(2).unwrap_or("");
                ["nextest run", "llvm-cov nextest", "mutants"]
                    .iter()
                    .any(|c| args.starts_with(c))
            })
            .collect();
        assert!(!nextest.is_empty(), "{name} ran no nextest: {log}");
        for call in nextest {
            let (profile, rest) = call.split_once('|').unwrap();
            let (config, args) = rest.split_once('|').unwrap();
            assert_eq!(
                (profile, config),
                ("unset", "unset"),
                "{name} sets a nextest variable: {call}"
            );
            let words: Vec<&str> = args.split_whitespace().collect();
            assert!(!picks_a_profile(&words), "{name}: {call}");
        }
    }

    // Nor does a workflow, by a step's command or any environment it sets.
    for (name, wf) in workflows() {
        let mut envs = vec![wf["env"].clone()];
        for (job, body) in wf["jobs"].as_mapping().unwrap() {
            envs.push(body["env"].clone());
            for step in body["steps"].as_sequence().unwrap() {
                envs.push(step["env"].clone());
                let run = step["run"].as_str().unwrap_or("");
                // Only a line that runs nextest: another tool may take a flag of that name.
                for line in run.lines().filter(|l| l.contains("nextest")) {
                    let words: Vec<&str> = line.split_whitespace().collect();
                    assert!(!picks_a_profile(&words), "{name} `{job:?}`: {line}");
                }
                for var in ["NEXTEST_PROFILE", "NEXTEST_CONFIG"] {
                    assert!(!run.contains(var), "{name} `{job:?}` sets {var}: {run}");
                }
            }
        }
        for env in envs.iter().filter_map(Value::as_mapping) {
            for var in ["NEXTEST_PROFILE", "NEXTEST_CONFIG"] {
                assert!(!env.contains_key(var), "{name} sets {var}: {env:?}");
            }
        }
    }
}

fn release_workflow() -> Value {
    let path = repo().join(".github/workflows/release.yml");
    serde_yaml::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn the_release_installs_nextest_between_the_detector_tools_and_the_tests() {
    // req:ci/test-tools@70e8cd5d
    let wf = release_workflow();
    let steps = wf["jobs"]["prepare"]["steps"].as_sequence().unwrap();
    let step = |script: &str| {
        let runs: Vec<usize> = steps
            .iter()
            .enumerate()
            .filter(|(_, s)| s["run"].as_str().is_some_and(|r| r.contains(script)))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(runs.len(), 1, "`prepare` runs {script} in {runs:?}");
        runs[0]
    };
    let detectors = step(".github/scripts/detector-tools.sh");
    let tests = step(".github/scripts/test-tools.sh");
    let prepare = step(".github/scripts/release-prepare.sh");
    assert!(detectors < tests && tests < prepare, "{steps:?}");
    // A step of its own: the test-tools step runs nothing else.
    assert_eq!(
        steps[tests]["run"].as_str().map(str::trim),
        Some(".github/scripts/test-tools.sh")
    );
}

#[test]
fn no_file_under_github_runs_the_tests_outside_nextest() {
    // req:ci/nextest-runner@b769cc74
    for (name, job, step) in workflow_steps() {
        let run = step["run"].as_str().unwrap_or("");
        // Line by line: a line's end ends its command.
        for line in run.lines() {
            let words: Vec<&str> = line.split_whitespace().collect();
            for (i, _) in words.iter().enumerate().filter(|(_, w)| **w == "cargo") {
                // The subcommand is the first word after `cargo` that is no toolchain
                // (`+stable`) or option (`--locked`).
                // Its arguments end where the shell starts another command.
                let rest: Vec<&str> = words[i + 1..]
                    .iter()
                    .skip_while(|w| w.starts_with('+') || w.starts_with('-'))
                    .take_while(|w| !matches!(**w, "&&" | "||" | ";" | "|"))
                    .copied()
                    .collect();
                if rest.first() == Some(&"test") {
                    assert!(rest.contains(&"--doc"), "{name} `{job}`: {run}");
                }
                if rest.first() == Some(&"llvm-cov") {
                    assert_eq!(rest.get(1), Some(&"nextest"), "{name} `{job}`: {run}");
                }
            }
        }
    }
}

/// Records each call, with whether `origin` held the release branch by then, and answers
/// `build` with a `clm` whose `upgrade --lock` succeeds and whose `pending` prints nothing;
/// does nothing else.
const FAKE_CARGO: &str = r#"#!/usr/bin/env bash
pushed=no
git ls-remote --exit-code origin "refs/heads/release/v$RELEASE_VERSION" >/dev/null && pushed=yes
echo "$pushed cargo $*" >> "$CI_LOG"
if [ "$1" = build ]; then
  mkdir -p "$CARGO_TARGET_DIR/release"
  printf '#!/bin/sh\nexit 0\n' > "$CARGO_TARGET_DIR/release/clm"
  chmod +x "$CARGO_TARGET_DIR/release/clm"
fi
exit 0
"#;

/// Answers `release list` with no release.
const FAKE_GH: &str = "#!/bin/sh\nexit 0\n";

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

/// A repository at version `0.1.0` in `dir/work`, its `origin` a bare repository in `dir`
/// holding `main`, with the fakes `cargo` and `gh` in `dir/bin`. Returns the work tree and the
/// fakes' directory.
fn release_tree(dir: &Path) -> (PathBuf, PathBuf) {
    let work = dir.join("work");
    let target = "x86_64-unknown-linux-musl";
    for (path, text) in [
        (
            "Cargo.toml",
            "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"0.1.0\"\n".to_owned(),
        ),
        ("Cargo.lock", String::new()),
        (
            "plugin/.claude-plugin/plugin.json",
            "{\n  \"version\": \"0.1.0\"\n}\n".to_owned(),
        ),
        (
            "plugin/.codex-plugin/plugin.json",
            "{\n  \"version\": \"0.1.0\"\n}\n".to_owned(),
        ),
        (
            "plugin-panel/.claude-plugin/plugin.json",
            "{\n  \"version\": \"0.1.0\"\n}\n".to_owned(),
        ),
        ("plugin/bin/clm", format!("#!/bin/sh\ntarget={target}\n")),
        (
            ".github/workflows/release.yml",
            format!(
                "jobs:\n  build:\n    strategy:\n      matrix:\n        target: [x]\n        include:\n          - target: {target}\n"
            ),
        ),
        ("crates/clm-upgrade/migrations.lock", String::new()),
        ("crates/clm-upgrade/corpus/.keep", String::new()),
    ] {
        let path = work.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    executable(
        &work.join("crates/clm-upgrade/corpus/make.sh"),
        "#!/bin/sh\necho entry > \"$(dirname \"$0\")/v$1.tar.gz\"\n",
    );
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    executable(&bin.join("cargo"), FAKE_CARGO);
    executable(&bin.join("gh"), FAKE_GH);
    let bare = dir.join("remote.git");
    git(&work, &["init", "-q", "-b", "main"]);
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-q", "-m", "init"]);
    git(dir, &["init", "-q", "--bare", bare.to_str().unwrap()]);
    git(&work, &["remote", "add", "origin", bare.to_str().unwrap()]);
    git(&work, &["push", "-q", "origin", "main"]);
    (work, bin)
}

/// Runs `release-prepare.sh 0.2.0` in `work` with `path` as `PATH`, the fake `cargo`'s calls
/// going to `dir/log`.
fn run_release(dir: &Path, work: &Path, path: &OsStr) -> std::process::Output {
    let output = dir.join("github_output");
    fs::write(&output, "").unwrap();
    Command::new(repo().join(".github/scripts/release-prepare.sh"))
        .arg("0.2.0")
        .current_dir(work)
        .env("PATH", path)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GITHUB_REF", "refs/heads/main")
        .env("GITHUB_SHA", "unused")
        .env("GITHUB_OUTPUT", &output)
        .env("CARGO_TARGET_DIR", dir.join("target"))
        .env("CI_LOG", dir.join("log"))
        .env("RELEASE_VERSION", "0.2.0")
        .env_remove("NEXTEST_PROFILE")
        .env_remove("NEXTEST_CONFIG")
        .output()
        .unwrap()
}

#[test]
fn the_release_runs_the_tests_under_nextest_before_it_pushes() {
    // req:ci/nextest-runner@b769cc74
    let dir = TempDir::new().unwrap();
    let (work, bin) = release_tree(dir.path());
    let system = std::env::var_os("PATH").unwrap_or_default();
    let path =
        std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(&system))).unwrap();
    let out = run_release(dir.path(), &work, &path);
    assert!(out.status.success(), "{out:?}");
    let log = fs::read_to_string(dir.path().join("log")).unwrap();
    let tests: Vec<&str> = log
        .lines()
        .filter(|l| l.contains(" cargo test") || l.contains(" cargo nextest"))
        .collect();
    // Both before the bump commit reached `origin`.
    assert_eq!(
        tests,
        [
            "no cargo nextest run --workspace --no-tests=pass",
            "no cargo test --doc --workspace",
        ],
        "{log}"
    );
    // And the bump was pushed after them.
    git(
        &work,
        &[
            "ls-remote",
            "--exit-code",
            "origin",
            "refs/heads/release/v0.2.0",
        ],
    );
}
