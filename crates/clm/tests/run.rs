//! `clm run` with a fake `claude` on PATH, so nothing touches the network or a model.
//! `clm run` prepares the build's branch and prints its prompt; it starts no session.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::*;
use tempfile::TempDir;

const SKILLS: [&str; 3] = [
    "writing-plans",
    "test-driven-development",
    "verification-before-completion",
];

/// A fake `claude` that records that it ran, and a superpowers copy holding the three
/// skills.
struct Fake {
    _dir: TempDir,
    bin: PathBuf,
    out: PathBuf,
    superpowers: PathBuf,
}

fn fake() -> Fake {
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
struct Ran {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn clm_run_in(cwd: &Path, root: &Path, f: &Fake, node: &str, env: &[(&str, &Path)]) -> Ran {
    clm_run_with(cwd, root, f, &[node], env)
}

/// `clm run <args>` in `cwd`.
fn clm_run_with(cwd: &Path, root: &Path, f: &Fake, args: &[&str], env: &[(&str, &Path)]) -> Ran {
    let path = format!(
        "{}:{}",
        f.bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    // The user's settings and a `CARGO_TARGET_DIR` from the test's own environment stay
    // out, unless a test sets them.
    let out = output(
        Command::new(env!("CARGO_BIN_EXE_clm"))
            .arg("run")
            .args(args)
            .arg("--root")
            .arg(root)
            .current_dir(cwd)
            .env("PATH", path)
            .env("FAKE_OUT", &f.out)
            .env("CLM_SUPERPOWERS_DIR", &f.superpowers)
            .env("XDG_CONFIG_HOME", &f.out)
            .env_remove("CARGO_TARGET_DIR")
            .envs(env.iter().map(|(k, v)| (*k, v.as_os_str()))),
    );
    Ran {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// What a `clm run` that must succeed printed, the launcher, and the prompt it wrote to
/// `.clm/prompts/<node>.md` under `root`, which the launcher names by its absolute path.
fn launched_in(
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

fn local_tip(r: &Repo, branch: &str) -> Option<String> {
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

#[test]
fn relative_root() {
    let (r, f) = (repo(), fake());
    let parent = r.work.parent().unwrap();
    // The launcher still names the prompt by its absolute path.
    launched_in(parent, Path::new("work"), &f, "b", &[]);
    assert!(local_tip(&r, "impl/b").is_some());
}
