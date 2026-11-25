//! Proves `.github/scripts/ci-check.sh`, the check command `clm land` runs here, and
//! `detector-tools.sh`: the script runs in a temporary directory with fake `cargo`, `npm`,
//! `npx`, `clm` and `claude` first on `PATH` (or, for one test, with no `claude` on `PATH` at
//! all), and the scripts' text is read for what a run can't show.

// clm:reads clm.toml

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn script_path(name: &str) -> PathBuf {
    repo().join(".github/scripts").join(name)
}

fn script(name: &str) -> String {
    fs::read_to_string(script_path(name)).unwrap()
}

/// The script's lines, trimmed, without comments or blank lines.
fn script_lines(name: &str) -> Vec<String> {
    script(name)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

/// Appends its name and arguments to `$CI_LOG`, and fails when they are `$CI_FAIL`.
const FAKE: &str = "#!/usr/bin/env bash\necho \"$(basename \"$0\") $*\" >> \"$CI_LOG\"\n[ \"$(basename \"$0\") $*\" != \"${CI_FAIL:-}\" ]\n";
/// As `FAKE`, led by the directory it runs in (`web$ npm test`, `capabilities/web$ npm test`),
/// for the package tools.
const FAKE_IN_DIR: &str = r#"#!/usr/bin/env bash
dir=$(basename "$PWD")
[ "$(basename "$(dirname "$PWD")")" != capabilities ] || dir=capabilities/$dir
[ "$(basename "$(dirname "$PWD")")" != web ] || dir=web/$dir
run="$dir\$ $(basename "$0") $*"
echo "$run" >> "$CI_LOG"
[ "$run" != "${CI_FAIL:-}" ]
"#;

const NEXTEST_ALL: &str = "cargo nextest run --workspace --no-tests=pass";
const DOC_TESTS: &str = "cargo test --doc --workspace";
const WASM: &str = "cargo build --target wasm32-unknown-unknown";
/// The crates built for wasm only with their default features disabled (ci/web-checks).
const NO_DEFAULTS: [&str; 4] = ["clm-check", "clm-orchestrator", "clm-view", "clm-ledger"];
/// Every wasm build the script runs (ci/web-checks), as the `cargo` fake records it.
fn wasm_builds() -> Vec<String> {
    std::iter::once(format!("{WASM} -p clm-model"))
        .chain(NO_DEFAULTS.map(|k| format!("{WASM} --no-default-features -p {k}")))
        .chain([format!("{WASM} -p clm-web")])
        .collect()
}
const BROWSER_INSTALL_CI: &str = "web$ npx playwright install --with-deps chromium";
const BROWSER_INSTALL: &str = "web$ npx playwright install chromium";
const WEB_TEST: &str = "web$ npm test";
const CAPABILITY_PACKAGES: [&str; 2] = ["capabilities/web", "capabilities/panel"];
/// `web.site`'s package, which runs after `web/`'s suite (ci/web-checks).
const SITE_PACKAGE: &str = "web/site";
const SITE_SUITE: [&str; 2] = ["web/site$ npm ci --ignore-scripts", "web/site$ npm test"];
const CAP_WEB_SUITE: [&str; 2] = [
    "capabilities/web$ npm ci --ignore-scripts",
    "capabilities/web$ npm test -- --retries=1",
];
const CAP_PANEL_SUITE: [&str; 2] = [
    "capabilities/panel$ npm ci --ignore-scripts",
    "capabilities/panel$ npm test",
];

use crate::exec::write_exec as executable;

/// A work tree holding the real scripts, `web/` and `plugin-panel/`, with the fakes in `bin/`
/// beside it.
pub(crate) struct Tree {
    pub(crate) dir: TempDir,
    /// The script a run runs, under `.github/scripts/`: `ci-check.sh` unless a test says.
    pub(crate) script: &'static str,
}

impl Tree {
    pub(crate) fn new() -> Self {
        let t = Tree {
            dir: TempDir::new().unwrap(),
            script: "ci-check.sh",
        };
        let bin = t.dir.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        // `clm` too, so a `clm check` the script ran shows in the log, and `claude`, so the
        // real one on this machine never runs.
        for tool in ["cargo", "clm", "claude"] {
            executable(&bin.join(tool), FAKE);
        }
        for tool in ["npm", "npx"] {
            executable(&bin.join(tool), FAKE_IN_DIR);
        }
        let work = t.work();
        for dir in ["web", "plugin-panel", ".github/scripts"] {
            fs::create_dir_all(work.join(dir)).unwrap();
        }
        // Both capability packages, as once `capability` has built them.
        for dir in CAPABILITY_PACKAGES.into_iter().chain([SITE_PACKAGE]) {
            fs::create_dir_all(work.join(dir)).unwrap();
            fs::write(work.join(dir).join("package.json"), "{}").unwrap();
        }
        for name in ["ci-check.sh", "detector-tools.sh", "test-tools.sh"] {
            executable(&work.join(".github/scripts").join(name), &script(name));
        }
        // `clm` has a binary, `clm-check` and `clm-land` a library, and `git` both and
        // `src/bin/`, with a package name that isn't its directory's, so a selection must
        // read the manifest.
        for (dir, manifest, sources) in [
            ("clm", "[package]\nname = \"clm\"\n", &["src/main.rs"][..]),
            (
                "clm-check",
                "[package]\nname = \"clm-check\"\n",
                &["src/lib.rs"],
            ),
            (
                "clm-run",
                "[package]\nname = \"clm-run\"\n",
                &["src/lib.rs", "superpowers.pin"],
            ),
            (
                "clm-land",
                "[package]\nname = \"clm-land\"\n",
                &["src/lib.rs", "tests/common/mod.rs"],
            ),
            (
                "git",
                "[lib]\nname = \"not_the_package\"\n\n[package]\nversion = \"1.0.0\"\nname = \"clm-git\"\n",
                &["src/lib.rs", "src/main.rs", "src/bin/extra.rs"],
            ),
        ] {
            let crate_dir = work.join("crates").join(dir);
            fs::create_dir_all(&crate_dir).unwrap();
            fs::write(crate_dir.join("Cargo.toml"), manifest).unwrap();
            for source in sources {
                let path = crate_dir.join(source);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, "").unwrap();
            }
        }
        t
    }

    fn work(&self) -> PathBuf {
        self.dir.path().join("work")
    }

    /// The tree without the capability packages' `package.json`, as before `capability` builds
    /// them: their directories stay.
    fn without_packages(self) -> Self {
        for dir in CAPABILITY_PACKAGES.into_iter().chain([SITE_PACKAGE]) {
            fs::remove_file(self.work().join(dir).join("package.json")).unwrap();
        }
        self
    }

    /// Runs `ci-check.sh` with `CI` set to `ci` (unset when `None`, whatever the environment
    /// says), and the fake whose name and arguments are `fail` failing. Returns whether it
    /// succeeded and the commands it ran, in order.
    fn run(&self, ci: Option<&str>, fail: &str) -> (bool, Vec<String>) {
        self.run_env(ci, fail, None)
    }

    /// Runs it outside CI with `CLM_SELECTION` naming a file that holds `selection`.
    fn run_selecting(&self, selection: &str) -> (bool, Vec<String>) {
        let file = self.dir.path().join("selection");
        fs::write(&file, selection).unwrap();
        let (ok, log, _) = self.exec(None, "", Some(file.as_os_str()), &self.path());
        (ok, log)
    }

    /// Replaces the fake `tool` with a script holding `text`.
    fn fake(&self, tool: &str, text: &str) {
        executable(&self.dir.path().join("bin").join(tool), text);
    }

    /// The fakes first, then this process's `PATH`.
    pub(crate) fn path(&self) -> std::ffi::OsString {
        let bin = self.dir.path().join("bin");
        let system = std::env::var_os("PATH").unwrap_or_default();
        std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(&system))).unwrap()
    }

    /// As `run`, with `CLM_SELECTION` set to `selection`, or unset when `None` (whatever the
    /// environment says: `clm land` sets it for the tests that run this).
    fn run_env(
        &self,
        ci: Option<&str>,
        fail: &str,
        selection: Option<&std::ffi::OsStr>,
    ) -> (bool, Vec<String>) {
        let (ok, log, _) = self.exec(ci, fail, selection, &self.path());
        (ok, log)
    }

    /// Runs it outside CI with no `claude` on `PATH`, neither the fake nor this machine's, and
    /// `CLM_SELECTION` naming a file that holds `selection`, or every test selected when `None`.
    /// A `PATH` directory holding a `claude` is replaced by a copy of it made of links to
    /// everything else it holds, since it may also hold `bash`. Returns its stderr too.
    fn run_without_claude(&self, selection: Option<&str>) -> (bool, Vec<String>, String) {
        let bin = self.dir.path().join("bin");
        fs::remove_file(bin.join("claude")).unwrap();
        let mut dirs = vec![bin];
        let system = std::env::var_os("PATH").unwrap_or_default();
        for (i, dir) in std::env::split_paths(&system).enumerate() {
            if !dir.join("claude").is_file() {
                dirs.push(dir);
                continue;
            }
            let mirror = self.dir.path().join(format!("path-{i}"));
            fs::create_dir_all(&mirror).unwrap();
            for entry in fs::read_dir(&dir).unwrap().flatten() {
                if entry.file_name() != "claude" {
                    let _ =
                        std::os::unix::fs::symlink(entry.path(), mirror.join(entry.file_name()));
                }
            }
            dirs.push(mirror);
        }
        let path = std::env::join_paths(dirs).unwrap();
        let file = self.dir.path().join("selection");
        if let Some(selection) = selection {
            fs::write(&file, selection).unwrap();
        }
        self.exec(None, "", selection.map(|_| file.as_os_str()), &path)
    }

    /// Runs the script with `path` as `PATH`, returning its success, the commands it ran and
    /// its stderr.
    fn exec(
        &self,
        ci: Option<&str>,
        fail: &str,
        selection: Option<&std::ffi::OsStr>,
        path: &std::ffi::OsStr,
    ) -> (bool, Vec<String>, String) {
        let out = self.output(ci, fail, selection, path);
        let log = fs::read_to_string(self.dir.path().join("log")).unwrap_or_default();
        (
            out.status.success(),
            log.lines().map(str::to_owned).collect(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    /// Runs the script with `path` as `PATH` and returns its whole output, the commands it ran
    /// being in `log` beside the tree.
    fn output(
        &self,
        ci: Option<&str>,
        fail: &str,
        selection: Option<&std::ffi::OsStr>,
        path: &std::ffi::OsStr,
    ) -> std::process::Output {
        self.output_env(ci, fail, selection, path, &[])
    }

    /// As `output`, with each of `env` also set.
    pub(crate) fn output_env(
        &self,
        ci: Option<&str>,
        fail: &str,
        selection: Option<&std::ffi::OsStr>,
        path: &std::ffi::OsStr,
        env: &[(&str, &str)],
    ) -> std::process::Output {
        let log = self.dir.path().join("log");
        let _ = fs::remove_file(&log);
        let mut c = Command::new(Path::new(".github/scripts").join(self.script));
        c.current_dir(self.work())
            .env("PATH", path)
            .env("CI_LOG", &log)
            .env("CI_FAIL", fail)
            .env_remove("CI")
            .env_remove("CLM_SELECTION")
            // nextest sets these for the test running this; the script must not inherit them.
            .env_remove("NEXTEST_PROFILE")
            .env_remove("NEXTEST_CONFIG");
        if let Some(ci) = ci {
            c.env("CI", ci);
        }
        if let Some(selection) = selection {
            c.env("CLM_SELECTION", selection);
        }
        c.envs(env.iter().copied());
        c.output().unwrap()
    }
}

#[test]
fn every_check() {
    // req:ci/web-checks@53cd2ed8
    // req:ci/detector-tools@1984d752
    let (ok, log) = Tree::new().run(None, "");
    assert!(ok, "{log:?}");
    // The real `detector-tools.sh` runs first: its install is the first `cargo` call.
    assert!(
        log[0].starts_with("cargo install ") && log[0].ends_with(" cargo-machete"),
        "{log:?}"
    );
    assert!(
        log[1].starts_with("cargo install ") && log[1].ends_with(" cargo-nextest"),
        "{log:?}"
    );
    // `clm-ledger` without its `git` feature (ledger/ledger-builds-for-wasm).
    for cmd in [
        "cargo fmt --all --check".to_owned(),
        "cargo clippy --all-targets -- -D warnings".to_owned(),
        NEXTEST_ALL.to_owned(),
        DOC_TESTS.to_owned(),
        "plugin-panel$ npm ci --ignore-scripts".to_owned(),
        "plugin-panel$ npm test".to_owned(),
        "web$ npm ci --ignore-scripts".to_owned(),
        WEB_TEST.to_owned(),
    ]
    .into_iter()
    .chain(wasm_builds())
    {
        assert!(log.contains(&cmd), "no `{cmd}` in {log:?}");
    }
    // Each capability package after the suite it follows, the panel's after its validation.
    for cmd in CAP_WEB_SUITE.iter().chain(&CAP_PANEL_SUITE) {
        assert!(log.contains(&cmd.to_string()), "no `{cmd}` in {log:?}");
    }
    // `web.site`'s package after `web/`'s suite and before the capability package.
    for cmd in &SITE_SUITE {
        assert!(log.contains(&cmd.to_string()), "no `{cmd}` in {log:?}");
    }
    assert!(runs_before(&log, WEB_TEST, SITE_SUITE[0]), "{log:?}");
    assert!(runs_before(&log, SITE_SUITE[0], SITE_SUITE[1]), "{log:?}");
    assert!(
        runs_before(&log, SITE_SUITE[1], CAP_WEB_SUITE[0]),
        "{log:?}"
    );
    assert!(runs_before(&log, WEB_TEST, CAP_WEB_SUITE[0]), "{log:?}");
    assert!(
        runs_before(&log, CAP_WEB_SUITE[0], CAP_WEB_SUITE[1]),
        "{log:?}"
    );
    assert!(
        runs_before(&log, "plugin-panel$ npm test", CAP_PANEL_SUITE[0]),
        "{log:?}"
    );
    assert!(
        runs_before(&log, PANEL_VALIDATE, CAP_PANEL_SUITE[0]),
        "{log:?}"
    );
    assert!(
        runs_before(&log, CAP_PANEL_SUITE[0], CAP_PANEL_SUITE[1]),
        "{log:?}"
    );
    // `clm land` runs `clm check` itself, after the command.
    assert!(!log.iter().any(|l| l.contains(" check")), "{log:?}");
    // req:ci/no-model-steps@2b603986
    // Nor runs `clm` or a model session: `claude` only validates.
    assert!(!ran(&log, "clm "), "{log:?}");
    let claude = log.iter().filter(|l| l.starts_with("claude "));
    assert!(
        claude.clone().count() > 0
            && claude
                .clone()
                .all(|l| l.starts_with("claude plugin validate ")),
        "{log:?}"
    );
}

#[test]
fn broken_wasm_build_fails() {
    // req:ci/web-checks@53cd2ed8
    // Each of the six builds failing in turn: the run fails and nothing runs after it.
    let builds = wasm_builds();
    assert_eq!(builds.len(), 6);
    for build in builds {
        let (ok, log) = Tree::new().run(None, &build);
        assert!(!ok, "{build}: {log:?}");
        assert_eq!(log.last(), Some(&build), "{log:?}");
    }
}

#[test]
fn failing_panel_test_fails() {
    // req:ci/web-checks@53cd2ed8
    for step in [
        "plugin-panel$ npm ci --ignore-scripts",
        "plugin-panel$ npm test",
        "web$ npm ci --ignore-scripts",
        WEB_TEST,
    ] {
        let (ok, log) = Tree::new().run(None, step);
        assert!(!ok, "{step}: {log:?}");
        assert_eq!(log.last().map(String::as_str), Some(step), "{log:?}");
    }
}

#[test]
fn a_web_change_leaves_the_site_out_and_a_site_change_runs_it() {
    // req:ci/web-checks@53cd2ed8
    let (ok, log) = Tree::new().run_selecting("file web/src/app.ts\n");
    assert!(ok, "{log:?}");
    let web: Vec<_> = WEB_SUITE.iter().chain(&CAP_WEB_SUITE).copied().collect();
    assert_eq!(npm_run(&log), web);
    assert!(!log.iter().any(|l| l.starts_with("web/site")), "{log:?}");
    let (ok, log) = Tree::new().run_selecting("file web/site/guide/index.md\n");
    assert!(ok, "{log:?}");
    let site: Vec<_> = WEB_SUITE
        .iter()
        .chain(&SITE_SUITE)
        .chain(&CAP_WEB_SUITE)
        .copied()
        .collect();
    assert_eq!(npm_run(&log), site);
}

#[test]
fn packages_not_built_yet_run_nothing() {
    // req:ci/web-checks@53cd2ed8
    let (ok, log) = Tree::new().without_packages().run(None, "");
    assert!(ok, "{log:?}");
    let both: Vec<_> = PANEL_SUITE.iter().chain(&WEB_SUITE).copied().collect();
    assert_eq!(npm_run(&log), both);
    assert!(
        !log.iter()
            .any(|l| l.starts_with("capabilities/") || l.starts_with("web/site")),
        "{log:?}"
    );
}

#[test]
fn the_web_package_retries_and_no_other_suite_does() {
    // req:ci/capability-web-retries@efb55ac0
    let (ok, log) = Tree::new().run(None, "");
    assert!(ok, "{log:?}");
    let tests: Vec<_> = npm_run(&log)
        .into_iter()
        .filter(|l| l.contains(" npm test") && !l.starts_with("web/site"))
        .collect();
    assert_eq!(
        tests,
        [
            "plugin-panel$ npm test",
            "capabilities/panel$ npm test",
            "web$ npm test",
            "capabilities/web$ npm test -- --retries=1",
        ],
        "{log:?}"
    );
}

#[test]
fn failing_on_both_attempts_fails() {
    // req:ci/capability-web-retries@efb55ac0
    let (ok, log) = Tree::new().run(None, "capabilities/web$ npm test -- --retries=1");
    assert!(!ok, "{log:?}");
}

#[test]
fn failing_capability_package_fails() {
    // req:ci/web-checks@53cd2ed8
    for step in CAP_PANEL_SUITE
        .iter()
        .chain(&CAP_WEB_SUITE)
        .chain(&SITE_SUITE)
    {
        let (ok, log) = Tree::new().run(None, step);
        assert!(!ok, "{step}: {log:?}");
        assert_eq!(log.last().map(String::as_str), Some(*step), "{log:?}");
    }
}

/// Whether `first` and then `then` are both in `log`, in that order.
fn runs_before(log: &[String], first: &str, then: &str) -> bool {
    let a = log.iter().position(|l| l == first);
    let b = log.iter().position(|l| l == then);
    matches!((a, b), (Some(a), Some(b)) if a < b)
}

#[test]
fn in_ci_installs_the_browser_with_deps() {
    // req:ci/browser-install@7732c646
    let (ok, log) = Tree::new().run(Some("true"), "");
    assert!(ok, "{log:?}");
    assert!(runs_before(&log, BROWSER_INSTALL_CI, WEB_TEST), "{log:?}");
    assert!(!log.contains(&BROWSER_INSTALL.to_owned()), "{log:?}");
}

#[test]
fn outside_ci_installs_the_browser_without_deps() {
    // req:ci/browser-install@7732c646
    // Unset, and any value but `true`.
    for ci in [None, Some("false"), Some("1"), Some("")] {
        let (ok, log) = Tree::new().run(ci, "");
        assert!(ok, "CI={ci:?}: {log:?}");
        assert!(
            runs_before(&log, BROWSER_INSTALL, WEB_TEST),
            "CI={ci:?}: {log:?}"
        );
        assert!(
            !log.iter().any(|l| l.contains("--with-deps")),
            "CI={ci:?}: {log:?}"
        );
    }
}

#[test]
fn failing_browser_install_fails() {
    // req:ci/browser-install@7732c646
    let (ok, log) = Tree::new().run(None, BROWSER_INSTALL);
    assert!(!ok, "{log:?}");
    assert_eq!(log.last().map(String::as_str), Some(BROWSER_INSTALL));
    assert!(!log.contains(&WEB_TEST.to_owned()), "{log:?}");
}

#[test]
fn capability_package_installs_the_browser() {
    // req:ci/browser-install@7732c646
    const CAP_INSTALL: &str = "capabilities/web$ npx playwright install chromium";
    let (ok, log) = Tree::new().run(None, "");
    assert!(ok, "{log:?}");
    assert!(runs_before(&log, CAP_WEB_SUITE[0], CAP_INSTALL), "{log:?}");
    assert!(runs_before(&log, CAP_INSTALL, CAP_WEB_SUITE[1]), "{log:?}");
    let (ok, log) = Tree::new().run(Some("true"), "");
    assert!(ok, "{log:?}");
    assert!(
        runs_before(
            &log,
            "capabilities/web$ npx playwright install --with-deps chromium",
            CAP_WEB_SUITE[1]
        ),
        "{log:?}"
    );
    assert!(!log.contains(&CAP_INSTALL.to_owned()), "{log:?}");
    // A failing install fails before the package's tests.
    let (ok, log) = Tree::new().run(None, CAP_INSTALL);
    assert!(!ok, "{log:?}");
    assert_eq!(log.last().map(String::as_str), Some(CAP_INSTALL));
    // No package, no install there.
    let (ok, log) = Tree::new().without_packages().run(None, "");
    assert!(ok, "{log:?}");
    assert!(!log.contains(&CAP_INSTALL.to_owned()), "{log:?}");
}

#[test]
fn detector_tools_installs_cargo_machete_pinned() {
    // req:ci/detector-tools@1984d752
    let lines = script_lines("detector-tools.sh");
    let install = lines
        .iter()
        .position(|l| l.starts_with("cargo install ") && l.contains("cargo-machete"))
        .unwrap_or_else(|| panic!("no cargo-machete install line: {lines:?}"));
    let words: Vec<&str> = lines[install].split_whitespace().collect();
    assert!(words.contains(&"--locked"), "{}", lines[install]);
    let version = words
        .iter()
        .position(|w| *w == "--version")
        .and_then(|i| words.get(i + 1))
        .unwrap_or_else(|| panic!("no --version: {}", lines[install]));
    let parts: Vec<&str> = version.split('.').collect();
    assert!(
        parts.len() == 3
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())),
        "not an exact x.y.z version: {version}"
    );
    let mode = fs::metadata(script_path("detector-tools.sh"))
        .unwrap()
        .permissions()
        .mode();
    assert!(mode & 0o111 != 0, "detector-tools.sh isn't executable");
}

/// What breaks `ci/detector-tools` in a `clm.toml` and `detector-tools.sh`'s lines: a
/// detector whose tool has no install line its `hint` repeats, a script line that installs
/// `clm` or `sh`; a `clm` detector, and one that runs `sh` on a file of the repository (under
/// `root`), needs no line.
fn detector_tool_problems(
    config: &toml::Table,
    lines: &[String],
    root: &std::path::Path,
) -> Vec<String> {
    let mut problems = Vec::new();
    for d in config["detectors"].as_array().unwrap() {
        let tool = d["command"][0].as_str().unwrap();
        let hint = d["hint"].as_str().unwrap();
        let command: Vec<&str> = d["command"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|w| w.as_str())
            .collect();
        if needs_no_install(&command, root) {
            // A `clm` detector runs the clone's own `clm`, or the binary under test, and
            // `sh <file in the repository>` needs only `sh` and `grep`: nothing installs either.
        } else if hint.split_whitespace().last() != Some(tool) {
            problems.push(format!("hint `{hint}` doesn't install `{tool}`"));
        } else if !lines.iter().any(|l| l == hint) {
            problems.push(format!("detector-tools.sh has no line `{hint}`"));
        }
    }
    for line in lines.iter().filter(|l| installs_clm(l)) {
        problems.push(format!("detector-tools.sh installs clm: `{line}`"));
    }
    for line in lines.iter().filter(|l| installs_sh(l)) {
        problems.push(format!("detector-tools.sh installs sh: `{line}`"));
    }
    problems
}

/// Whether a detector's `command` needs no tool installed (ci/detector-tools): its first word is
/// `clm`, or it is `sh` and the path of a file of the repository under `root`.
fn needs_no_install(command: &[&str], root: &std::path::Path) -> bool {
    command.first() == Some(&"clm")
        || (command.first() == Some(&"sh")
            && command.get(1).is_some_and(|p| {
                !p.starts_with('/') && !p.contains("..") && root.join(p).is_file()
            }))
}

/// Whether a line installs `sh`: an install command naming it.
fn installs_sh(line: &str) -> bool {
    let words: Vec<&str> = line.split_whitespace().collect();
    words.iter().any(|w| *w == "install" || *w == "binstall") && words.contains(&"sh")
}

/// Whether a line installs `clm`: an install command (`cargo install`, `cargo binstall`)
/// naming the crate `clm` or a path to it, or a run of `scripts/dev-install.sh`.
fn installs_clm(line: &str) -> bool {
    let words: Vec<&str> = line.split_whitespace().collect();
    let install = words.iter().any(|w| *w == "install" || *w == "binstall");
    let names_clm = words
        .iter()
        .any(|w| *w == "clm" || w.ends_with("/clm") || w.ends_with("/clm/"));
    (install && names_clm) || line.contains("dev-install.sh")
}

#[test]
fn every_detector_has_its_tool_installed() {
    // req:ci/detector-tools@1984d752
    // The detectors that need a tool installed: neither a `clm` one nor `sh` on a file of the
    // repository. Each one's hint ends in its tool and starts with a command a fake stands in
    // for.
    let config: toml::Table = fs::read_to_string(repo().join("clm.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let detectors = config["detectors"].as_array().unwrap();
    assert!(!detectors.is_empty());
    let mut tree = Tree::new();
    tree.script = "detector-tools.sh";
    let mut hints = Vec::new();
    for d in detectors {
        let command: Vec<&str> = d["command"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| w.as_str().unwrap())
            .collect();
        let exempt = needs_no_install(&command, &repo());
        if exempt {
            continue;
        }
        let hint = d["hint"].as_str().unwrap();
        assert_eq!(
            hint.split_whitespace().last(),
            Some(command[0]),
            "the hint `{hint}` doesn't end in the detector's tool"
        );
        tree.fake(hint.split_whitespace().next().unwrap(), FAKE);
        hints.push(hint.to_owned());
    }
    assert!(!hints.is_empty());
    let (ok, log) = tree.run(None, "");
    assert!(ok, "{log:?}");
    for hint in &hints {
        assert!(
            log.contains(hint),
            "detector-tools.sh didn't run `{hint}`: {log:?}"
        );
    }
    for call in &log {
        assert!(!installs_clm(call), "installs clm: {call}");
        assert!(!installs_sh(call), "installs sh: {call}");
    }
}

#[test]
fn clm_detector_needs_no_install_line() {
    // req:ci/detector-tools@1984d752
    let machete = "cargo install --locked --version 0.9.2 cargo-machete";
    let config = |clm_hint: &str| -> toml::Table {
        format!(
            "[[detectors]]\nname = \"unused-dependencies\"\ncommand = [\"cargo-machete\"]\n\
             hint = \"{machete}\"\n\n[[detectors]]\nname = \"unreferenced-public-items\"\n\
             command = [\"clm\", \"unreferenced\"]\nhint = \"{clm_hint}\"\n"
        )
        .parse()
        .unwrap()
    };
    let lines = |extra: &[&str]| -> Vec<String> {
        std::iter::once(machete)
            .chain(extra.iter().copied())
            .map(str::to_owned)
            .collect()
    };
    let ok = config("scripts/dev-install.sh");
    assert_eq!(
        detector_tool_problems(&ok, &lines(&[]), &repo()),
        Vec::<String>::new()
    );
    // The script installing `clm` itself, however it is written.
    for install in [
        "cargo install --locked clm",
        "cargo install clm --locked",
        "cargo install --locked --path crates/clm",
        "cargo binstall clm",
        "scripts/dev-install.sh",
    ] {
        let problems = detector_tool_problems(&ok, &lines(&[install]), &repo());
        assert!(
            problems.len() == 1 && problems[0].contains("installs clm"),
            "{install}: {problems:?}"
        );
    }
    // Another detector's tool without its line.
    assert_eq!(detector_tool_problems(&ok, &[], &repo()).len(), 1);
}

#[test]
fn sh_detector_on_a_repository_file_needs_no_install_line() {
    // req:ci/detector-tools@1984d752
    let machete = "cargo install --locked --version 0.9.2 cargo-machete";
    let config = |script: &str| -> toml::Table {
        format!(
            "[[detectors]]\nname = \"unused-dependencies\"\ncommand = [\"cargo-machete\"]\n\
             hint = \"{machete}\"\n\n[[detectors]]\nname = \"retired-terms\"\n\
             command = [\"sh\", \"{script}\"]\nhint = \"none needed\"\n"
        )
        .parse()
        .unwrap()
    };
    let lines = vec![machete.to_owned()];
    // The script exists in the repository: no install line.
    let root = repo();
    assert_eq!(
        detector_tool_problems(&config(".github/scripts/ci-check.sh"), &lines, &root),
        Vec::<String>::new()
    );
    // A file that doesn't exist falls back to the install-line rule.
    assert_eq!(
        detector_tool_problems(&config("scripts/nope.sh"), &lines, &root).len(),
        1
    );
    // The script installing `sh`.
    let mut with_sh = lines.clone();
    with_sh.push("cargo install sh".to_owned());
    let problems = detector_tool_problems(&config(".github/scripts/ci-check.sh"), &with_sh, &root);
    assert!(
        problems.len() == 1 && problems[0].contains("installs sh"),
        "{problems:?}"
    );
}

#[test]
fn only_detector_tools_installs_tools() {
    // req:ci/detector-tools@1984d752
    // Other tools, such as the release's `cargo-zigbuild`, may be installed elsewhere.
    let config: toml::Table = fs::read_to_string(repo().join("clm.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let tools: Vec<_> = config["detectors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["command"][0].as_str().unwrap().to_owned())
        .collect();
    let github = repo().join(".github");
    for dir in ["workflows", "scripts"] {
        for entry in fs::read_dir(github.join(dir)).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().unwrap() == "detector-tools.sh" {
                continue;
            }
            let text = fs::read_to_string(&path).unwrap();
            for line in text.lines().filter(|l| l.contains("cargo install")) {
                assert!(
                    !tools.iter().any(|t| line.contains(t.as_str())),
                    "{} installs a detector's tool itself: {line}",
                    path.display()
                );
            }
        }
    }
}

#[test]
fn check_script_installs_the_tools() {
    // req:ci/detector-tools@1984d752
    let (ok, log) = Tree::new().run(None, "");
    assert!(ok, "{log:?}");
    let cargo = cargo_calls(&log);
    assert!(
        is_pinned_install(cargo[0], "cargo-machete"),
        "the first cargo call: {cargo:?}"
    );
    let fmt = cargo
        .iter()
        .position(|c| c.starts_with("cargo fmt"))
        .unwrap_or_else(|| panic!("no fmt: {cargo:?}"));
    assert!(0 < fmt, "{cargo:?}");
    // The only other `cargo install` is `test-tools.sh`'s.
    let installs: Vec<_> = cargo
        .iter()
        .filter(|c| c.starts_with("cargo install"))
        .collect();
    assert_eq!(installs.len(), 2, "{installs:?}");
    assert!(
        is_pinned_install(installs[1], "cargo-nextest"),
        "{installs:?}"
    );
}

/// The `cargo` calls a run made, in order.
fn cargo_calls(log: &[String]) -> Vec<&str> {
    log.iter()
        .map(String::as_str)
        .filter(|l| l.starts_with("cargo "))
        .collect()
}

/// Whether `call` is `cargo install --locked --version <x.y.z> <krate>`, an exact version.
fn is_pinned_install(call: &str, krate: &str) -> bool {
    let words: Vec<&str> = call.split_whitespace().collect();
    let ["cargo", "install", "--locked", "--version", version, name] = words[..] else {
        return false;
    };
    let parts: Vec<&str> = version.split('.').collect();
    name == krate
        && parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// The commands a run ran to run Rust tests, `cargo nextest run` or `cargo test`, in order.
fn tests_run(log: &[String]) -> Vec<&str> {
    log.iter()
        .map(String::as_str)
        .filter(|l| l.starts_with("cargo nextest run") || l.starts_with("cargo test"))
        .collect()
}

/// The `npm` commands a run ran, in order.
fn npm_run(log: &[String]) -> Vec<&str> {
    log.iter()
        .map(String::as_str)
        .filter(|l| l.contains("$ npm "))
        .collect()
}

const PANEL_SUITE: [&str; 2] = [
    "plugin-panel$ npm ci --ignore-scripts",
    "plugin-panel$ npm test",
];
const WEB_SUITE: [&str; 2] = ["web$ npm ci --ignore-scripts", "web$ npm test"];

/// fmt, clippy, the detector tools and every wasm build ran, whatever the selection.
fn assert_whole_tree_checks(log: &[String]) {
    assert!(log[0].starts_with("cargo install "), "{log:?}");
    assert!(log[1].starts_with("cargo install "), "{log:?}");
    for cmd in [
        "cargo fmt --all --check".to_owned(),
        "cargo clippy --all-targets -- -D warnings".to_owned(),
    ]
    .into_iter()
    .chain(wasm_builds())
    {
        assert!(log.contains(&cmd), "no `{cmd}` in {log:?}");
    }
}

#[test]
fn one_integration_test() {
    // req:ci/selected-tests@a8fbc68d
    let (ok, log) =
        Tree::new().run_selecting("node orchestrator.run\nfile crates/clm/tests/run.rs\n");
    assert!(ok, "{log:?}");
    assert_whole_tree_checks(&log);
    assert_eq!(
        tests_run(&log),
        ["cargo nextest run --no-tests=pass -p clm --test run"]
    );
    assert_eq!(npm_run(&log), Vec::<&str>::new());
}

#[test]
fn helper_and_source_files() {
    // req:ci/selected-tests@a8fbc68d
    let (ok, log) = Tree::new().run_selecting(
        "file crates/clm-land/tests/common/mod.rs\nfile crates/clm-check/src/lib.rs\n",
    );
    assert!(ok, "{log:?}");
    assert_eq!(
        tests_run(&log),
        [
            "cargo nextest run --no-tests=pass -p clm-check --lib",
            "cargo nextest run --no-tests=pass -p clm-land --tests"
        ]
    );
}

#[test]
fn web_file() {
    // req:ci/selected-tests@a8fbc68d
    let (ok, log) = Tree::new().run_selecting("file web/src/app.ts\n");
    assert!(ok, "{log:?}");
    assert_whole_tree_checks(&log);
    assert_eq!(tests_run(&log), Vec::<&str>::new());
    let web: Vec<_> = WEB_SUITE.iter().chain(&CAP_WEB_SUITE).copied().collect();
    assert_eq!(npm_run(&log), web);
    // req:ci/panel-validate@b6720879
    assert!(!log.iter().any(|l| l.starts_with("claude")), "{log:?}");
    let (ok, log) = Tree::new().run_selecting("file plugin-panel/src/panel.ts\n");
    assert!(ok, "{log:?}");
    let panel: Vec<_> = PANEL_SUITE
        .iter()
        .chain(&CAP_PANEL_SUITE)
        .copied()
        .collect();
    assert_eq!(npm_run(&log), panel);
    // req:ci/panel-validate@b6720879
    assert!(log.contains(&PANEL_VALIDATE.to_owned()), "{log:?}");
}

#[test]
fn capability_package_file() {
    // req:ci/selected-tests@a8fbc68d
    let (ok, log) = Tree::new().run_selecting(
        "file capabilities/panel/watch-beside-chat.test.ts\n\
         file capabilities/web/follow-from-browser.spec.ts\n\
         file capabilities/web/other.spec.ts\n",
    );
    assert!(ok, "{log:?}");
    assert_whole_tree_checks(&log);
    assert_eq!(tests_run(&log), Vec::<&str>::new());
    let expected: Vec<_> = CAP_PANEL_SUITE
        .iter()
        .copied()
        .chain([WEB_SUITE[0]])
        .chain(CAP_WEB_SUITE)
        .collect();
    assert_eq!(npm_run(&log), expected);
    // Both `web/` and its package selected: `web/`'s `npm ci` still runs once.
    let (ok, log) = Tree::new()
        .run_selecting("file web/src/app.ts\nfile capabilities/web/follow-from-browser.spec.ts\n");
    assert!(ok, "{log:?}");
    let web: Vec<_> = WEB_SUITE.iter().chain(&CAP_WEB_SUITE).copied().collect();
    assert_eq!(npm_run(&log), web);
    // `web/`'s `npm ci` failing before the package alone fails the run.
    let tree = Tree::new();
    let file = tree.dir.path().join("selection");
    fs::write(&file, "file capabilities/web/follow-from-browser.spec.ts\n").unwrap();
    let (ok, log) = tree.run_env(None, WEB_SUITE[0], Some(file.as_os_str()));
    assert!(!ok, "{log:?}");
    assert_eq!(
        log.last().map(String::as_str),
        Some(WEB_SUITE[0]),
        "{log:?}"
    );
    // A package not built yet runs nothing, not even `web/`'s `npm ci`.
    let (ok, log) = Tree::new()
        .without_packages()
        .run_selecting("file capabilities/web/follow-from-browser.spec.ts\n");
    assert!(ok, "{log:?}");
    assert_eq!(npm_run(&log), Vec::<&str>::new());
}

const PANEL_VALIDATE: &str = "claude plugin validate plugin-panel";
const NO_CLAUDE: &str = "skipped claude plugin validate: claude is not on PATH";

#[test]
fn a_refused_module_fails_the_check() {
    // req:ci/panel-validate@b6720879
    let (ok, log) = Tree::new().run(None, PANEL_VALIDATE);
    assert!(!ok, "{log:?}");
    // It runs after the panel's tests, and nothing runs after it fails.
    let panel_test = log.iter().position(|l| l == "plugin-panel$ npm test");
    let validate = log.iter().position(|l| l == PANEL_VALIDATE);
    assert!(
        matches!((panel_test, validate), (Some(t), Some(v)) if t < v),
        "{log:?}"
    );
    assert_eq!(log.last().map(String::as_str), Some(PANEL_VALIDATE));
}

#[test]
fn a_valid_module_passes() {
    // req:ci/panel-validate@b6720879
    let (ok, log) = Tree::new().run(None, "");
    assert!(ok, "{log:?}");
    // Exactly once, with neither `--strict` nor `--profile`, beside the plugin's own
    // validation (ci/plugin-validate).
    let runs: Vec<_> = log.iter().filter(|l| l.starts_with("claude ")).collect();
    assert_eq!(runs, [PANEL_VALIDATE, PLUGIN_VALIDATE], "{log:?}");
}

#[test]
fn no_claude_on_path_skips() {
    // req:ci/panel-validate@b6720879
    let (ok, log, stderr) = Tree::new().run_without_claude(None);
    assert!(ok, "{log:?} {stderr}");
    assert!(stderr.lines().any(|l| l == NO_CLAUDE), "{stderr}");
    // It carries on: `web/`'s tests still run after it.
    assert!(log.contains(&WEB_TEST.to_owned()), "{log:?}");
}

#[test]
fn no_validation_waits_on_a_prompt() {
    // req:ci/panel-validate@b6720879
    // A `claude` that reads a line from its standard input and records whether it read one,
    // and the script's own standard input a pipe holding `y`, then closed.
    let tree = Tree::new();
    tree.fake(
        "claude",
        "#!/usr/bin/env bash\nif read -r line; then read=yes; else read=no; fi\necho \"claude $* read=$read\" >> \"$CI_LOG\"\n",
    );
    let log = tree.dir.path().join("log");
    let mut child = Command::new(".github/scripts/ci-check.sh")
        .current_dir(tree.work())
        .env("PATH", tree.path())
        .env("CI_LOG", &log)
        .env("CI_FAIL", "")
        .env_remove("CI")
        .env_remove("CLM_SELECTION")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        let mut stdin = child.stdin.take().unwrap();
        // A script that ended before reading leaves the pipe closed: the line is then unread,
        // and the run's status says the rest.
        match stdin.write_all(b"y\n") {
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
            other => other.unwrap(),
        }
    }
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let log = fs::read_to_string(&log).unwrap();
    let claude: Vec<&str> = log.lines().filter(|l| l.starts_with("claude ")).collect();
    assert_eq!(
        claude,
        [
            "claude plugin validate plugin-panel read=no",
            "claude plugin validate plugin read=no",
        ],
        "{log}"
    );
}

#[test]
fn panel_not_selected_runs_no_claude() {
    // req:ci/panel-validate@b6720879
    let (ok, log) = Tree::new().run_selecting("file crates/clm/src/main.rs\n");
    assert!(ok, "{log:?}");
    assert!(!log.iter().any(|l| l.starts_with("claude")), "{log:?}");
}

const PLUGIN_VALIDATE: &str = "claude plugin validate plugin";

#[test]
fn a_refused_plugin_fails_the_check() {
    // req:ci/plugin-validate@8b626d97
    let tree = Tree::new();
    let file = tree.dir.path().join("selection");
    fs::write(&file, "file plugin/skills/propose/SKILL.md\n").unwrap();
    let (ok, log) = tree.run_env(None, PLUGIN_VALIDATE, Some(file.as_os_str()));
    assert!(!ok, "{log:?}");
    assert_eq!(log.last().map(String::as_str), Some(PLUGIN_VALIDATE));
}

#[test]
fn a_valid_plugin_passes() {
    // req:ci/plugin-validate@8b626d97
    let (ok, log) = Tree::new().run(None, "");
    assert!(ok, "{log:?}");
    for validate in [PLUGIN_VALIDATE, PANEL_VALIDATE] {
        assert!(log.contains(&validate.to_owned()), "{validate}: {log:?}");
    }
    // A selection that names a file under `plugin/` validates the plugin alone.
    let (ok, log) = Tree::new().run_selecting("file plugin/hooks/hooks.json\n");
    assert!(ok, "{log:?}");
    let runs: Vec<_> = log.iter().filter(|l| l.starts_with("claude ")).collect();
    assert_eq!(runs, [PLUGIN_VALIDATE], "{log:?}");
}

#[test]
fn no_claude_skips_the_plugin_validation() {
    // req:ci/plugin-validate@8b626d97
    let (ok, log, stderr) = Tree::new().run_without_claude(Some("file plugin/hooks/hooks.json\n"));
    assert!(ok, "{log:?} {stderr}");
    assert!(stderr.lines().any(|l| l == NO_CLAUDE), "{stderr}");
}

#[test]
fn plugin_not_selected_runs_no_plugin_validation() {
    // req:ci/plugin-validate@8b626d97
    for selection in [
        "file crates/clm/src/main.rs\n",
        "file plugin-panel/src/panel.ts\n",
    ] {
        let (ok, log) = Tree::new().run_selecting(selection);
        assert!(ok, "{log:?}");
        assert!(!log.contains(&PLUGIN_VALIDATE.to_owned()), "{log:?}");
    }
}

#[test]
fn everything() {
    // req:ci/selected-tests@a8fbc68d
    let tree = Tree::new();
    let all_line = tree.run_selecting("all Cargo.lock\nfile crates/clm/tests/run.rs\n");
    let unset = tree.run(None, "");
    let empty = tree.run_env(None, "", Some("".as_ref()));
    // A selection path where there is no regular file can't narrow anything: nothing there,
    // or a directory.
    let missing = tree.dir.path().join("missing");
    let missing = tree.run_env(None, "", Some(missing.as_os_str()));
    let directory = tree.dir.path().join("a-directory");
    fs::create_dir_all(&directory).unwrap();
    let directory = tree.run_env(None, "", Some(directory.as_os_str()));
    for (what, (ok, log)) in [
        ("all line", all_line),
        ("unset", unset),
        ("empty", empty),
        ("missing file", missing),
        ("directory", directory),
    ] {
        assert!(ok, "{what}: {log:?}");
        assert_whole_tree_checks(&log);
        assert_eq!(tests_run(&log), [NEXTEST_ALL, DOC_TESTS], "{what}");
        // req:ci/panel-validate@b6720879
        assert!(log.contains(&PANEL_VALIDATE.to_owned()), "{what}: {log:?}");
        let all: Vec<_> = PANEL_SUITE
            .iter()
            .chain(&CAP_PANEL_SUITE)
            .chain(&WEB_SUITE)
            .chain(&SITE_SUITE)
            .chain(&CAP_WEB_SUITE)
            .copied()
            .collect();
        assert_eq!(npm_run(&log), all, "{what}");
    }
}

#[test]
fn empty_selection() {
    // req:ci/selected-tests@a8fbc68d
    let (ok, log) = Tree::new().run_selecting("");
    assert!(ok, "{log:?}");
    assert_whole_tree_checks(&log);
    assert_eq!(tests_run(&log), Vec::<&str>::new());
    assert_eq!(npm_run(&log), Vec::<&str>::new());
}

#[test]
fn one_cargo_test_per_package_in_package_order() {
    // req:ci/selected-tests@a8fbc68d
    let (ok, log) = Tree::new().run_selecting(
        "file crates/git/tests/b.rs\n\
         node clm\n\
         file crates/clm/build.rs\n\
         file crates/git/src/lib.rs\n\
         file crates/git/tests/a.rs\n\
         file crates/git/tests/b.rs\n\
         file crates/clm/src/main.rs\n\
         something unexpected\n\
         file README.md\n\
         file crates/gone/src/lib.rs\n\
         file crates/gone/tests/x.rs\n\
         file crates/clm-check/tests/fixtures/notes.txt\n\
         file crates/clm-check/Cargo.toml",
    );
    assert!(ok, "{log:?}");
    assert_eq!(
        tests_run(&log),
        [
            "cargo nextest run --no-tests=pass -p clm --tests --bins",
            "cargo nextest run --no-tests=pass -p clm-check --tests",
            "cargo nextest run --no-tests=pass -p clm-git --test b --lib --bins --test a",
        ]
    );
    assert_eq!(npm_run(&log), Vec::<&str>::new());
}

#[test]
fn a_deleted_crate_selects_nothing() {
    // req:ci/selected-tests@a8fbc68d
    // `crates/gone/` holds no `Cargo.toml`, and `crates/ws/`'s names a `[workspace]` and no
    // `[package]`.
    let tree = Tree::new();
    let ws = tree.work().join("crates/ws");
    fs::create_dir_all(ws.join("src")).unwrap();
    fs::write(
        ws.join("Cargo.toml"),
        "[workspace]\nmembers = []\nname = \"ws\"\n",
    )
    .unwrap();
    fs::write(ws.join("src/lib.rs"), "").unwrap();
    let (ok, log) = tree.run_selecting(
        "file crates/gone/src/lib.rs\nfile crates/gone/tests/x.rs\nfile crates/ws/src/lib.rs\n",
    );
    assert!(ok, "{log:?}");
    assert_eq!(tests_run(&log), Vec::<&str>::new(), "{log:?}");
}

#[test]
fn only_file_lines_select() {
    // req:ci/selected-tests@a8fbc68d
    // Lines that would select something if their path were read without the `file ` word.
    let (ok, log) = Tree::new().run_selecting(
        "node crates/clm/tests/run.rs\n\
         crates/clm-check/src/lib.rs\n\
         files web/src/app.ts\n\
         \tfile plugin-panel/x.ts\n\
         file crates/clm-land/src/lib.rs\n",
    );
    assert!(ok, "{log:?}");
    assert_eq!(
        tests_run(&log),
        ["cargo nextest run --no-tests=pass -p clm-land --lib"]
    );
    assert_eq!(npm_run(&log), Vec::<&str>::new());
}

#[test]
fn web_not_selected_installs_no_browser() {
    // req:ci/selected-tests@a8fbc68d
    // req:ci/browser-install@7732c646
    let (ok, log) = Tree::new().run_selecting("file web/index.html\n");
    assert!(ok, "{log:?}");
    assert!(runs_before(&log, BROWSER_INSTALL, WEB_TEST), "{log:?}");
    let (ok, log) = Tree::new().run_selecting("file crates/clm/src/main.rs\n");
    assert!(ok, "{log:?}");
    assert!(
        !log.iter().any(|l| l.contains("playwright install")),
        "{log:?}"
    );
}

#[test]
fn a_failing_selected_test_fails() {
    // req:ci/selected-tests@a8fbc68d
    let tree = Tree::new();
    let file = tree.dir.path().join("selection");
    fs::write(
        &file,
        "file crates/clm-check/src/lib.rs\nfile crates/clm-land/src/lib.rs\n",
    )
    .unwrap();
    let fail = "cargo nextest run --no-tests=pass -p clm-check --lib";
    let (ok, log) = tree.run_env(None, fail, Some(file.as_os_str()));
    assert!(!ok, "{log:?}");
    assert_eq!(log.last().map(String::as_str), Some(fail), "{log:?}");
}

/// Whether a run ran a command starting `prefix`.
fn ran(log: &[String], prefix: &str) -> bool {
    log.iter().any(|l| l.starts_with(prefix))
}

#[test]
fn the_pin_changing_runs_its_crate_s_tests() {
    // req:ci/no-model-steps@2b603986
    let (ok, log) = Tree::new().run_selecting("file crates/clm-run/superpowers.pin\n");
    assert!(ok, "{log:?}");
    assert_eq!(
        tests_run(&log),
        ["cargo nextest run --no-tests=pass -p clm-run --tests"]
    );
    for prefix in ["cargo run", "clm ", "claude "] {
        assert!(!ran(&log, prefix), "{prefix}: {log:?}");
    }
}

#[test]
fn a_deleted_eval_file_only_validates_the_plugin() {
    // req:ci/no-model-steps@2b603986
    let (ok, log) = Tree::new().run_selecting("file plugin/evals/superpowers-unattended/run.sh\n");
    assert!(ok, "{log:?}");
    let claude: Vec<_> = log.iter().filter(|l| l.starts_with("claude ")).collect();
    assert_eq!(claude, [PLUGIN_VALIDATE], "{log:?}");
    assert_eq!(tests_run(&log), Vec::<&str>::new(), "{log:?}");
    for prefix in ["cargo run", "clm "] {
        assert!(!ran(&log, prefix), "{prefix}: {log:?}");
    }
}

/// A `cargo` that records its arguments and writes a record for `<tree>/crates/clm/src/main.rs`
/// at the `--output-path` it is given. Where `bin/fail-cargo` exists it then exits 100 for
/// `llvm-cov nextest`, as nextest does when a test fails or is ended at its bound, so only the
/// exit status, not a missing tracefile, can stop the script.
const CARGO_COVERAGE: &str = r#"#!/usr/bin/env bash
echo "cargo $*" >> "$CI_LOG"
prev=
for a; do
  if [ "$prev" = --output-path ]; then
    printf 'SF:%s\nDA:1,1\nend_of_record\n' "$PWD/crates/clm/src/main.rs" >"$a"
  fi
  prev=$a
done
if [ "$1 $2" = "llvm-cov nextest" ] && [ -e "$(dirname "$0")/fail-cargo" ]; then exit 100; fi
"#;
/// An `npx` that records its arguments, led by its directory, prints a version with more words
/// after it for `vitest --version`, and for `vitest run` writes `lcov.info` under the
/// `--coverage.reportsDirectory` it is given, with a relative record and an absolute one. It
/// fails that run where `bin/fail-<dir>` exists, and prints no version where
/// `bin/no-version-<dir>` does.
const NPX_COVERAGE: &str = r#"#!/usr/bin/env bash
here=$(basename "$PWD")
echo "$here\$ npx $*" >> "$CI_LOG"
if [ "$*" = "vitest --version" ]; then
  [ -e "$(dirname "$0")/no-version-$here" ] || echo "vitest/5.0.5 linux-x64 node-v22.0.0"
elif [ "$1 $2" = "vitest run" ]; then
  [ ! -e "$(dirname "$0")/fail-$here" ] || exit 1
  for a; do
    case $a in
      --coverage.reportsDirectory=*)
        mkdir -p "${a#*=}"
        printf 'SF:src/app.ts\nend_of_record\nSF:%s/abs.ts\nend_of_record\n' "$PWD" \
          >"${a#*=}/lcov.info"
        ;;
    esac
  done
fi
"#;

/// A tree that runs `coverage.sh`, with the fakes for it.
pub(crate) fn coverage_tree() -> Tree {
    let mut tree = Tree::new();
    tree.script = "coverage.sh";
    executable(
        &tree.work().join(".github/scripts/coverage.sh"),
        &script("coverage.sh"),
    );
    tree.fake("cargo", CARGO_COVERAGE);
    tree.fake("npx", NPX_COVERAGE);
    tree.fake("rustup", FAKE);
    tree
}

/// The `SF:` lines of the tree's `target/lcov.info`.
fn tracefile_sources(tree: &Tree) -> Vec<String> {
    fs::read_to_string(tree.work().join("target/lcov.info"))
        .unwrap()
        .lines()
        .filter_map(|l| l.strip_prefix("SF:").map(str::to_owned))
        .collect()
}

#[test]
fn coverage_writes_one_tracefile_from_three_suites() {
    // req:ci/coverage-script@ae8713bc
    let tree = coverage_tree();
    let (ok, log, stderr) = tree.exec(None, "", None, &tree.path());
    assert!(ok, "{log:?} {stderr}");
    let at = |pred: &dyn Fn(&str) -> bool| {
        log.iter()
            .position(|l| pred(l))
            .unwrap_or_else(|| panic!("{log:?}"))
    };
    // The detector tools first, then the coverage tools, then the suites.
    assert!(log[0].ends_with(" cargo-machete"), "{log:?}");
    let rustup = at(&|l| l == "rustup component add llvm-tools-preview");
    let nextest = at(&|l| {
        l.starts_with("cargo install --locked --version ") && l.ends_with(" cargo-nextest")
    });
    let install = at(&|l| {
        l.starts_with("cargo install --locked --version ") && l.ends_with(" cargo-llvm-cov")
    });
    let cargo_run = at(&|l| {
        l.starts_with("cargo llvm-cov nextest --workspace --no-rustc-wrapper --lcov --output-path ")
    });
    assert!(
        0 < rustup && rustup < cargo_run && install < cargo_run && nextest < cargo_run,
        "{log:?}"
    );
    // The Rust tests run only under nextest: no `cargo test`, no plain `cargo llvm-cov`.
    for line in log.iter().filter(|l| l.starts_with("cargo ")) {
        let words: Vec<&str> = line.split_whitespace().collect();
        assert_ne!(words[1], "test", "{line}");
        assert!(
            words[1] != "llvm-cov" || words.get(2) == Some(&"nextest"),
            "{line}"
        );
    }
    for dir in ["web", "plugin-panel"] {
        let ci = at(&|l| l == format!("{dir}$ npm ci --ignore-scripts"));
        let version = at(&|l| l == format!("{dir}$ npx vitest --version"));
        let add = at(&|l| {
            l == format!("{dir}$ npm install --no-save --ignore-scripts @vitest/coverage-v8@5.0.5")
        });
        let run = at(&|l| l.starts_with(&format!("{dir}$ npx vitest run ")));
        assert!(ci < version && version < add && add < run, "{dir}: {log:?}");
        let words: Vec<&str> = log[run].split_whitespace().collect();
        for flag in [
            "--coverage.enabled",
            "--coverage.provider=v8",
            "--coverage.reporter=lcovonly",
            "--coverage.include=src/**",
        ] {
            assert!(words.contains(&flag), "{dir}: no {flag} in {}", log[run]);
        }
    }
    let panel_run = at(&|l| l.starts_with("plugin-panel$ npx vitest run "));
    assert!(
        log[panel_run].contains(" --coverage.include=hooks/**/*.tsx"),
        "{}",
        log[panel_run]
    );
    // Every path relative to the root, none naming the tree.
    let sources = tracefile_sources(&tree);
    assert_eq!(
        sources,
        [
            "crates/clm/src/main.rs",
            "web/src/app.ts",
            "web/abs.ts",
            "plugin-panel/src/app.ts",
            "plugin-panel/abs.ts",
        ]
    );
    let root = fs::canonicalize(tree.work()).unwrap();
    let root = root.to_str().unwrap();
    assert!(sources.iter().all(|s| !s.contains(root)), "{sources:?}");
}

#[test]
fn coverage_reads_the_same_from_a_checkout_reached_through_a_link() {
    // req:ci/coverage-script@ae8713bc
    let tree = coverage_tree();
    // The tool writes real paths, `pwd -P`'s; the shell's `$PWD` names the checkout by its link.
    let physical = CARGO_COVERAGE.replace("\"$PWD/crates", "\"$(pwd -P)/crates");
    assert_ne!(physical, CARGO_COVERAGE);
    tree.fake("cargo", &physical);
    let link = tree.dir.path().join("link");
    std::os::unix::fs::symlink(tree.work(), &link).unwrap();
    let out = Command::new(".github/scripts/coverage.sh")
        .current_dir(&link)
        .env("PWD", &link)
        .env("PATH", tree.path())
        .env("CI_LOG", tree.dir.path().join("log"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        tracefile_sources(&tree),
        [
            "crates/clm/src/main.rs",
            "web/src/app.ts",
            "web/abs.ts",
            "plugin-panel/src/app.ts",
            "plugin-panel/abs.ts",
        ]
    );
}

#[test]
fn a_failing_coverage_suite_fails_the_script() {
    // req:ci/coverage-script@ae8713bc
    let tree = coverage_tree();
    // A tracefile from an earlier run stays as it was.
    fs::create_dir_all(tree.work().join("target")).unwrap();
    fs::write(tree.work().join("target/lcov.info"), "SF:old\n").unwrap();
    fs::write(tree.dir.path().join("bin/fail-plugin-panel"), "").unwrap();
    let (ok, log, _) = tree.exec(None, "", None, &tree.path());
    assert!(!ok, "{log:?}");
    assert!(
        log.last()
            .unwrap()
            .starts_with("plugin-panel$ npx vitest run "),
        "{log:?}"
    );
    assert_eq!(tracefile_sources(&tree), ["old"]);

    // A failing Rust run, as nextest's exit 100 for a test that failed or met its bound, stops
    // the script before any npm suite.
    let tree = coverage_tree();
    fs::create_dir_all(tree.work().join("target")).unwrap();
    fs::write(tree.work().join("target/lcov.info"), "SF:old\n").unwrap();
    fs::write(tree.dir.path().join("bin/fail-cargo"), "").unwrap();
    let (ok, log, _) = tree.exec(None, "", None, &tree.path());
    assert!(!ok, "{log:?}");
    assert!(
        log.last().unwrap().starts_with("cargo llvm-cov nextest "),
        "{log:?}"
    );
    assert!(!log.iter().any(|l| l.contains("$ npm ")), "{log:?}");
    assert_eq!(tracefile_sources(&tree), ["old"]);
}

#[test]
fn no_vitest_version_fails_before_the_coverage_install() {
    // req:ci/coverage-script@ae8713bc
    let tree = coverage_tree();
    fs::create_dir_all(tree.work().join("target")).unwrap();
    fs::write(tree.work().join("target/lcov.info"), "SF:old\n").unwrap();
    fs::write(tree.dir.path().join("bin/no-version-plugin-panel"), "").unwrap();
    let (ok, log, stderr) = tree.exec(None, "", None, &tree.path());
    assert!(!ok, "{log:?}");
    assert!(
        stderr
            .lines()
            .any(|l| l == "coverage.sh: no vitest version in plugin-panel"),
        "{stderr}"
    );
    assert!(
        !log.iter().any(
            |l| l.starts_with("plugin-panel$ npm install") && l.contains("@vitest/coverage-v8")
        ),
        "{log:?}"
    );
    assert_eq!(tracefile_sources(&tree), ["old"]);
}

#[test]
fn nothing_else_runs_coverage() {
    // req:ci/coverage-script@ae8713bc
    let tree = Tree::new();
    executable(&tree.work().join(".github/scripts/coverage.sh"), FAKE);
    let (ok, log) = tree.run(None, "");
    assert!(ok, "{log:?}");
    assert!(!log.iter().any(|l| l.contains("llvm-cov")), "{log:?}");
    assert!(
        !log.iter().any(|l| l.starts_with("coverage.sh")),
        "the stand-in ran: {log:?}"
    );
    for (file, job, step) in crate::workflow_steps() {
        let run = step["run"].as_str().unwrap_or("");
        assert!(
            !run.contains("coverage.sh") && !run.contains("llvm-cov"),
            "{file} `{job}`: {run}"
        );
    }
}

#[test]
fn coverage_tools_are_pinned() {
    // req:ci/coverage-script@ae8713bc
    let tree = coverage_tree();
    let (ok, log, stderr) = tree.exec(None, "", None, &tree.path());
    assert!(ok, "{log:?} {stderr}");
    let cargo = cargo_calls(&log);
    let installs: Vec<_> = cargo
        .iter()
        .filter(|c| c.starts_with("cargo install"))
        .collect();
    assert_eq!(installs.len(), 3, "{cargo:?}");
    for (call, krate) in installs
        .iter()
        .zip(["cargo-machete", "cargo-nextest", "cargo-llvm-cov"])
    {
        assert!(is_pinned_install(call, krate), "{krate}: {cargo:?}");
    }
    let run = cargo
        .iter()
        .position(|c| c.starts_with("cargo llvm-cov nextest "))
        .unwrap_or_else(|| panic!("no llvm-cov nextest: {cargo:?}"));
    let last_install = cargo
        .iter()
        .rposition(|c| c.starts_with("cargo install"))
        .unwrap();
    assert!(last_install < run, "{cargo:?}");
    assert!(
        cargo[run]
            .split_whitespace()
            .any(|w| w == "--no-rustc-wrapper"),
        "{}",
        cargo[run]
    );
    assert!(
        !log.iter().any(|l| l.contains("--ignore-run-fail")),
        "{log:?}"
    );
    let vitest: Vec<_> = log
        .iter()
        .filter(|l| l.contains("@vitest/coverage-v8@"))
        .collect();
    assert_eq!(vitest.len(), 2, "{log:?}");
    for install in vitest {
        assert!(
            install.split_whitespace().any(|w| w == "--no-save"),
            "{install}"
        );
    }
    let mode = fs::metadata(script_path("coverage.sh"))
        .unwrap()
        .permissions()
        .mode();
    assert!(mode & 0o111 != 0, "coverage.sh isn't executable");
}

#[test]
fn nextest_replaces_cargo_test_but_for_doctests() {
    // req:ci/nextest-runner@b769cc74
    let (ok, log) = Tree::new().run(None, "");
    assert!(ok, "{log:?}");
    assert_eq!(tests_run(&log), [NEXTEST_ALL, DOC_TESTS]);
    // The doctests follow nextest, never the other way round.
    assert!(runs_before(&log, NEXTEST_ALL, DOC_TESTS), "{log:?}");
    // A failing nextest run fails the script before the doctests.
    let (ok, log) = Tree::new().run(None, NEXTEST_ALL);
    assert!(!ok, "{log:?}");
    assert_eq!(log.last().map(String::as_str), Some(NEXTEST_ALL));
}

#[test]
fn test_tools_installs_only_nextest_pinned() {
    // req:ci/test-tools@70e8cd5d
    let mut tree = Tree::new();
    tree.script = "test-tools.sh";
    let (ok, log) = tree.run(None, "");
    assert!(ok, "{log:?}");
    assert_eq!(log.len(), 1, "{log:?}");
    assert!(is_pinned_install(&log[0], "cargo-nextest"), "{log:?}");
    let mode = fs::metadata(script_path("test-tools.sh"))
        .unwrap()
        .permissions()
        .mode();
    assert!(mode & 0o111 != 0, "test-tools.sh isn't executable");
}

#[test]
fn test_tools_run_between_the_detector_tools_and_cargo() {
    // req:ci/test-tools@70e8cd5d
    let check = Tree::new();
    let coverage = coverage_tree();
    for tree in [&check, &coverage] {
        let (ok, log, stderr) = tree.exec(None, "", None, &tree.path());
        assert!(ok, "{}: {log:?} {stderr}", tree.script);
        let cargo = cargo_calls(&log);
        assert!(
            is_pinned_install(cargo[0], "cargo-machete")
                && is_pinned_install(cargo[1], "cargo-nextest"),
            "{}: {cargo:?}",
            tree.script
        );
    }
}

#[test]
fn a_full_run_times_every_phase_it_runs() {
    // req:ci/phase-times@10cc2378
    let tree = Tree::new();
    let out = tree.output(None, "", None, &tree.path());
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    let summary = stderr.split("ci-check: phases\n").nth(1).unwrap();
    let names: Vec<&str> = summary
        .lines()
        .filter_map(|l| l.split_once("s ").map(|(_, name)| name))
        .collect();
    assert_eq!(
        names,
        [
            "tools",
            "fmt",
            "clippy",
            "wasm",
            "tests",
            "plugin-panel",
            "validate plugin-panel",
            "capabilities/panel",
            "validate plugin",
            "web",
            "web/site",
            "capabilities/web",
            "total"
        ],
        "{stderr}"
    );
}

/// The lines of `text` starting `ci-check: `.
fn phase_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| l.starts_with("ci-check: "))
        .collect()
}

#[test]
fn a_passing_run_prints_its_phase_times() {
    // req:ci/phase-times@10cc2378
    let tree = Tree::new();
    let file = tree.dir.path().join("selection");
    fs::write(&file, "").unwrap();
    let out = tree.output(None, "", Some(file.as_os_str()), &tree.path());
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(phase_lines(&stdout).is_empty(), "{stdout}");
    let lines: Vec<&str> = stderr.lines().collect();
    let secs = |l: &str, suffix: &str| {
        l.strip_suffix(suffix)
            .and_then(|l| l.strip_suffix('s'))
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    };
    // `ci-check: <phase> <n>s` after each phase, in order.
    let each: Vec<&str> = lines
        .iter()
        .copied()
        .take_while(|l| *l != "ci-check: phases")
        .filter(|l| l.starts_with("ci-check: "))
        .collect();
    for (line, phase) in each.iter().zip(["tools", "fmt", "clippy", "wasm"]) {
        let rest = line.strip_prefix("ci-check: ").unwrap();
        let (name, n) = rest.rsplit_once(' ').unwrap();
        assert!(name == phase && secs(n, ""), "{line}");
    }
    assert_eq!(each.len(), 5, "{stderr}");
    // The summary: one `<n>s <phase>` line per phase, then the total, last.
    let summary = lines.iter().position(|l| *l == "ci-check: phases").unwrap();
    for (line, phase) in lines[summary + 1..]
        .iter()
        .zip(["tools", "fmt", "clippy", "wasm"])
    {
        assert!(secs(line, &format!(" {phase}")), "{line}");
    }
    assert!(secs(lines.last().unwrap(), " total"), "{stderr}");
    assert_eq!(lines.len() - summary, 1 + 5 + 1, "{stderr}");
}

#[test]
fn a_failing_phase_prints_only_where_it_failed() {
    // req:ci/phase-times@10cc2378
    let tree = Tree::new();
    tree.fake(
        "cargo",
        "#!/usr/bin/env bash\necho \"cargo $*\" >> \"$CI_LOG\"\n[ \"$1\" != clippy ] || exit 3\n",
    );
    let file = tree.dir.path().join("selection");
    fs::write(&file, "").unwrap();
    let out = tree.output(None, "", Some(file.as_os_str()), &tree.path());
    assert_eq!(out.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&out.stderr);
    let lines: Vec<&str> = stderr.lines().collect();
    let last = lines.last().unwrap();
    let rest = last
        .strip_prefix("ci-check: failed in clippy after ")
        .unwrap_or_else(|| panic!("{stderr}"));
    let (took, total) = rest.split_once("s, ").unwrap();
    assert!(took.bytes().all(|b| b.is_ascii_digit()), "{last}");
    assert!(
        total
            .strip_suffix("s total")
            .is_some_and(|n| n.bytes().all(|b| b.is_ascii_digit())),
        "{last}"
    );
    assert!(!stderr.contains("ci-check: phases"), "{stderr}");
    let phases = phase_lines(&stderr);
    assert!(
        !phases
            .iter()
            .any(|l| l.contains("wasm") || l.contains("tests")),
        "{stderr}"
    );
}

#[test]
fn a_clock_that_steps_back_prints_no_negative_time() {
    // req:ci/phase-times@10cc2378
    let tree = Tree::new();
    let file = tree.dir.path().join("selection");
    fs::write(&file, "").unwrap();
    // An exported bash function runs in the script's own shell, so it moves the script's
    // clock: `cargo fmt` sets `SECONDS` back by 100, logging that it did, then runs the fake on
    // `PATH`.
    let cargo = "() { if [ \"$1\" = fmt ]; then SECONDS=$((SECONDS - 100)); echo stepped >> \"$CI_LOG\"; fi; command cargo \"$@\"; }";
    let out = tree.output_env(
        None,
        "",
        Some(file.as_os_str()),
        &tree.path(),
        &[("BASH_FUNC_cargo%%", cargo)],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    // The clock did step back: the script ran the function, not only the fake.
    let log = fs::read_to_string(tree.dir.path().join("log")).unwrap();
    assert!(log.lines().any(|l| l == "stepped"), "{log}");
    let lines: Vec<&str> = stderr.lines().collect();
    assert!(lines.contains(&"ci-check: fmt 0s"), "{stderr}");
    let secs = |n: &str| {
        n.strip_suffix('s')
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    };
    // Each phase's line, then the summary's lines and the total.
    let summary = lines.iter().position(|l| *l == "ci-check: phases").unwrap();
    let phases: Vec<&str> = lines[..summary]
        .iter()
        .filter_map(|l| l.strip_prefix("ci-check: "))
        .collect();
    assert_eq!(phases.len(), 5, "{stderr}");
    for rest in phases {
        let (_, n) = rest.rsplit_once(' ').unwrap();
        assert!(secs(n), "{rest}: {stderr}");
    }
    assert_eq!(lines.len() - summary, 1 + 5 + 1, "{stderr}");
    for line in &lines[summary + 1..] {
        let (n, _) = line.split_once(' ').unwrap();
        assert!(secs(n), "{line}: {stderr}");
    }
    assert!(lines.last().unwrap().ends_with("s total"), "{stderr}");
}

#[test]
fn a_failure_after_the_clock_steps_back_prints_no_negative_time() {
    // req:ci/phase-times@10cc2378
    let tree = Tree::new();
    let file = tree.dir.path().join("selection");
    fs::write(&file, "").unwrap();
    // `cargo clippy` sets the script's clock back far past any run's length and fails, so both
    // the failing phase and the run started before the clock's step.
    let cargo = "() { if [ \"$1\" = clippy ]; then SECONDS=$((SECONDS - 100000)); return 3; fi; command cargo \"$@\"; }";
    let out = tree.output_env(
        None,
        "",
        Some(file.as_os_str()),
        &tree.path(),
        &[("BASH_FUNC_cargo%%", cargo)],
    );
    assert_eq!(out.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        stderr.lines().last(),
        Some("ci-check: failed in clippy after 0s, 0s total"),
        "{stderr}"
    );
}

#[test]
fn a_failure_before_the_first_phase_is_in_setup() {
    // req:ci/phase-times@10cc2378
    let tree = Tree::new();
    let file = tree.dir.path().join("selection");
    fs::write(&file, "file crates/clm/src/main.rs\n").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read(&file).is_ok() {
        // Root reads any file: there is no unreadable selection to give it.
        return;
    }
    let out = tree.output(None, "", Some(file.as_os_str()), &tree.path());
    assert!(!out.status.success());
    let log = fs::read_to_string(tree.dir.path().join("log")).unwrap_or_default();
    assert!(!log.lines().any(|l| l.starts_with("cargo ")), "{log}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.contains("::group::"), "{stdout}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let last = stderr.lines().last().unwrap_or("");
    let rest = last
        .strip_prefix("ci-check: failed in setup after ")
        .unwrap_or_else(|| panic!("{stderr}"));
    let (took, total) = rest.split_once("s, ").unwrap();
    assert!(
        !took.is_empty() && took.bytes().all(|b| b.is_ascii_digit()),
        "{last}"
    );
    assert!(
        total
            .strip_suffix("s total")
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())),
        "{last}"
    );
}

/// The stdout lines of a passing run with `selection` (everything selected when `None`).
fn stdout_of(tree: &Tree, selection: Option<&str>) -> Vec<String> {
    let file = tree.dir.path().join("selection");
    if let Some(selection) = selection {
        fs::write(&file, selection).unwrap();
    }
    let out = tree.output(None, "", selection.map(|_| file.as_os_str()), &tree.path());
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The files marked inside the phase `name`, between its `::group::` and `::endgroup::` lines,
/// or `None` when the phase has no group.
fn marked_in<'a>(lines: &'a [String], name: &str) -> Option<Vec<&'a str>> {
    let start = lines
        .iter()
        .position(|l| l == &format!("::group::{name}"))?;
    let len = lines[start..].iter().position(|l| l == "::endgroup::")?;
    Some(
        lines[start + 1..start + len]
            .iter()
            .filter_map(|l| l.strip_prefix("::clm-selected::"))
            .collect(),
    )
}

#[test]
fn a_web_change_marks_the_phases_it_selected() {
    // req:ci/step-markers@5ed839d2
    let tree = Tree::new();
    let lines = stdout_of(&tree, Some("file web/src/main.ts\n"));
    // A phase every run has names no file, and nothing sits between its markers.
    let at = lines.iter().position(|l| l == "::group::fmt").unwrap();
    assert_eq!(lines[at + 1], "::endgroup::", "{lines:?}");
    for phase in ["web", "capabilities/web"] {
        assert_eq!(
            marked_in(&lines, phase).unwrap(),
            ["web/src/main.ts"],
            "{phase}: {lines:?}"
        );
    }
    assert!(marked_in(&lines, "web/site").is_none(), "{lines:?}");
    for phase in ["plugin-panel", "capabilities/panel", "validate plugin"] {
        assert!(marked_in(&lines, phase).is_none(), "{phase}: {lines:?}");
    }
    // The Rust tests ran no file's package.
    assert_eq!(marked_in(&lines, "tests").unwrap(), [] as [&str; 0]);
}

#[test]
fn a_site_change_marks_the_site_and_the_phases_it_turned_on() {
    // req:ci/step-markers@5ed839d2
    let tree = Tree::new();
    let lines = stdout_of(&tree, Some("file web/site/guide/index.md\n"));
    for phase in ["web", "web/site", "capabilities/web"] {
        assert_eq!(
            marked_in(&lines, phase).unwrap(),
            ["web/site/guide/index.md"],
            "{phase}: {lines:?}"
        );
    }
}

#[test]
fn the_site_marks_only_its_own_files() {
    // req:ci/step-markers@5ed839d2
    let tree = Tree::new();
    let lines = stdout_of(
        &tree,
        Some("file web/src/main.ts\nfile web/site/guide/index.md\n"),
    );
    assert_eq!(
        marked_in(&lines, "web/site").unwrap(),
        ["web/site/guide/index.md"],
        "{lines:?}"
    );
    assert_eq!(
        marked_in(&lines, "web").unwrap(),
        ["web/src/main.ts", "web/site/guide/index.md"],
        "{lines:?}"
    );
}

#[test]
fn a_full_run_marks_each_phase_and_names_no_file() {
    // req:ci/step-markers@5ed839d2
    let tree = Tree::new();
    for selection in [None, Some("all Cargo.lock\nfile web/src/main.ts\n")] {
        let lines = stdout_of(&tree, selection);
        assert!(!lines.iter().any(|l| l.contains("::clm-selected::")));
        let groups = lines.iter().filter(|l| l.starts_with("::group::")).count();
        let ends = lines.iter().filter(|l| *l == "::endgroup::").count();
        assert_eq!(groups, 12, "{lines:?}");
        assert_eq!(ends, 12, "{lines:?}");
    }
}

#[test]
fn every_selected_phase_names_the_files_that_made_it_run() {
    // req:ci/step-markers@5ed839d2
    let tree = Tree::new();
    let lines = stdout_of(
        &tree,
        Some(
            "component clm\nfile crates/clm/tests/run.rs\nfile crates/clm/tests/run.rs\n\
             file crates/clm-check/src/lib.rs\nfile crates/gone/src/lib.rs\n\
             file plugin-panel/src/a.ts\nfile capabilities/panel/x.test.ts\n\
             file plugin/skills/x/SKILL.md\nfile capabilities/web/y.spec.ts\n\
             file docs/guide/x.md\n",
        ),
    );
    let expect: [(&str, &[&str]); 8] = [
        (
            "tests",
            &["crates/clm/tests/run.rs", "crates/clm-check/src/lib.rs"],
        ),
        ("plugin-panel", &["plugin-panel/src/a.ts"]),
        ("validate plugin-panel", &["plugin-panel/src/a.ts"]),
        (
            "capabilities/panel",
            &["plugin-panel/src/a.ts", "capabilities/panel/x.test.ts"],
        ),
        ("validate plugin", &["plugin/skills/x/SKILL.md"]),
        ("capabilities/web", &["capabilities/web/y.spec.ts"]),
        ("fmt", &[]),
        ("wasm", &[]),
    ];
    for (phase, files) in expect {
        assert_eq!(
            marked_in(&lines, phase).unwrap(),
            files,
            "{phase}: {lines:?}"
        );
    }
    assert!(marked_in(&lines, "web").is_none(), "{lines:?}");
    assert!(marked_in(&lines, "web/site").is_none(), "{lines:?}");
}

/// Runs the script with a `cargo` fake that records the cargo variables it saw, `land_dir`
/// as `CLM_LAND_DIR` (unset when `None`) and `target` as `CARGO_TARGET_DIR` (unset when
/// `None`), `CARGO_INCREMENTAL` unset. Returns the fake's records, one line per call.
fn cargo_variables(land_dir: Option<&str>, target: Option<&str>) -> Vec<String> {
    let tree = Tree::new();
    tree.fake(
        "cargo",
        "#!/usr/bin/env bash\necho \"${CARGO_TARGET_DIR-unset} ${CARGO_INCREMENTAL-unset}\" >> \"$CI_ENV_LOG\"\n",
    );
    let seen = tree.dir.path().join("seen");
    let mut c = Command::new(Path::new(".github/scripts").join(tree.script));
    c.current_dir(tree.work())
        .env("PATH", tree.path())
        .env("CI_LOG", tree.dir.path().join("log"))
        .env("CI_ENV_LOG", &seen)
        .env_remove("CI")
        .env_remove("CLM_SELECTION")
        .env_remove("CLM_LAND_DIR")
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("CARGO_INCREMENTAL");
    if let Some(dir) = land_dir {
        c.env("CLM_LAND_DIR", dir);
    }
    if let Some(dir) = target {
        c.env("CARGO_TARGET_DIR", dir);
    }
    let out = c.output().unwrap();
    assert!(out.status.success(), "{out:?}");
    let seen = fs::read_to_string(seen).unwrap();
    assert!(!seen.is_empty());
    seen.lines().map(str::to_owned).collect()
}

#[test]
fn a_lands_place_holds_its_cargo_output() {
    // req:ci/land-dir@ad032ef2
    let calls = cargo_variables(Some("/tmp/x/land"), None);
    assert!(calls.iter().all(|c| c == "/tmp/x/land 0"), "{calls:?}");
}

#[test]
fn without_a_land_place_the_cargo_variables_stay() {
    // req:ci/land-dir@ad032ef2
    for land_dir in [None, Some("")] {
        let calls = cargo_variables(land_dir, Some("/tmp/y"));
        assert!(calls.iter().all(|c| c == "/tmp/y unset"), "{calls:?}");
    }
}

/// A `cargo` for `mutate.sh`: records its arguments.
///
/// `cargo llvm-cov` prints `bin/llvm-cov-version` for `--version` (nothing when absent), and
/// otherwise writes its directory to `$CI_LOG.cov-cwd` and, unless `bin/llvm-cov-no-report`
/// exists, the lcov report `--output-path` names: for each `code` file of the plan `CLM_MUTATE`
/// names, an `SF:` line naming it absolutely under the real path of the directory it runs in, as
/// the tool does, and a `DA:` line for each of its lines 1 to 40, run once unless `bin/lcov-run`
/// lists the lines run (one per line, `<n>` for every file or `<file>:<n>` for one, then, after a
/// space, the times it ran when not once; an empty file for none). Given
/// `--no-default-ignore-filename-regex`, it reports the plan's `test` files the same way, then a
/// dependency's file outside the tree with its line 1 run, and a record for each `<file>:<n>`
/// line of `bin/lcov-extra`, with that line run. A call with no filterset
/// (`-E`), a run of every test of a package, exits first with the status in
/// `bin/llvm-cov-all-exit` when it exists, or 0 when `bin/llvm-cov-all-no-report` does, writing
/// nothing, and reads `bin/lcov-run-all` in place
/// of `bin/lcov-run` when it exists. Then it exits with the status in `bin/llvm-cov-exit` when it
/// exists, the report written, so the status alone tells a failed run.
///
/// `cargo mutants` prints nothing for `--version`. For `--list --json` it prints, as the tool
/// does, a pretty JSON array of the mutants `bin/list` lists as `<first line> <last line>
/// <name>`, each in the file its name names before the first `:` and in a function spanning lines
/// 1 to 99, which is not the mutant's span; with `--in-diff`, those of `bin/diff-list` when it
/// exists. Without `bin/list`, one mutant on line 1 of the first `--file`. For a run it writes the
/// outcome lists: `bin/diff-caught` and `bin/diff-missed` hold the mutants caught and missed for
/// a run on the diff, `bin/further-caught` and `bin/further-missed` for the run on further
/// mutants; when neither of a run's files exists, every mutant it is given by `--re` is missed.
/// It exits 2 when a mutant was missed, as `cargo mutants` does, and with the status in
/// `bin/mutants-exit` for a run on the repository's files. A run of the script's fixture, which
/// names one of its two tests, catches the mutants of the first and misses those of the second,
/// unless `bin/fixture-catches-all` or `bin/fixture-catches-none` exists. The directory of each
/// `cargo mutants` run goes to `$CI_LOG.cwd`, and again to `$CI_LOG.git` when it holds a `.git`
/// directory. A run of the fixture copies its `src/lib.rs` to `$CI_LOG.fixture-src`, its
/// `.git/objects/info/alternates` to `$CI_LOG.fixture-alternates`, and, as the tool would, copies
/// its directory, changes `src/lib.rs` in the copy and writes what `git status` says there of
/// tracked files to `$CI_LOG.fixture-status`. The first run on the repository's files copies
/// the directory it runs in to `$CI_LOG.tree`, since the script removes it when it ends.
///
/// A call's block is `fixture` for the fixture's, or the `<x>` of its `--file .../<x>.rs` or of
/// its filterset's `test(~proves_<x>)` (`letter_blocks`). With `bin/record-calls`, each `mutants`
/// and `llvm-cov` call appends `start <pid> <block> <CLM_SLOT> <CARGO_TARGET_DIR>` to
/// `$CI_LOG.calls`, and `end <pid>` as it exits, and a block's call first waits until another
/// block's call runs or two seconds pass. A run of block `<x>`'s mutants exits with the status in
/// `bin/exit-<x>`, after creating `$CI_LOG.failed-<x>`; waits, when `bin/after-fail-<x>` names a
/// block, until that block has failed so and a second more; waits until `$CI_LOG.gate` exists
/// when `bin/gate-<x>` exists; and writes its pid to `$CI_LOG.sleeper` and becomes a long sleep
/// when `bin/sleep-<x>` exists.
const CARGO_MUTANTS: &str = r#"#!/usr/bin/env bash
echo "cargo $*" >> "$CI_LOG"
bin=$(dirname "$0")
arg_after() {
  local want=$1 prev= a
  shift
  for a; do
    [ "$prev" != "$want" ] || { echo "$a"; return; }
    prev=$a
  done
}
# Waits until the file $1 exists, for at most $2 tenths of a second.
wait_for() {
  local i=0
  while [ ! -e "$1" ] && [ "$i" -lt "$2" ]; do sleep 0.1; i=$((i + 1)); done
}
# The block a call is for: `fixture` for the fixture's, else the `<x>` of its
# `--file crates/<dir>/src/<x>.rs`, or of its filterset's `test(~proves_<x>)`.
block_of() {
  local f
  case " $* " in *"test(~fixture_"*) echo fixture; return ;; esac
  f=$(arg_after --file "$@")
  if [ -n "$f" ]; then f=${f##*/}; echo "${f%.rs}"; return; fi
  printf '%s\n' "$*" | sed -n 's/.*test(~proves_\([a-z0-9]*\)).*/\1/p'
}
# `mutants --Zmutate-file <file> --json` lists one mutant of the script's fixture's test file,
# on its line 1, and none of another file unless `bin/zlist` holds `<first> <last>` lines: a
# mutant for each, which appends ` /* mutant <first> */` to the file's line `<first>`, with
# the diff that does so. For another file it exits with the status in `bin/zlist-exit`, prints
# what is no JSON with `bin/zlist-garbage`, and lists a mutant on line 6 whose diff applies to
# no file with `bin/zlist-stale`; each such file it lists goes to `$CI_LOG.zlisted`.
if [ "$1" = mutants ] && [ "$2" = --Zmutate-file ]; then
  f=$3
  case $f in
    *clm_mutate_fixture.rs) lines="1 1" ;;
    *)
      [ ! -e "$bin/zlist-exit" ] || exit "$(cat "$bin/zlist-exit")"
      if [ -e "$bin/zlist-garbage" ]; then printf 'not json'; exit 0; fi
      if [ -e "$bin/zlist-stale" ]; then
        printf '[{"file":"x","name":"x:1:5: stale","diff":"--- x\\n+++ y\\n@@ -1,1 +1,1 @@\\n-no such line\\n+a mutant\\n","span":{"start":{"line":6,"column":5},"end":{"line":6,"column":9}}}]'
        exit 0
      fi
      echo "$f" >> "$CI_LOG.zlisted"
      lines=$(cat "$bin/zlist" 2>/dev/null || true)
      ;;
  esac
  printf '['
  sep=
  while read -r first last; do
    [ -n "$last" ] || continue
    d=$(diff -u --label "${f##*/}" --label mutant "$f" <(sed "${first}s|\$| /* mutant $first */|" "$f") |
      sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' -e 's/\t/\\t/g' | awk '{ printf "%s\\n", $0 }')
    printf '%s{"file":"%s","name":"%s:%s:5: mutant %s","diff":"%s",' "$sep" "${f##*/}" "${f##*/}" "$first" "$first" "$d"
    printf '"span":{"start":{"line":%s,"column":5},"end":{"line":%s,"column":9}}}' "$first" "$last"
    sep=,
  done <<EOF
$lines
EOF
  printf ']'
  exit 0
fi
# `nextest run` exits 100, as when a test fails, for the fixture's test file's test that
# notices, and for its blind one too when `bin/nextest-fixture-catches-all` exists, 0 for its
# blind one, after copying the fixture's test file to `$CI_LOG.fixture-test-file`, its
# `.git/objects/info/alternates` to `$CI_LOG.fixture-file-alternates` and what `git status`
# says of tracked files to `$CI_LOG.fixture-file-status`; for any other run it appends its
# directory to `$CI_LOG.nextest-cwd` and its `CLM_SLOT` and `CARGO_TARGET_DIR` to
# `$CI_LOG.nextest-env` and what `git status` says there of tracked files to
# `$CI_LOG.nextest-status`, copies the plan's first test file there to `$CI_LOG.nextest-file`, and
# exits with the status in `bin/nextest-exit`, or 0.
if [ "$1" = nextest ] && [ "$2" = run ]; then
  case " $* " in *"test(~fixture_file_"*)
    cp tests/clm_mutate_fixture.rs "$CI_LOG.fixture-test-file"
    cp .git/objects/info/alternates "$CI_LOG.fixture-file-alternates" 2>/dev/null || true
    git status --porcelain --untracked-files=no > "$CI_LOG.fixture-file-status" 2>&1 ||
      echo "git status failed" >> "$CI_LOG.fixture-file-status"
    ;;
  esac
  case " $* " in
    *"test(~fixture_file_notices)"*) exit 100 ;;
    *"test(~fixture_file_blind)"*) [ ! -e "$bin/nextest-fixture-catches-all" ] || exit 100; exit 0 ;;
  esac
  echo "$PWD" >> "$CI_LOG.nextest-cwd"
  echo "${CLM_SLOT-unset} ${CARGO_TARGET_DIR-unset}" >> "$CI_LOG.nextest-env"
  git status --porcelain --untracked-files=no >> "$CI_LOG.nextest-status" 2>&1 ||
    echo "git status failed" >> "$CI_LOG.nextest-status"
  t=$(sed -n 's/^test //p' "$CLM_MUTATE" | head -n 1 | cut -f1)
  [ ! -f "$t" ] || cp "$t" "$CI_LOG.nextest-file"
  exit "$(cat "$bin/nextest-exit" 2>/dev/null || echo 0)"
fi
case " $* " in *" --version "*) ;; *)
  if [ -e "$bin/record-calls" ] && { [ "$1" = llvm-cov ] || [ "$1" = mutants ]; }; then
    me=$(block_of "$@")
    echo "start $$ $me ${CLM_SLOT-unset} ${CARGO_TARGET_DIR-unset}" >> "$CI_LOG.calls"
    trap 'echo "end $$" >> "$CI_LOG.calls"' EXIT
    # A block's call waits until another block's call runs, or two seconds pass.
    if [ "$me" != fixture ]; then
      for i in $(seq 1 20); do
        awk -v me="$me" '$1 == "start" && $3 != me && $3 != "fixture" { run[$2] = 1 }
          $1 == "end" { delete run[$2] } END { for (p in run) exit 0; exit 1 }' "$CI_LOG.calls" && break
        sleep 0.1
      done
    fi
  fi ;;
esac
if [ "$1" = llvm-cov ]; then
  case " $* " in
    *" --version "*) cat "$bin/llvm-cov-version" 2>/dev/null; exit 0 ;;
  esac
  echo "$PWD" >> "$CI_LOG.cov-cwd"
  report=$(arg_after --output-path "$@")
  # A call with no filterset runs every test of the package.
  runs=$bin/lcov-run
  case " $* " in *" -E "*) ;; *)
    [ ! -e "$bin/llvm-cov-all-exit" ] || exit "$(cat "$bin/llvm-cov-all-exit")"
    [ ! -e "$bin/lcov-run-all" ] || runs=$bin/lcov-run-all
    [ ! -e "$bin/llvm-cov-all-no-report" ] || exit 0
    ;;
  esac
  [ ! -e "$bin/llvm-cov-no-report" ] || exit 0
  : > "$report"
  {
    sed -n 's/^code //p' "$CLM_MUTATE"
    # The files under `tests/`, and the dependencies' files, only when they aren't left out.
    case " $* " in *" --no-default-ignore-filename-regex "*)
      sed -n 's/^test //p' "$CLM_MUTATE" | cut -f1 | sort -u
      ;;
    esac
  } | while IFS= read -r f; do
    echo "SF:$(pwd -P)/$f" >> "$report"
    for n in $(seq 1 40); do
      run=1
      if [ -e "$runs" ]; then
        run=$(awk -v n="$n" -v fn="$f:$n" '$1 == n || $1 == fn { print ($2 == "" ? 1 : $2); found = 1; exit }
          END { if (!found) print 0 }' "$runs")
      fi
      echo "DA:$n,$run" >> "$report"
    done
    echo end_of_record >> "$report"
  done
  case " $* " in *" --no-default-ignore-filename-regex "*)
    # A dependency's file, outside the tree, and each `<file>:<line>` of `bin/lcov-extra` run.
    printf 'SF:/nonexistent-registry/dep-1.0.0/src/lib.rs\nDA:1,1\nend_of_record\n' >> "$report"
    while IFS= read -r x; do
      [ -n "$x" ] || continue
      printf 'SF:%s/%s\nDA:%s,1\nend_of_record\n' "$(pwd -P)" "${x%:*}" "${x##*:}" >> "$report"
    done < <(cat "$bin/lcov-extra" 2>/dev/null || true)
    ;;
  esac
  [ ! -e "$bin/llvm-cov-exit" ] || exit "$(cat "$bin/llvm-cov-exit")"
  exit 0
fi
[ "$1" = mutants ] || exit 0
case " $* " in
  *" --version "*) exit 0 ;;
esac
echo "$PWD" >> "$CI_LOG.cwd"
[ ! -d .git ] || echo "$PWD" >> "$CI_LOG.git"
case " $* " in
  *"test(~fixture_"*)
    cp src/lib.rs "$CI_LOG.fixture-src"
    cp .git/objects/info/alternates "$CI_LOG.fixture-alternates" 2>/dev/null || true
    copy=$(mktemp -d)
    cp -R . "$copy/c"
    echo '// a mutant' >> "$copy/c/src/lib.rs"
    git -C "$copy/c" status --porcelain --untracked-files=no > "$CI_LOG.fixture-status" 2>&1 ||
      echo "git status failed" >> "$CI_LOG.fixture-status"
    rm -rf "$copy"
    ;;
  *) [ -e "$CI_LOG.tree" ] || cp -R . "$CI_LOG.tree" ;;
esac
case " $* " in
  *" --list "*)
    list=$bin/list
    case " $* " in *" --in-diff "*) [ ! -e "$bin/diff-list" ] || list=$bin/diff-list ;; esac
    if [ -e "$list" ]; then
      lines=$(cat "$list")
    else
      lines="1 1 $(arg_after --file "$@"):1:1: replace f with ()"
    fi
    # As the tool does, a diff with no mutant of the files prints nothing, not `[]`.
    case " $* " in *" --in-diff "*) [ -n "$lines" ] || exit 0 ;; esac
    echo "["
    sep=
    while read -r first last name; do
      [ -n "$name" ] || continue
      printf '%s  {\n' "$sep"
      printf '    "file": "%s",\n' "${name%%:*}"
      printf '    "function": {\n      "function_name": "f",\n      "span": {\n'
      printf '        "end": {\n          "column": 2,\n          "line": 99\n        },\n'
      printf '        "start": {\n          "column": 1,\n          "line": 1\n        }\n      }\n    },\n'
      printf '    "name": "%s",\n' "$(printf '%s' "$name" | sed 's/["\\]/\\&/g')"
      printf '    "span": {\n'
      printf '      "end": {\n        "column": 9,\n        "line": %s\n      },\n' "$last"
      printf '      "start": {\n        "column": 5,\n        "line": %s\n      }\n    }\n' "$first"
      printf '  }'
      sep=,
    done <<EOF
$lines
EOF
    printf '\n]\n'
    exit 0
    ;;
esac
out=$(arg_after --output "$@")
mode=further
case " $* " in
  *" --in-diff "*) mode=diff ;;
  *"test(~fixture_notices)"*) mode=notices ;;
  *"test(~fixture_blind)"*) mode=blind ;;
esac
case $mode in
  notices) caught=1; missed=0; [ ! -e "$bin/fixture-catches-none" ] || { caught=0; missed=1; } ;;
  blind) caught=0; missed=1; [ ! -e "$bin/fixture-catches-all" ] || { caught=1; missed=0; } ;;
  *)
    x=$(block_of "$@")
    if [ -e "$bin/exit-$x" ]; then : > "$CI_LOG.failed-$x"; exit "$(cat "$bin/exit-$x")"; fi
    if [ -e "$bin/after-fail-$x" ]; then
      wait_for "$CI_LOG.failed-$(cat "$bin/after-fail-$x")" 300
      sleep 1
    fi
    [ ! -e "$bin/gate-$x" ] || wait_for "$CI_LOG.gate" 600
    if [ -e "$bin/sleep-$x" ]; then echo $$ > "$CI_LOG.sleeper"; exec sleep 600; fi
    [ ! -e "$bin/mutants-exit" ] || exit "$(cat "$bin/mutants-exit")"
    if [ -e "$bin/$mode-caught" ] || [ -e "$bin/$mode-missed" ]; then
      caught=$(cat "$bin/$mode-caught" 2>/dev/null || echo 0)
      missed=$(cat "$bin/$mode-missed" 2>/dev/null || echo 0)
    else
      caught=0
      missed=$(printf '%s\n' "$@" | grep -cx -- --re || true)
    fi
    ;;
esac
mkdir -p "$out/mutants.out"
for i in $(seq 1 "$caught"); do echo "caught $i" >> "$out/mutants.out/caught.txt"; done
for i in $(seq 1 "$missed"); do echo "missed $i" >> "$out/mutants.out/missed.txt"; done
[ "$missed" -eq 0 ] || exit 2
"#;
/// An `npx` for `mutate.sh`: records its arguments led by its directory and, for `stryker run
/// <config>`, writes the report the configuration file names with the mutants of
/// `bin/stryker-killed`, `bin/stryker-survived` and `bin/stryker-nocov` (reported `NoCoverage`),
/// each on a line of its own counted from 1 in each run, so two runs report the same mutants,
/// and one `CompileError` unless `bin/stryker-nocov` exists without `bin/stryker-compile-error`;
/// and keeps copies of the configuration file
/// and of the vitest configuration it names in `$CI_LOG.stryker` and `$CI_LOG.vitest`, and in
/// `$CI_LOG.fixture-stryker`, `$CI_LOG.fixture-gitignore` and `$CI_LOG.fixture-dirs` a fixture
/// run's configuration, the fixture's `.gitignore` and every directory it holds, and in
/// `$CI_LOG.tempdir` the path of the `tempDirName` it names, then a `<entry> <link target>` line
/// for each entry that directory holds (`-` for one that is no link), and in
/// `$CI_LOG.stale-tempdirs` any other `.stryker-tmp*` directory beside it. It keeps the fixture's
/// test file in `$CI_LOG.fixture-test`. A run on
/// the script's fixture reports its first function's mutant killed and its second's survived,
/// unless `bin/stryker-fixture-kills-all` exists, or reports another of the second's `NoCoverage` when
/// `bin/stryker-fixture-blind-nocov` exists. A fixture run whose one test file imports none of
/// the files its `mutate` names reports one mutant `NoCoverage`, or one survived when
/// `bin/stryker-fixture-unrelated-survives` exists, or none when
/// `bin/stryker-fixture-unrelated-empty` does. Each run keeps its configuration, and the test
/// files its vitest configuration names (`tests`, one a line, and `test-<n>`, a copy of each), in
/// `$CI_LOG.runs/<n>/`, numbered from 0. When `bin/stryker-related` exists, a run none of whose
/// test files imports a file its `mutate` names exits 1 writing no report, as StrykerJS does when
/// vitest keeps only the tests that import one. A run on anything else exits with the status in
/// `bin/stryker-exit`, writing no report.
const NPX_STRYKER: &str = r#"#!/usr/bin/env bash
echo "$(basename "$PWD")\$ npx $*" >> "$CI_LOG"
bin=$(dirname "$0")
[ "$1 $2" = "stryker run" ] || exit 0
cfg=$3
report=$(grep -o '"fileName": "[^"]*"' "$cfg" | sed -e 's/^"fileName": "//' -e 's/"$//')
tmp=$(grep -o '"tempDirName": "[^"]*"' "$cfg" | sed -e 's/^"tempDirName": "//' -e 's/"$//')
vcfg=$(grep -o '"configFile": "[^"]*"' "$cfg" | sed -e 's/^"configFile": "//' -e 's/"$//')
# Each run's configuration and the test files its vitest configuration names, in order.
tests=$(grep -o "include: \[[^]]*\]" "$vcfg" | grep -o "'[^']*'" | tr -d "'")
run=$CI_LOG.runs/$(ls "$CI_LOG.runs" 2>/dev/null | wc -l)
mkdir -p "$run"
cp "$cfg" "$run/stryker.config.json"
i=0
imports=
for t in $tests; do
  i=$((i + 1))
  echo "$t" >> "$run/tests"
  cp "$t" "$run/test-$i" 2>/dev/null || true
  for m in $(grep -o '"mutate": \[.*\]' "$cfg" | grep -o '"[^"]*:[0-9]*-[0-9]*"' | tr -d '"'); do
    stem=$(basename "${m%%:*}"); stem=${stem%.*}
    ! grep -qE "from '([^']*/)?$stem(\.ts)?'" "$t" 2>/dev/null || imports=1
  done
done
# As StrykerJS does when vitest keeps only the tests that import a mutated file.
[ ! -e "$bin/stryker-related" ] || [ -n "$imports" ] || exit 1
nocompile=
if grep -q clm-mutate-fixture "$cfg"; then
  cp clm-mutate-fixture/fixture.test.ts "$CI_LOG.fixture-test"
  cp clm-mutate-fixture/.gitignore "$CI_LOG.fixture-gitignore" 2>/dev/null || true
  cp "$cfg" "$CI_LOG.fixture-stryker"
  find clm-mutate-fixture -type d > "$CI_LOG.fixture-dirs"
  if [ -e "$bin/stryker-fixture-kills-none" ]; then
    killed=0; survived=1
  elif [ "$i" -eq 1 ] && [ -z "$imports" ]; then
    # The test file that imports nothing: its mutants are `NoCoverage`, unless a switch says
    # otherwise.
    killed=0; survived=0; nocov=1; nocompile=1
    [ ! -e "$bin/stryker-fixture-unrelated-survives" ] || { survived=1; nocov=0; }
    [ ! -e "$bin/stryker-fixture-unrelated-empty" ] || nocov=0
  elif grep -q 'fixture.ts:1-1' "$cfg" || [ -e "$bin/stryker-fixture-kills-all" ]; then
    killed=1; survived=0
  elif [ -e "$bin/stryker-fixture-blind-nocov" ]; then
    killed=0; survived=1; nocov=1
  else
    killed=0; survived=1
  fi
else
  # The sandbox's directory, then each entry it holds and where it links, `-` for none.
  {
    echo "$PWD/$tmp"
    for e in "$tmp"/* "$tmp"/.*; do
      case ${e##*/} in . | .. | '*' | '.*') continue ;; esac
      echo "${e##*/} $(readlink "$e" || echo -)"
    done
  } > "$CI_LOG.tempdir"
  # Any other run's sandbox directory still in the npm directory.
  ls -A | grep '^\.stryker-tmp' | grep -vx -- "$tmp" > "$CI_LOG.stale-tempdirs" || true
  cp "$cfg" "$CI_LOG.stryker"
  cp "$(grep -o '"configFile": "[^"]*"' "$cfg" | sed -e 's/^"configFile": "//' -e 's/"$//')" "$CI_LOG.vitest"
  [ ! -e "$bin/stryker-exit" ] || exit "$(cat "$bin/stryker-exit")"
  killed=$(cat "$bin/stryker-killed" 2>/dev/null || echo 0)
  survived=$(cat "$bin/stryker-survived" 2>/dev/null || echo 0)
  nocov=$(cat "$bin/stryker-nocov" 2>/dev/null || echo 0)
fi
n=0
mutant() {
  n=$((n + 1))
  printf '{"status":"%s","mutatorName":"M","replacement":"r","location":{"start":{"line":%s,"column":1},"end":{"line":%s,"column":2}}}' "$1" "$n" "$n"
}
{
  printf '{"files":{"a":{"mutants":['
  sep=
  for i in $(seq 1 "$killed"); do printf '%s' "$sep"; mutant Killed; sep=,; done
  for i in $(seq 1 "$survived"); do printf '%s' "$sep"; mutant Survived; sep=,; done
  for i in $(seq 1 "${nocov:-0}"); do printf '%s' "$sep"; mutant NoCoverage; sep=,; done
  if [ -z "$nocompile" ] && { [ ! -e "$bin/stryker-nocov" ] || [ -e "$bin/stryker-compile-error" ]; }; then
    printf '%s' "$sep"; mutant CompileError
  fi
  printf ']}}}'
} >"$report"
"#;

/// A tree that runs `mutate.sh` with the fakes for it and a `coverage.sh` that logs and writes
/// `target/lcov.info`, and the plan file `plan` holding `plan_text`.
pub(crate) fn mutate_tree(plan_text: &str) -> Tree {
    let mut tree = Tree::new();
    tree.script = "mutate.sh";
    let scripts = tree.work().join(".github/scripts");
    executable(&scripts.join("mutate.sh"), &script("mutate.sh"));
    executable(
        &scripts.join("coverage.sh"),
        "#!/usr/bin/env bash\necho coverage.sh >> \"$CI_LOG\"\necho \"${CLM_SLOT-unset}\" >> \"$CI_LOG.coverage-slot\"\nls -A web | grep '^\\.stryker-tmp' >> \"$CI_LOG.coverage-tempdirs\"\n[ -z \"${COVERAGE_FAIL:-}\" ] || exit 1\nmkdir -p target\necho 'SF:x' > target/lcov.info\n",
    );
    tree.fake("cargo", CARGO_MUTANTS);
    tree.fake("npx", NPX_STRYKER);
    tree.fake(
        "rustup",
        "#!/usr/bin/env bash\necho \"rustup $*\" >> \"$CI_LOG\"\n",
    );
    fs::write(tree.dir.path().join("plan"), plan_text).unwrap();
    // A checkout at a commit, as the verify tree always is.
    git(&tree.work(), &["init", "-q", "-b", "main"]);
    git(&tree.work(), &["add", "-A"]);
    git(&tree.work(), &["commit", "-q", "-m", "tree"]);
    tree
}

/// Runs `git` in `dir` with an identity of its own and none of the caller's git variables;
/// returns its trimmed stdout and panics when it fails.
fn git(dir: &Path, args: &[&str]) -> String {
    let mut c = Command::new("git");
    c.current_dir(dir).args(args);
    without_git_env(&mut c);
    let out = c
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_owned()
}

/// Removes the caller's git variables from `c` and keeps the machine's and the user's git
/// configuration from it, so a signing, hook or remote-name setting there changes nothing.
fn without_git_env(c: &mut Command) {
    for (k, _) in std::env::vars_os() {
        if k.to_string_lossy().starts_with("GIT_") {
            c.env_remove(k);
        }
    }
    c.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null");
}

impl Tree {
    /// Sets what a fake in `bin/` reports.
    fn report(&self, name: &str, value: &str) {
        fs::write(self.dir.path().join("bin").join(name), value).unwrap();
    }

    /// Runs `mutate.sh` on the plan with `extra` environment variables; returns its status, the
    /// commands it ran, its result file and its stderr.
    fn mutate(&self, extra: &[(&str, &str)]) -> (bool, Vec<String>, Vec<String>, String) {
        let out = self.mutate_command(extra).output().unwrap();
        (
            out.status.success(),
            self.log_lines(""),
            self.log_lines("result"),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    /// The command that runs `mutate.sh` on the plan with `extra` environment variables, its
    /// earlier run's logs removed.
    pub(crate) fn mutate_command(&self, extra: &[(&str, &str)]) -> Command {
        let log = self.dir.path().join("log");
        let _ = fs::remove_file(&log);
        for kept in [
            "cwd",
            "git",
            "fixture-src",
            "fixture-alternates",
            "fixture-status",
            "cov-cwd",
            "stryker",
            "vitest",
            "calls",
            "gate",
            "sleeper",
            "coverage-slot",
            "coverage-tempdirs",
            "tempdir",
            "stale-tempdirs",
            "fixture-test",
            "fixture-test-file",
            "fixture-file-alternates",
            "fixture-file-status",
            "nextest-cwd",
            "nextest-env",
            "nextest-file",
            "zlisted",
            "nextest-status",
        ] {
            let _ = fs::remove_file(self.dir.path().join(format!("log.{kept}")));
        }
        let _ = fs::remove_dir_all(self.dir.path().join("log.tree"));
        let _ = fs::remove_dir_all(self.dir.path().join("log.runs"));
        let mut c = Command::new(".github/scripts/mutate.sh");
        c.current_dir(self.work())
            .env("PATH", self.path())
            .env("CI_LOG", &log)
            .env("CLM_MUTATE", self.dir.path().join("plan"))
            .env("CLM_MUTATE_RESULT", self.dir.path().join("result"))
            .env_remove("CLM_LAND_DIR")
            .env_remove("CLM_MUTATE_JOBS")
            .env_remove("CLM_SLOT")
            .env_remove("NEXTEST_PROFILE")
            .env_remove("NEXTEST_CONFIG");
        without_git_env(&mut c);
        for (k, v) in extra {
            c.env(k, v);
        }
        c
    }

    /// The lines of the file `name` beside the tree (`log.<name>`, or `log` when empty, for
    /// the logs), none when it is missing.
    fn log_lines(&self, name: &str) -> Vec<String> {
        let file = match name {
            "" => "log".to_owned(),
            "result" => "result".to_owned(),
            n => format!("log.{n}"),
        };
        fs::read_to_string(self.dir.path().join(file))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

pub(crate) const RUST_BLOCK: &str = "requirement a/x\ncode crates/clm-check/src/lib.rs\ndiff /tmp/the.diff\ntest crates/clm-check/tests/it.rs\tproves_x\nlimit 20\n";

#[test]
fn a_rust_requirement_whose_test_notices_is_killed() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(RUST_BLOCK);
    tree.report("diff-caught", "1");
    tree.report(
        "list",
        "1 1 crates/clm-check/src/lib.rs:1:1: replace d with ()\n2 2 crates/clm-check/src/lib.rs:2:1: replace f with ()\n",
    );
    tree.report(
        "diff-list",
        "1 1 crates/clm-check/src/lib.rs:1:1: replace d with ()\n",
    );
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x killed"]);
    let tries: Vec<&String> = log
        .iter()
        .filter(|l| l.starts_with("cargo mutants ") && l.contains("--re "))
        .collect();
    let run = tries.first().unwrap_or_else(|| panic!("{log:?}"));
    for part in [
        "--in-diff /tmp/the.diff",
        "--test-package clm-check",
        "--file crates/clm-check/src/lib.rs",
        "package(=clm-check)",
        "binary_id(=clm-check::it)",
        "test(~proves_x)",
        // A test that asks git about the repository passes in cargo-mutants' copy of the tree.
        "--copy-vcs true",
        // A test in another package than the code: the baseline matches no test and still passes.
        "-- --no-tests=pass -E",
    ] {
        assert!(run.contains(part), "no {part} in {run}");
    }
    assert!(!run.contains("binary("), "{run}");
    // Caught on the diff, so no further mutant is tried, and the pass ends with coverage.
    assert_eq!(tries.len(), 1, "{log:?}");
    assert_eq!(log.last().unwrap(), "coverage.sh", "{log:?}");
}

#[test]
fn tests_that_notice_nothing_survive_with_the_count_of_both_runs() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(RUST_BLOCK);
    tree.report("diff-missed", "3");
    tree.report("further-missed", "2");
    tree.report(
        "list",
        "1 1 crates/clm-check/src/lib.rs:1:1: replace g with \"x\"\n2 2 crates/clm-check/src/lib.rs:2:1: replace f with ()\n",
    );
    tree.report(
        "diff-list",
        "1 1 crates/clm-check/src/lib.rs:1:1: replace g with \"x\"\n",
    );
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 5"]);
    // The diff's run names the diff's mutant; the further run names the listed mutant that
    // isn't the diff's, and the tests are still the block's alone.
    let tried: Vec<&String> = log
        .iter()
        .filter(|l| l.starts_with("cargo mutants ") && l.contains("--re "))
        .collect();
    assert_eq!(tried.len(), 2, "{log:?}");
    assert!(
        tried[0].contains("--in-diff") && tried[0].contains("replace g with \"x\""),
        "{}",
        tried[0]
    );
    let further = tried[1];
    assert!(
        further.contains("replace f with \\(\\)")
            && !further.contains("replace g")
            && !further.contains("--in-diff")
            && further.contains("test(~proves_x)"),
        "{further}"
    );
    // The listings and the runs, and the run that finds the lines the tests run, are all in the
    // one tree, made once, outside the checkout.
    let dirs = fs::read_to_string(tree.dir.path().join("log.cwd")).unwrap();
    let block: Vec<&str> = dirs.lines().skip(2).collect();
    assert_eq!(block.len(), 4, "{dirs}");
    let cov = fs::read_to_string(tree.dir.path().join("log.cov-cwd")).unwrap();
    assert_eq!(cov.lines().collect::<Vec<_>>(), [block[0]], "{cov}");
    assert!(block.iter().all(|d| *d == block[0]), "{dirs}");
    let work = fs::canonicalize(tree.work()).unwrap();
    assert!(!Path::new(block[0]).starts_with(&work), "{dirs}");
    assert_eq!(
        log.iter()
            .filter(|l| l.starts_with("cargo mutants ") && l.contains("fixture_"))
            .count(),
        2,
        "{log:?}"
    );
}

#[test]
fn a_block_of_playwright_specs_is_not_judged() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(
        "requirement w/y\ncode web/src/app.ts\ntest web/e2e/app.spec.ts\tpaints\nlimit 20\n",
    );
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["w/y not-judged"]);
    assert!(
        !log.iter()
            .any(|l| l.contains("stryker") || l.contains("mutants")),
        "{log:?}"
    );
}

#[test]
fn a_typescript_requirement_runs_stryker_on_its_diff_and_its_vitest_tests() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree("");
    let diff = tree.dir.path().join("the.diff");
    fs::write(
        &diff,
        "--- a/web/src/app.ts\n+++ b/web/src/app.ts\n@@ -3,0 +4,2 @@\n+x\n+y\n",
    )
    .unwrap();
    let plan = format!(
        "requirement w/y\ncode web/src/app.ts\ndiff {}\ntest web/test/app.test.ts\tpaints\nlimit 20\n",
        diff.display()
    );
    fs::write(tree.dir.path().join("plan"), plan).unwrap();
    tree.report("stryker-killed", "1");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["w/y killed"]);
    // Added to `node_modules/` alone, then run on the diff's lines with the vitest runner.
    let install = log
        .iter()
        .position(|l| {
            l.starts_with("web$ npm install --no-save --ignore-scripts @stryker-mutator/")
        })
        .unwrap_or_else(|| panic!("{log:?}"));
    let run = log
        .iter()
        .position(|l| l.starts_with("web$ npx stryker run "))
        .unwrap_or_else(|| panic!("{log:?}"));
    assert!(install < run, "{log:?}");
    let pinned = |pkg: &str| {
        log[install].split_whitespace().any(|w| {
            w.strip_prefix(&format!("@stryker-mutator/{pkg}@"))
                .is_some_and(|v| {
                    v.split('.').count() == 3 && v.split('.').all(|p| p.parse::<u32>().is_ok())
                })
        })
    };
    assert!(
        pinned("core") && pinned("vitest-runner"),
        "{}",
        log[install]
    );
    // StrykerJS takes its options from a file under `web/node_modules/`, not from the command
    // line: the vitest runner, a vitest configuration that includes that test alone, run whether
    // or not it imports the mutated file (`related` false), and the diff's ranges.
    let words: Vec<&str> = log[run].split_whitespace().collect();
    assert_eq!(words[2..4], ["stryker", "run"], "{}", log[run]);
    assert!(
        words[4].contains("/web/node_modules/") && words.len() == 5,
        "{}",
        log[run]
    );
    assert!(
        words.iter().all(|w| !w.starts_with("--vitest")),
        "{}",
        log[run]
    );
    let config = fs::read_to_string(tree.dir.path().join("log.stryker")).unwrap();
    assert!(
        config.contains("\"testRunner\": \"vitest\"")
            && config.contains("\"related\": false")
            && config.contains("\"mutate\": [\"src/app.ts:4-5\"]")
            && !config.contains("thresholds"),
        "{config}"
    );
    let vitest = fs::read_to_string(tree.dir.path().join("log.vitest")).unwrap();
    assert!(
        vitest.contains("include: ['test/app.test.ts',]"),
        "{vitest}"
    );
}

#[test]
fn a_plan_with_nothing_to_mutate_is_not_judged() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree("requirement d/z\ncode docs/readme.md\ntest docs/t.rs\tt\nlimit 20\n");
    let (ok, log, result, _) = tree.mutate(&[]);
    assert!(ok, "{log:?}");
    assert_eq!(result, ["d/z not-judged"]);
}

#[test]
fn the_pass_ends_with_coverage_copied_to_the_land_dir_and_fails_with_it() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(RUST_BLOCK);
    tree.report("diff-caught", "1");
    let land = tree.dir.path().join("land");
    fs::create_dir_all(&land).unwrap();
    let (ok, log, _, stderr) = tree.mutate(&[("CLM_LAND_DIR", land.to_str().unwrap())]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(
        fs::read_to_string(land.join("lcov.info")).unwrap(),
        "SF:x\n"
    );
    let (ok, _, result, _) = tree.mutate(&[("COVERAGE_FAIL", "1")]);
    assert!(!ok);
    assert_eq!(result, ["a/x killed"]);
}

#[test]
fn the_mutation_tools_are_pinned_and_installed_when_absent() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(RUST_BLOCK);
    tree.report("diff-caught", "1");
    let (_, log, _, _) = tree.mutate(&[]);
    let install = log
        .iter()
        .find(|l| l.starts_with("cargo install "))
        .unwrap_or_else(|| panic!("{log:?}"));
    let words: Vec<&str> = install.split_whitespace().collect();
    assert!(
        words.contains(&"--locked") && words.contains(&"cargo-mutants"),
        "{install}"
    );
    let version = words[words.iter().position(|w| *w == "--version").unwrap() + 1];
    assert!(
        version.split('.').count() == 3
            && version
                .split('.')
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())),
        "{install}"
    );
    // `cargo-llvm-cov`, which finds the lines the tests run, at the version `coverage.sh` pins,
    // installed as `coverage.sh` installs it.
    let pin = script_lines("coverage.sh")
        .into_iter()
        .find(|l| l.starts_with("cargo install ") && l.ends_with(" cargo-llvm-cov"))
        .expect("coverage.sh installs cargo-llvm-cov");
    let llvm_cov = pin.split_whitespace().nth(4).unwrap().to_owned();
    assert!(log.contains(&pin), "no `{pin}` in {log:?}");
    let rustup = log
        .iter()
        .position(|l| l == "rustup component add llvm-tools-preview")
        .unwrap_or_else(|| panic!("{log:?}"));
    let cov = log
        .iter()
        .position(|l| l.starts_with("cargo llvm-cov nextest "))
        .unwrap_or_else(|| panic!("{log:?}"));
    assert!(rustup < cov, "{log:?}");
    // Present at the pinned versions, neither is installed again.
    tree.fake(
        "cargo",
        &CARGO_MUTANTS.replace(
            "*\" --version \"*) exit 0 ;;",
            &format!("*\" --version \"*) echo \"cargo-mutants {version}\"; exit 0 ;;"),
        ),
    );
    tree.report("llvm-cov-version", &format!("cargo-llvm-cov {llvm_cov}\n"));
    let (_, log, _, _) = tree.mutate(&[]);
    assert!(
        !log.iter()
            .any(|l| l.starts_with("cargo install ") || l.starts_with("rustup ")),
        "{log:?}"
    );
}

#[test]
fn the_mutation_script_is_in_no_land_check_or_workflow() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = Tree::new();
    executable(&tree.work().join(".github/scripts/mutate.sh"), FAKE);
    // Outside CI, as `clm land` runs it, and in CI, as the release workflow does.
    for ci in [None, Some("true")] {
        let (ok, log) = tree.run(ci, "");
        assert!(ok, "{ci:?}: {log:?}");
        assert!(
            !log.iter()
                .any(|l| l.contains("cargo mutants") || l.contains("stryker")),
            "{ci:?}: {log:?}"
        );
        assert!(
            !log.iter().any(|l| l.starts_with("mutate.sh")),
            "the stand-in ran ({ci:?}): {log:?}"
        );
    }
    for (file, job, step) in crate::workflow_steps() {
        let run = step["run"].as_str().unwrap_or("");
        for line in run.lines().filter(|l| !l.trim_start().starts_with('#')) {
            for word in ["mutate.sh", "cargo mutants", "stryker"] {
                assert!(!line.contains(word), "{file} `{job}`: {line}");
            }
        }
    }
}

#[test]
fn a_bad_environment_is_named_before_anything_runs() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(RUST_BLOCK);
    let result = tree.dir.path().join("result");
    let cases: [(&str, Option<&str>, &str); 5] = [
        ("CLM_MUTATE", None, "CLM_MUTATE:"),
        ("CLM_MUTATE", Some(""), "CLM_MUTATE:"),
        ("CLM_MUTATE_RESULT", None, "CLM_MUTATE_RESULT:"),
        ("CLM_MUTATE_RESULT", Some(""), "CLM_MUTATE_RESULT:"),
        (
            "CLM_SLOT",
            Some("x"),
            "mutate.sh: CLM_SLOT is not a whole number: x",
        ),
    ];
    for (var, value, said) in cases {
        fs::write(&result, "x/old killed\n").unwrap();
        let mut c = tree.mutate_command(&[]);
        match value {
            None => c.env_remove(var),
            Some(v) => c.env(var, v),
        };
        let out = c.output().unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{var}={value:?}");
        assert!(stderr.contains(said), "{var}={value:?}: {stderr}");
        assert_eq!(tree.log_lines(""), Vec::<String>::new(), "{var}={value:?}");
        assert_eq!(
            tree.log_lines("result"),
            ["x/old killed"],
            "{var}={value:?}"
        );
    }
}

#[test]
fn a_result_file_from_before_is_emptied() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(RUST_BLOCK);
    tree.report("diff-caught", "1");
    tree.report(
        "list",
        "1 1 crates/clm-check/src/lib.rs:1:1: replace d with ()\n",
    );
    tree.report(
        "diff-list",
        "1 1 crates/clm-check/src/lib.rs:1:1: replace d with ()\n",
    );
    fs::write(tree.dir.path().join("result"), "x/old killed\n").unwrap();
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x killed"]);
}

#[test]
fn no_nextest_stops_it_before_the_first_rust_block() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(RUST_BLOCK);
    tree.fake(
        "cargo",
        &CARGO_MUTANTS.replace(
            "bin=$(dirname \"$0\")\n",
            "bin=$(dirname \"$0\")\n[ \"$1 $2\" != \"nextest --version\" ] || exit 1\n",
        ),
    );
    fs::write(tree.dir.path().join("result"), "x/old killed\n").unwrap();
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(!ok, "{log:?}");
    assert!(stderr.contains("cargo-nextest"), "{stderr}");
    assert!(
        !log.iter().any(|l| l.starts_with("cargo mutants")),
        "{log:?}"
    );
    assert_eq!(result, Vec::<String>::new());
}

#[test]
fn vitest_tests_that_notice_nothing_survive() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(
        "requirement p/q\ncode plugin-panel/src/a.ts\ntest plugin-panel/test/a.test.ts\tt\nlimit 20\n",
    );
    tree.report("stryker-survived", "2");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["p/q survived 2"]);
    assert!(
        log.iter()
            .any(|l| l.starts_with("plugin-panel$ npx stryker run ")),
        "{log:?}"
    );
    let config = fs::read_to_string(tree.dir.path().join("log.stryker")).unwrap();
    assert!(
        config.contains("\"mutate\": [\"src/a.ts:1-20\"]"),
        "{config}"
    );
}

#[test]
fn stryker_mutates_no_test_file() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree("");
    let diff = tree.dir.path().join("the.diff");
    fs::write(
        &diff,
        "--- a/plugin-panel/src/a.ts\n+++ b/plugin-panel/src/a.ts\n@@ -3,0 +4,2 @@\n+x\n+y\n\
         --- a/plugin-panel/test/a.test.ts\n+++ b/plugin-panel/test/a.test.ts\n@@ -1,0 +2,1 @@\n+z\n",
    )
    .unwrap();
    let plan = format!(
        "requirement p/q\ncode plugin-panel/src/a.ts\ncode plugin-panel/test/a.test.ts\ndiff {}\n\
         test plugin-panel/test/a.test.ts\tt\nlimit 20\n",
        diff.display()
    );
    fs::write(tree.dir.path().join("plan"), plan).unwrap();
    tree.report("stryker-killed", "1");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["p/q killed"]);
    let config = fs::read_to_string(tree.dir.path().join("log.stryker")).unwrap();
    let mutate = config
        .lines()
        .find(|l| l.contains("\"mutate\":"))
        .unwrap_or_else(|| panic!("{config}"));
    let ranges: Vec<&str> = mutate.split('"').skip(3).step_by(2).collect();
    assert!(!ranges.is_empty(), "{config}");
    assert!(
        ranges.iter().all(|r| r.starts_with("src/a.ts:")),
        "{config}"
    );
    // Nor the further mutants' run, which a block with no diff makes alone.
    let tree = mutate_tree(
        "requirement p/q\ncode plugin-panel/src/a.ts\ncode plugin-panel/test/a.test.ts\n\
         test plugin-panel/test/a.test.ts\tt\nlimit 20\n",
    );
    tree.report("stryker-killed", "1");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["p/q killed"]);
    let config = fs::read_to_string(tree.dir.path().join("log.stryker")).unwrap();
    assert!(
        config.contains("\"mutate\": [\"src/a.ts:1-20\"]"),
        "{config}"
    );
    // A vitest test file is neither kind of code file: a block naming only one, `.tsx` too,
    // runs no StrykerJS and is not judged.
    let tree = mutate_tree(
        "requirement p/r\ncode web/src/a.test.tsx\ntest web/src/a.test.tsx\tt\nlimit 20\n",
    );
    tree.report("stryker-killed", "1");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["p/r not-judged"]);
    assert!(
        !log.iter().any(|l| l.contains("npx stryker run")),
        "{log:?}"
    );
}

#[test]
fn a_tool_that_fails_leaves_no_verdict() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(RUST_BLOCK);
    tree.report("diff-caught", "1");
    tree.report("mutants-exit", "4");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(!ok, "{log:?}");
    assert!(
        stderr.contains("cargo mutants") && stderr.contains("a/x"),
        "{stderr}"
    );
    assert!(result.is_empty(), "{result:?}");
    // A status of 2 or 3 is a verdict, not a failure.
    fs::remove_file(tree.dir.path().join("bin/mutants-exit")).unwrap();
    let (ok, _, result, _) = tree.mutate(&[]);
    assert!(ok);
    assert_eq!(result, ["a/x killed"]);
}

/// A block for a requirement of `web/src/` with one vitest test and no diff.
const WEB_BLOCK: &str =
    "requirement w/y\ncode web/src/app.ts\ntest web/test/app.test.ts\tpaints\nlimit 20\n";

#[test]
fn stryker_that_runs_no_test_leaves_no_verdict() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(WEB_BLOCK);
    tree.report("stryker-exit", "1");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(!ok, "{log:?}");
    assert!(
        stderr.contains("StrykerJS") && stderr.contains("w/y"),
        "{stderr}"
    );
    assert!(result.is_empty(), "{result:?}");
}

#[test]
fn the_fixture_is_judged_first_outside_the_repository() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(RUST_BLOCK);
    tree.report("diff-caught", "1");
    let before = snapshot(&tree.work());
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x killed"]);
    // The runs of the crate's mutants: the listing of its test file's goes apart
    // (`the_fixtures_test_file_is_judged_with_its_every_mutant_in_a_copy_made_as_the_tree_is`).
    let runs: Vec<&String> = log
        .iter()
        .filter(|l| {
            l.starts_with("cargo mutants ")
                && !l.ends_with(" --version")
                && !l.contains("--Zmutate-file")
        })
        .collect();
    let first_block = runs
        .iter()
        .position(|l| l.contains("crates/clm-check/src/lib.rs"))
        .unwrap_or_else(|| panic!("{log:?}"));
    // Two fixture runs, the test that notices and the one that doesn't, before the block's.
    assert_eq!(first_block, 2, "{runs:?}");
    assert!(runs[0].contains("fixture_notices") && runs[1].contains("fixture_blind"));
    // The fixture is made as the tree is: a repository whose `.git` is a directory of its own,
    // borrowing another repository's objects, in which git sees no change to a tracked file
    // that a mutant changed. The tool copies that `.git`, and the fixture's tests ask git for
    // its top and its status: a tool that left `.git` out of its copy, or a copy in which git
    // sees the mutant, would fail the fixture.
    for run in &runs[..2] {
        assert!(run.contains("--copy-vcs true"), "{run}");
    }
    let alternates =
        fs::read_to_string(tree.dir.path().join("log.fixture-alternates")).unwrap_or_default();
    let store = alternates.trim();
    assert!(
        Path::new(store).is_absolute() && store.ends_with("/objects"),
        "alternates: {alternates:?}"
    );
    let status = fs::read_to_string(tree.dir.path().join("log.fixture-status")).unwrap();
    assert_eq!(status, "", "git saw the fixture's mutant");
    let src = fs::read_to_string(tree.dir.path().join("log.fixture-src")).unwrap();
    assert!(
        src.contains("rev-parse")
            && src.contains("--show-toplevel")
            && src.contains("status.success()")
            && src.contains("CARGO_MANIFEST_DIR")
            && src.contains("assert_eq!(std::fs::canonicalize(top.trim()).unwrap(), here)")
            && src.matches("in_git_repository();").count() == 2
            && src.contains("\"--porcelain\", \"--untracked-files=no\"")
            && src.contains("assert_eq!(String::from_utf8(status.stdout).unwrap(), \"\")")
            && src.matches("tracked_files_unchanged();").count() == 1,
        "{src}"
    );
    // The blind test, not the one that notices, fails when git sees the mutant.
    let blind = src.split("fn fixture_blind()").nth(1).unwrap_or_default();
    let blind = &blind[..blind.find("\n    }").unwrap_or(blind.len())];
    assert!(blind.contains("tracked_files_unchanged();"), "{src}");
    let dirs = fs::read_to_string(tree.dir.path().join("log.cwd")).unwrap();
    let with_git = fs::read_to_string(tree.dir.path().join("log.git")).unwrap_or_default();
    for dir in dirs.lines().take(2) {
        assert!(with_git.lines().any(|d| d == dir), "no .git in {dir}");
        assert!(
            !store.starts_with(&format!("{dir}/")),
            "{dir} holds the store it borrows"
        );
    }
    // Every run, the fixture's and the block's, is outside the checkout, and the block's are in
    // one tree.
    let work = fs::canonicalize(tree.work()).unwrap();
    for dir in dirs.lines() {
        let dir = fs::canonicalize(dir).unwrap_or_else(|_| PathBuf::from(dir));
        assert!(!dir.starts_with(&work), "{dir:?} is in the repository");
    }
    let block: Vec<&str> = dirs.lines().skip(2).collect();
    assert!(
        !block.is_empty() && block.iter().all(|d| *d == block[0]),
        "{dirs}"
    );
    assert_eq!(snapshot(&tree.work()), before);
}

#[test]
fn the_tree_and_the_fixture_are_made_once_for_every_rust_block() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(&format!("{RUST_BLOCK}{}", RUST_BLOCK.replace("a/x", "a/y")));
    tree.report("diff-caught", "1");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x killed", "a/y killed"]);
    let dirs = fs::read_to_string(tree.dir.path().join("log.cwd")).unwrap();
    let block: Vec<&str> = dirs.lines().skip(2).collect();
    // Each block lists its mutants, those of its diff, and tries the diff's.
    assert_eq!(block.len(), 6, "{dirs}");
    assert!(block.iter().all(|d| *d == block[0]), "{dirs}");
    assert_eq!(
        log.iter()
            .filter(|l| l.starts_with("cargo mutants ") && l.contains("fixture_"))
            .count(),
        2,
        "{log:?}"
    );
}

#[test]
fn the_tree_the_tool_copies_has_a_git_directory_of_its_own() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(RUST_BLOCK);
    tree.report("diff-caught", "1");
    // The checkout is a linked worktree of a repository with a branch and a remote-tracking
    // branch, as the verify tree is.
    let main = tree.dir.path().join("main");
    fs::rename(tree.work(), &main).unwrap();
    let first = git(&main, &["rev-parse", "HEAD"]);
    fs::write(main.join("second.txt"), "2\n").unwrap();
    git(&main, &["add", "second.txt"]);
    git(&main, &["commit", "-q", "-m", "second"]);
    let second = git(&main, &["rev-parse", "HEAD"]);
    git(&main, &["branch", "side", &first]);
    git(&main, &["update-ref", "refs/remotes/origin/main", &first]);
    git(&main, &["gc", "-q"]);
    let work = tree.work();
    git(
        &main,
        &["worktree", "add", "-q", "--detach", work.to_str().unwrap()],
    );
    // Its `HEAD` is a commit of its own, which no branch names.
    fs::write(work.join("third.txt"), "3\n").unwrap();
    git(&work, &["add", "third.txt"]);
    git(&work, &["commit", "-q", "-m", "third"]);
    let third = git(&work, &["rev-parse", "HEAD"]);
    let index = git(&work, &["rev-parse", "--git-path", "index"]);
    let index = work.join(index);
    let index_before = fs::read(&index).unwrap();
    // A caller's git directory and index, and a user's default remote name, change nothing.
    let config = tree.dir.path().join("gitconfig");
    fs::write(&config, "[clone]\n\tdefaultRemoteName = upstream\n").unwrap();
    let main_git = main.join(".git");
    let (ok, log, result, stderr) = tree.mutate(&[
        ("GIT_DIR", main_git.to_str().unwrap()),
        ("GIT_INDEX_FILE", index.to_str().unwrap()),
        ("GIT_CONFIG_GLOBAL", config.to_str().unwrap()),
    ]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x killed"]);
    // The directory the tool ran in for the block, kept by the fake.
    let dirs = fs::read_to_string(tree.dir.path().join("log.cwd")).unwrap();
    let ran = Path::new(dirs.lines().nth(2).unwrap_or_else(|| panic!("{dirs}")));
    let real = fs::canonicalize(&work).unwrap();
    assert!(
        !fs::canonicalize(ran)
            .unwrap_or_else(|_| ran.to_path_buf())
            .starts_with(&real),
        "{ran:?} is in the checkout"
    );
    let copy = tree.dir.path().join("log.tree");
    assert!(copy.join(".git").is_dir(), "no .git directory in {ran:?}");
    let packs: Vec<_> = fs::read_dir(copy.join(".git/objects/pack"))
        .map(|d| {
            d.flatten()
                .filter(|e| e.path().extension().is_some_and(|x| x == "pack"))
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default();
    assert!(packs.is_empty(), "{packs:?}");
    // Nor any loose object: the commits it names are all in the checkout's store.
    let loose: Vec<_> = fs::read_dir(copy.join(".git/objects"))
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().len() == 2 && e.path().is_dir())
        .filter(|e| fs::read_dir(e.path()).unwrap().next().is_some())
        .map(|e| e.path())
        .collect();
    assert!(loose.is_empty(), "{loose:?}");
    assert_eq!(git(&copy, &["rev-parse", "HEAD"]), third);
    assert_eq!(git(&copy, &["rev-parse", "refs/heads/side"]), first);
    assert_eq!(
        git(&copy, &["rev-parse", "refs/remotes/origin/main"]),
        first
    );
    assert_eq!(git(&copy, &["rev-parse", "refs/heads/main"]), second);
    // A mutant written into a tracked file leaves git there reporting no change.
    fs::write(copy.join("third.txt"), "a mutant\n").unwrap();
    assert_eq!(
        git(&copy, &["status", "--porcelain", "--untracked-files=no"]),
        ""
    );
    assert_eq!(fs::read(&index).unwrap(), index_before);
}

#[test]
fn a_tool_that_judges_its_fixture_wrongly_judges_nothing() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(RUST_BLOCK);
    tree.report("diff-caught", "1");
    tree.report("fixture-catches-all", "");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(!ok, "{log:?}");
    assert!(
        stderr.contains("cargo mutants") && stderr.contains("fixture"),
        "{stderr}"
    );
    assert!(
        !log.iter()
            .any(|l| l.contains("crates/clm-check/src/lib.rs")),
        "{log:?}"
    );
    assert!(result.is_empty(), "{result:?}");
    // Nor does one that misses the mutants of the test that notices.
    fs::remove_file(tree.dir.path().join("bin/fixture-catches-all")).unwrap();
    tree.report("fixture-catches-none", "");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(!ok, "{log:?}");
    assert!(stderr.contains("fixture"), "{stderr}");
    assert!(result.is_empty(), "{result:?}");
}

#[test]
fn stryker_is_judged_on_a_fixture_in_the_directory_before_its_first_block() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(WEB_BLOCK);
    tree.report("stryker-killed", "1");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["w/y killed"]);
    let runs: Vec<&String> = log
        .iter()
        .filter(|l| l.starts_with("web$ npx stryker run "))
        .collect();
    // Once for each function of the fixture and once for its test file that imports nothing,
    // then the block; the fixture is gone afterwards.
    assert_eq!(runs.len(), 4, "{log:?}");
    assert!(!tree.work().join("web/clm-mutate-fixture").exists());
    // A tool that kills the blind test's mutants judged the fixture wrongly.
    tree.report("stryker-fixture-kills-all", "");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(!ok, "{log:?}");
    assert!(
        stderr.contains("StrykerJS") && stderr.contains("fixture"),
        "{stderr}"
    );
    assert!(result.is_empty(), "{result:?}");
    assert!(!tree.work().join("web/clm-mutate-fixture").exists());
    // Nor does one that leaves the mutants of the test that notices alive.
    fs::remove_file(tree.dir.path().join("bin/stryker-fixture-kills-all")).unwrap();
    tree.report("stryker-fixture-kills-none", "");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(!ok, "{log:?}");
    assert!(
        stderr.contains("StrykerJS") && stderr.contains("fixture"),
        "{stderr}"
    );
    assert!(result.is_empty(), "{result:?}");
}

#[test]
fn a_typescript_test_reads_a_file_of_the_repository() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(WEB_BLOCK);
    // A dot entry, and one git doesn't track, are entries of the top as much as the rest.
    fs::write(tree.work().join(".hidden"), "").unwrap();
    fs::create_dir_all(tree.work().join("target/debug")).unwrap();
    tree.report("stryker-killed", "1");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["w/y killed"]);
    let lines = tree.log_lines("tempdir");
    let (dir, entries) = lines.split_first().unwrap_or_else(|| panic!("{log:?}"));
    let dir = Path::new(dir);
    // The sandbox's directory stands in `web/`, so the sandbox stands where `web/` does.
    assert_eq!(
        fs::canonicalize(dir.parent().unwrap()).unwrap(),
        fs::canonicalize(tree.work().join("web")).unwrap(),
        "{}",
        dir.display()
    );
    let mut got: Vec<(String, PathBuf)> = entries
        .iter()
        .map(|l| {
            let (name, target) = l.split_once(' ').unwrap();
            let target = fs::canonicalize(target)
                .unwrap_or_else(|_| panic!("{name} is no link to an entry: {target}"));
            (name.to_owned(), target)
        })
        .collect();
    let mut want: Vec<(String, PathBuf)> = fs::read_dir(tree.work())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n != ".git" && n != "web")
        .map(|n| {
            let path = fs::canonicalize(tree.work().join(&n)).unwrap();
            (n, path)
        })
        .collect();
    got.sort();
    want.sort();
    assert!(want.iter().any(|(n, _)| n == ".github"), "{want:?}");
    assert_eq!(got, want);
    // Gone once StrykerJS has returned, so no later run finds the links: the fixture's runs'
    // were gone by the time the block's ran, and the block's once the script ends.
    assert!(tree.dir.path().join("log.stale-tempdirs").exists());
    assert_eq!(tree.log_lines("stale-tempdirs"), Vec::<String>::new());
    // Nor does `coverage.sh`, which runs after the block's.
    assert!(tree.dir.path().join("log.coverage-tempdirs").exists());
    assert_eq!(tree.log_lines("coverage-tempdirs"), Vec::<String>::new());
    assert!(!dir.exists(), "{}", dir.display());
    assert!(
        !fs::read_dir(tree.work().join("web")).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".stryker-tmp")),
        "{log:?}"
    );
}

#[test]
fn the_stryker_fixture_reads_a_file_of_the_repository() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(WEB_BLOCK);
    tree.report("stryker-killed", "1");
    let (ok, log, _, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    let test = fs::read_to_string(tree.dir.path().join("log.fixture-test")).unwrap();
    let (notices, _) = test
        .split_once("test('fixture_blind'")
        .unwrap_or_else(|| panic!("{test}"));
    // The test that notices a change to its function also reads a file by a path relative to
    // its own file, which lies in the repository outside `web/`.
    assert!(
        notices.contains("readFileSync(new URL(") && notices.contains("import.meta.url"),
        "{test}"
    );
    let rel = notices
        .split("new URL('")
        .nth(1)
        .and_then(|r| r.split('\'').next())
        .unwrap_or_else(|| panic!("{test}"));
    let mut target = tree.work().join("web/clm-mutate-fixture");
    for part in Path::new(rel).components() {
        match part {
            std::path::Component::ParentDir => {
                target.pop();
            }
            p => target.push(p),
        }
    }
    assert!(
        target.starts_with(tree.work()) && !target.starts_with(tree.work().join("web")),
        "{rel} reaches {}",
        target.display()
    );
    assert!(target.is_file(), "{rel} reaches {}", target.display());
}

#[test]
fn the_stryker_fixture_fails_on_what_git_ignores() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(WEB_BLOCK);
    fs::create_dir_all(tree.work().join("web/site")).unwrap();
    fs::write(tree.work().join("web/site/.gitignore"), ".test-dist/\n").unwrap();
    tree.report("stryker-killed", "1");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["w/y killed"]);
    let ignore = fs::read_to_string(tree.dir.path().join("log.fixture-gitignore"))
        .unwrap_or_else(|_| panic!("the fixture held no .gitignore: {log:?}"));
    let dirs = tree.log_lines("fixture-dirs");
    // A rule with no slash but a trailing one, naming a directory one level below the fixture's
    // files that the fixture's directory held.
    let (name, below) = ignore
        .lines()
        .filter_map(|r| r.strip_suffix('/'))
        .filter(|n| !n.is_empty() && !n.contains('/'))
        .find_map(|n| {
            dirs.iter()
                .filter_map(|d| d.strip_prefix("clm-mutate-fixture/"))
                .find(|d| d.split('/').count() == 2 && d.ends_with(&format!("/{n}")))
                .map(|d| (n.to_owned(), d.to_owned()))
        })
        .unwrap_or_else(|| panic!("{ignore:?} {dirs:?}"));
    // The run's configuration carries the rule, so StrykerJS leaves that directory out.
    let config: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(tree.dir.path().join("log.fixture-stryker")).unwrap(),
    )
    .unwrap();
    let web = TempDir::new().unwrap();
    git(web.path(), &["init", "-q"]);
    let patterns: Vec<&str> = config["ignorePatterns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    fs::write(web.path().join(".gitignore"), patterns.join("\n") + "\n").unwrap();
    fs::create_dir_all(web.path().join("clm-mutate-fixture").join(&below)).unwrap();
    assert!(
        git_ignores(web.path(), &format!("clm-mutate-fixture/{below}")),
        "{patterns:?}"
    );
    // The test that notices a change to its function also fails when it finds that directory,
    // by a path relative to its own file.
    let test = fs::read_to_string(tree.dir.path().join("log.fixture-test")).unwrap();
    let (notices, _) = test
        .split_once("test('fixture_blind'")
        .unwrap_or_else(|| panic!("{test}"));
    let found = notices
        .split("existsSync(new URL('")
        .skip(1)
        .filter_map(|r| r.split_once("', import.meta.url))"))
        .any(|(rel, rest)| {
            Path::new("clm-mutate-fixture")
                .join(rel)
                .components()
                .collect::<PathBuf>()
                == Path::new("clm-mutate-fixture").join(&below)
                && rest.starts_with(").toBe(false)")
        });
    assert!(found, "{name}: {test}");
}

/// Whether git's own matcher, in the repository `dir`, takes `path` as ignored by the
/// `.gitignore` files there (`git check-ignore --no-index`).
fn git_ignores(dir: &Path, path: &str) -> bool {
    let mut c = Command::new("git");
    c.current_dir(dir)
        .args(["check-ignore", "--no-index", "-q", path]);
    without_git_env(&mut c);
    match c.status().unwrap().code() {
        Some(0) => true,
        Some(1) => false,
        other => panic!("git check-ignore {path}: {other:?}"),
    }
}

#[test]
fn stryker_copies_nothing_git_ignores() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(WEB_BLOCK);
    let work = tree.work();
    // The top's rules: one anchored under another directory, one anchored under `web/`, and
    // unanchored ones that reach any depth, one led by `**/` and one ending in a carriage return.
    fs::write(
        work.join(".gitignore"),
        "/target/\nweb/node_modules/\n.wrangler/\ncrates/x/build/\n# a comment\n\n.clm/   \n*.tmp\n**/clm-any/\ncrlf-dir/\r\n*/clm-glob/\n/w?b/clm-q/\n/[wx]eb/clm-class/\nweb/**/clm-deep/\n",
    )
    .unwrap();
    // The site's own, untracked, as git reads it too, whose negations come after the top's
    // rules and its own, and whose rules hold a quote, an escape and an escaped trailing space;
    // and one inside an ignored directory, which git never reads.
    fs::create_dir_all(work.join("web/site")).unwrap();
    fs::write(
        work.join("web/site/.gitignore"),
        ".test-dist/\n.vitepress/cache/\n*.log\n!keep.log\n!keep.tmp\nquote\"d/\nback\\#slash/\nsp\\ \n",
    )
    .unwrap();
    // Nor one in another run's StrykerJS sandbox, which may vanish while it is read.
    fs::create_dir_all(work.join("web/.stryker-tmp-clm-old/x")).unwrap();
    fs::write(
        work.join("web/.stryker-tmp-clm-old/x/.gitignore"),
        "clm-sandbox-rule/\n",
    )
    .unwrap();
    fs::create_dir_all(work.join("web/node_modules/pkg")).unwrap();
    fs::write(
        work.join("web/node_modules/pkg/.gitignore"),
        "clm-not-read/\n",
    )
    .unwrap();
    git(&work, &["add", ".gitignore"]);
    git(&work, &["commit", "-q", "-m", "ignores"]);
    assert!(!work.join("web/site/.test-dist").exists());
    tree.report("stryker-killed", "1");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["w/y killed"]);
    let config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(tree.dir.path().join("log.stryker")).unwrap())
            .unwrap();
    let patterns: Vec<&str> = config["ignorePatterns"]
        .as_array()
        .unwrap_or_else(|| panic!("{config}"))
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    // The script's own sandboxes, then what it carried from the `.gitignore` files, each
    // anchored, since StrykerJS matches a rule with no slash but a trailing one only at the top.
    let (own, carried) = patterns.split_first().unwrap_or_else(|| panic!("{config}"));
    assert_eq!(*own, ".stryker-tmp-clm-*");
    assert!(!carried.is_empty(), "{patterns:?}");
    for p in carried {
        assert!(
            p.strip_prefix('!').unwrap_or(p).starts_with('/'),
            "{p} in {patterns:?}"
        );
        assert!(
            !p.contains("clm-not-read") && !p.contains("clm-sandbox-rule"),
            "{patterns:?}"
        );
    }
    // Read as `.gitignore` rules from `web/`, by git itself.
    let web = TempDir::new().unwrap();
    git(web.path(), &["init", "-q"]);
    fs::write(web.path().join(".gitignore"), carried.join("\n") + "\n").unwrap();
    for d in [
        "site/.test-dist/drift-src/docs",
        "src/deep/.wrangler",
        ".wrangler",
        "site/.clm",
        "node_modules",
        "site/.vitepress/cache",
        "src/.test-dist",
        "src/.vitepress/cache",
        "src/clm-any",
        "src/crlf-dir",
        "site/quote\"d",
        "site/back#slash",
        "clm-glob",
        "clm-q",
        "clm-class",
        "src/a/clm-deep",
        "target",
        "crates/x/build",
        "src",
        "test",
    ] {
        fs::create_dir_all(web.path().join(d)).unwrap();
    }
    for ignored in [
        "site/.test-dist/drift-src/docs",
        "src/deep/.wrangler",
        ".wrangler",
        "site/.clm",
        "site/.vitepress/cache",
        "node_modules",
        "src/clm-any",
        "src/crlf-dir",
        "site/x.log",
        "site/x.tmp",
        "src/x.tmp",
        "site/quote\"d",
        "site/back#slash",
        "site/sp ",
        "clm-glob",
        "clm-q",
        "clm-class",
        "src/a/clm-deep",
    ] {
        assert!(git_ignores(web.path(), ignored), "{ignored}: {patterns:?}");
    }
    // A sub-directory's rule reaches no further than its own directory, and a later file's
    // negation wins.
    for kept in [
        "src/app.ts",
        "test/app.test.ts",
        "target",
        "crates/x/build",
        "src/.test-dist",
        "src/.vitepress/cache",
        "src/x.log",
        "site/keep.log",
        "site/keep.tmp",
        "site/sp",
    ] {
        assert!(!git_ignores(web.path(), kept), "{kept}: {patterns:?}");
    }
}

/// A Rust block with no diff and one test, whose file has two mutants, one on line 3 alone and
/// one on line 30 alone, and whose tests the fake `cargo llvm-cov` reports running `run`.
fn lines_tree(run: &str) -> Tree {
    let tree = mutate_tree(
        "requirement a/x\ncode crates/clm-check/src/lib.rs\ntest crates/clm-check/tests/it.rs\tproves_x\nlimit 20\n",
    );
    tree.report(
        "list",
        "3 3 crates/clm-check/src/lib.rs:3:5: replace on_three with ()\n30 30 crates/clm-check/src/lib.rs:30:5: replace on_thirty with ()\n",
    );
    tree.report("lcov-run", run);
    tree
}

/// The `cargo mutants` calls of `log` that try mutants of the block's file.
fn block_tries(log: &[String]) -> Vec<&String> {
    log.iter()
        .filter(|l| {
            l.starts_with("cargo mutants ")
                && l.contains("--file crates/clm-check/src/lib.rs")
                && !l.contains(" --list")
        })
        .collect()
}

#[test]
fn a_mutant_the_tests_never_run_is_not_tried() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = lines_tree("3\n");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 1"]);
    // The lines are learnt from the block's tests alone, under the block's filterset, over its
    // test packages, before any mutant is tried.
    let cov = log
        .iter()
        .position(|l| l.starts_with("cargo llvm-cov nextest "))
        .unwrap_or_else(|| panic!("{log:?}"));
    for part in [
        "--no-rustc-wrapper",
        "--no-tests=pass",
        "--lcov",
        "--package clm-check",
        "binary_id(=clm-check::it)",
        "test(~proves_x)",
    ] {
        assert!(log[cov].contains(part), "no {part} in {}", log[cov]);
    }
    assert!(!log[cov].contains("--workspace"), "{}", log[cov]);
    // After the fixture has been judged.
    let fixture = log
        .iter()
        .rposition(|l| l.starts_with("cargo mutants ") && l.contains("fixture_"))
        .unwrap_or_else(|| panic!("{log:?}"));
    assert!(fixture < cov, "{log:?}");
    let tries = block_tries(&log);
    assert_eq!(tries.len(), 1, "{log:?}");
    assert!(
        tries[0].contains("replace on_three") && !tries[0].contains("replace on_thirty"),
        "{}",
        tries[0]
    );
    let first_try = log.iter().position(|l| l == tries[0]).unwrap();
    assert!(cov < first_try, "{log:?}");
}

#[test]
fn a_mutant_is_tried_when_any_line_of_its_span_runs() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = lines_tree("12\n");
    tree.report(
        "list",
        "10 14 crates/clm-check/src/lib.rs:10:5: replace spanning with ()\n20 20 crates/clm-check/src/lib.rs:20:5: replace elsewhere with ()\n",
    );
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 1"]);
    let tries = block_tries(&log);
    assert_eq!(tries.len(), 1, "{log:?}");
    assert!(
        tries[0].contains("replace spanning") && !tries[0].contains("replace elsewhere"),
        "{}",
        tries[0]
    );
}

#[test]
fn the_limit_counts_only_mutants_the_script_may_try() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(
        "requirement a/x\ncode crates/clm-check/src/lib.rs\ntest crates/clm-check/tests/it.rs\tproves_x\nlimit 2\n",
    );
    tree.report(
        "list",
        "1 1 crates/clm-check/src/lib.rs:1:5: replace m1 with ()\n2 2 crates/clm-check/src/lib.rs:2:5: replace m2 with ()\n3 3 crates/clm-check/src/lib.rs:3:5: replace m3 with ()\n4 4 crates/clm-check/src/lib.rs:4:5: replace m4 with ()\n",
    );
    tree.report("lcov-run", "2\n3\n4\n");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 2"]);
    let tries = block_tries(&log);
    assert_eq!(tries.len(), 1, "{log:?}");
    let named: Vec<&str> = ["m1", "m2", "m3", "m4"]
        .into_iter()
        .filter(|m| tries[0].contains(&format!("replace {m} ")))
        .collect();
    assert_eq!(named, ["m2", "m3"], "{}", tries[0]);
}

#[test]
fn a_list_of_further_mutants_far_longer_than_the_limit_is_cut_to_it() {
    // req:ci/mutate-script@5f0ee0b6
    // Far more than a pipe holds, so a reader that stops at the limit would leave the writer
    // to die of SIGPIPE and end the script.
    let pad = "x".repeat(300);
    let list: String = (0..600)
        .map(|i| format!("1 1 crates/clm-check/src/lib.rs:1:5: replace m{i}_{pad} with ()\n"))
        .collect();
    let tree = mutate_tree(
        "requirement a/x\ncode crates/clm-check/src/lib.rs\ntest crates/clm-check/tests/it.rs\tproves_x\nlimit 2\n",
    );
    tree.report("list", &list);
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 2"]);
    let tries = block_tries(&log);
    assert_eq!(tries.len(), 1, "{log:?}");
    assert_eq!(tries[0].matches("--re ").count(), 2, "{}", tries[0]);
    assert!(
        tries[0].contains("replace m0_") && tries[0].contains("replace m1_"),
        "{}",
        tries[0]
    );
}

/// `blocks` Rust blocks with no diff, `a/x`, `a/y`..., each with one test of `clm-check` and
/// `limit`, whose file has a mutant on line 3 alone and one on line 30 alone; the block's tests
/// run each line once, and every test of the package line 3 ten times and line 30 once.
fn share_tree(blocks: &[&str], limit: u32) -> Tree {
    let plan: String = blocks
        .iter()
        .map(|b| {
            format!(
                "requirement a/{b}\ncode crates/clm-check/src/lib.rs\ntest crates/clm-check/tests/it.rs\tproves_{b}\nlimit {limit}\n"
            )
        })
        .collect();
    let tree = mutate_tree(&plan);
    tree.report(
        "list",
        "3 3 crates/clm-check/src/lib.rs:3:5: replace on_three with ()\n30 30 crates/clm-check/src/lib.rs:30:5: replace on_thirty with ()\n",
    );
    tree.report("lcov-run", "3\n30\n");
    tree.report("lcov-run-all", "3 10\n30 1\n");
    tree
}

/// The `cargo llvm-cov` runs of `log` with no filterset: runs of every test of a package.
fn package_runs(log: &[String]) -> Vec<&String> {
    log.iter()
        .filter(|l| l.starts_with("cargo llvm-cov nextest ") && !l.contains(" -E "))
        .collect()
}

#[test]
fn the_further_mutants_of_the_requirements_own_code_come_first() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = share_tree(&["x"], 1);
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 1"]);
    let runs = package_runs(&log);
    assert_eq!(runs.len(), 1, "{log:?}");
    for part in [
        "--no-rustc-wrapper",
        "--no-tests=pass",
        "--ignore-run-fail",
        "--lcov",
        "--package clm-check",
    ] {
        assert!(runs[0].contains(part), "no {part} in {}", runs[0]);
    }
    // After the run that finds the lines the block's tests run.
    let cov = log
        .iter()
        .position(|l| l.starts_with("cargo llvm-cov nextest ") && l.contains(" -E "))
        .unwrap_or_else(|| panic!("{log:?}"));
    let all = log.iter().position(|l| l == runs[0]).unwrap();
    assert!(cov < all, "{log:?}");
    let tries = block_tries(&log);
    assert_eq!(tries.len(), 1, "{log:?}");
    assert!(
        tries[0].contains("replace on_thirty") && !tries[0].contains("replace on_three"),
        "{}",
        tries[0]
    );
}

#[test]
fn a_mutants_share_is_its_best_lines_capped_at_one_and_ties_keep_the_list_order() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = share_tree(&["x"], 3);
    // In list order: `low` (share 0.1); `span`, whose line 5 has 0.25 and line 6 has 1;
    // `unrun`, whose line no test of the package runs (1); `even` (1); and `over`, whose line
    // the block's tests run twice and the package's once (2, so 1).
    tree.report(
        "list",
        "9 9 crates/clm-check/src/lib.rs:9:5: replace low with ()\n5 6 crates/clm-check/src/lib.rs:5:5: replace span with ()\n8 8 crates/clm-check/src/lib.rs:8:5: replace unrun with ()\n10 10 crates/clm-check/src/lib.rs:10:5: replace even with ()\n7 7 crates/clm-check/src/lib.rs:7:5: replace over with ()\n",
    );
    tree.report("lcov-run", "5\n6\n7 2\n8\n9\n10\n");
    tree.report("lcov-run-all", "5 4\n6 1\n7 1\n9 10\n10 1\n");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 3"]);
    let tries = block_tries(&log);
    assert_eq!(tries.len(), 1, "{log:?}");
    let named: Vec<&str> = ["low", "span", "unrun", "even", "over"]
        .into_iter()
        .filter(|m| tries[0].contains(&format!("replace {m} ")))
        .collect();
    assert_eq!(named, ["span", "unrun", "even"], "{}", tries[0]);
}

#[test]
fn no_more_further_mutants_than_the_limit_runs_no_package() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = share_tree(&["x"], 2);
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 2"]);
    assert!(package_runs(&log).is_empty(), "{log:?}");
    let tries = block_tries(&log);
    assert_eq!(tries.len(), 1, "{log:?}");
    assert!(
        tries[0].contains("replace on_thirty") && tries[0].contains("replace on_three"),
        "{}",
        tries[0]
    );
}

#[test]
fn a_packages_tests_run_once_in_a_pass() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = share_tree(&["x", "y"], 1);
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 1", "a/y survived 1"]);
    assert_eq!(package_runs(&log).len(), 1, "{log:?}");
    // The second block takes its order from the report the first kept.
    let tries = block_tries(&log);
    assert_eq!(tries.len(), 2, "{log:?}");
    for t in tries {
        assert!(
            t.contains("replace on_thirty") && !t.contains("replace on_three"),
            "{t}"
        );
    }
}

#[test]
fn the_run_of_a_packages_tests_failing_leaves_no_verdict() {
    // req:ci/mutate-script@5f0ee0b6
    for fake in ["llvm-cov-all-exit", "llvm-cov-all-no-report"] {
        let tree = share_tree(&["x"], 1);
        tree.report(fake, "1");
        let (ok, log, result, stderr) = tree.mutate(&[]);
        assert!(!ok, "{fake}: {log:?}");
        assert!(
            stderr.contains("cargo llvm-cov") && stderr.contains("a/x"),
            "{fake}: {stderr}"
        );
        assert!(result.is_empty(), "{fake}: {result:?}");
        assert!(block_tries(&log).is_empty(), "{fake}: {log:?}");
    }
}

#[test]
fn the_package_run_is_written_in_the_blocks_job_before_it_is_kept() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = share_tree(&["x"], 1);
    let (ok, log, _, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    let runs = package_runs(&log);
    assert_eq!(runs.len(), 1, "{log:?}");
    let path = runs[0]
        .split(" --output-path ")
        .nth(1)
        .and_then(|r| r.split(' ').next())
        .unwrap_or_else(|| panic!("{}", runs[0]));
    assert!(path.contains("/block-0/"), "{path}");
}

#[test]
fn a_lines_count_is_summed_over_the_runs_of_every_test_package() {
    // req:ci/mutate-script@5f0ee0b6
    // Tests of two packages, each package's run giving line 3 two runs and line 30 one: line 3,
    // run three times by the block's tests, has 3/4, and line 30, run twice, has 2/2. Read from
    // one package's run alone, both would have 1 and keep the list's order.
    let tree = share_tree(&["x"], 1);
    fs::write(
        tree.dir.path().join("plan"),
        "requirement a/x\ncode crates/clm-check/src/lib.rs\ntest crates/clm-check/tests/it.rs\tproves_x\ntest crates/clm-run/tests/it.rs\tproves_x\nlimit 1\n",
    )
    .unwrap();
    tree.report("lcov-run", "3 3\n30 2\n");
    tree.report("lcov-run-all", "3 2\n30 1\n");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 1"]);
    let runs = package_runs(&log);
    assert_eq!(runs.len(), 2, "{log:?}");
    assert!(
        runs.iter().any(|r| r.contains("--package clm-run")),
        "{log:?}"
    );
    let tries = block_tries(&log);
    assert_eq!(tries.len(), 1, "{log:?}");
    assert!(
        tries[0].contains("replace on_thirty") && !tries[0].contains("replace on_three"),
        "{}",
        tries[0]
    );
}

#[test]
fn a_block_caught_on_its_diff_runs_no_package() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = share_tree(&["x"], 1);
    fs::write(
        tree.dir.path().join("plan"),
        "requirement a/x\ncode crates/clm-check/src/lib.rs\ndiff /tmp/the.diff\ntest crates/clm-check/tests/it.rs\tproves_x\nlimit 1\n",
    )
    .unwrap();
    tree.report(
        "list",
        "1 1 crates/clm-check/src/lib.rs:1:5: replace d with ()\n3 3 crates/clm-check/src/lib.rs:3:5: replace on_three with ()\n30 30 crates/clm-check/src/lib.rs:30:5: replace on_thirty with ()\n",
    );
    tree.report(
        "diff-list",
        "1 1 crates/clm-check/src/lib.rs:1:5: replace d with ()\n",
    );
    tree.report("lcov-run", "1\n3\n30\n");
    tree.report("diff-caught", "1");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x killed"]);
    assert!(package_runs(&log).is_empty(), "{log:?}");
}

#[test]
fn tests_that_run_none_of_the_code_survive_untried() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = lines_tree("");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 2"]);
    assert!(block_tries(&log).is_empty(), "{log:?}");
}

#[test]
fn tests_that_read_the_repositorys_files_and_run_none_of_the_code_are_not_judged() {
    // req:ci/mutate-script@5f0ee0b6
    // The block of `tests_that_run_none_of_the_code_count_the_diffs_mutants_and_at_most_limit_more`,
    // whose diff's mutant and further ones would survive, with a `reads` line.
    let tree = mutate_tree(&RUST_BLOCK.replace("limit 20", "reads specs/**\nlimit 20"));
    tree.report(
        "list",
        "1 1 crates/clm-check/src/lib.rs:1:5: replace d with ()\n3 3 crates/clm-check/src/lib.rs:3:5: replace m3 with ()\n4 4 crates/clm-check/src/lib.rs:4:5: replace m4 with ()\n",
    );
    tree.report(
        "diff-list",
        "1 1 crates/clm-check/src/lib.rs:1:5: replace d with ()\n",
    );
    tree.report("lcov-run", "");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x not-judged"]);
    assert!(block_tries(&log).is_empty(), "{log:?}");
    // A `reads` line changes nothing when the tests run a mutant's line: it is tried and judged.
    tree.report("lcov-run", "3\n");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(block_tries(&log).len(), 1, "{log:?}");
    assert_eq!(result, ["a/x survived 1"]);
    tree.report("further-caught", "1");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x killed"]);
}

#[test]
fn tests_that_run_the_code_but_no_mutant_of_it_are_not_judged() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = lines_tree("10\n");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x not-judged"]);
    assert!(block_tries(&log).is_empty(), "{log:?}");
}

#[test]
fn the_run_that_finds_the_lines_failing_leaves_no_verdict() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = lines_tree("3\n");
    tree.report("llvm-cov-exit", "1");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(!ok, "{log:?}");
    assert!(
        stderr.contains("cargo llvm-cov") && stderr.contains("a/x"),
        "{stderr}"
    );
    assert!(result.is_empty(), "{result:?}");
    assert!(block_tries(&log).is_empty(), "{log:?}");
    // Nor does one that ends well but writes no report.
    fs::remove_file(tree.dir.path().join("bin/llvm-cov-exit")).unwrap();
    tree.report("llvm-cov-no-report", "");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(!ok, "{log:?}");
    assert!(
        stderr.contains("cargo llvm-cov") && stderr.contains("a/x"),
        "{stderr}"
    );
    assert!(result.is_empty(), "{result:?}");
    assert!(block_tries(&log).is_empty(), "{log:?}");
}

#[test]
fn a_diff_mutant_the_tests_never_run_is_not_tried() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(RUST_BLOCK);
    tree.report(
        "list",
        "1 1 crates/clm-check/src/lib.rs:1:5: replace d1 with ()\n5 5 crates/clm-check/src/lib.rs:5:5: replace d5 with ()\n3 3 crates/clm-check/src/lib.rs:3:5: replace f3 with ()\n",
    );
    tree.report(
        "diff-list",
        "1 1 crates/clm-check/src/lib.rs:1:5: replace d1 with ()\n5 5 crates/clm-check/src/lib.rs:5:5: replace d5 with ()\n",
    );
    tree.report("lcov-run", "3\n5\n");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 2"]);
    let tries = block_tries(&log);
    assert_eq!(tries.len(), 2, "{log:?}");
    assert!(
        tries[0].contains("--in-diff")
            && tries[0].contains("replace d5")
            && !tries[0].contains("replace d1"),
        "{}",
        tries[0]
    );
    assert!(tries[1].contains("replace f3"), "{}", tries[1]);
    // With no diff mutant the tests run, none of the diff's is tried, and the further ones are.
    tree.report("lcov-run", "3\n");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 1"]);
    let tries = block_tries(&log);
    assert_eq!(tries.len(), 1, "{log:?}");
    assert!(
        !tries[0].contains("--in-diff") && tries[0].contains("replace f3"),
        "{}",
        tries[0]
    );
}

#[test]
fn lines_run_only_in_a_file_with_no_mutant_count_as_none_run() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(
        "requirement a/x\ncode crates/clm-check/src/lib.rs\ncode crates/clm-check/src/other.rs\ntest crates/clm-check/tests/it.rs\tproves_x\nlimit 20\n",
    );
    tree.report(
        "list",
        "3 3 crates/clm-check/src/lib.rs:3:5: replace on_three with ()\n",
    );
    tree.report("lcov-run", "crates/clm-check/src/other.rs:3\n");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 1"]);
    assert!(block_tries(&log).is_empty(), "{log:?}");
}

#[test]
fn the_lines_are_read_from_a_tree_reached_through_a_link() {
    // req:ci/mutate-script@5f0ee0b6
    // The tool writes real paths; the temporary directory the tree is made in is a link.
    let tree = lines_tree("3\n");
    let real = tree.dir.path().join("real-tmp");
    fs::create_dir_all(&real).unwrap();
    let link = tree.dir.path().join("linked-tmp");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let (ok, log, result, stderr) = tree.mutate(&[("TMPDIR", link.to_str().unwrap())]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 1"]);
    let cov = fs::read_to_string(tree.dir.path().join("log.cov-cwd")).unwrap();
    assert!(cov.starts_with(link.to_str().unwrap()), "{cov}");
}

#[test]
fn tests_that_run_none_of_the_code_count_the_diffs_mutants_and_at_most_limit_more() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(RUST_BLOCK.replace("limit 20", "limit 2").as_str());
    tree.report(
        "list",
        "1 1 crates/clm-check/src/lib.rs:1:5: replace d with ()\n2 2 crates/clm-check/src/lib.rs:2:5: replace m2 with ()\n3 3 crates/clm-check/src/lib.rs:3:5: replace m3 with ()\n4 4 crates/clm-check/src/lib.rs:4:5: replace m4 with ()\n",
    );
    tree.report(
        "diff-list",
        "1 1 crates/clm-check/src/lib.rs:1:5: replace d with ()\n",
    );
    tree.report("lcov-run", "");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    // The diff's one mutant and the first 2 of the 3 further ones.
    assert_eq!(result, ["a/x survived 3"]);
    assert!(block_tries(&log).is_empty(), "{log:?}");
}

#[test]
fn a_mutant_stryker_finds_no_test_running_is_neither_caught_nor_survived() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(WEB_BLOCK);
    tree.report("stryker-survived", "1");
    tree.report("stryker-nocov", "2");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["w/y survived 1"]);
}

#[test]
fn stryker_finding_no_test_running_any_mutant_counts_them_all_survived() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(WEB_BLOCK);
    tree.report("stryker-nocov", "2");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["w/y survived 2"]);
    // Not when it reports any other mutant, as one that didn't build.
    tree.report("stryker-compile-error", "");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["w/y not-judged"]);
}

#[test]
fn stryker_finding_no_test_running_any_mutant_of_a_block_that_reads_files_is_not_judged() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(&WEB_BLOCK.replace("limit 20", "reads docs/**\nlimit 20"));
    tree.report("stryker-nocov", "2");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["w/y not-judged"]);
    // A survivor beside them still counts, `reads` line or not.
    tree.report("stryker-survived", "1");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["w/y survived 1"]);
}

#[test]
fn a_mutant_both_stryker_runs_report_counts_once() {
    // req:ci/mutate-script@5f0ee0b6
    // The diff's lines are among the first `limit`, so the further run reports its mutants again.
    let tree = mutate_tree("");
    let diff = tree.dir.path().join("the.diff");
    fs::write(
        &diff,
        "--- a/web/src/app.ts\n+++ b/web/src/app.ts\n@@ -3,0 +4,2 @@\n+x\n+y\n",
    )
    .unwrap();
    fs::write(
        tree.dir.path().join("plan"),
        format!("{WEB_BLOCK}diff {}\n", diff.display()),
    )
    .unwrap();
    tree.report("stryker-nocov", "2");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    let runs = log
        .iter()
        .filter(|l| l.starts_with("web$ npx stryker run "))
        .count();
    // The fixture's three, the diff's and the further one.
    assert_eq!(runs, 5, "{log:?}");
    assert_eq!(result, ["w/y survived 2"]);
    tree.report("stryker-survived", "1");
    fs::remove_file(tree.dir.path().join("bin/stryker-nocov")).unwrap();
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["w/y survived 1"]);
}

#[test]
fn stryker_whose_fixture_shows_no_test_running_the_blind_function_judges_nothing() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(WEB_BLOCK);
    tree.report("stryker-killed", "1");
    tree.report("stryker-fixture-blind-nocov", "");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(!ok, "{log:?}");
    assert!(
        stderr.contains("StrykerJS") && stderr.contains("fixture"),
        "{stderr}"
    );
    assert!(result.is_empty(), "{result:?}");
}

/// A `stryker run` the fake recorded: its configuration, the test files its vitest
/// configuration named, and what each held.
struct StrykerRun {
    config: serde_json::Value,
    tests: Vec<(String, String)>,
}

/// The `stryker run`s of the last pass, in the order they ran.
fn stryker_runs(tree: &Tree) -> Vec<StrykerRun> {
    let dir = tree.dir.path().join("log.runs");
    let mut runs: Vec<(u32, StrykerRun)> = fs::read_dir(&dir)
        .map(|d| d.collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .map(|e| {
            let p = e.unwrap().path();
            let n: u32 = p.file_name().unwrap().to_string_lossy().parse().unwrap();
            let config =
                serde_json::from_str(&fs::read_to_string(p.join("stryker.config.json")).unwrap())
                    .unwrap();
            let tests = fs::read_to_string(p.join("tests"))
                .unwrap_or_default()
                .lines()
                .enumerate()
                .map(|(i, t)| {
                    let text =
                        fs::read_to_string(p.join(format!("test-{}", i + 1))).unwrap_or_default();
                    (t.to_owned(), text)
                })
                .collect();
            (n, StrykerRun { config, tests })
        })
        .collect();
    runs.sort_by_key(|(n, _)| *n);
    runs.into_iter().map(|(_, r)| r).collect()
}

impl StrykerRun {
    fn is_fixtures(&self) -> bool {
        self.config.to_string().contains("clm-mutate-fixture")
    }
}

#[test]
fn the_stryker_fixture_runs_a_test_that_imports_nothing() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(WEB_BLOCK);
    tree.report("stryker-killed", "1");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["w/y killed"]);
    let runs = stryker_runs(&tree);
    let block = runs
        .iter()
        .position(|r| !r.is_fixtures())
        .unwrap_or_else(|| panic!("no block run: {log:?}"));
    // The fixture's run for its test that notices: the test imports the fixture's source.
    let notices = runs[..block]
        .iter()
        .find(|r| {
            r.tests
                .iter()
                .any(|(_, t)| t.contains("fixture_notices") && t.contains("from './fixture'"))
        })
        .unwrap_or_else(|| panic!("no run of the test that notices: {log:?}"));
    // One before the block's that names a single test file, which imports none of the fixture's
    // source files and reads a file of the repository, with the same `mutate` ranges.
    let alone = runs[..block]
        .iter()
        .find(|r| {
            r.tests.len() == 1
                && !r.tests[0].1.contains("from './fixture")
                && r.tests[0].1.contains("readFileSync(new URL(")
        })
        .unwrap_or_else(|| panic!("no run of a test that imports nothing: {log:?}"));
    let (path, text) = &alone.tests[0];
    assert!(path.starts_with("clm-mutate-fixture/"), "{path}");
    assert!(!text.contains("import { double"), "{text}");
    assert_eq!(alone.config["mutate"], notices.config["mutate"]);
    assert!(
        alone.config["mutate"]
            .as_array()
            .is_some_and(|m| !m.is_empty()),
        "{}",
        alone.config
    );
}

#[test]
fn a_stryker_that_runs_only_the_tests_that_import_a_mutated_file_judges_nothing() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(WEB_BLOCK);
    tree.report("stryker-killed", "1");
    tree.report("stryker-related", "");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(!ok, "{log:?}");
    assert!(
        stderr.contains("StrykerJS") && stderr.contains("fixture"),
        "{stderr}"
    );
    let runs = stryker_runs(&tree);
    assert!(!runs.is_empty(), "{log:?}");
    assert!(
        runs.iter()
            .all(|r| !r.config["mutate"].to_string().contains("app.ts")),
        "{log:?}"
    );
    assert!(result.is_empty(), "{result:?}");
}

#[test]
fn a_fixture_test_that_imports_nothing_reported_other_than_no_coverage_judges_nothing() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(WEB_BLOCK);
    tree.report("stryker-killed", "1");
    // A mutant that test survives, and then no mutant at all.
    for switch in [
        "stryker-fixture-unrelated-survives",
        "stryker-fixture-unrelated-empty",
    ] {
        tree.report(switch, "");
        let (ok, log, result, stderr) = tree.mutate(&[]);
        assert!(!ok, "{switch}: {log:?}");
        assert!(
            stderr.contains("StrykerJS") && stderr.contains("fixture"),
            "{switch}: {stderr}"
        );
        assert!(
            stryker_runs(&tree).iter().all(StrykerRun::is_fixtures),
            "{switch}: {log:?}"
        );
        assert!(result.is_empty(), "{switch}: {result:?}");
        fs::remove_file(tree.dir.path().join("bin").join(switch)).unwrap();
    }
}

#[test]
fn a_diff_with_no_mutant_lists_none_and_the_further_ones_are_tried() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(RUST_BLOCK);
    // The real tool prints nothing at all for such a diff.
    tree.report("diff-list", "");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 1"]);
    let tries = block_tries(&log);
    assert_eq!(tries.len(), 1, "{log:?}");
    assert!(!tries[0].contains("--in-diff"), "{}", tries[0]);
}

#[test]
fn a_block_whose_files_have_no_mutant_is_not_judged_without_finding_lines() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(RUST_BLOCK);
    tree.report("list", "");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x not-judged"]);
    assert!(
        !log.iter().any(|l| l.starts_with("cargo llvm-cov nextest")),
        "{log:?}"
    );
}

/// The tree of *a rule in the test's own file*: a Rust block with no diff whose one test is in
/// `crates/clm-check/tests/it.rs`, a file of 45 lines, whose code file has one mutant, on line
/// 30, which its tests don't run, whose tests run lines 5 to 9 of the test file, whose test file
/// has two mutants, on line 6 and on line 40, and whose `cargo nextest run` exits 100.
fn test_file_tree() -> Tree {
    let tree = mutate_tree(
        "requirement a/x\ncode crates/clm-check/src/lib.rs\ntest crates/clm-check/tests/it.rs\tproves_x\nlimit 20\n",
    );
    let text: String = (1..=45).map(|n| format!("// line {n}\n")).collect();
    fs::create_dir_all(tree.work().join("crates/clm-check/tests")).unwrap();
    fs::write(tree.work().join("crates/clm-check/tests/it.rs"), text).unwrap();
    git(&tree.work(), &["add", "-A"]);
    git(&tree.work(), &["commit", "-q", "-m", "a test file"]);
    tree.report(
        "list",
        "30 30 crates/clm-check/src/lib.rs:30:5: replace on_thirty with ()\n",
    );
    tree.report("lcov-run", &test_file_lines(5..=9));
    tree.report("zlist", "6 6\n40 40\n");
    tree.report("nextest-exit", "100");
    tree
}

/// `lcov-run` lines that report `lines` of the test file of `test_file_tree` run.
fn test_file_lines(lines: impl IntoIterator<Item = u32>) -> String {
    lines
        .into_iter()
        .map(|n| format!("crates/clm-check/tests/it.rs:{n}\n"))
        .collect()
}

/// The `cargo nextest run` calls of `log` for a block, not for the fixture.
fn block_nextest_runs(log: &[String]) -> Vec<&String> {
    log.iter()
        .filter(|l| l.starts_with("cargo nextest run") && !l.contains("fixture_file_"))
        .collect()
}

#[test]
fn a_rule_in_the_tests_own_file_is_judged_by_the_mutants_of_that_file() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = test_file_tree();
    let file = tree.work().join("crates/clm-check/tests/it.rs");
    let original = fs::read_to_string(&file).unwrap();
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x killed"]);
    // The run that finds the lines reports the files under `tests/`.
    let cov = log
        .iter()
        .find(|l| l.starts_with("cargo llvm-cov nextest ") && l.contains(" -E "))
        .unwrap_or_else(|| panic!("{log:?}"));
    assert!(cov.contains("--no-default-ignore-filename-regex"), "{cov}");
    // The tool lists the mutants of the tree's copy of the test file, not the checkout's.
    let listed = log
        .iter()
        .find(|l| {
            l.contains("--Zmutate-file") && l.ends_with("crates/clm-check/tests/it.rs --json")
        })
        .unwrap_or_else(|| panic!("{log:?}"));
    let work = fs::canonicalize(tree.work()).unwrap();
    assert!(
        !listed.contains(&*work.to_string_lossy())
            && !listed.contains(&*tree.work().to_string_lossy()),
        "{listed}"
    );
    assert!(block_tries(&log).is_empty(), "{log:?}");
    // Run once: the mutant on line 6 caught, the one on line 40, which no test runs, untried,
    // under the block's filterset, over its test packages, with the job's build directory.
    let runs = block_nextest_runs(&log);
    assert_eq!(runs.len(), 1, "{log:?}");
    for part in [
        "--no-tests=pass",
        "--package clm-check",
        "package(=clm-check)",
        "binary_id(=clm-check::it)",
        "test(~proves_x)",
    ] {
        assert!(runs[0].contains(part), "no {part} in {}", runs[0]);
    }
    let env = tree.log_lines("nextest-env");
    assert!(
        env.len() == 1 && env[0].starts_with("0 /") && env[0].ends_with("/mutate-job-1"),
        "{env:?}"
    );
    // In a copy of its own: neither the checkout nor the tree `cargo mutants` ran in.
    let dirs = tree.log_lines("nextest-cwd");
    assert_eq!(dirs.len(), 1, "{dirs:?}");
    let ran = PathBuf::from(&dirs[0]);
    assert!(
        !ran.starts_with(&work) && !ran.starts_with(tree.work()),
        "{ran:?}"
    );
    let trees = tree.log_lines("cwd");
    assert!(trees.len() > 2, "{trees:?}");
    assert!(!trees.contains(&dirs[0]), "{dirs:?} {trees:?}");
    // With the test file there changed as the mutant on line 6 says, and the checkout's as it
    // was.
    let mutated = fs::read_to_string(tree.dir.path().join("log.nextest-file")).unwrap();
    let want: Vec<String> = original
        .lines()
        .map(|l| {
            if l == "// line 6" {
                "// line 6 /* mutant 6 */".to_owned()
            } else {
                l.to_owned()
            }
        })
        .collect();
    assert_eq!(mutated.lines().collect::<Vec<_>>(), want);
    assert_eq!(fs::read_to_string(&file).unwrap(), original);
    // Where git, as the tree's index marks every tracked file unchanged, sees no change.
    assert_eq!(tree.log_lines("nextest-status"), Vec::<String>::new());
}

#[test]
fn a_test_files_listing_that_fails_or_cant_be_read_leaves_no_verdict() {
    // req:ci/mutate-script@5f0ee0b6
    for (fake, what) in [
        ("zlist-exit", "--Zmutate-file"),
        ("zlist-garbage", "--Zmutate-file"),
        ("zlist-stale", "--Zmutate-file"),
    ] {
        let tree = test_file_tree();
        let file = tree.work().join("crates/clm-check/tests/it.rs");
        let original = fs::read(&file).unwrap();
        tree.report(fake, "1");
        let (ok, log, result, stderr) = tree.mutate(&[]);
        assert!(!ok, "{fake}: {log:?}");
        assert!(
            stderr.contains(what) && stderr.contains("a/x"),
            "{fake}: {stderr}"
        );
        assert!(result.is_empty(), "{fake}: {result:?}");
        assert!(block_nextest_runs(&log).is_empty(), "{fake}: {log:?}");
        assert_eq!(fs::read(&file).unwrap(), original);
    }
}

#[test]
fn a_block_with_a_diff_and_code_run_on_no_mutants_line_is_judged_by_its_test_file() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = test_file_tree();
    fs::write(
        tree.dir.path().join("plan"),
        "requirement a/x\ncode crates/clm-check/src/lib.rs\ndiff /tmp/the.diff\ntest crates/clm-check/tests/it.rs\tproves_x\nlimit 20\n",
    )
    .unwrap();
    tree.report(
        "diff-list",
        "30 30 crates/clm-check/src/lib.rs:30:5: replace on_thirty with ()\n",
    );
    // The code file runs, on a line with no mutant: it is the block's own, not another's.
    let mut run = test_file_lines(5..=9);
    run.push_str("crates/clm-check/src/lib.rs:10\n");
    tree.report("lcov-run", &run);
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert!(block_tries(&log).is_empty(), "{log:?}");
    assert_eq!(block_nextest_runs(&log).len(), 1, "{log:?}");
    assert_eq!(result, ["a/x killed"]);
}

#[test]
fn only_files_under_a_crates_tests_are_listed_each_once() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = test_file_tree();
    let deeper = tree.work().join("crates/clm-check/src/tests");
    fs::create_dir_all(&deeper).unwrap();
    fs::write(deeper.join("y.rs"), "// y\n").unwrap();
    git(&tree.work(), &["add", "-A"]);
    git(&tree.work(), &["commit", "-q", "-m", "a deeper tests/"]);
    fs::write(
        tree.dir.path().join("plan"),
        "requirement a/x\ncode crates/clm-check/src/lib.rs\ntest crates/clm-check/src/tests/y.rs\tproves_y\ntest crates/clm-check/tests/it.rs\tproves_x\ntest crates/clm-check/tests/it.rs\tproves_z\nlimit 20\n",
    )
    .unwrap();
    let (ok, log, _, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    let listed = tree.log_lines("zlisted");
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert!(
        listed[0].ends_with("/crates/clm-check/tests/it.rs"),
        "{listed:?}"
    );
}

#[test]
fn a_jobs_copy_is_reused_by_its_next_block_with_the_file_put_back() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = test_file_tree();
    let block =
        "code crates/clm-check/src/lib.rs\ntest crates/clm-check/tests/it.rs\tproves_x\nlimit 20\n";
    fs::write(
        tree.dir.path().join("plan"),
        format!("requirement a/x\n{block}requirement a/y\n{block}"),
    )
    .unwrap();
    tree.report("nextest-exit", "0");
    let file = tree.work().join("crates/clm-check/tests/it.rs");
    let original = fs::read_to_string(&file).unwrap();
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 1", "a/y survived 1"]);
    // One copy for the job, the file in it put back after the first block: the second block's
    // run sees its own mutant alone.
    let dirs = tree.log_lines("nextest-cwd");
    assert!(dirs.len() == 2 && dirs[0] == dirs[1], "{dirs:?}");
    let mutated = fs::read_to_string(tree.dir.path().join("log.nextest-file")).unwrap();
    assert_eq!(
        mutated,
        original.replace("// line 6\n", "// line 6 /* mutant 6 */\n")
    );
    // The fixture's test file is judged once in the pass.
    assert_eq!(
        log.iter()
            .filter(|l| l.contains("--Zmutate-file") && l.contains("clm_mutate_fixture.rs"))
            .count(),
        1,
        "{log:?}"
    );
    assert_eq!(
        log.iter()
            .filter(|l| l.starts_with("cargo nextest run") && l.contains("fixture_file_"))
            .count(),
        2,
        "{log:?}"
    );
}

#[test]
fn a_rule_in_the_tests_own_file_that_its_tests_dont_notice_survives() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = test_file_tree();
    tree.report("nextest-exit", "0");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 1"]);
    assert_eq!(block_nextest_runs(&log).len(), 1, "{log:?}");
    // With both mutants' lines run, both are tried, each in turn, and the file put back
    // between them: the second run sees only its own mutant.
    tree.report("lcov-run", &test_file_lines((5..=9).chain([40])));
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 2"]);
    assert_eq!(block_nextest_runs(&log).len(), 2, "{log:?}");
    let last = fs::read_to_string(tree.dir.path().join("log.nextest-file")).unwrap();
    assert!(
        last.contains("// line 40 /* mutant 40 */") && !last.contains("mutant 6"),
        "{last}"
    );
    // Up to the block's limit.
    fs::write(
        tree.dir.path().join("plan"),
        "requirement a/x\ncode crates/clm-check/src/lib.rs\ntest crates/clm-check/tests/it.rs\tproves_x\nlimit 1\n",
    )
    .unwrap();
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x survived 1"]);
    assert_eq!(block_nextest_runs(&log).len(), 1, "{log:?}");
    // And none after the first caught.
    fs::write(
        tree.dir.path().join("plan"),
        "requirement a/x\ncode crates/clm-check/src/lib.rs\ntest crates/clm-check/tests/it.rs\tproves_x\nlimit 20\n",
    )
    .unwrap();
    tree.report("nextest-exit", "100");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x killed"]);
    assert_eq!(block_nextest_runs(&log).len(), 1, "{log:?}");
}

#[test]
fn a_test_files_mutant_that_doesnt_build_is_not_judged() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = test_file_tree();
    tree.report("nextest-exit", "101");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x not-judged"]);
}

#[test]
fn a_rule_in_the_tests_own_file_of_a_block_that_reads_files_is_judged() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = test_file_tree();
    fs::write(
        tree.dir.path().join("plan"),
        "requirement a/x\ncode crates/clm-check/src/lib.rs\ntest crates/clm-check/tests/it.rs\tproves_x\nreads crates/**\nlimit 20\n",
    )
    .unwrap();
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x killed"]);
}

#[test]
fn an_empty_test_in_a_file_with_helpers_is_judged_by_its_code() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = test_file_tree();
    tree.report("lcov-run", &test_file_lines([2, 3]));
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert!(block_nextest_runs(&log).is_empty(), "{log:?}");
    assert_eq!(result, ["a/x survived 1"]);
}

#[test]
fn a_test_that_runs_another_components_code_is_judged_by_its_code() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = test_file_tree();
    tree.report("lcov-extra", "crates/clm-land/src/lib.rs:4\n");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert!(block_nextest_runs(&log).is_empty(), "{log:?}");
    assert_eq!(result, ["a/x survived 1"]);
}

#[test]
fn the_mutants_of_the_code_come_before_those_of_the_test_file() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = test_file_tree();
    let mut run = test_file_lines(5..=9);
    run.push_str("crates/clm-check/src/lib.rs:30\n");
    tree.report("lcov-run", &run);
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    let tries = block_tries(&log);
    assert_eq!(tries.len(), 1, "{log:?}");
    assert!(tries[0].contains("replace on_thirty"), "{}", tries[0]);
    assert!(block_nextest_runs(&log).is_empty(), "{log:?}");
    // Its test file's mutants aren't even listed.
    assert_eq!(tree.log_lines("zlisted"), Vec::<String>::new());
    assert_eq!(result, ["a/x survived 1"]);
}

#[test]
fn nextest_failing_for_a_test_files_mutant_leaves_no_verdict() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = test_file_tree();
    tree.report("nextest-exit", "2");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(!ok, "{log:?}");
    assert!(
        stderr.contains("cargo nextest") && stderr.contains("a/x"),
        "{stderr}"
    );
    assert!(result.is_empty(), "{result:?}");
    // And the run that finds the lines failing leaves none either, with no mutant tried.
    tree.report("nextest-exit", "100");
    tree.report("llvm-cov-exit", "1");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(!ok, "{log:?}");
    assert!(
        stderr.contains("cargo llvm-cov") && stderr.contains("a/x"),
        "{stderr}"
    );
    assert!(result.is_empty(), "{result:?}");
    assert!(block_nextest_runs(&log).is_empty(), "{log:?}");
}

#[test]
fn a_tool_that_judges_the_fixtures_test_file_wrongly_judges_nothing() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(RUST_BLOCK);
    tree.report("diff-caught", "1");
    tree.report("nextest-fixture-catches-all", "");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(!ok, "{log:?}");
    assert!(stderr.contains("fixture"), "{stderr}");
    assert!(
        !log.iter()
            .any(|l| l.contains("crates/clm-check/src/lib.rs")),
        "{log:?}"
    );
    assert!(result.is_empty(), "{result:?}");
}

#[test]
fn the_fixtures_test_file_is_judged_with_its_every_mutant_in_a_copy_made_as_the_tree_is() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(RUST_BLOCK);
    tree.report("diff-caught", "1");
    let (ok, log, result, stderr) = tree.mutate(&[]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result, ["a/x killed"]);
    // The fixture's test file is listed, and its one mutant tried with each of its two tests,
    // before any call names the block's files.
    let listed = log
        .iter()
        .position(|l| l.contains("--Zmutate-file") && l.contains("tests/clm_mutate_fixture.rs"))
        .unwrap_or_else(|| panic!("{log:?}"));
    let fixture_runs: Vec<usize> = log
        .iter()
        .enumerate()
        .filter(|(_, l)| l.starts_with("cargo nextest run") && l.contains("fixture_file_"))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(fixture_runs.len(), 2, "{log:?}");
    assert!(log[fixture_runs[0]].contains("test(~fixture_file_notices)"));
    assert!(log[fixture_runs[1]].contains("test(~fixture_file_blind)"));
    assert!(log[fixture_runs[0]].contains("binary_id(=clm-mutate-fixture::clm_mutate_fixture)"));
    let block = log
        .iter()
        .position(|l| l.contains("crates/clm-check/src/lib.rs"))
        .unwrap_or_else(|| panic!("{log:?}"));
    assert!(
        listed < fixture_runs[0] && fixture_runs[1] < block,
        "{log:?}"
    );
    // Tried in a copy made as the tree is: its `.git` borrows another repository's object store,
    // and git there sees no change to the test file the mutant changed.
    let alternates =
        fs::read_to_string(tree.dir.path().join("log.fixture-file-alternates")).unwrap_or_default();
    let store = alternates.trim();
    assert!(
        Path::new(store).is_absolute() && store.ends_with("/objects"),
        "alternates: {alternates:?}"
    );
    let status = fs::read_to_string(tree.dir.path().join("log.fixture-file-status")).unwrap();
    assert_eq!(status, "", "git saw the fixture's test file's mutant");
    let test_file = fs::read_to_string(tree.dir.path().join("log.fixture-test-file")).unwrap();
    assert!(
        test_file
            .lines()
            .next()
            .unwrap_or_default()
            .ends_with(" /* mutant 1 */"),
        "{test_file}"
    );
    // Its helpers are macros, which no tool mutates, so the function no test is holds its only
    // mutants; its tests ask git for the repository's top and, the blind one, for the working
    // tree's status.
    assert!(
        test_file.contains("macro_rules! in_git_repository")
            && test_file.contains("macro_rules! tracked_files_unchanged")
            && test_file.contains("--show-toplevel")
            && test_file.contains("\"--porcelain\", \"--untracked-files=no\""),
        "{test_file}"
    );
    let blind = test_file
        .split("fn fixture_file_blind()")
        .nth(1)
        .unwrap_or_default();
    assert!(
        blind.contains("in_git_repository!();") && blind.contains("tracked_files_unchanged!();"),
        "{test_file}"
    );
    let notices = test_file
        .split("fn fixture_file_notices()")
        .nth(1)
        .and_then(|s| s.split("#[test]").next())
        .unwrap_or_default();
    assert!(notices.contains("in_git_repository!();"), "{test_file}");
}

/// Every file under `dir`, but `target/`, which the script's coverage step writes, with its
/// contents, to tell that a run changed none.
fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        for entry in fs::read_dir(&d).unwrap().flatten() {
            let path = entry.path();
            if path == dir.join("target") {
                continue;
            }
            if path.is_dir() {
                todo.push(path);
            } else {
                files.push((path.clone(), fs::read(&path).unwrap()));
            }
        }
    }
    files.sort();
    files
}

/// A plan of one Rust block for each letter, `a/<x>`, whose code is `crates/clm-check/src/<x>.rs`
/// and whose test is `proves_<x>`, so the fake `cargo` tells their calls apart.
fn letter_blocks(letters: &[&str]) -> String {
    letters
        .iter()
        .map(|x| {
            format!(
                "requirement a/{x}\ncode crates/clm-check/src/{x}.rs\ntest crates/clm-check/tests/it.rs\tproves_{x}\nlimit 20\n"
            )
        })
        .collect()
}

/// Waits, for at most a minute, until `done` holds.
fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let start = std::time::Instant::now();
    while !done() {
        assert!(
            start.elapsed() < std::time::Duration::from_secs(60),
            "timed out waiting for {what}"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// Waits, for at most a minute, for `child` to exit; kills it and fails when it doesn't.
fn wait_exit(child: &mut std::process::Child) -> std::process::ExitStatus {
    let start = std::time::Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if start.elapsed() > std::time::Duration::from_secs(60) {
            let _ = child.kill();
            panic!("mutate.sh didn't exit");
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// Spawns `mutate.sh` with `extra` environment variables, its output dropped.
fn spawn_mutate(tree: &Tree, extra: &[(&str, &str)]) -> std::process::Child {
    tree.mutate_command(extra)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap()
}

/// One call of the fake `cargo` from `log.calls`: its block, `CLM_SLOT` and `CARGO_TARGET_DIR`.
struct Call {
    block: String,
    slot: String,
    target: String,
}

#[test]
fn blocks_run_at_once_up_to_the_jobs_each_with_a_slot_and_a_build_directory_of_its_own() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(&letter_blocks(&["a", "b", "c"]));
    tree.report("record-calls", "");
    tree.report("further-caught", "1");
    let (ok, log, result, stderr) = tree.mutate(&[("CLM_MUTATE_JOBS", "2"), ("CLM_SLOT", "5")]);
    assert!(ok, "{log:?} {stderr}");
    assert_eq!(result.len(), 3, "{result:?}");
    let events = tree.log_lines("calls");
    let mut calls: std::collections::HashMap<String, Call> = Default::default();
    let mut running: Vec<String> = Vec::new();
    let mut overlapped = false;
    let mut block_began = false;
    for e in &events {
        let w: Vec<&str> = e.split_whitespace().collect();
        if w[0] != "start" {
            running.retain(|p| p != w[1]);
            continue;
        }
        let call = Call {
            block: w[2].to_owned(),
            slot: w[3].to_owned(),
            target: w[4].to_owned(),
        };
        if call.block == "fixture" {
            // The fixture's calls keep the slot the script was given, and all end before the
            // first block's call begins.
            assert_eq!(call.slot, "5", "{events:?}");
            assert!(!block_began, "a fixture call after a block's: {events:?}");
        } else {
            block_began = true;
            assert!(
                running.iter().all(|p| calls[p].block != "fixture"),
                "{events:?}"
            );
            assert!(["5", "6"].contains(&call.slot.as_str()), "{events:?}");
            for other in running.iter().map(|p| &calls[p]) {
                if other.block != call.block {
                    overlapped = true;
                    assert_ne!(other.slot, call.slot, "{events:?}");
                    assert_ne!(other.target, call.target, "{events:?}");
                }
            }
        }
        calls.insert(w[1].to_owned(), call);
        running.push(w[1].to_owned());
        let mut blocks: Vec<&str> = running.iter().map(|p| calls[p].block.as_str()).collect();
        blocks.sort();
        blocks.dedup();
        assert!(blocks.len() <= 2, "three blocks at once: {events:?}");
    }
    assert!(overlapped, "no two blocks ran at once: {events:?}");
    // `coverage.sh` runs with the slot the script was given.
    assert_eq!(tree.log_lines("coverage-slot"), ["5"]);
    for x in ["fixture", "a", "b", "c"] {
        assert!(
            calls.values().any(|c| c.block == x),
            "no call of {x}: {events:?}"
        );
    }
    // Each block's calls ran with one slot and one build directory.
    for x in ["a", "b", "c"] {
        let mine: Vec<&Call> = calls.values().filter(|c| c.block == x).collect();
        assert!(
            mine.iter().all(|c| c.slot == mine[0].slot
                && c.target == mine[0].target
                && c.target != "unset"),
            "{events:?}"
        );
    }
}

#[test]
fn each_line_is_written_as_its_block_is_judged() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(&letter_blocks(&["a", "b"]));
    tree.report("further-caught", "1");
    tree.report("gate-b", "");
    let gate = tree.dir.path().join("log.gate");
    let mut child = spawn_mutate(&tree, &[]);
    let result = tree.dir.path().join("result");
    wait_until("the first line", || {
        fs::read_to_string(&result).unwrap_or_default() == "a/a killed\n"
            || child.try_wait().unwrap().is_some()
    });
    let early = fs::read_to_string(&result).unwrap_or_default();
    // Still running: block b waits for the gate, which only this test opens.
    let gated = child.try_wait().unwrap().is_none();
    fs::write(&gate, "").unwrap();
    let status = wait_exit(&mut child);
    assert_eq!(early, "a/a killed\n");
    assert!(gated);
    assert!(status.success());
    assert_eq!(tree.log_lines("result"), ["a/a killed", "a/b killed"]);
}

#[test]
fn without_a_positive_count_of_jobs_it_judges_one_block_at_a_time() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(&letter_blocks(&["a", "b"]));
    tree.report("further-caught", "1");
    tree.report("gate-a", "");
    let gate = tree.dir.path().join("log.gate");
    for jobs in [None, Some("0"), Some("x")] {
        let _ = fs::remove_file(&gate);
        let extra: Vec<(&str, &str)> = jobs.map(|j| ("CLM_MUTATE_JOBS", j)).into_iter().collect();
        let mut child = spawn_mutate(&tree, &extra);
        let log = tree.dir.path().join("log");
        // Block a's run of its mutants waits at the gate; block b must not start meanwhile.
        wait_until("block a's mutants", || {
            fs::read_to_string(&log)
                .unwrap_or_default()
                .lines()
                .any(|l| l.contains("src/a.rs") && l.contains("--re "))
        });
        std::thread::sleep(std::time::Duration::from_secs(1));
        let early = tree.log_lines("");
        fs::write(&gate, "").unwrap();
        let status = wait_exit(&mut child);
        assert!(
            !early.iter().any(|l| l.contains("src/b.rs")),
            "{jobs:?}: {early:?}"
        );
        assert!(status.success(), "{jobs:?}");
        assert_eq!(tree.log_lines("result"), ["a/a killed", "a/b killed"]);
    }
}

#[test]
fn a_failing_block_lets_the_running_ones_finish_and_starts_no_more() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(&letter_blocks(&["a", "b", "c"]));
    tree.report("further-caught", "1");
    tree.report("exit-a", "4");
    tree.report("after-fail-b", "a");
    let (ok, log, result, stderr) = tree.mutate(&[("CLM_MUTATE_JOBS", "2")]);
    assert!(!ok, "{log:?}");
    assert!(
        stderr.contains("cargo mutants") && stderr.contains("a/a"),
        "{stderr}"
    );
    assert_eq!(result, ["a/b killed"]);
    assert!(
        !log.iter().any(|l| l.contains("crates/clm-check/src/c.rs")),
        "{log:?}"
    );
    assert!(!log.iter().any(|l| l == "coverage.sh"), "{log:?}");
}

#[test]
fn ended_part_way_it_ends_its_blocks_and_removes_its_temporary_directory() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(&letter_blocks(&["a", "b"]));
    tree.report("further-caught", "1");
    tree.report("sleep-b", "");
    end_part_way(&tree, "TERM");
}

#[test]
fn interrupted_part_way_it_exits_143_on_either_signal() {
    // req:ci/mutate-script@5f0ee0b6
    let tree = mutate_tree(&letter_blocks(&["a", "b"]));
    tree.report("further-caught", "1");
    tree.report("sleep-b", "");
    for signal in ["INT", "TERM"] {
        end_part_way(&tree, signal);
    }
}

/// Runs `mutate.sh` on `tree`, whose second block's stand-in sleeps, sends it `signal` once the
/// first line is written, and asserts it exits 143, its stand-in ended, the first line alone
/// kept and its temporary directory gone.
fn end_part_way(tree: &Tree, signal: &str) {
    let tmp = tree.dir.path().join("tmp");
    fs::create_dir_all(&tmp).unwrap();
    let mut child = spawn_mutate(tree, &[("TMPDIR", tmp.to_str().unwrap())]);
    let sleeper = tree.dir.path().join("log.sleeper");
    wait_until("the second block's stand-in", || {
        sleeper.exists() || child.try_wait().unwrap().is_some()
    });
    assert_eq!(tree.log_lines("result"), ["a/a killed"], "{signal}");
    let pid = fs::read_to_string(&sleeper).unwrap().trim().to_owned();
    let sent = Command::new("kill")
        .args([&format!("-{signal}"), &child.id().to_string()])
        .status()
        .unwrap();
    assert!(sent.success());
    let status = wait_exit(&mut child);
    assert_eq!(status.code(), Some(143), "{signal}: {status:?}");
    // The stand-in is ended with it.
    wait_until("the stand-in to end", || {
        !Command::new("kill")
            .args(["-0", &pid])
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success()
    });
    assert_eq!(tree.log_lines("result"), ["a/a killed"]);
    let left: Vec<_> = fs::read_dir(&tmp)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    assert!(left.is_empty(), "{left:?}");
}
