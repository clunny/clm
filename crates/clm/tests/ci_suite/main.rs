//! `ci`'s own tests (specs/SPEC.md names this directory `ci`'s): how this repository runs its
//! Rust tests, read from `.config/nextest.toml`, the files under `.github/`, and a run of
//! `.github/scripts/release-prepare.sh` against a temporary repository whose `origin` is a
//! local bare repository, with fake `gh` and `cargo` first on `PATH`.

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

/// Every file under `.github/`, with its path relative to the repository and its text.
fn github_files() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else {
                out.push(path);
            }
        }
    }
    let mut paths = Vec::new();
    walk(&repo().join(".github"), &mut paths);
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let name = p.strip_prefix(repo()).unwrap().display().to_string();
            (name, fs::read_to_string(&p).unwrap_or_default())
        })
        .collect()
}

/// The lines of `text` that aren't blank or comments.
fn code_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
}

#[test]
fn every_nextest_run_ends_a_test_after_fifteen_minutes() {
    // req:ci/tests-time-bounded@f2d9da83
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
    // No run picks another profile or configuration file.
    for (name, text) in github_files() {
        for line in code_lines(&text) {
            for flag in ["--profile", "--config-file", "--tool-config-file"] {
                assert!(
                    !line.split_whitespace().any(|w| w.starts_with(flag)),
                    "{name} passes {flag}: {line}"
                );
            }
            // Nor by nextest's short flag or its environment variables.
            assert!(
                !(line.contains("nextest") && line.split_whitespace().any(|w| w.starts_with("-P"))),
                "{name} passes -P: {line}"
            );
            for var in ["NEXTEST_PROFILE", "NEXTEST_CONFIG"] {
                assert!(!line.contains(var), "{name} sets {var}: {line}");
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
    // req:ci/test-tools@8da0e835
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
    for (job, body) in wf["jobs"].as_mapping().unwrap() {
        if job.as_str() == Some("prepare") {
            continue;
        }
        let text = serde_yaml::to_string(body).unwrap();
        assert!(
            !text.contains("test-tools.sh") && !text.contains("nextest"),
            "job {job:?} names test-tools.sh or nextest"
        );
    }
}

#[test]
fn no_file_under_github_runs_the_tests_outside_nextest() {
    // req:ci/nextest-runner@26bed54d
    for (name, text) in github_files() {
        for line in code_lines(&text) {
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
                    assert!(rest.contains(&"--doc"), "{name}: {line}");
                }
                if rest.first() == Some(&"llvm-cov") {
                    assert_eq!(rest.get(1), Some(&"nextest"), "{name}: {line}");
                }
            }
        }
    }
}

/// Records each call, with whether `origin` held the release branch by then, and answers
/// `build` with a `clm` whose `upgrade --lock` succeeds; does nothing else.
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

#[test]
fn the_release_runs_the_tests_under_nextest_before_it_pushes() {
    // req:ci/nextest-runner@26bed54d
    let dir = TempDir::new().unwrap();
    let work = dir.path().join("work");
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
    let bin = dir.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    executable(&bin.join("cargo"), FAKE_CARGO);
    executable(&bin.join("gh"), FAKE_GH);
    let bare = dir.path().join("remote.git");
    git(&work, &["init", "-q", "-b", "main"]);
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-q", "-m", "init"]);
    git(
        dir.path(),
        &["init", "-q", "--bare", bare.to_str().unwrap()],
    );
    git(&work, &["remote", "add", "origin", bare.to_str().unwrap()]);
    git(&work, &["push", "-q", "origin", "main"]);

    let log = dir.path().join("log");
    let output = dir.path().join("github_output");
    fs::write(&output, "").unwrap();
    let system = std::env::var_os("PATH").unwrap_or_default();
    let path =
        std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(&system))).unwrap();
    let out = Command::new(repo().join(".github/scripts/release-prepare.sh"))
        .arg("0.2.0")
        .current_dir(&work)
        .env("PATH", path)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GITHUB_REF", "refs/heads/main")
        .env("GITHUB_SHA", "unused")
        .env("GITHUB_OUTPUT", &output)
        .env("CARGO_TARGET_DIR", dir.path().join("target"))
        .env("CI_LOG", &log)
        .env("RELEASE_VERSION", "0.2.0")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let log = fs::read_to_string(&log).unwrap();
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
