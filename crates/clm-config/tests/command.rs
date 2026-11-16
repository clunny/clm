//! `clm config` as `clm-config`'s call: the command's arguments, its streams and its exit
//! status, in-process (`commands-in-crate`).

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

/// What a run of the command gave: its exit status and what it wrote to each stream.
#[derive(Debug)]
struct Output {
    status: Status,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

#[derive(Debug)]
struct Status(u8);

impl Status {
    fn code(&self) -> Option<i32> {
        Some(self.0.into())
    }
}

/// The CLM repository `crates/clm` finds from `cwd` (`clm/root-from-cwd`): the first of `cwd`
/// and its parents that holds `specs/SPEC.md`, looking no further up than the first that holds
/// `.git`.
fn found_root(cwd: &Path) -> Option<PathBuf> {
    for dir in cwd.ancestors() {
        if dir.join("specs/SPEC.md").is_file() {
            return Some(dir.to_path_buf());
        }
        if dir.join(".git").exists() {
            return None;
        }
    }
    None
}

/// `clm <args>` run in `cwd` as `crates/clm` routes it, with `vars` the only CLM variables set
/// and a user config dir that holds no `clm/config.toml`, so the developer's own settings never
/// leak in. A variable reads as `clm_config::process_env` reads the process's.
fn clm_in(cwd: &Path, args: &[&str], vars: &[(&str, OsString)]) -> Output {
    let (name, rest) = args.split_first().unwrap();
    assert_eq!(*name, "config");
    let empty = Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-user-config");
    let env = |key: &str| {
        if let Some((_, value)) = vars.iter().find(|(k, _)| *k == key) {
            return Some(value.to_string_lossy().into_owned());
        }
        match key {
            "XDG_CONFIG_HOME" => Some(empty.display().to_string()),
            "CLM_AUTOSYNC" | "CLM_DEFECTS" | "CLM_MAX_BUILDS" | "CLM_WRITER" => None,
            _ => clm_config::process_env(key),
        }
    };
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let status = clm_config::command(
        rest.iter().copied(),
        found_root(cwd).as_deref(),
        cwd,
        &env,
        &mut stdout,
        &mut stderr,
    );
    Output {
        status: Status(status),
        stdout,
        stderr,
    }
}

/// Runs `clm config get <key>` in `cwd`.
fn clm_config_get(cwd: &Path, key: &str) -> Output {
    clm_in(cwd, &["config", "get", key], &[])
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn a_command_called_in_a_test() {
    // req:config/commands-in-crate@ecba0654
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    write(&root, "specs/SPEC.md", "");
    fs::create_dir(root.join(".git")).unwrap();
    write(&root, "clm.toml", "defects = \"backlog\"\n");
    let out = clm_config_get(&root, "defects");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("backlog\t{}\n", root.join("clm.toml").display())
    );
    assert!(out.stderr.is_empty());

    write(&root, "clm.toml", "defect = \"off\"\n");
    let out = clm_config_get(&root, "defects");
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.starts_with("clm config: "), "{stderr}");
    assert!(stderr.ends_with('\n'), "{stderr}");
}

#[test]
fn arguments_it_cannot_parse() {
    // req:config/commands-in-crate@ecba0654
    let dir = TempDir::new().unwrap();
    for args in [
        &["config", "get", "defects", "--nope"][..],
        &["config", "nope"][..],
        &["config", "get"][..],
    ] {
        let out = clm_in(dir.path(), args, &[]);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(stderr.starts_with("error: "), "{stderr}");
        assert!(stderr.contains("Usage: clm config"), "{stderr}");
    }
    // Help is no error: it goes to stdout with status 0, as clap's does.
    let out = clm_in(dir.path(), &["config", "--help"], &[]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8(out.stdout).unwrap().contains("get"));
}
