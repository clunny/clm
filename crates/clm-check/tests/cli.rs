//! `clm check` and `clm backlog` through `clm-check`'s calls (`commands-in-crate`), with the
//! settings `crates/clm` would pass them given as arguments.
// clm:reads specs/**
// clm:reads changes/**
// clm:reads backlog/**
// clm:reads ARCHITECTURE.md
// clm:reads CAPABILITIES.md

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clm_check::{CheckInputs, Detector, Violation, backlog_command, check_command};
use tempfile::TempDir;

/// The exit status a call returned, read as a process's.
#[derive(Debug)]
struct Status(u8);

impl Status {
    fn code(&self) -> Option<i32> {
        Some(self.0.into())
    }
}

/// What a call wrote to its two streams, and what it returned.
#[derive(Debug)]
struct Output {
    status: Status,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn no_env(_: &str) -> Option<String> {
    None
}

/// `[[detectors]]` of `root`'s `clm.toml`, as `crates/clm` passes them: a `timeout` it doesn't
/// set is 600 seconds.
fn repository_detectors(root: &Path) -> Result<Vec<Detector>, Violation> {
    let Ok(text) = fs::read_to_string(root.join("clm.toml")) else {
        return Ok(Vec::new());
    };
    let table: toml::Table = text.parse().unwrap();
    let Some(list) = table.get("detectors").and_then(|d| d.as_array()) else {
        return Ok(Vec::new());
    };
    Ok(list
        .iter()
        .map(|d| Detector {
            name: d["name"].as_str().unwrap().into(),
            command: d["command"]
                .as_array()
                .unwrap()
                .iter()
                .map(|w| w.as_str().unwrap().into())
                .collect(),
            hint: d.get("hint").and_then(|h| h.as_str()).map(Into::into),
            timeout: d
                .get("timeout")
                .and_then(|t| t.as_integer())
                .map_or(600, |t| t as u64),
        })
        .collect())
}

/// `clm check <args>` called with `root` as the root found from the current directory and
/// `detectors` as the settings' detectors.
fn call_check(
    root: &Path,
    args: &[OsString],
    detectors: &dyn Fn(&Path) -> Result<Vec<Detector>, Violation>,
) -> Output {
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let inputs = CheckInputs {
        detectors,
        more: &|_| Vec::new(),
    };
    let code = check_command(args, root, &no_env, &mut stdout, &mut stderr, &inputs);
    Output {
        status: Status(code),
        stdout,
        stderr,
    }
}

fn args(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
}

/// `clm check --root <root>`, with the detectors `root`'s `clm.toml` names.
fn clm_check(root: &Path) -> Output {
    let mut words = args(&["--root"]);
    words.push(root.into());
    call_check(Path::new("."), &words, &repository_detectors)
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn a_command_called_in_a_test_writes_its_streams_and_returns_its_status() {
    // req:check/commands-in-crate@ed866c99
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "specs/SPEC.md",
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    );
    write(dir.path(), "specs/a/SPEC.md", "no frontmatter\n");
    write(
        dir.path(),
        "clm.toml",
        "[[detectors]]\nname = \"loud\"\ncommand = [\"sh\", \"-c\", \"echo said-out; echo said-err >&2; exit 3\"]\n",
    );
    // The root found from the current directory, with no `--root`.
    let out = call_check(dir.path(), &[], &repository_detectors);
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(1), "{stdout}{stderr}");
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    assert!(lines[0].starts_with("specs/a/SPEC.md: "), "{stdout}");
    assert_eq!(lines[1], "clm.toml: check/detectors: loud failed (exit 3)");
    // The detector's own output goes to the error stream, and nothing else does.
    let mut said: Vec<_> = stderr.lines().collect();
    said.sort();
    assert_eq!(said, ["said-err", "said-out"], "{stderr}");

    // `--root` overrides the root found, and a clean repository returns 0 writing nothing.
    let clean = TempDir::new().unwrap();
    write(
        clean.path(),
        "specs/SPEC.md",
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    );
    let mut words = args(&["--no-detectors", "--root"]);
    words.push(clean.path().into());
    let out = call_check(dir.path(), &words, &repository_detectors);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{out:?}");
}

#[test]
fn arguments_it_cant_parse_return_2_with_the_usage_error() {
    // req:check/commands-in-crate@ed866c99
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "specs/SPEC.md",
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    );
    let out = call_check(dir.path(), &args(&["--frobnicate"]), &repository_detectors);
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("--frobnicate"), "{stderr}");
    assert!(stderr.contains("Usage: clm check"), "{stderr}");
    assert!(out.stdout.is_empty());
    // The same for `clm backlog`, and the test's process goes on.
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let code = backlog_command(["extra"], dir.path(), &no_env, &mut stdout, &mut stderr);
    assert_eq!(code, 2);
    assert!(
        String::from_utf8(stderr)
            .unwrap()
            .contains("Usage: clm backlog")
    );
    assert!(stdout.is_empty());
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

/// The root of this repository.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn this_repository_passes_its_rules() {
    // req:check/report-and-exit@2528ee97
    let mut words = args(&["--no-detectors", "--root"]);
    words.push(repo_root().into());
    let out = call_check(Path::new("."), &words, &repository_detectors);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
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
    let mut words = args(&["--root"]);
    words.push(dir.path().into());
    let out = call_check(elsewhere.path(), &words, &repository_detectors);
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
    let mut words = args(&["--no-detectors", "--root"]);
    words.push(dir.path().into());
    let out = call_check(Path::new("."), &words, &repository_detectors);
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
    let user_file = config.path().join("clm/config.toml");
    // The settings `crates/clm` loads with this user file can't be loaded: it passes why.
    let unloadable = |_: &Path| {
        Err(Violation {
            path: user_file.clone(),
            rule: "check/detectors",
            message: "TOML parse error at line 1, column 3: unknown field `detectors`, expected one of `autosync`, `defects`, `max_builds`, `writer`".into(),
        })
    };
    let mut words = args(&["--root"]);
    words.push(dir.path().into());
    let out = call_check(Path::new("."), &words, &unloadable);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(1), "{stdout}");
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
    // req:check/undeclared-imports@5d13552f
    let dir = import_repo("[]", "[\"crates/b/**\"]");
    let line = "crates/a/Cargo.toml: check/undeclared-imports: imports crates/b/Cargo.toml of component `b`, and `a` doesn't depend on `b`\n";
    let out = clm_check(dir.path());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert_eq!(stdout, line);
    let mut words = args(&["--no-detectors", "--root"]);
    words.push(dir.path().into());
    let out = call_check(Path::new("."), &words, &repository_detectors);
    assert_eq!(String::from_utf8(out.stdout).unwrap(), line);
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn check_passes_a_declared_or_unowned_import() {
    // req:check/undeclared-imports@5d13552f
    for dir in [
        import_repo("[b]", "[\"crates/b/**\"]"),
        import_repo("[]", "[\"crates/b/src/**\"]"),
    ] {
        let out = clm_check(dir.path());
        assert_eq!(out.status.code(), Some(0), "{out:?}");
    }
}

/// `clm <command> <args> --root <root>`, for `check` and `backlog`: the status and stdout.
fn clm(root: &Path, words: &[&str]) -> (Option<i32>, String) {
    let (command, rest) = words.split_first().unwrap();
    let mut words = args(rest);
    words.push("--root".into());
    words.push(root.into());
    let out = match *command {
        "check" => call_check(Path::new("."), &words, &repository_detectors),
        "backlog" => {
            let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
            let code = backlog_command(&words, Path::new("."), &no_env, &mut stdout, &mut stderr);
            Output {
                status: Status(code),
                stdout,
                stderr,
            }
        }
        other => panic!("no call for `{other}`"),
    };
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
fn backlog_lists_each_item() {
    // req:check/backlog-report@392ec9fd
    let dir = change_repo("---\ncomponent: clm\n---\n");
    write(
        dir.path(),
        "backlog/a.md",
        "---\ntitle: Ay\nkind: question\n---\n",
    );
    let (code, out) = clm(dir.path(), &["backlog"]);
    assert_eq!((code, out.as_str()), (Some(0), "a question P5 [] Ay\n"));
}

const KNIP_DETECTOR: &str =
    "[[detectors]]\nname = \"knip\"\ncommand = [\"node_modules/.bin/knip\"]\nhint = \"npm ci\"\n";

/// `clm check --root <root>` with `repo/sub` as the root found from the current directory, so
/// a relative first word can't resolve there.
fn check_from_subdirectory(repo: &Path) -> Output {
    let sub = repo.join("sub");
    fs::create_dir_all(&sub).unwrap();
    let mut words = args(&["--root"]);
    words.push(repo.into());
    call_check(&sub, &words, &repository_detectors)
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
