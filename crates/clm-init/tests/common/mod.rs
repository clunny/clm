//! Calls of `clm-init`'s commands as `crates/clm` makes them, for the tests that ran the
//! binary (`commands-in-crate`).

#![allow(dead_code)]

use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{ExitStatus, Output};
use std::sync::{Mutex, PoisonError};

/// Held for the length of a call that sets this process's environment, so a `cargo test`
/// running tests on threads gives each call its own; `cargo nextest` runs one per process.
static CALLS: Mutex<()> = Mutex::new(());

/// The CLM repository `crates/clm` finds from `cwd`, as `clm-hooks`' `clm_root` finds it: the
/// first of `cwd` and its parents holding `specs/SPEC.md`, looking no further up than the
/// first holding `.git`.
pub fn found_root(cwd: &Path) -> Option<&Path> {
    for dir in cwd.ancestors() {
        if dir.join("specs/SPEC.md").is_file() {
            return Some(dir);
        }
        if dir.join(".git").exists() {
            return None;
        }
    }
    None
}

/// Calls `clm <args>` in `cwd` as `crates/clm` does: `init`, `survey` and `guide` through
/// `clm-init`, `config` through `clm-config`, with `env` as the environment's lookup. Sets
/// `vars` on this process for the length of the call, for `git` and the tools a command
/// starts, which inherit the process's environment. Returns what the binary returned: its
/// exit status and both streams.
pub fn call(
    cwd: &Path,
    args: &[&str],
    env: &dyn Fn(&str) -> Option<String>,
    vars: &[(&str, &str)],
) -> Output {
    let (name, rest) = args.split_first().expect("a command");
    let root = found_root(cwd);
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let _one = CALLS.lock().unwrap_or_else(PoisonError::into_inner);
    let before: Vec<_> = vars
        .iter()
        .map(|(k, _)| (*k, std::env::var_os(k)))
        .collect();
    for (k, v) in vars {
        // SAFETY: the tests that set variables hold `CALLS`, and nextest runs one per process.
        unsafe { std::env::set_var(k, v) };
    }
    let rest = rest.iter().copied();
    let code = match *name {
        "init" => clm_init::init_command(rest, root, cwd, env, &mut out, &mut err),
        "survey" => clm_init::survey_command(rest, root, cwd, env, &mut out, &mut err),
        "guide" => clm_init::guide_command(rest, root, cwd, env, &mut out, &mut err),
        "config" => clm_config::command(rest, root, cwd, env, &mut out, &mut err),
        other => panic!("no call for clm {other}"),
    };
    for (k, v) in before {
        // SAFETY: as above.
        unsafe {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
    }
    Output {
        status: ExitStatus::from_raw(i32::from(code) << 8),
        stdout: out,
        stderr: err,
    }
}

/// The variables the binary's runs set to keep the user's git config out.
pub const ISOLATED_GIT: [(&str, &str); 2] = [
    ("GIT_CONFIG_GLOBAL", "/dev/null"),
    ("GIT_CONFIG_NOSYSTEM", "1"),
];

/// The process's own environment, as the binary read it.
pub fn process_env(key: &str) -> Option<String> {
    std::env::var(key).ok()
}
