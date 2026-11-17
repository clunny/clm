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

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn this_repository_is_clean() {
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

#[test]
fn unreferenced_is_clean_with_the_allow_list() {
    // req:clm/check-across-components@45a113b3
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
fn hash_is_no_command() {
    // req:clm/old-command-names-retired@24a5a6f9
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
fn coverage_command_prints_report() {
    // req:check.coverage/coverage-report@533d2f1a
    let dir = change_repo("---\ncomponent: clm\n---\n");
    write(
        dir.path(),
        "t.rs",
        concat!("// req", ":clm/r\n#[test]\nfn t() {}\n"),
    );
    let (code, out) = clm(dir.path(), &["coverage"]);
    assert_eq!(code, Some(0));
    assert_eq!(out, "clm/r behind t.rs:1\n");
    let (graph, _) = clm_model::load(dir.path());
    let v = graph.requirement("clm/r").unwrap().version().to_string();
    write(
        dir.path(),
        "t.rs",
        &in_test(&format!("// {}clm/r@{v}\n", concat!("req", ":"))),
    );
    let (_, out) = clm(dir.path(), &["coverage"]);
    assert_eq!(out, "clm/r current t.rs:1\n");
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
fn check_fails_on_a_retired_term_in_a_spec() {
    // req:clm/check-across-components@45a113b3
    // req:config/retired-terms-detector@1e881e15
    let script = fs::read_to_string(repo_root().join("scripts/retired-terms.sh")).unwrap();
    let dir = detector_repo(None);
    write(dir.path(), "scripts/retired-terms.sh", &script);
    write(dir.path(), "scripts/retired-terms.txt", "grommet stub\n");
    write(
        dir.path(),
        "clm.toml",
        "[[detectors]]\nname = \"retired-terms\"\ncommand = [\"sh\", \"scripts/retired-terms.sh\"]\nhint = \"scripts/retired-terms.sh needs only sh and grep\"\n",
    );
    write(dir.path(), "specs/notes.md", "make it a grommet stub\n");
    let out = clm_check(dir.path());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(1), "{stdout}{stderr}");
    assert!(
        stderr.contains("specs/notes.md:1: grommet stub"),
        "{stderr}"
    );
    assert!(
        stdout.contains("clm.toml: check/detectors: retired-terms failed (exit 1)"),
        "{stdout}"
    );
}

/// `s` as a Rust file whose markers count as a test's: those on its third line on are inside
/// a test, the earlier ones directly above it. Every line of `s` keeps its line number.
fn in_test(s: &str) -> String {
    let (head, body) = match s.strip_prefix('\n') {
        Some(rest) => ("", rest),
        None => s.split_once("\n\n").map_or((s, ""), |(a, b)| (a, b)),
    };
    let nl = if body.ends_with('\n') || body.is_empty() {
        ""
    } else {
        "\n"
    };
    if s.starts_with('\n') {
        format!("#[test]\nfn t() {{{body}{nl}}}\n")
    } else if body.is_empty() {
        format!("{s}#[test]\nfn t() {{}}\n")
    } else {
        format!("{head}\n#[test]\nfn t() {{{body}{nl}}}\n")
    }
}
