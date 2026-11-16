//! `clm config` as `clm-config`'s call: the command's arguments, its streams and its exit
//! status, in-process (`commands-in-crate`).

// clm:reads clm.toml

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

/// The root of this repository.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
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
}

/// `clm config get <key>`'s stdout in a fresh CLM repository with `clm_toml` (when given),
/// and the path of its `clm.toml`.
fn config_get_in_repo(clm_toml: Option<&str>, key: &str) -> (String, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    write(&root, "specs/SPEC.md", "");
    fs::create_dir(root.join(".git")).unwrap();
    if let Some(text) = clm_toml {
        write(&root, "clm.toml", text);
    }
    let out = clm_config_get(&root, key);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    (
        String::from_utf8(out.stdout).unwrap(),
        root.join("clm.toml"),
    )
}

/// `clm config list` in a fresh CLM repository holding `clm.toml` and `clm.local.toml`.
fn config_list_in_repo(clm_toml: &str, local: &str) -> (Output, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    write(&root, "specs/SPEC.md", "");
    fs::create_dir(root.join(".git")).unwrap();
    write(&root, "clm.toml", clm_toml);
    write(&root, "clm.local.toml", local);
    let out = clm_in(&root, &["config", "list"], &[]);
    (out, root)
}

#[test]
fn config_get_from_the_repository() {
    // req:config/config-get@e2bb5643
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    write(&root, "specs/SPEC.md", "");
    fs::create_dir(root.join(".git")).unwrap();
    write(&root, "clm.toml", "defects = \"backlog\"\n");

    let out = clm_config_get(&root.join("specs"), "defects");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("backlog\t{}\n", root.join("clm.toml").display())
    );
}

#[test]
fn config_get_detectors_prints_their_names() {
    // req:config/repository-only@d2cdaefd
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    write(&root, "specs/SPEC.md", "");
    fs::create_dir(root.join(".git")).unwrap();
    write(
        &root,
        "clm.toml",
        "[[detectors]]\nname = \"a\"\ncommand = [\"a\"]\n\
         [[detectors]]\nname = \"b\"\ncommand = [\"b\"]\n",
    );

    let out = clm_config_get(&root, "detectors");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("a,b\t{}\n", root.join("clm.toml").display())
    );
}

#[test]
fn config_get_default_outside_a_repository() {
    // req:config/config-get@e2bb5643
    let dir = TempDir::new().unwrap();
    write(dir.path(), "clm.toml", "defects = \"backlog\"\n");
    let out = clm_config_get(dir.path(), "autosync");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "true\tdefault\n");
}

#[test]
fn config_get_host_defaults_to_github() {
    // req:config/host-setting@5ef6c885
    let (out, _) = config_get_in_repo(None, "host");
    assert_eq!(out, "github\tdefault\n");
}

#[test]
fn config_get_host_from_the_repository() {
    // req:config/host-setting@5ef6c885
    let (out, path) = config_get_in_repo(Some("host = \"git\"\n"), "host");
    assert_eq!(out, format!("git\t{}\n", path.display()));
}

#[test]
fn config_get_host_azure() {
    // req:config/host-setting@5ef6c885
    let (out, path) = config_get_in_repo(Some("host = \"azure\"\n"), "host");
    assert_eq!(out, format!("azure\t{}\n", path.display()));
}

#[test]
fn config_get_approval_defaults_to_any() {
    // req:config/approval-setting@ee7ad72c
    let (out, _) = config_get_in_repo(None, "approval");
    assert_eq!(out, "any\tdefault\n");
}

#[test]
fn config_get_approval_from_the_repository() {
    // req:config/approval-setting@ee7ad72c
    let (out, path) = config_get_in_repo(Some("approval = \"other\"\n"), "approval");
    assert_eq!(out, format!("other\t{}\n", path.display()));
}

#[test]
fn config_get_land_check() {
    // req:config/land-check-setting@1baa537e
    let (out, path) =
        config_get_in_repo(Some("[land]\ncheck = \"scripts/clm-check.sh\"\n"), "land");
    assert_eq!(
        out,
        format!("check=scripts/clm-check.sh\t{}\n", path.display())
    );
}

#[test]
fn config_get_land_check_target() {
    // req:config/land-check-setting@1baa537e
    let (out, path) = config_get_in_repo(
        Some("[land]\ncheck = \"scripts/clm-check.sh\"\ncheck_target_seconds = 90\n"),
        "land",
    );
    assert_eq!(
        out,
        format!(
            "check=scripts/clm-check.sh,check_target_seconds=90\t{}\n",
            path.display()
        )
    );
}

#[test]
fn config_get_land_defaults_to_nothing() {
    // req:config/land-check-setting@1baa537e
    let (out, _) = config_get_in_repo(None, "land");
    assert_eq!(out, "\tdefault\n");
}

#[test]
fn this_repositorys_clm_toml() {
    // req:config/land-check-setting@1baa537e
    // req:config/host-setting@5ef6c885
    // req:config/affected-setting@7b51c4ec
    // req:config/health-lcov-here@62a2a08d
    // req:config/verify-here@826e104c
    // This repository's `clm.toml`, in a fresh repository, so a `clm.local.toml` in the
    // developer's clone can't change the answer.
    let text =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../clm.toml")).unwrap();
    for (key, want) in [
        ("land", "check=.github/scripts/ci-check.sh"),
        ("host", "github"),
        (
            "affected",
            "all=Cargo.toml,Cargo.lock,rust-toolchain.toml,crates/clm/src/**,.github/scripts/ci-check.sh",
        ),
        (
            "health",
            "reconsider=3,scenarios=6,exceptions=4,stale_days=30,lcov=.clm/verify-build/lcov.info",
        ),
        (
            "verify",
            "mode=local,check=.github/scripts/ci-check.sh,triggers=queue-drained,every.lands=20,mutate=.github/scripts/mutate.sh",
        ),
    ] {
        let (out, file) = config_get_in_repo(Some(&text), key);
        assert_eq!(out, format!("{want}\t{}\n", file.display()));
    }
    // `[health]` sets `lcov` and no other key, not even one at its default.
    let table: toml::Table = text.parse().unwrap();
    let health: Vec<&String> = table["health"].as_table().unwrap().keys().collect();
    assert_eq!(health, ["lcov"]);
}

#[test]
fn config_get_in_a_linked_worktree_reads_the_main_worktrees_local_file() {
    // req:config/local-settings@6ef008e6
    // req:config/max-builds-setting@11205a64
    let dir = TempDir::new().unwrap();
    let base = dir.path().canonicalize().unwrap();
    let main = base.join("main");
    write(&main, "specs/SPEC.md", "");
    write(&main, "clm.toml", "defects = \"backlog\"\nmax_builds = 2\n");
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .args(args)
            .current_dir(&main)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&[
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "commit",
        "-qm",
        "init",
    ]);
    let linked = base.join("linked");
    git(&["worktree", "add", "-q", linked.to_str().unwrap()]);
    let stdout = |key: &str| String::from_utf8(clm_config_get(&linked, key).stdout).unwrap();
    let local = main.join("clm.local.toml");

    assert_eq!(
        stdout("defects"),
        format!("backlog\t{}\n", linked.join("clm.toml").display())
    );
    fs::write(&local, "defects = \"off\"\nmax_builds = 5\n").unwrap();
    assert_eq!(stdout("defects"), format!("off\t{}\n", local.display()));
    assert_eq!(stdout("max_builds"), format!("5\t{}\n", local.display()));
    let out = clm_in(
        &linked,
        &["config", "get", "max_builds"],
        &[("CLM_MAX_BUILDS", "1".into())],
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "1\tenv CLM_MAX_BUILDS\n"
    );
}

#[test]
fn config_get_writer_unset() {
    // req:config/writer-setting@16b0cd7d
    let (out, _) = config_get_in_repo(None, "writer");
    assert_eq!(out, "(git user.name)\tdefault\n");
}

#[test]
fn config_get_writer_from_the_variable() {
    // req:config/writer-setting@16b0cd7d
    let dir = TempDir::new().unwrap();
    let out = clm_in(
        dir.path(),
        &["config", "get", "writer"],
        &[("CLM_WRITER", "christopher".into())],
    );
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "christopher\tenv CLM_WRITER\n"
    );
}

#[test]
fn config_get_health_defaults() {
    // req:config/health-setting@69ee7c01
    let (out, _) = config_get_in_repo(None, "health");
    assert_eq!(
        out,
        "reconsider=3,scenarios=6,exceptions=4,stale_days=30\tdefault\n"
    );
}

#[test]
fn config_get_health_from_the_repository() {
    // req:config/health-setting@69ee7c01
    let (out, path) = config_get_in_repo(
        Some("[health]\nreconsider = 5\nlcov = \"target/lcov.info\"\n"),
        "health",
    );
    assert_eq!(
        out,
        format!(
            "reconsider=5,scenarios=6,exceptions=4,stale_days=30,lcov=target/lcov.info\t{}\n",
            path.display()
        )
    );
}

#[test]
fn config_get_component_size_defaults() {
    // req:config/component-size-setting@8f8238ab
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", "");
    fs::create_dir(dir.path().join(".git")).unwrap();
    let out = clm_config_get(dir.path(), "component_size");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "spec_tokens=12000,code_tokens=40000\tdefault\n"
    );
}

#[test]
fn config_get_component_size_from_the_repository() {
    // req:config/component-size-setting@8f8238ab
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    write(&root, "specs/SPEC.md", "");
    fs::create_dir(root.join(".git")).unwrap();
    write(&root, "clm.toml", "[component_size]\ncode_tokens = 60000\n");
    let out = clm_config_get(&root, "component_size");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!(
            "spec_tokens=12000,code_tokens=60000\t{}\n",
            root.join("clm.toml").display()
        )
    );
}

#[test]
fn config_get_max_builds() {
    // req:config/max-builds-setting@11205a64
    let (out, _) = config_get_in_repo(None, "max_builds");
    assert_eq!(out, "3\tdefault\n");
    let (out, path) = config_get_in_repo(Some("max_builds = 2\n"), "max_builds");
    assert_eq!(out, format!("2\t{}\n", path.display()));
}

#[test]
fn config_get_affected() {
    // req:config/affected-setting@7b51c4ec
    let (out, _) = config_get_in_repo(None, "affected");
    assert_eq!(out, "\tdefault\n");
    let text = "[affected]\nall = [\"Cargo.lock\", \"rust-toolchain.toml\"]\n";
    let (out, path) = config_get_in_repo(Some(text), "affected");
    assert_eq!(
        out,
        format!("all=Cargo.lock,rust-toolchain.toml\t{}\n", path.display())
    );
}

#[test]
fn config_get_unknown_key_exits_1() {
    // req:config/config-get@e2bb5643
    let dir = TempDir::new().unwrap();
    let out = clm_config_get(dir.path(), "colour");
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("colour"));
}

#[cfg(unix)]
#[test]
fn config_get_rejects_a_variable_that_is_not_utf8() {
    // req:config/invalid-settings@ec741034
    use std::os::unix::ffi::OsStrExt;
    let dir = TempDir::new().unwrap();
    let out = clm_in(
        dir.path(),
        &["config", "get", "defects"],
        &[(
            "CLM_DEFECTS",
            std::ffi::OsStr::from_bytes(b"ask\xff").into(),
        )],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("CLM_DEFECTS"));
}

#[test]
fn config_get_exits_1_when_settings_cannot_load() {
    // req:config/config-get@e2bb5643
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", "");
    write(dir.path(), "clm.toml", "defect = \"off\"\n");
    let out = clm_config_get(dir.path(), "defects");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("clm.toml") && stderr.contains("`defect`"),
        "{stderr}"
    );
}

#[test]
fn config_get_prints_each_upgrade_key_with_its_file() {
    // req:config/upgrade-settings@ec4b2181
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    write(&root, "specs/SPEC.md", "");
    fs::create_dir(root.join(".git")).unwrap();
    write(
        &root,
        "clm.toml",
        "upgraded = \"20261105-clm-upgrade\"\ndeclined_migrations = [\"n\"]\n",
    );
    write(&root, "clm.local.toml", "skip_migrations = [\"x\"]\n");
    for (key, want, file) in [
        ("upgraded", "20261105-clm-upgrade", "clm.toml"),
        ("declined_migrations", "n", "clm.toml"),
        ("skip_migrations", "x", "clm.local.toml"),
    ] {
        let out = clm_config_get(&root, key);
        assert_eq!(out.status.code(), Some(0), "{key}");
        assert_eq!(
            String::from_utf8(out.stdout).unwrap(),
            format!("{want}\t{}\n", root.join(file).display())
        );
    }
}

#[test]
fn config_list_prints_every_key_with_its_layer() {
    // req:config/config-list@7bccae0d
    let (out, root) = config_list_in_repo("defects = \"backlog\"\n", "max_builds = 5\n");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 18, "{stdout}");
    let keys: Vec<&str> = lines
        .iter()
        .map(|l| l.split('\t').next().unwrap())
        .collect();
    assert_eq!(keys, clm_config::KEYS);
    assert!(
        lines.contains(&format!("defects\tbacklog\t{}", root.join("clm.toml").display()).as_str())
    );
    assert!(
        lines.contains(
            &format!("max_builds\t5\t{}", root.join("clm.local.toml").display()).as_str()
        )
    );
    assert!(lines.contains(&"autosync\ttrue\tdefault"));
    assert!(lines.contains(&"env\t\tdefault"));
}

#[test]
fn config_list_prints_a_line_per_env_variable() {
    // req:config/config-list@7bccae0d
    let (out, root) = config_list_in_repo("[env]\nA = \"1\"\n", "[env]\nB = \"2\"\n");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 19, "{stdout}");
    let at = lines.iter().position(|l| l.starts_with("build\t")).unwrap();
    assert_eq!(
        lines[at + 1],
        format!("env\tA=1\t{}", root.join("clm.toml").display())
    );
    assert_eq!(
        lines[at + 2],
        format!("env\tB=2\t{}", root.join("clm.local.toml").display())
    );
    assert!(lines[at + 3].starts_with("verify\t"));
}

#[test]
fn config_list_of_an_invalid_edit_prints_nothing_and_fails() {
    // req:config/config-list@7bccae0d
    let (out, _) = config_list_in_repo("", "max_builds = 0\n");
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("clm.local.toml"), "{stderr}");
    assert!(stderr.contains("max_builds"), "{stderr}");
}

#[test]
fn every_key_config_list_prints_is_one_config_get_accepts() {
    // req:config/config-list@7bccae0d
    let dir = TempDir::new().unwrap();
    for key in clm_config::KEYS {
        let out = clm_config_get(dir.path(), key);
        assert_eq!(out.status.code(), Some(0), "{key}");
    }
}

#[test]
fn config_get_build_prints_the_setup_and_the_file() {
    // req:config/build-setup-setting@f43d20f0
    let (out, file) = config_get_in_repo(Some("[build]\nsetup = \"scripts/setup.sh\"\n"), "build");
    assert_eq!(out, format!("setup=scripts/setup.sh\t{}\n", file.display()));
    let (out, _) = config_get_in_repo(None, "build");
    assert_eq!(out, "\tdefault\n");
}

#[test]
fn config_get_env_prints_a_line_per_variable() {
    // req:config/env-setting@000d4bed
    let (out, file) = config_get_in_repo(Some("[env]\nB = \"2\"\nA = \"1\"\n"), "env");
    assert_eq!(out, format!("A=1\t{0}\nB=2\t{0}\n", file.display()));
    let (out, _) = config_get_in_repo(None, "env");
    assert_eq!(out, "\tdefault\n");
}

#[test]
fn this_repository_names_the_retired_terms_detector() {
    // req:config/retired-terms-detector@afaa1f7a
    let out = clm_config_get(&repo_root(), "detectors");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.starts_with("unused-dependencies,unreferenced-public-items,retired-terms\t"),
        "{stdout}"
    );
}

#[test]
fn config_get_verify_prints_the_table_and_the_file() {
    // req:config/verify-setting@93c5194a
    let (out, file) = config_get_in_repo(
        Some("[verify]\ncheck = \"scripts/slow.sh\"\nmode = \"local\"\n"),
        "verify",
    );
    assert_eq!(
        out,
        format!("mode=local,check=scripts/slow.sh\t{}\n", file.display())
    );
    let (out, _) = config_get_in_repo(None, "verify");
    assert_eq!(out, "\tdefault\n");
}
