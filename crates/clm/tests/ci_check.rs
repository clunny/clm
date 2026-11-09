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

#[path = "../../clm-git/tests/exec/mod.rs"]
mod exec;
use exec::write_exec as executable;

/// A work tree holding the real scripts, `web/` and `plugin-panel/`, with the fakes in `bin/`
/// beside it.
struct Tree {
    dir: TempDir,
    /// The script a run runs, under `.github/scripts/`: `ci-check.sh` unless a test says.
    script: &'static str,
}

impl Tree {
    fn new() -> Self {
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
    fn path(&self) -> std::ffi::OsString {
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
        let log = self.dir.path().join("log");
        let _ = fs::remove_file(&log);
        let mut c = Command::new(Path::new(".github/scripts").join(self.script));
        c.current_dir(self.work())
            .env("PATH", path)
            .env("CI_LOG", &log)
            .env("CI_FAIL", fail)
            .env_remove("CI")
            .env_remove("CLM_SELECTION");
        if let Some(ci) = ci {
            c.env("CI", ci);
        }
        if let Some(selection) = selection {
            c.env("CLM_SELECTION", selection);
        }
        c.output().unwrap()
    }
}

#[test]
fn every_check() {
    // req:ci/web-checks@72a04372
    // req:ci/detector-tools@156131ed
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
    let no_defaults = NO_DEFAULTS.map(|k| format!("{WASM} --no-default-features -p {k}"));
    for cmd in [
        "cargo fmt --all --check".to_owned(),
        "cargo clippy --all-targets -- -D warnings".to_owned(),
        NEXTEST_ALL.to_owned(),
        DOC_TESTS.to_owned(),
        format!("{WASM} -p clm-model -p clm-context"),
        format!("{WASM} -p clm-web"),
        "plugin-panel$ npm ci --ignore-scripts".to_owned(),
        "plugin-panel$ npm test".to_owned(),
        "web$ npm ci --ignore-scripts".to_owned(),
        WEB_TEST.to_owned(),
    ]
    .into_iter()
    .chain(no_defaults)
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
    // req:ci/no-model-steps@00759a81
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
    // However `clm` is named in the script.
    let lines = script_lines("ci-check.sh");
    let runs_clm = |w: &str| w == "clm" || w.ends_with("/clm") || w.ends_with("clm.sh");
    assert!(
        !lines.iter().any(|l| l.split_whitespace().any(runs_clm)),
        "{lines:?}"
    );
}

#[test]
fn every_wasm_build_stands_alone() {
    // req:ci/web-checks@72a04372
    // Whole lines, so nothing like `|| true` follows a build. Each `--no-default-features`
    // crate builds on its own: built with a crate that depends on it with its default
    // features, cargo would turn them back on.
    let text = script("ci-check.sh");
    assert!(text.contains("\nset -euo pipefail\n"), "{text}");
    let lines = script_lines("ci-check.sh");
    // Any line naming the target is a wasm build, and each has the one shape checked here.
    let builds: Vec<_> = lines
        .iter()
        .filter(|l| l.contains("wasm32-unknown-unknown"))
        .collect();
    for build in &builds {
        assert!(build.starts_with(WASM), "{build}");
        if build.contains("--no-default-features") {
            assert_eq!(build.matches(" -p ").count(), 1, "{build}");
        }
    }
    // These build only with their default features disabled, and turn none back on.
    for krate in NO_DEFAULTS {
        let named: Vec<_> = builds
            .iter()
            .filter(|b| b.split([' ', '=']).any(|w| w.ends_with(krate)))
            .collect();
        assert!(!named.is_empty(), "no wasm build of {krate}");
        for build in named {
            assert!(build.contains("--no-default-features"), "{build}");
            let turns_on = |w: &str| {
                w.starts_with("--features") || w.starts_with("-F") || w == "--all-features"
            };
            assert!(!build.split_whitespace().any(turns_on), "{build}");
        }
    }
}

#[test]
fn broken_wasm_build_fails() {
    // req:ci/web-checks@72a04372
    for build in [
        format!("{WASM} -p clm-model -p clm-context"),
        format!("{WASM} --no-default-features -p clm-ledger"),
        format!("{WASM} -p clm-web"),
    ] {
        let (ok, log) = Tree::new().run(None, &build);
        assert!(!ok, "{log:?}");
        // Nothing runs after the failing step.
        assert_eq!(log.last(), Some(&build), "{log:?}");
    }
}

#[test]
fn failing_panel_test_fails() {
    // req:ci/web-checks@72a04372
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
    // req:ci/web-checks@72a04372
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
    // req:ci/web-checks@72a04372
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
    // req:ci/web-checks@72a04372
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

/// Trees built and run on many threads at once never fail: a fake is never run while some
/// process holds it open for writing (`Text file busy`).
#[test]
fn trees_built_and_run_in_parallel_never_run_a_busy_fake() {
    let threads: Vec<_> = (0..16)
        .map(|_| {
            std::thread::spawn(|| {
                for _ in 0..32 {
                    let (ok, log) = Tree::new().run(None, "");
                    assert!(ok, "{log:?}");
                }
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
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
    // req:ci/browser-install@6f2b09cf
    let (ok, log) = Tree::new().run(Some("true"), "");
    assert!(ok, "{log:?}");
    assert!(runs_before(&log, BROWSER_INSTALL_CI, WEB_TEST), "{log:?}");
    assert!(!log.contains(&BROWSER_INSTALL.to_owned()), "{log:?}");
}

#[test]
fn outside_ci_installs_the_browser_without_deps() {
    // req:ci/browser-install@6f2b09cf
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
    // req:ci/browser-install@6f2b09cf
    let (ok, log) = Tree::new().run(None, BROWSER_INSTALL);
    assert!(!ok, "{log:?}");
    assert_eq!(log.last().map(String::as_str), Some(BROWSER_INSTALL));
    assert!(!log.contains(&WEB_TEST.to_owned()), "{log:?}");
}

#[test]
fn capability_package_installs_the_browser() {
    // req:ci/browser-install@6f2b09cf
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
    // req:ci/detector-tools@156131ed
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
/// detector whose tool has no install line its `hint` repeats, a `clm` detector whose hint
/// doesn't name `scripts/dev-install.sh`, or a script line that installs `clm`.
fn detector_tool_problems(config: &toml::Table, lines: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    for d in config["detectors"].as_array().unwrap() {
        let tool = d["command"][0].as_str().unwrap();
        let hint = d["hint"].as_str().unwrap();
        if tool == "clm" {
            // It runs the clone's own `clm`, or the binary under test; nothing installs it.
            if !hint.contains("scripts/dev-install.sh") {
                problems.push(format!(
                    "`clm` detector's hint `{hint}` doesn't name scripts/dev-install.sh"
                ));
            }
        } else if hint.split_whitespace().last() != Some(tool) {
            problems.push(format!("hint `{hint}` doesn't install `{tool}`"));
        } else if !lines.iter().any(|l| l == hint) {
            problems.push(format!("detector-tools.sh has no line `{hint}`"));
        }
    }
    for line in lines.iter().filter(|l| installs_clm(l)) {
        problems.push(format!("detector-tools.sh installs clm: `{line}`"));
    }
    problems
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
    // req:ci/detector-tools@156131ed
    // Each detector's tool has an install line, and its `hint` is that line, so the
    // version is pinned in one place; a `clm` detector has none.
    let config: toml::Table = fs::read_to_string(repo().join("clm.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let lines = script_lines("detector-tools.sh");
    assert!(!config["detectors"].as_array().unwrap().is_empty());
    let problems = detector_tool_problems(&config, &lines);
    assert!(problems.is_empty(), "{problems:?}: {lines:?}");
}

#[test]
fn clm_detector_needs_no_install_line() {
    // req:ci/detector-tools@156131ed
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
        detector_tool_problems(&ok, &lines(&[])),
        Vec::<String>::new()
    );
    // A `clm` detector whose hint doesn't say how a clone gets `main`'s `clm`.
    let problems = detector_tool_problems(&config("cargo install clm"), &lines(&[]));
    assert!(
        problems.len() == 1 && problems[0].contains("dev-install.sh"),
        "{problems:?}"
    );
    // The script installing `clm` itself, however it is written.
    for install in [
        "cargo install --locked clm",
        "cargo install clm --locked",
        "cargo install --locked --path crates/clm",
        "cargo binstall clm",
        "scripts/dev-install.sh",
    ] {
        let problems = detector_tool_problems(&ok, &lines(&[install]));
        assert!(
            problems.len() == 1 && problems[0].contains("installs clm"),
            "{install}: {problems:?}"
        );
    }
    // Another detector's tool without its line.
    assert_eq!(detector_tool_problems(&ok, &[]).len(), 1);
}

#[test]
fn only_detector_tools_installs_tools() {
    // req:ci/detector-tools@156131ed
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
    // req:ci/detector-tools@156131ed
    let lines = script_lines("ci-check.sh");
    let tools = lines
        .iter()
        .position(|l| l == ".github/scripts/detector-tools.sh")
        .unwrap_or_else(|| panic!("ci-check.sh doesn't run detector-tools.sh: {lines:?}"));
    let cargo = lines
        .iter()
        .position(|l| l.split_whitespace().any(|w| w == "cargo"))
        .unwrap_or_else(|| panic!("no cargo line: {lines:?}"));
    assert!(
        tools < cargo,
        "detector-tools.sh runs after cargo: {lines:?}"
    );
    assert!(
        !lines.iter().any(|l| l.contains("cargo install")),
        "{lines:?}"
    );
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
        format!("{WASM} -p clm-model -p clm-context"),
        format!("{WASM} -p clm-web"),
    ]
    .into_iter()
    .chain(NO_DEFAULTS.map(|k| format!("{WASM} --no-default-features -p {k}")))
    {
        assert!(log.contains(&cmd), "no `{cmd}` in {log:?}");
    }
}

#[test]
fn one_integration_test() {
    // req:ci/selected-tests@bf58d446
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
    // req:ci/selected-tests@bf58d446
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
    // req:ci/selected-tests@bf58d446
    let (ok, log) = Tree::new().run_selecting("file web/src/app.ts\n");
    assert!(ok, "{log:?}");
    assert_whole_tree_checks(&log);
    assert_eq!(tests_run(&log), Vec::<&str>::new());
    let web: Vec<_> = WEB_SUITE.iter().chain(&CAP_WEB_SUITE).copied().collect();
    assert_eq!(npm_run(&log), web);
    // req:ci/panel-validate@82219e3e
    assert!(!log.iter().any(|l| l.starts_with("claude")), "{log:?}");
    let (ok, log) = Tree::new().run_selecting("file plugin-panel/src/panel.ts\n");
    assert!(ok, "{log:?}");
    let panel: Vec<_> = PANEL_SUITE
        .iter()
        .chain(&CAP_PANEL_SUITE)
        .copied()
        .collect();
    assert_eq!(npm_run(&log), panel);
    // req:ci/panel-validate@82219e3e
    assert!(log.contains(&PANEL_VALIDATE.to_owned()), "{log:?}");
}

#[test]
fn capability_package_file() {
    // req:ci/selected-tests@bf58d446
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
    // req:ci/panel-validate@82219e3e
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
    // req:ci/panel-validate@82219e3e
    let (ok, log) = Tree::new().run(None, "");
    assert!(ok, "{log:?}");
    // Exactly once, with neither `--strict` nor `--profile`, beside the plugin's own
    // validation (ci/plugin-validate).
    let runs: Vec<_> = log.iter().filter(|l| l.starts_with("claude ")).collect();
    assert_eq!(runs, [PANEL_VALIDATE, PLUGIN_VALIDATE], "{log:?}");
}

#[test]
fn no_claude_on_path_skips() {
    // req:ci/panel-validate@82219e3e
    let (ok, log, stderr) = Tree::new().run_without_claude(None);
    assert!(ok, "{log:?} {stderr}");
    assert!(stderr.lines().any(|l| l == NO_CLAUDE), "{stderr}");
    // It carries on: `web/`'s tests still run after it.
    assert!(log.contains(&WEB_TEST.to_owned()), "{log:?}");
}

#[test]
fn panel_not_selected_runs_no_claude() {
    // req:ci/panel-validate@82219e3e
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
    // req:ci/selected-tests@bf58d446
    let tree = Tree::new();
    let all_line = tree.run_selecting("all Cargo.lock\nfile crates/clm/tests/run.rs\n");
    let unset = tree.run(None, "");
    let empty = tree.run_env(None, "", Some("".as_ref()));
    // A selection file that isn't there can't narrow anything.
    let missing = tree.dir.path().join("missing");
    let missing = tree.run_env(None, "", Some(missing.as_os_str()));
    for (what, (ok, log)) in [
        ("all line", all_line),
        ("unset", unset),
        ("empty", empty),
        ("missing file", missing),
    ] {
        assert!(ok, "{what}: {log:?}");
        assert_whole_tree_checks(&log);
        assert_eq!(tests_run(&log), [NEXTEST_ALL, DOC_TESTS], "{what}");
        // req:ci/panel-validate@82219e3e
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
    // req:ci/selected-tests@bf58d446
    let (ok, log) = Tree::new().run_selecting("");
    assert!(ok, "{log:?}");
    assert_whole_tree_checks(&log);
    assert_eq!(tests_run(&log), Vec::<&str>::new());
    assert_eq!(npm_run(&log), Vec::<&str>::new());
}

#[test]
fn one_cargo_test_per_package_in_package_order() {
    // req:ci/selected-tests@bf58d446
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
fn only_file_lines_select() {
    // req:ci/selected-tests@bf58d446
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
    // req:ci/selected-tests@bf58d446
    // req:ci/browser-install@6f2b09cf
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
    // req:ci/selected-tests@bf58d446
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
    // req:ci/no-model-steps@00759a81
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
    // req:ci/no-model-steps@00759a81
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
/// at the `--output-path` it is given.
const CARGO_COVERAGE: &str = r#"#!/usr/bin/env bash
echo "cargo $*" >> "$CI_LOG"
prev=
for a; do
  if [ "$prev" = --output-path ]; then
    printf 'SF:%s\nDA:1,1\nend_of_record\n' "$PWD/crates/clm/src/main.rs" >"$a"
  fi
  prev=$a
done
"#;
/// An `npx` that records its arguments, led by its directory, prints a version with more words
/// after it for `vitest --version`, and for `vitest run` writes `lcov.info` under the
/// `--coverage.reportsDirectory` it is given, with a relative record and an absolute one. It
/// fails that run where `bin/fail-<dir>` exists.
const NPX_COVERAGE: &str = r#"#!/usr/bin/env bash
here=$(basename "$PWD")
echo "$here\$ npx $*" >> "$CI_LOG"
if [ "$*" = "vitest --version" ]; then
  echo "vitest/5.0.5 linux-x64 node-v22.0.0"
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
fn coverage_tree() -> Tree {
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
    // req:ci/coverage-script@dd389308
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
    let install = at(&|l| {
        l.starts_with("cargo install --locked --version ") && l.ends_with(" cargo-llvm-cov")
    });
    let cargo_run = at(&|l| {
        l.starts_with("cargo llvm-cov --workspace --no-rustc-wrapper --lcov --output-path ")
    });
    assert!(
        0 < rustup && rustup < cargo_run && install < cargo_run,
        "{log:?}"
    );
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
    let root = fs::canonicalize(tree.work()).unwrap();
    assert_eq!(
        tracefile_sources(&tree),
        [
            format!("{}/crates/clm/src/main.rs", root.display()),
            "web/src/app.ts".to_owned(),
            format!("{}/web/abs.ts", root.display()),
            "plugin-panel/src/app.ts".to_owned(),
            format!("{}/plugin-panel/abs.ts", root.display()),
        ]
    );
}

#[test]
fn a_failing_coverage_suite_fails_the_script() {
    // req:ci/coverage-script@dd389308
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
}

#[test]
fn nothing_else_runs_coverage() {
    // req:ci/coverage-script@dd389308
    for path in [
        script_path("ci-check.sh"),
        script_path("detector-tools.sh"),
        repo().join(".github/workflows/release.yml"),
    ] {
        let text = fs::read_to_string(&path).unwrap();
        for line in text.lines().filter(|l| !l.trim_start().starts_with('#')) {
            assert!(
                !line.contains("coverage.sh") && !line.contains("llvm-cov"),
                "{}: {line}",
                path.display()
            );
        }
    }
}

#[test]
fn coverage_tools_are_pinned() {
    // req:ci/coverage-script@dd389308
    let lines = script_lines("coverage.sh");
    let tools = lines
        .iter()
        .position(|l| l == ".github/scripts/detector-tools.sh")
        .unwrap_or_else(|| panic!("coverage.sh doesn't run detector-tools.sh: {lines:?}"));
    let cargo = lines
        .iter()
        .position(|l| l.split_whitespace().any(|w| w == "cargo"))
        .unwrap_or_else(|| panic!("no cargo line: {lines:?}"));
    assert!(tools < cargo, "{lines:?}");
    let install = lines
        .iter()
        .find(|l| l.starts_with("cargo install ") && l.ends_with(" cargo-llvm-cov"))
        .unwrap_or_else(|| panic!("no cargo-llvm-cov install: {lines:?}"));
    let words: Vec<&str> = install.split_whitespace().collect();
    assert!(words.contains(&"--locked"), "{install}");
    let version = words
        .iter()
        .position(|w| *w == "--version")
        .and_then(|i| words.get(i + 1))
        .unwrap_or_else(|| panic!("no --version: {install}"));
    let parts: Vec<&str> = version.split('.').collect();
    assert!(
        parts.len() == 3
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())),
        "not an exact x.y.z version: {version}"
    );
    // The instrumentation goes in `RUSTFLAGS`, which the nested wasm builds remove, and a
    // failing test fails the run.
    let llvm_cov = lines
        .iter()
        .find(|l| l.starts_with("cargo llvm-cov "))
        .unwrap_or_else(|| panic!("no cargo llvm-cov line: {lines:?}"));
    assert!(
        llvm_cov
            .split_whitespace()
            .any(|w| w == "--no-rustc-wrapper"),
        "{llvm_cov}"
    );
    assert!(
        !lines.iter().any(|l| l.contains("--ignore-run-fail")),
        "{lines:?}"
    );
    let vitest = lines
        .iter()
        .find(|l| l.contains("@vitest/coverage-v8@"))
        .unwrap_or_else(|| panic!("no @vitest/coverage-v8 install: {lines:?}"));
    assert!(
        vitest.split_whitespace().any(|w| w == "--no-save"),
        "{vitest}"
    );
    let mode = fs::metadata(script_path("coverage.sh"))
        .unwrap()
        .permissions()
        .mode();
    assert!(mode & 0o111 != 0, "coverage.sh isn't executable");
}

#[test]
fn nextest_replaces_cargo_test_but_for_doctests() {
    // req:ci/nextest-runner@dd9030d0
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
    // req:ci/test-tools@b136de2a
    let lines = script_lines("test-tools.sh");
    let installs: Vec<_> = lines
        .iter()
        .filter(|l| l.contains("cargo install"))
        .collect();
    assert_eq!(installs.len(), 1, "{lines:?}");
    let words: Vec<&str> = installs[0].split_whitespace().collect();
    assert!(
        words.starts_with(&["cargo", "install"]) && words.last() == Some(&"cargo-nextest"),
        "{}",
        installs[0]
    );
    assert!(words.contains(&"--locked"), "{}", installs[0]);
    let version = words
        .iter()
        .position(|w| *w == "--version")
        .and_then(|i| words.get(i + 1))
        .unwrap_or_else(|| panic!("no --version: {}", installs[0]));
    let parts: Vec<&str> = version.split('.').collect();
    assert!(
        parts.len() == 3
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())),
        "not an exact x.y.z version: {version}"
    );
    let mode = fs::metadata(script_path("test-tools.sh"))
        .unwrap()
        .permissions()
        .mode();
    assert!(mode & 0o111 != 0, "test-tools.sh isn't executable");
}

#[test]
fn test_tools_run_between_the_detector_tools_and_cargo() {
    // req:ci/test-tools@b136de2a
    let lines = script_lines("ci-check.sh");
    let at = |line: &str| {
        lines
            .iter()
            .position(|l| l == line)
            .unwrap_or_else(|| panic!("ci-check.sh doesn't run {line}: {lines:?}"))
    };
    let detectors = at(".github/scripts/detector-tools.sh");
    let tests = at(".github/scripts/test-tools.sh");
    let cargo = lines
        .iter()
        .position(|l| l.split_whitespace().any(|w| w == "cargo"))
        .unwrap();
    assert!(detectors < tests && tests < cargo, "{lines:?}");
    // And nothing else runs it or installs `cargo-nextest`.
    let github = repo().join(".github");
    for dir in ["workflows", "scripts"] {
        for entry in fs::read_dir(github.join(dir)).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let text = fs::read_to_string(&path).unwrap();
            for line in text.lines().filter(|l| !l.trim_start().starts_with('#')) {
                assert!(
                    !line.contains("test-tools.sh") || name == "ci-check.sh",
                    "{name} runs test-tools.sh: {line}"
                );
                assert!(
                    !(line.contains("cargo install") && line.contains("cargo-nextest"))
                        || name == "test-tools.sh",
                    "{name} installs cargo-nextest: {line}"
                );
            }
        }
    }
}

#[test]
fn the_release_builds_no_nextest() {
    // req:ci/test-tools@b136de2a
    for path in [
        repo().join(".github/workflows/release.yml"),
        script_path("release-prepare.sh"),
    ] {
        let text = fs::read_to_string(&path).unwrap();
        assert!(
            !text.contains("test-tools.sh") && !text.contains("nextest"),
            "{}",
            path.display()
        );
    }
}

#[test]
fn a_full_run_times_every_phase_it_runs() {
    // req:ci/phase-times@e8375a0b
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
    // req:ci/phase-times@e8375a0b
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
    // req:ci/phase-times@e8375a0b
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
    // req:ci/land-dir@23d7af00
    let calls = cargo_variables(Some("/tmp/x/land"), None);
    assert!(calls.iter().all(|c| c == "/tmp/x/land 0"), "{calls:?}");
}

#[test]
fn without_a_land_place_the_cargo_variables_stay() {
    // req:ci/land-dir@23d7af00
    for land_dir in [None, Some("")] {
        let calls = cargo_variables(land_dir, Some("/tmp/y"));
        assert!(calls.iter().all(|c| c == "/tmp/y unset"), "{calls:?}");
    }
}
