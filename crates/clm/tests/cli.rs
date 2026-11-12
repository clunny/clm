// clm:reads specs/**
// clm:reads backlog/**
// clm:reads clm.toml

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

/// `clm`, with a user config dir that holds no `clm/config.toml` and no CLM variables, so the
/// developer's own settings never leak in.
fn clm_command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_clm"));
    cmd.env(
        "XDG_CONFIG_HOME",
        Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-user-config"),
    )
    .env_remove("CLM_AUTOSYNC")
    .env_remove("CLM_DEFECTS")
    .env_remove("CLM_MAX_BUILDS")
    .env_remove("CLM_WRITER");
    cmd
}

fn check_command(root: &Path) -> Command {
    let mut cmd = clm_command();
    cmd.args(["check", "--root"]).arg(root);
    cmd
}

fn clm_check(root: &Path) -> Output {
    check_command(root).output().unwrap()
}

/// `clm check` on `root` with `PATH` set to the directory of the `clm` under test followed by
/// `path`, so a detector whose command is `clm` runs this build wherever the test runs.
fn clm_check_on_path(root: &Path, path: &std::ffi::OsStr) -> Command {
    let own = Path::new(env!("CARGO_BIN_EXE_clm")).parent().unwrap();
    let joined =
        std::env::join_paths(std::iter::once(own.to_path_buf()).chain(std::env::split_paths(path)))
            .unwrap();
    let mut cmd = check_command(root);
    cmd.env("PATH", joined);
    cmd
}

/// Runs `clm config get <key>` in `cwd`.
fn clm_config_get(cwd: &Path, key: &str) -> Output {
    clm_command()
        .args(["config", "get", key])
        .current_dir(cwd)
        .output()
        .unwrap()
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn reports_every_violation_and_exits_1() {
    // req:check/report-and-exit@2528ee97
    let dir = TempDir::new().unwrap();
    let fm = |deps: &str| {
        format!("---\ntitle: T\nstatus: specd\nowners: []\ndepends_on: [{deps}]\n---\n")
    };
    write(dir.path(), "specs/SPEC.md", &fm(""));
    write(dir.path(), "specs/a/SPEC.md", &fm("b"));
    write(
        dir.path(),
        "specs/b/SPEC.md",
        &format!(
            "{}## Requirements\n### Requirement: x\n#### Scenario: s\n### Requirement: x\n#### Scenario: s\n",
            fm("a")
        ),
    );

    let out = clm_check(dir.path());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    assert!(
        stdout.contains("specs/a/SPEC.md: model/dependency-dag: "),
        "{stdout}"
    );
    assert!(
        stdout.contains("specs/b/SPEC.md: model/unique-requirement-ids: "),
        "{stdout}"
    );
}

#[test]
fn this_repository_is_clean() {
    // req:check/report-and-exit@2528ee97
    // req:clm/clean-check-runs-own-clm@06b04860
    let repo_root = repo_root();
    let out = clm_check_on_path(&repo_root, &std::env::var_os("PATH").unwrap_or_default())
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// The directory of the first `git` on the test process's `PATH`.
fn git_dir() -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .find(|dir| {
            dir.join(format!("git{}", std::env::consts::EXE_SUFFIX))
                .is_file()
        })
        .expect("git on PATH")
}

#[test]
fn clean_check_runs_the_build_under_test_as_clm() {
    // req:clm/clean-check-runs-own-clm@06b04860
    let dir = detector_repo(Some(
        "[[detectors]]\nname = \"own\"\ncommand = [\"clm\", \"--version\"]\n",
    ));
    let out = clm_check_on_path(dir.path(), git_dir().as_os_str())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(0), "{stdout}{stderr}");
    assert!(!stdout.contains("could not run"), "{stdout}");
    // The detector printed this build's version, not another `clm`'s.
    let own = clm_command().arg("--version").output().unwrap();
    let own = String::from_utf8(own.stdout).unwrap();
    assert!(stderr.contains(own.trim()), "{stderr}");
}

#[test]
fn clean_check_keeps_the_inherited_path() {
    // req:clm/clean-check-runs-own-clm@06b04860
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    let repo_root = repo_root();
    let cmd = clm_check_on_path(&repo_root, &inherited);
    let path = cmd
        .get_envs()
        .find(|(key, _)| *key == "PATH")
        .and_then(|(_, value)| value)
        .expect("PATH set")
        .to_owned();
    let mut entries = std::env::split_paths(&path);
    assert_eq!(
        entries.next().as_deref(),
        Path::new(env!("CARGO_BIN_EXE_clm")).parent()
    );
    let rest: Vec<_> = entries.collect();
    assert_eq!(rest, std::env::split_paths(&inherited).collect::<Vec<_>>());
}

/// The root of this repository.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The detectors this repository's `clm.toml` names, loaded from a copy of it alone, so neither
/// a clone's `clm.local.toml` nor the developer's own settings leak in.
fn this_repository_detectors() -> Vec<clm_config::Detector> {
    let dir = TempDir::new().unwrap();
    fs::copy(repo_root().join("clm.toml"), dir.path().join("clm.toml")).unwrap();
    let empty = dir.path().join("no-user-config");
    let env = |key: &str| (key == "XDG_CONFIG_HOME").then(|| empty.display().to_string());
    clm_config::load(Some(dir.path()), &env)
        .unwrap()
        .detectors
        .value
}

fn unreferenced_detector() -> clm_config::Detector {
    this_repository_detectors()
        .into_iter()
        .find(|d| d.name == "unreferenced-public-items")
        .expect("unreferenced-public-items detector")
}

/// The lines inside the `unreferenced-public-items` detector's `command = [ ... ]` array, as
/// this repository's `clm.toml` writes them.
fn unreferenced_command_lines() -> Vec<String> {
    let text = fs::read_to_string(repo_root().join("clm.toml")).unwrap();
    let mut lines = text
        .lines()
        .skip_while(|l| l.trim() != r#"name = "unreferenced-public-items""#)
        .skip_while(|l| l.trim() != "command = [");
    assert!(lines.next().is_some(), "no multi-line command array");
    lines
        .take_while(|l| l.trim() != "]")
        .map(|l| l.trim().to_string())
        .collect()
}

/// The value of a `"<flag>", "<value>",` line.
fn pair_value<'a>(line: &'a str, flag: &str) -> Option<&'a str> {
    line.strip_prefix(&format!(r#""{flag}", ""#))?
        .strip_suffix(r#"","#)
        .filter(|v| !v.is_empty() && !v.contains('"'))
}

#[test]
fn unreferenced_detector_is_named_here() {
    // req:config/unreferenced-detector@f0c53e15
    let names: Vec<_> = this_repository_detectors()
        .into_iter()
        .map(|d| d.name)
        .collect();
    let machete = names.iter().position(|n| n == "unused-dependencies");
    let unreferenced = names.iter().position(|n| n == "unreferenced-public-items");
    assert!(
        matches!((machete, unreferenced), (Some(m), Some(u)) if m < u),
        "{names:?}"
    );

    let detector = unreferenced_detector();
    assert_eq!(detector.hint.as_deref(), Some("scripts/dev-install.sh"));
    let command = &detector.command;
    assert_eq!(command[..2], ["clm", "unreferenced"], "{command:?}");
    let pairs = command[2..].chunks(2);
    assert!(command.len().is_multiple_of(2), "{command:?}");
    let mut allowing = false;
    for pair in pairs {
        match pair[0].as_str() {
            "--entry" => assert!(!allowing, "--entry after --allow: {pair:?}"),
            "--allow" => {
                allowing = true;
                // `<crate>::<name>` or `<crate>::<Type>::<name>`, never a whole library.
                let parts: Vec<_> = pair[1].split("::").collect();
                assert!(
                    (2..=3).contains(&parts.len())
                        && parts.iter().all(|p| {
                            !p.is_empty() && p.chars().all(|c| c.is_alphanumeric() || c == '_')
                        }),
                    "not one item: {pair:?}"
                );
            }
            _ => panic!("not an --entry or --allow pair: {pair:?}"),
        }
    }
}

/// The ID, with `.` replaced by `-`, of the node whose `paths` hold the library `krate`
/// (`clm_web` -> `crates/clm-web/**` -> `specs/web/worker/SPEC.md` -> `web-worker`).
fn crate_node_slug(krate: &str) -> String {
    let path = format!("\"crates/{}/**\"", krate.replace('_', "-"));
    let specs = repo_root().join("specs");
    let mut dirs = vec![specs.clone()];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap().path();
            if entry.is_dir() {
                dirs.push(entry);
            } else if entry.file_name().is_some_and(|n| n == "SPEC.md")
                && fs::read_to_string(&entry)
                    .unwrap()
                    .lines()
                    .any(|l| l.starts_with("paths:") && l.contains(&path))
            {
                let node = dir.strip_prefix(&specs).unwrap();
                let parts: Vec<_> = node.iter().map(|p| p.to_string_lossy()).collect();
                return parts.join("-");
            }
        }
    }
    panic!("no node owns {path}");
}

#[test]
fn unreferenced_allow_list_gives_every_entry_a_reason() {
    // req:config/unreferenced-detector@f0c53e15
    let lines = unreferenced_command_lines();
    let mut lines = lines.iter().filter(|l| !l.is_empty());
    assert_eq!(
        lines.next().map(String::as_str),
        Some(r#""clm", "unreferenced","#)
    );
    // The comment lines since the last pair: the reason the next pairs come under.
    let mut reason: Vec<&str> = Vec::new();
    let mut after_pair = false;
    let mut allowing = false;
    for line in lines {
        if let Some(comment) = line.strip_prefix('#') {
            if after_pair {
                reason.clear();
                after_pair = false;
            }
            reason.push(comment);
            assert!(
                !comment.contains("narrow-public-items-config"),
                "a pending config item: {line}"
            );
            let mut rest = comment;
            while let Some(at) = rest.find("backlog/narrow-public-items-") {
                rest = &rest[at + "backlog/".len()..];
                let slug: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
                    .collect();
                let item = repo_root().join("backlog").join(format!("{slug}.md"));
                assert!(item.is_file(), "{line}: no {}", item.display());
            }
            continue;
        }
        assert!(!reason.is_empty(), "a pair before any comment: {line}");
        after_pair = true;
        if pair_value(line, "--entry").is_some() {
            assert!(!allowing, "--entry after --allow: {line}");
        } else if let Some(entry) = pair_value(line, "--allow") {
            allowing = true;
            let reason = reason.join(" ");
            let krate = entry.split("::").next().unwrap();
            if krate == "clm_config" || !reason.contains("narrow-public-items-") {
                assert!(
                    reason.contains("Kept"),
                    "{entry} is neither kept nor pending"
                );
            } else {
                let item = format!("backlog/narrow-public-items-{}", crate_node_slug(krate));
                assert!(reason.contains(&item), "{entry} is pending, not on {item}");
            }
        } else {
            panic!("neither a comment nor one pair: {line}");
        }
    }
    // An empty list is fine; a reason with nothing under it isn't.
    assert!(
        after_pair || reason.is_empty(),
        "no pairs after the last comment"
    );
}

#[test]
fn unreferenced_is_clean_with_the_allow_list() {
    // req:config/unreferenced-detector@f0c53e15
    let detector = unreferenced_detector();
    let out = clm_command()
        .args(&detector.command[1..])
        .arg("--root")
        .arg(repo_root())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(stdout, "");
}

#[test]
fn check_reports_marker_for_finished_change() {
    // req:check/remove-by-markers@4ca98d5a
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "specs/SPEC.md",
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    );
    write(
        dir.path(),
        "changes/20261023-drop-old/change.md",
        "---\nstatus: done\n---\n",
    );
    let marker = concat!("clm:remove", "-by 20261023-drop-old");
    write(
        dir.path(),
        "src/old.rs",
        &format!("fn a() {{}}\n\n// The old path.\n// {marker}\nfn old() {{}}\n"),
    );

    let out = clm_check(dir.path());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert!(
        stdout.lines().any(|l| l
            == "src/old.rs:4: check/remove-by-markers: marker names 20261023-drop-old, which is done"),
        "{stdout}"
    );
}

/// A clean repo with `clm_toml` as its `clm.toml`.
fn detector_repo(clm_toml: Option<&str>) -> TempDir {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "specs/SPEC.md",
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    );
    if let Some(text) = clm_toml {
        write(dir.path(), "clm.toml", text);
    }
    dir
}

const FAILING_DETECTORS: &str = "[[detectors]]\nname = \"ok\"\ncommand = [\"true\"]\n\n[[detectors]]\nname = \"bad\"\ncommand = [\"sh\", \"-c\", \"echo unused-dep; exit 3\"]\n";

#[test]
fn check_reports_failing_detectors() {
    // req:check/detectors@8bf99b6d
    let dir = detector_repo(Some(FAILING_DETECTORS));
    let out = clm_check(dir.path());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(1), "{stdout}{stderr}");
    assert_eq!(stdout, "clm.toml: check/detectors: bad failed (exit 3)\n");
    assert!(stderr.contains("unused-dep"), "{stderr}");
}

#[cfg(unix)]
#[test]
fn check_stops_a_hanging_detector() {
    // req:check/detectors@8bf99b6d
    let dir = detector_repo(Some(
        "[[detectors]]\nname = \"hang\"\ncommand = [\"sh\", \"-c\", \"echo started; sleep 30 & echo $! > child.pid; wait\"]\ntimeout = 1\n\n[[detectors]]\nname = \"bad\"\ncommand = [\"sh\", \"-c\", \"exit 3\"]\n",
    ));
    let started = std::time::Instant::now();
    let out = clm_check(dir.path());
    assert!(started.elapsed().as_secs() < 10, "{:?}", started.elapsed());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(1), "{stdout}{stderr}");
    assert_eq!(
        stdout,
        "clm.toml: check/detectors: hang timed out after 1s\nclm.toml: check/detectors: bad failed (exit 3)\n"
    );
    assert!(stderr.contains("started"), "{stderr}");
    let pid = fs::read_to_string(dir.path().join("child.pid")).unwrap();
    let alive = std::process::Command::new("sh")
        .args(["-c", &format!("ps -o stat= -p {} | grep -v Z", pid.trim())])
        .output()
        .unwrap();
    assert!(!alive.status.success(), "the detector's child still runs");
}

#[test]
fn check_reports_a_detector_that_cannot_run() {
    // req:check/detectors@8bf99b6d
    let dir = detector_repo(Some(
        "[[detectors]]\nname = \"machete\"\ncommand = [\"no-such-tool-xyz\"]\nhint = \"cargo install cargo-machete\"\n",
    ));
    let out = clm_check(dir.path());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    let line = stdout.trim_end();
    assert!(
        line.starts_with("clm.toml: check/detectors: machete could not run:"),
        "{stdout}"
    );
    assert!(
        line.ends_with("; install with: cargo install cargo-machete"),
        "{stdout}"
    );
}

#[test]
fn check_runs_detectors_from_the_repository_root() {
    // req:check/detectors@8bf99b6d
    let dir = detector_repo(Some(
        "[[detectors]]\nname = \"root\"\ncommand = [\"test\", \"-f\", \"specs/SPEC.md\"]\n",
    ));
    let elsewhere = TempDir::new().unwrap();
    let out = clm_command()
        .current_dir(elsewhere.path())
        .args(["check", "--root"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
}

#[test]
fn check_without_detectors_passes() {
    // req:check/detectors@8bf99b6d
    for clm_toml in [None, Some("defects = \"backlog\"\n")] {
        let dir = detector_repo(clm_toml);
        let out = clm_check(dir.path());
        assert_eq!(out.status.code(), Some(0), "{clm_toml:?}: {out:?}");
    }
}

#[test]
fn scenario_detectors_skipped_on_request() {
    // req:check/detectors@8bf99b6d
    let dir = detector_repo(Some(
        "[[detectors]]\nname = \"bad\"\ncommand = [\"sh\", \"-c\", \"touch ran; exit 3\"]\n",
    ));
    let out = clm_command()
        .args(["check", "--no-detectors", "--root"])
        .arg(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(!stdout.contains("check/detectors"), "{stdout}");
    assert!(!dir.path().join("ran").exists(), "the detector ran");
    // Without the flag the same repository fails on the detector.
    let out = clm_check(dir.path());
    assert_eq!(out.status.code(), Some(1));
    assert!(dir.path().join("ran").exists());
}

#[test]
fn check_reports_settings_errors_and_still_runs_rules() {
    // req:check/detectors@8bf99b6d
    let dir = detector_repo(Some(FAILING_DETECTORS));
    // A spec violation `run` finds: an unparseable node.
    write(dir.path(), "specs/a/SPEC.md", "no frontmatter\n");
    let config = TempDir::new().unwrap();
    write(
        config.path(),
        "clm/config.toml",
        "[[detectors]]\nname = \"x\"\ncommand = [\"true\"]\n",
    );
    let out = clm_command()
        .env("XDG_CONFIG_HOME", config.path())
        .args(["check", "--root"])
        .arg(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    let user_file = config.path().join("clm/config.toml");
    let settings_line = stdout
        .lines()
        .find(|l| l.contains("check/detectors"))
        .unwrap_or_else(|| panic!("{stdout}"));
    let error = settings_line
        .strip_prefix(&format!("{}: check/detectors: ", user_file.display()))
        .unwrap_or_else(|| panic!("{stdout}"));
    assert!(error.contains("detectors"), "{stdout}");
    assert!(!error.contains('^'), "no source snippet: {stdout}");
    // The detectors were skipped, `run`'s violations still print, one line each.
    assert!(!stdout.contains("bad failed"), "{stdout}");
    assert!(stdout.contains("specs/a/SPEC.md: "), "{stdout}");
    assert_eq!(stdout.lines().count(), 2, "{stdout}");
}

/// A git repository with nodes `a` (`crates/a/**`, depending on `a_deps`) and `b`
/// (`crates/b/**`, or `b_paths`), and a Cargo workspace where `a` has a path dependency on `b`.
fn import_repo(a_deps: &str, b_paths: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let node = |paths: &str, deps: &str| {
        format!(
            "---\ntitle: N\nstatus: active\nowners: []\ndepends_on: {deps}\npaths: {paths}\n---\n## Contract\n"
        )
    };
    write(root, "specs/SPEC.md", &node("[]", "[]"));
    write(root, "specs/a/SPEC.md", &node("[\"crates/a/**\"]", a_deps));
    write(root, "specs/b/SPEC.md", &node(b_paths, "[]"));
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*\"]\n",
    );
    write(
        root,
        "crates/a/Cargo.toml",
        "[package]\nname = \"a\"\n\n[dependencies]\nb = { path = \"../b\" }\n",
    );
    write(root, "crates/a/src/lib.rs", "");
    write(root, "crates/b/Cargo.toml", "[package]\nname = \"b\"\n");
    write(root, "crates/b/src/lib.rs", "");
    for args in [&["init", "-q"][..], &["add", "."]] {
        let status = Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .unwrap();
        assert!(status.success());
    }
    dir
}

#[test]
fn check_reports_an_undeclared_import() {
    // req:check/undeclared-imports@f84e6527
    let dir = import_repo("[]", "[\"crates/b/**\"]");
    let line = "crates/a/Cargo.toml: check/undeclared-imports: imports crates/b/Cargo.toml of component `b`, and `a` doesn't depend on `b`\n";
    let out = clm_check(dir.path());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert_eq!(stdout, line);
    let out = clm_command()
        .args(["check", "--no-detectors", "--root"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(out.stdout).unwrap(), line);
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn check_passes_a_declared_or_unowned_import() {
    // req:check/undeclared-imports@f84e6527
    for dir in [
        import_repo("[b]", "[\"crates/b/**\"]"),
        import_repo("[]", "[\"crates/b/src/**\"]"),
    ] {
        let out = clm_check(dir.path());
        assert_eq!(out.status.code(), Some(0), "{out:?}");
    }
}

#[test]
fn no_review_command() {
    // After-merge review is gone with yes/no review (`yes-no-review`).
    let dir = TempDir::new().unwrap();
    let out = clm_command()
        .args(["review", "x", "--root"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("unrecognized subcommand 'review'"),
        "{stderr}"
    );
}

fn clm(root: &Path, args: &[&str]) -> (Option<i32>, String) {
    let out = clm_command()
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
        .unwrap();
    (out.status.code(), String::from_utf8(out.stdout).unwrap())
}

const NODE: &str = "---\ntitle: T\nstatus: specd\nowners: []\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n\n### Requirement: r\nR.\n\n#### Scenario: s\n- ok\n";

fn change_repo(delta: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", NODE);
    write(
        dir.path(),
        "changes/c/change.md",
        "---\nstatus: proposed\n---\n",
    );
    write(dir.path(), "changes/c/deltas/clm.md", delta);
    dir
}

#[test]
fn hashes_prints_elements_usable_as_bases() {
    // req:apply/hash-command@971912c5
    let dir = change_repo("---\ncomponent: clm\n---\n");
    let (code, out) = clm(dir.path(), &["hashes", "clm"]);
    assert_eq!(code, Some(0));
    let elements: Vec<_> = out.lines().map(|l| l.split(' ').next().unwrap()).collect();
    assert_eq!(
        elements,
        ["frontmatter", "contract", "design", "requirement:r"]
    );
    let hash = |n: usize| out.lines().nth(n).unwrap().split(' ').nth(1).unwrap();
    let (contract, r) = (hash(1), hash(3));
    write(
        dir.path(),
        "changes/c/deltas/clm.md",
        &format!(
            "---\ncomponent: clm\nbase: {{ contract: \"{contract}\" }}\n---\n## MODIFIED Contract\nc2\n\n## MODIFIED Requirements\n### Requirement: r <!-- base: {r} -->\nR2.\n\n#### Scenario: s\n- ok\n"
        ),
    );
    let (code, out) = clm(dir.path(), &["apply", "c"]);
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn hashes_prints_asset_lines() {
    // req:apply/hash-command@971912c5 req:apply/apply-assets@786adb62
    let dir = change_repo("---\ncomponent: clm\n---\n");
    write(dir.path(), "specs/a/SPEC.md", NODE);
    write(dir.path(), "specs/a/assets/m.html", "<p>m</p>\n");
    let (code, out) = clm(dir.path(), &["hashes", "a"]);
    assert_eq!(code, Some(0));
    let expected = format!("asset:m.html {}", clm_model::asset_hash(b"<p>m</p>\n"));
    assert_eq!(out.lines().last(), Some(expected.as_str()), "{out}");
}

#[test]
fn hash_is_no_command() {
    // req:apply/hash-command@971912c5
    let dir = change_repo("---\ncomponent: clm\n---\n");
    write(dir.path(), "specs/a/SPEC.md", NODE);
    // Not stderr whole: clap's for `hash` may suggest `hashes`, unlike a made-up name's.
    let run = |name: &str| {
        let out = clm_command()
            .args([name, "a", "--root"])
            .arg(dir.path())
            .output()
            .unwrap();
        let stderr = String::from_utf8(out.stderr).unwrap();
        (
            out.status.code(),
            String::from_utf8(out.stdout).unwrap(),
            stderr.contains("unrecognized subcommand"),
        )
    };
    let unknown = run("nosuchcommand");
    assert_ne!(unknown.0, Some(0));
    assert!(unknown.2);
    assert_eq!(run("hash"), unknown);
    assert_eq!(unknown.1, "");
    let out = clm_command().arg("--help").output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    let help = String::from_utf8(out.stdout).unwrap();
    // Whole words, since `hashes` contains `hash`: no subcommand or alias is named `hash`.
    assert!(
        help.lines().any(|l| l.trim_start().starts_with("hashes ")),
        "{help}"
    );
    assert!(
        !help
            .split(|c: char| !c.is_alphanumeric())
            .any(|w| w == "hash"),
        "{help}"
    );
}

#[test]
fn hashes_fails_on_an_unknown_node() {
    // req:apply/hash-command@971912c5
    let dir = change_repo("---\ncomponent: clm\n---\n");
    let out = clm_command()
        .args(["hashes", "nope", "--root"])
        .arg(dir.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr.contains("clm hashes: no valid component `nope`"),
        "{stderr}"
    );
}

#[test]
fn apply_reports_and_exits_1_writing_nothing() {
    // req:apply/atomic@bb8aa354
    let dir = change_repo(
        "---\ncomponent: clm\nbase: { contract: \"0000000000000000\" }\n---\n## MODIFIED Contract\nc2\n",
    );
    let before = fs::read_to_string(dir.path().join("specs/SPEC.md")).unwrap();
    let (code, out) = clm(dir.path(), &["apply", "c"]);
    assert_eq!(code, Some(1));
    assert!(
        out.contains("changes/c/deltas/clm.md: apply/stale-base: "),
        "{out}"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("specs/SPEC.md")).unwrap(),
        before
    );
}

#[test]
fn apply_exits_1_on_a_stale_asset_and_reports_removals() {
    // req:apply/apply-assets@786adb62
    let dir = change_repo("---\ncomponent: clm\n---\n");
    let d = dir.path();
    write(d, "specs/a/SPEC.md", NODE);
    write(d, "specs/a/assets/m.html", "main's edit\n");
    write(d, "specs/a/assets/old.png", "png");
    write(
        d,
        "changes/c/deltas/a.md",
        "---\ncomponent: a\nbase: { assets: { \"m.html\": \"0000000000000000\" } }\n---\n",
    );
    write(d, "changes/c/assets/a/m.html", "new\n");
    let (code, out) = clm(d, &["apply", "c"]);
    assert_eq!(code, Some(1));
    assert!(
        out.contains("apply/stale-base: ") && out.contains("m.html"),
        "{out}"
    );
    assert_eq!(
        fs::read_to_string(d.join("specs/a/assets/m.html")).unwrap(),
        "main's edit\n"
    );

    let old = clm_model::asset_hash(b"png");
    let m = clm_model::asset_hash(b"main's edit\n");
    write(
        d,
        "changes/c/deltas/a.md",
        &format!(
            "---\ncomponent: a\nbase: {{ assets: {{ \"m.html\": \"{m}\" }} }}\n---\n## REMOVED Assets\n- `old.png` <!-- base: {old} -->\n"
        ),
    );
    let (code, out) = clm(d, &["apply", "c"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("  wrote specs/a/assets/m.html\n"), "{out}");
    assert!(out.contains("  removed specs/a/assets/old.png\n"), "{out}");
}

#[test]
fn check_fails_stale_pending_change_but_ignores_applied_one() {
    // req:check/stale-deltas@c244701b
    let stale = "---\ncomponent: clm\nbase: { contract: \"0000000000000000\" }\n---\n## MODIFIED Contract\nc2\n";
    let dir = change_repo(stale);
    let (code, out) = clm(dir.path(), &["check"]);
    assert_eq!(code, Some(1));
    assert!(
        out.contains("changes/c/deltas/clm.md: apply/stale-base: contract"),
        "{out}"
    );

    write(
        dir.path(),
        "changes/c/change.md",
        "---\nstatus: applied\n---\n",
    );
    let (code, out) = clm(dir.path(), &["check"]);
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn coverage_command_prints_report() {
    // req:check.coverage/coverage-report@1fec41ce
    let dir = change_repo("---\ncomponent: clm\n---\n");
    write(dir.path(), "t.rs", concat!("// req", ":clm/r\n"));
    let (code, out) = clm(dir.path(), &["coverage"]);
    assert_eq!(code, Some(0));
    assert_eq!(out, "clm/r behind t.rs:1\n");
    let (graph, _) = clm_model::load(dir.path());
    let v = graph.requirement("clm/r").unwrap().version().to_string();
    write(
        dir.path(),
        "t.rs",
        &format!("// {}clm/r@{v}\n", concat!("req", ":")),
    );
    let (_, out) = clm(dir.path(), &["coverage"]);
    assert_eq!(out, "clm/r current t.rs:1\n");
}

#[test]
fn backlog_and_archive_commands() {
    // req:check/backlog-report@392ec9fd
    let dir = change_repo("---\ncomponent: clm\n---\n");
    write(
        dir.path(),
        "backlog/a.md",
        "---\ntitle: Ay\nkind: question\n---\n",
    );
    let (code, out) = clm(dir.path(), &["backlog"]);
    assert_eq!((code, out.as_str()), (Some(0), "a question P5 [] Ay\n"));

    // req:apply/archive-command@c68462bb
    let (code, _) = clm(dir.path(), &["archive", "c"]);
    assert_eq!(code, Some(1), "proposed change must not archive");
    write(
        dir.path(),
        "changes/c/change.md",
        "---\nstatus: abandoned\n---\n",
    );
    let (code, out) = clm(dir.path(), &["archive", "c"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(dir.path().join("changes/archive/c/change.md").is_file());
}

#[test]
fn orchestrator_commands_exit_2_when_git_fails() {
    // req:orchestrator/next@1f34c883 req:orchestrator/claim@026fab7a
    // req:orchestrator/backlog-claim@50959cb7 req:orchestrator/proposals@ba5ce6a8
    // req:orchestrator/claim-fix@5a3c119e
    let dir = change_repo("---\ncomponent: clm\n---\n");
    // Not a git repository, so there's no `origin` to fetch.
    assert_eq!(clm(dir.path(), &["next"]).0, Some(2));
    assert_eq!(clm(dir.path(), &["claim", "clm"]).0, Some(2));
    let fix = clm(dir.path(), &["claim", "clm", "--fix", "backlog/q"]);
    assert_eq!(fix.0, Some(2));
    assert_eq!(clm(dir.path(), &["claim", "backlog/q"]).0, Some(2));
    assert_eq!(clm(dir.path(), &["proposals"]).0, Some(2));
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

#[test]
fn config_get_host_defaults_to_github() {
    // req:config/host-setting@cbcd8366
    let (out, _) = config_get_in_repo(None, "host");
    assert_eq!(out, "github\tdefault\n");
}

#[test]
fn config_get_host_from_the_repository() {
    // req:config/host-setting@cbcd8366
    let (out, path) = config_get_in_repo(Some("host = \"git\"\n"), "host");
    assert_eq!(out, format!("git\t{}\n", path.display()));
}

#[test]
fn config_get_host_azure() {
    // req:config/host-setting@cbcd8366
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
    // req:config/host-setting@cbcd8366
    // req:config/affected-setting@7b51c4ec
    // req:config/health-lcov-here@1b784026
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
            "reconsider=3,scenarios=6,exceptions=4,stale_days=30,lcov=target/lcov.info",
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
    let out = clm_command()
        .args(["config", "get", "max_builds"])
        .current_dir(&linked)
        .env("CLM_MAX_BUILDS", "1")
        .output()
        .unwrap();
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
    let out = clm_command()
        .args(["config", "get", "writer"])
        .current_dir(dir.path())
        .env("CLM_WRITER", "christopher")
        .output()
        .unwrap();
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
    let out = clm_command()
        .args(["config", "get", "defects"])
        .current_dir(dir.path())
        .env("CLM_DEFECTS", std::ffi::OsStr::from_bytes(b"ask\xff"))
        .output()
        .unwrap();
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
fn hashes_of_layers() {
    // req:apply/hash-command@971912c5
    let dir = change_repo("---\ncomponent: clm\n---\n");
    let entry = |slug: &str| {
        format!("### Capability: {slug}\nOne can.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n")
    };
    // File order, not sorted: `b` first.
    let (a, b) = (entry("b"), entry("a"));
    write(
        dir.path(),
        "CAPABILITIES.md",
        &format!("## Capabilities\n\n{a}\n{b}"),
    );
    let x = "### Constraint: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n";
    write(
        dir.path(),
        "ARCHITECTURE.md",
        &format!("## Constraints\n\n{x}"),
    );
    let hash = |text: &str| clm_model::content_hash(text);
    let (code, out) = clm(dir.path(), &["hashes", "capability"]);
    assert_eq!(code, Some(0), "{out}");
    // Each block runs to the next heading: the first holds the blank line before the second.
    let expected = format!(
        "capability:b {}\ncapability:a {}\n",
        hash(&format!("{a}\n")),
        hash(&b)
    );
    assert_eq!(out, expected);
    let (code, out) = clm(dir.path(), &["hashes", "architecture"]);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(out, format!("requirement:x {}\n", hash(x)));
}

#[test]
fn hashes_of_the_capability_node_come_first() {
    // req:apply/hash-command@971912c5
    let dir = change_repo("---\ncomponent: clm\n---\n");
    write(
        dir.path(),
        "CAPABILITIES.md",
        "## Capabilities\n\n### Capability: a\nOne can.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n",
    );
    write(
        dir.path(),
        "specs/capability/SPEC.md",
        "---\ntitle: Capabilities\nstatus: specd\nowners: []\ndepends_on: []\n---\n\n## Contract\nc\n\n## Design\nd\n",
    );
    let (code, out) = clm(dir.path(), &["hashes", "capability"]);
    assert_eq!(code, Some(0), "{out}");
    let elements: Vec<_> = out.lines().map(|l| l.split(' ').next().unwrap()).collect();
    assert_eq!(
        elements,
        ["frontmatter", "contract", "design", "capability:a"]
    );
    let design = out.lines().nth(2).unwrap().split(' ').nth(1).unwrap();
    write(
        dir.path(),
        "changes/c/deltas/clm.md",
        &format!(
            "---\ncomponent: capability\nbase: {{ design: \"{design}\" }}\n---\n## MODIFIED Design\nnew\n"
        ),
    );
    let (code, out) = clm(dir.path(), &["apply", "c"]);
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn help_lists_the_new_command_names_only() {
    // req:clm/renamed-commands@164ad075
    let out = clm_command().arg("--help").output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let commands: Vec<&str> = text
        .lines()
        .skip_while(|l| !l.starts_with("Commands:"))
        .skip(1)
        .take_while(|l| l.starts_with("  "))
        .filter_map(|l| l.split_whitespace().next())
        .collect();
    for new in ["rebase", "wrap-up", "pr"] {
        assert!(commands.contains(&new), "{new} in {commands:?}");
    }
    for old in ["sync", "checkpoint", "page"] {
        assert!(!commands.contains(&old), "{old} in {commands:?}");
    }
}

#[test]
fn help_calls_a_branchs_pull_request_a_pull_request() {
    // req:clm/help-says-pull-request@d8f42a64
    let help = |args: &[&str]| {
        let out = clm_command().args(args).output().unwrap();
        String::from_utf8(out.stdout).unwrap()
    };
    for args in [&["pr", "open", "--help"][..], &["pr", "body", "--help"][..]] {
        let text = help(args);
        assert!(text.contains("pull request"), "{text}");
        if args[1] == "open" {
            assert!(text.contains("pull request's title"), "{text}");
            assert!(text.contains("pull request's body"), "{text}");
        }
        let has_page = text
            .split(|c: char| !c.is_alphanumeric())
            .any(|w| w.eq_ignore_ascii_case("page") || w.eq_ignore_ascii_case("pages"));
        assert!(!has_page, "{text}");
    }
    let draft = help(&["draft", "--help"]);
    let fold: String = draft
        .lines()
        .skip_while(|l| !l.contains("--fold"))
        .take_while(|l| !l.contains("--answers"))
        .collect::<Vec<_>>()
        .join(" ");
    assert!(fold.contains("one pull request"), "{draft}");
}

const KNIP_DETECTOR: &str =
    "[[detectors]]\nname = \"knip\"\ncommand = [\"node_modules/.bin/knip\"]\nhint = \"npm ci\"\n";

/// `clm check --root <root>` run from `repo/sub`, so a relative first word can't resolve there.
fn check_from_subdirectory(repo: &Path) -> Output {
    let sub = repo.join("sub");
    fs::create_dir_all(&sub).unwrap();
    check_command(repo).current_dir(&sub).output().unwrap()
}

#[test]
fn scenario_relative_detector_tool_not_installed() {
    // req:check/detector-relative-command@9c07fea0
    let dir = detector_repo(Some(KNIP_DETECTOR));
    let out = check_from_subdirectory(dir.path());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    let line = stdout.trim_end();
    assert!(
        line.starts_with("clm.toml: check/detectors: knip could not run:"),
        "{stdout}"
    );
    assert!(line.ends_with("; install with: npm ci"), "{stdout}");
}

#[cfg(unix)]
#[test]
fn scenario_relative_detector_tool_installed() {
    // req:check/detector-relative-command@9c07fea0
    let dir = detector_repo(Some(KNIP_DETECTOR));
    // A link to `true` stands for an installed tool, with no freshly written executable to start.
    let bin = dir.path().join("node_modules/.bin");
    fs::create_dir_all(&bin).unwrap();
    std::os::unix::fs::symlink("/bin/true", bin.join("knip")).unwrap();
    let out = check_from_subdirectory(dir.path());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("check/detectors"), "{stdout}");
    assert_eq!(out.status.code(), Some(0), "{stdout}");
}

const REQUIRES_DETECTOR: &str = "[[detectors]]\nname = \"knip\"\ncommand = [\"sh\", \"-c\", \"touch ran; exit 3\"]\nrequires = \"package.json\"\n";

#[test]
fn check_skips_a_detector_whose_required_file_is_missing() {
    // req:clm/detector-requires-skip@3977bf75
    let dir = detector_repo(Some(REQUIRES_DETECTOR));
    let out = clm_check(dir.path());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(0), "{stdout}{stderr}");
    assert!(!stdout.contains("check/detectors"), "{stdout}");
    assert!(stderr.contains("knip skipped: no package.json"), "{stderr}");
    assert!(!dir.path().join("ran").exists());
}

#[test]
fn check_runs_a_detector_whose_required_file_exists() {
    // req:clm/detector-requires-skip@3977bf75
    let dir = detector_repo(Some(REQUIRES_DETECTOR));
    write(dir.path(), "package.json", "{}");
    let out = clm_check(dir.path());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("clm.toml: check/detectors: knip failed (exit 3)"),
        "{stdout}"
    );
    assert!(dir.path().join("ran").exists());
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

/// `clm config list` in a fresh CLM repository holding `clm.toml` and `clm.local.toml`.
fn config_list_in_repo(clm_toml: &str, local: &str) -> (Output, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = dir.path().canonicalize().unwrap();
    write(&root, "specs/SPEC.md", "");
    fs::create_dir(root.join(".git")).unwrap();
    write(&root, "clm.toml", clm_toml);
    write(&root, "clm.local.toml", local);
    let out = clm_command()
        .args(["config", "list"])
        .current_dir(&root)
        .output()
        .unwrap();
    (out, root)
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
fn config_get_verify_prints_the_table_and_the_file() {
    // req:config/verify-setting@5bf87fcc
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
