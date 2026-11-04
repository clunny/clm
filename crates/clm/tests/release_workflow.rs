//! Proves the release workflow's shape, and runs `.github/scripts/release-prepare.sh` against
//! a fixture workspace whose `origin` is a local bare repository, with a fake `gh`, because the
//! workflow itself only runs on GitHub.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_yaml::Value;
use tempfile::TempDir;

#[path = "../../clm-git/tests/exec/mod.rs"]
mod exec;
use exec::write_exec as executable;

/// The build matrix's targets, each with the runner it builds on and the binary it packs.
const TARGETS: [(&str, &str, &str); 5] = [
    ("x86_64-unknown-linux-musl", "ubuntu-latest", "clm"),
    ("aarch64-unknown-linux-musl", "ubuntu-latest", "clm"),
    ("aarch64-apple-darwin", "macos-latest", "clm"),
    ("x86_64-apple-darwin", "macos-latest", "clm"),
    ("x86_64-pc-windows-msvc", "windows-latest", "clm.exe"),
];

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn workflow() -> Value {
    let path = repo().join(".github/workflows/release.yml");
    serde_yaml::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn steps<'a>(wf: &'a Value, job: &str) -> &'a [Value] {
    wf["jobs"][job]["steps"]
        .as_sequence()
        .unwrap_or_else(|| panic!("no steps in job `{job}`"))
}

/// Index of the first step in `job` whose `run` contains `text`.
fn step_running(wf: &Value, job: &str, text: &str) -> usize {
    steps(wf, job)
        .iter()
        .position(|s| s["run"].as_str().is_some_and(|r| r.contains(text)))
        .unwrap_or_else(|| panic!("no step in `{job}` runs `{text}`"))
}

fn needs(wf: &Value, job: &str) -> Vec<String> {
    match &wf["jobs"][job]["needs"] {
        Value::String(s) => vec![s.clone()],
        Value::Sequence(s) => s.iter().map(|v| v.as_str().unwrap().to_owned()).collect(),
        _ => vec![],
    }
}

#[test]
fn triggers_only_on_dispatch_with_a_required_version() {
    // req:ci/release@dc99ac5d
    let wf = workflow();
    let on = wf["on"].as_mapping().unwrap();
    let triggers: Vec<_> = on.keys().map(|k| k.as_str().unwrap()).collect();
    assert_eq!(triggers, ["workflow_dispatch"]);
    let version = &wf["on"]["workflow_dispatch"]["inputs"]["version"];
    assert_eq!(version["required"].as_bool(), Some(true));
}

#[test]
fn may_write_contents_and_never_runs_twice_at_once() {
    // req:ci/release@dc99ac5d
    let wf = workflow();
    assert_eq!(wf["permissions"]["contents"].as_str(), Some("write"));
    assert_eq!(wf["concurrency"]["group"].as_str(), Some("release"));
    assert_eq!(
        wf["concurrency"]["cancel-in-progress"].as_bool(),
        Some(false)
    );
}

#[test]
fn prepare_runs_the_script_that_bumps_both_versions() {
    // req:ci/release@dc99ac5d
    let wf = workflow();
    step_running(&wf, "prepare", ".github/scripts/release-prepare.sh");
    let outputs = wf["jobs"]["prepare"]["outputs"].as_mapping().unwrap();
    assert!(outputs.contains_key("sha") && outputs.contains_key("bumped"));
    let script = fs::read_to_string(script()).unwrap();
    assert!(script.contains("Cargo.toml"));
    assert!(script.contains("plugin/.claude-plugin/plugin.json"));
}

#[test]
fn prepare_installs_detector_tools_before_the_tests() {
    // req:ci/detector-tools@156131ed
    let wf = workflow();
    let toolchain = step_running(&wf, "prepare", "rustup show");
    let tools = step_running(&wf, "prepare", ".github/scripts/detector-tools.sh");
    let prepare = step_running(&wf, "prepare", ".github/scripts/release-prepare.sh");
    assert!(
        toolchain < tools && tools < prepare,
        "steps: toolchain {toolchain}, detector tools {tools}, prepare {prepare}"
    );
}

/// A step's `run` with the matrix expressions filled in for one target, as a runner sees it.
fn expand(run: &str, target: &str, exe: &str) -> String {
    run.replace("${{ matrix.target }}", target)
        .replace("${{ matrix.exe }}", exe)
}

/// Runs `script` with `bash -eo pipefail` in `dir`, with `bin` first on `PATH`.
fn bash_in(dir: &Path, bin: &Path, script: &str, env: &[(&str, &str)]) -> std::process::Output {
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    Command::new("bash")
        .args(["-eo", "pipefail", "-c", script])
        .current_dir(dir)
        .env("PATH", path)
        .envs(env.iter().copied())
        .output()
        .unwrap()
}

/// Whether `version` is an exact `x.y.z`.
fn exact(version: &str) -> bool {
    let parts: Vec<_> = version.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

#[test]
fn builds_the_five_targets_from_the_prepared_commit() {
    // req:ci/release@dc99ac5d
    let wf = workflow();
    assert_eq!(needs(&wf, "build"), ["prepare"]);
    let build = &wf["jobs"]["build"];
    let include = build["strategy"]["matrix"]["include"]
        .as_sequence()
        .unwrap();
    let matrix: Vec<_> = include
        .iter()
        .map(|t| {
            (
                t["target"].as_str().unwrap(),
                t["os"].as_str().unwrap(),
                t["exe"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(matrix, TARGETS);
    assert_eq!(build["runs-on"].as_str(), Some("${{ matrix.os }}"));
    // Git Bash on Windows, so every target packs with the same script.
    assert_eq!(build["defaults"]["run"]["shell"].as_str(), Some("bash"));

    let build_steps = steps(&wf, "build");
    let checkout = build_steps
        .iter()
        .position(|s| {
            s["uses"]
                .as_str()
                .is_some_and(|u| u.starts_with("actions/checkout"))
        })
        .unwrap();
    assert!(
        build_steps[checkout]["with"]["ref"]
            .as_str()
            .is_some_and(|r| r.contains("needs.prepare.outputs.sha"))
    );
    let native = step_running(&wf, "build", "cargo build --release");
    let zig = step_running(&wf, "build", "cargo zigbuild --release");
    for i in [native, zig] {
        let run = build_steps[i]["run"].as_str().unwrap();
        assert!(run.contains("--locked") && run.contains("-p clm"), "{run}");
        assert!(run.contains("--target ${{ matrix.target }}"), "{run}");
    }
    // Linux builds with Zig, everything else with the runner's own compiler.
    let musl = "endsWith(matrix.target, '-musl')";
    assert_eq!(build_steps[zig]["if"].as_str(), Some(musl));
    assert_eq!(
        build_steps[native]["if"].as_str(),
        Some(format!("!{musl}").as_str())
    );
    let tools = step_running(&wf, "build", "cargo-zigbuild");
    assert_eq!(build_steps[tools]["if"].as_str(), Some(musl));
    let run = build_steps[tools]["run"].as_str().unwrap();
    let zig_line = run.lines().find(|l| l.contains("pip install")).unwrap();
    let zig_version = zig_line.split("ziglang==").nth(1).unwrap_or("");
    assert!(
        exact(zig_version.split_whitespace().next().unwrap_or("")),
        "{zig_line}"
    );
    let install = run
        .lines()
        .find(|l| l.contains("cargo install"))
        .unwrap()
        .trim();
    assert!(install.contains("--locked"), "{install}");
    let version = install
        .split_whitespace()
        .skip_while(|w| *w != "--version")
        .nth(1);
    assert!(
        version.is_some_and(exact),
        "not an exact version: {install}"
    );
    assert!(install.ends_with(" cargo-zigbuild"), "{install}");

    // The pinned toolchain first, then the matrix target on top of it, from the checkout,
    // so the target goes on the toolchain `rust-toolchain.toml` names.
    let show = build_steps
        .iter()
        .position(|s| s["run"].as_str().is_some_and(|r| r.trim() == "rustup show"))
        .expect("a `rustup show` step");
    let target = step_running(&wf, "build", "rustup target add ${{ matrix.target }}");
    assert!(
        checkout < show && show < target && target < tools && tools < zig && target < native,
        "{checkout} {show} {target} {tools} {zig} {native}"
    );
    assert!(build_steps[target]["working-directory"].is_null());
    assert!(build["defaults"]["run"]["working-directory"].is_null());
    assert!(wf["defaults"]["run"]["working-directory"].is_null());
    // Nothing from apt, and no `-gnu` target or linker anywhere.
    let text = fs::read_to_string(repo().join(".github/workflows/release.yml")).unwrap();
    assert!(!text.contains("apt-get"), "{text}");
    assert!(!text.contains("-gnu"), "{text}");
}

/// Runs the static build step for `x86_64-unknown-linux-musl`, with a fake `cargo` that
/// builds nothing and a fake `file` that reports `report`.
fn static_build(report: &str) -> std::process::Output {
    let wf = workflow();
    let i = step_running(&wf, "build", "cargo zigbuild --release");
    let target = "x86_64-unknown-linux-musl";
    let run = expand(
        steps(&wf, "build")[i]["run"].as_str().unwrap(),
        target,
        "clm",
    );
    let dir = TempDir::new().unwrap();
    let bin = dir.path().join("bin");
    let work = dir.path().join("work");
    let release = work.join(format!("target/{target}/release"));
    fs::create_dir_all(&release).unwrap();
    fs::write(release.join("clm"), "").unwrap();
    fs::create_dir_all(&bin).unwrap();
    executable(&bin.join("cargo"), "#!/bin/sh\nexit 0\n");
    executable(
        &bin.join("file"),
        &format!("#!/bin/sh\necho \"$1: ELF 64-bit LSB executable, {report}, stripped\"\n"),
    );
    bash_in(&work, &bin, &run, &[])
}

#[test]
fn linux_binaries_must_be_statically_linked() {
    // req:ci/release@dc99ac5d
    for report in ["static-pie linked", "statically linked"] {
        let out = static_build(report);
        assert!(out.status.success(), "{report}: {out:?}");
    }
    let out = static_build("dynamically linked, interpreter /lib/ld-musl-x86_64.so.1");
    assert!(!out.status.success(), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("isn't statically linked"), "{stderr}");
}

#[test]
fn packs_and_passes_on_both_assets_per_target() {
    // req:ci/release@dc99ac5d
    let wf = workflow();
    let i = step_running(&wf, "build", "tar czf");
    let pack = &steps(&wf, "build")[i];
    assert_eq!(
        pack["env"]["ASSET"].as_str(),
        Some("clm-${{ matrix.target }}.tar.gz")
    );
    // Each target's archive holds just its binary, `clm.exe` on Windows, and its checksum
    // file checks the archive.
    for (target, _, exe) in TARGETS {
        let dir = TempDir::new().unwrap();
        let work = dir.path().join("work");
        let release = work.join(format!("target/{target}/release"));
        fs::create_dir_all(&release).unwrap();
        fs::write(release.join(exe), "binary").unwrap();
        fs::write(release.join("clm.d"), "").unwrap();
        let asset = format!("clm-{target}.tar.gz");
        let run = expand(pack["run"].as_str().unwrap(), target, exe);
        let out = bash_in(&work, &dir.path().join("none"), &run, &[("ASSET", &asset)]);
        assert!(out.status.success(), "{target}: {out:?}");
        let list = Command::new("tar")
            .args(["tzf", &asset])
            .current_dir(&work)
            .output()
            .unwrap();
        let list = String::from_utf8_lossy(&list.stdout);
        assert_eq!(list.trim(), exe, "{target}");
        // As the shim checks it: `sha256sum`, or `shasum` where there is none (macOS).
        let check = Command::new("sh")
            .args([
                "-c",
                r#"if command -v sha256sum >/dev/null; then sha256sum -c "$1"; else shasum -a 256 -c "$1"; fi"#,
                "sh",
                &format!("{asset}.sha256"),
            ])
            .current_dir(&work)
            .output()
            .unwrap();
        assert!(check.status.success(), "{target}: {check:?}");
    }
    let upload = steps(&wf, "build")
        .iter()
        .find(|s| {
            s["uses"]
                .as_str()
                .is_some_and(|u| u.starts_with("actions/upload-artifact"))
        })
        .unwrap();
    let path = upload["with"]["path"].as_str().unwrap();
    assert!(path.contains("clm-${{ matrix.target }}.tar.gz\n"), "{path}");
    assert!(
        path.contains("clm-${{ matrix.target }}.tar.gz.sha256"),
        "{path}"
    );

    let download = steps(&wf, "publish")
        .iter()
        .find(|s| {
            s["uses"]
                .as_str()
                .is_some_and(|u| u.starts_with("actions/download-artifact"))
        })
        .unwrap();
    assert_eq!(download["with"]["pattern"].as_str(), Some("clm-*"));
    assert_eq!(download["with"]["merge-multiple"].as_bool(), Some(true));
    assert_eq!(download["with"]["path"].as_str(), Some("assets"));
}

#[test]
fn publishes_the_release_before_pushing_main() {
    // req:ci/release@dc99ac5d
    let wf = workflow();
    let n = needs(&wf, "publish");
    assert!(n.iter().any(|j| j == "build") && n.iter().any(|j| j == "prepare"));
    let create = step_running(&wf, "publish", "gh release create");
    let push = step_running(&wf, "publish", "git push origin main");
    assert!(create < push, "release create must come before the push");

    let release = &steps(&wf, "publish")[create];
    let run = release["run"].as_str().unwrap();
    assert!(
        run.contains(r#"gh release create "v$VERSION" --target "$SHA" --generate-notes assets/*"#),
        "{run}"
    );
    assert_eq!(
        release["env"]["SHA"].as_str(),
        Some("${{ needs.prepare.outputs.sha }}")
    );
    // A failed upload leaves a draft, which would block dispatching the version again.
    assert!(run.contains(r#"gh release delete "v$VERSION""#), "{run}");

    let merge = &steps(&wf, "publish")[push];
    assert_eq!(
        merge["if"].as_str(),
        Some("needs.prepare.outputs.bumped == 'true'")
    );
    for s in steps(&wf, "publish") {
        assert!(s["continue-on-error"].is_null(), "{s:?}");
        assert!(
            !s["if"].as_str().unwrap_or("").contains("always()"),
            "{s:?}"
        );
    }
}

#[test]
fn merge_retries_then_prints_the_commands_for_a_human() {
    // req:ci/release@dc99ac5d
    let wf = workflow();
    let i = step_running(&wf, "publish", "git push origin main");
    let run = steps(&wf, "publish")[i]["run"].as_str().unwrap();
    assert!(run.contains("for try in 1 2 3"), "{run}");
    assert!(
        run.contains(r#"git merge --no-edit -m "Release v$VERSION" "origin/$branch""#),
        "{run}"
    );
    // Deleting the branch after main is pushed mustn't fail the job.
    assert!(
        run.contains(r#"git push origin --delete "$branch" ||"#),
        "{run}"
    );
    let fallback = &run[run.rfind("done").unwrap()..];
    assert!(fallback.contains("git push origin main"), "{fallback}");
    assert!(fallback.trim_end().ends_with("exit 1"), "{fallback}");
}

#[test]
fn deploys_web_after_publishing() {
    // req:ci/web-deploy@0a3e4888
    let wf = workflow();
    // After `publish`, so a failed deploy doesn't undo the release.
    assert!(needs(&wf, "web").iter().any(|j| j == "publish"));
    // Whenever the release was created, even if `publish` failed later bringing the bump
    // into main. A job's `if` can't call `hashFiles`.
    let cond = wf["jobs"]["web"]["if"].as_str().unwrap_or("");
    assert_eq!(
        cond,
        "${{ !cancelled() && needs.publish.outputs.released == 'true' }}"
    );
    assert_eq!(
        wf["jobs"]["publish"]["outputs"]["released"].as_str(),
        Some("${{ steps.release.outputs.released }}")
    );
    let release = &steps(&wf, "publish")[step_running(&wf, "publish", "gh release create")];
    assert_eq!(release["id"].as_str(), Some("release"));
    let run = release["run"].as_str().unwrap();
    assert!(
        run.trim_end()
            .ends_with(r#"echo "released=true" >> "$GITHUB_OUTPUT""#),
        "{run}"
    );
    // Every step runs: none is skipped by a condition.
    let web = steps(&wf, "web");
    let checkout = web
        .iter()
        .position(|s| s["uses"].as_str() == Some("actions/checkout@v5"))
        .expect("checkout step");
    for s in web {
        assert!(s["if"].is_null(), "{s:?}");
    }
    let checkout = &web[checkout];
    assert_eq!(
        checkout["with"]["ref"].as_str(),
        Some("${{ needs.prepare.outputs.sha }}")
    );
    let deploy = &steps(&wf, "web")[step_running(&wf, "web", "wrangler deploy")];
    assert_eq!(deploy["working-directory"].as_str(), Some("web"));
    for name in ["CLOUDFLARE_API_TOKEN", "CLOUDFLARE_ACCOUNT_ID"] {
        assert_eq!(
            deploy["env"][name].as_str(),
            Some(format!("${{{{ secrets.{name} }}}}").as_str()),
            "{name}"
        );
    }
    // npm runs no dependency's install script (`unattended-commands`).
    let install = &steps(&wf, "web")[step_running(&wf, "web", "npm ci")];
    assert!(
        install["run"]
            .as_str()
            .unwrap()
            .contains("--ignore-scripts"),
        "{install:?}"
    );
}

#[test]
fn web_deploy_reads_only_the_cloudflare_secrets() {
    // req:ci/web-deploy@0a3e4888
    let text = serde_yaml::to_string(&workflow()["jobs"]["web"]).unwrap();
    let read: std::collections::BTreeSet<_> = text
        .match_indices("secrets.")
        .map(|(i, m)| {
            text[i + m.len()..]
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .next()
                .unwrap()
        })
        .collect();
    assert_eq!(
        read.into_iter().collect::<Vec<_>>(),
        ["CLOUDFLARE_ACCOUNT_ID", "CLOUDFLARE_API_TOKEN"]
    );
}

#[test]
fn no_run_step_interpolates_the_version_input() {
    // req:ci/release@dc99ac5d
    let wf = workflow();
    for (job, def) in wf["jobs"].as_mapping().unwrap() {
        for s in def["steps"].as_sequence().into_iter().flatten() {
            let run = s["run"].as_str().unwrap_or("");
            assert!(
                !run.contains("inputs.version"),
                "job {job:?} interpolates inputs.version in `run`"
            );
        }
    }
}

/// Each action a workflow may use, with its first major that runs on Node.js 24.
const NODE24_ACTIONS: [(&str, u32); 4] = [
    ("actions/checkout", 5),
    ("actions/upload-artifact", 6),
    ("actions/download-artifact", 7),
    ("actions/setup-node", 5),
];

/// The `uses:` values of a workflow's text that don't name a listed action at or above its
/// Node.js 24 major: an action not in the list, or one at an older or unreadable version.
fn node20_actions(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| {
            let l = l.trim_start().trim_start_matches("- ").trim_start();
            l.strip_prefix("uses:")
        })
        .map(|u| u.trim().trim_matches(['"', '\'']).to_owned())
        .filter(|u| {
            let Some((action, version)) = u.split_once('@') else {
                return true;
            };
            let major = version
                .strip_prefix('v')
                .and_then(|v| v.split('.').next())
                .and_then(|m| m.parse::<u32>().ok());
            !NODE24_ACTIONS
                .iter()
                .any(|(a, min)| *a == action && major.is_some_and(|m| m >= *min))
        })
        .collect()
}

#[test]
fn uses_only_node24_actions() {
    // req:ci/node24-actions@8798286f
    for entry in fs::read_dir(repo().join(".github/workflows")).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("uses:"), "{}", path.display());
        assert_eq!(
            node20_actions(&text),
            Vec::<String>::new(),
            "{}",
            path.display()
        );
    }
}

#[test]
fn node20_action_fails_naming_it() {
    // req:ci/node24-actions@8798286f
    let text = fs::read_to_string(repo().join(".github/workflows/release.yml")).unwrap();
    let checkout = text
        .lines()
        .find(|l| l.contains("uses: actions/checkout@"))
        .unwrap();
    let old = text.replacen(checkout, "      - uses: actions/checkout@v4", 1);
    assert_eq!(node20_actions(&old), ["actions/checkout@v4"]);
    // Every Node.js 20 major, an unlisted action and a version that isn't a major fail too.
    for uses in [
        "actions/upload-artifact@v5",
        "actions/download-artifact@v6",
        "actions/setup-node@v4",
        "actions/cache@v9",
        "actions/checkout@main",
        "actions/checkout",
    ] {
        let text = format!("steps:\n  - uses: {uses}\n  - uses: actions/checkout@v6\n");
        assert_eq!(node20_actions(&text), [uses], "{uses}");
    }
}

fn script() -> PathBuf {
    repo().join(".github/scripts/release-prepare.sh")
}

/// Logs every call; answers `release list` from `releases`, and fails if `gh-fails` exists.
const FAKE_GH: &str = r#"#!/usr/bin/env bash
dir=$(dirname "$0")
echo "$*" >> "$dir/gh.log"
[ -e "$dir/gh-fails" ] && exit 1
if [ "$1 $2" = "release list" ]; then cat "$dir/releases"; fi
exit 0
"#;

/// A git repository whose `origin` is a local bare repository with `main` pushed.
struct Fixture {
    dir: TempDir,
}

/// What a run of the script did.
struct Run {
    ok: bool,
    stderr: String,
    outputs: BTreeMap<String, String>,
    gh_log: String,
    refs_before: String,
    refs_after: String,
}

impl Fixture {
    fn new(version: &str) -> Self {
        let dir = TempDir::new().unwrap();
        let f = Fixture { dir };
        let work = f.work();
        fs::create_dir_all(work.join("m/src")).unwrap();
        for dir in [
            "plugin/.claude-plugin",
            "plugin/bin",
            "plugin-panel/.claude-plugin",
            ".github/workflows",
        ] {
            fs::create_dir_all(work.join(dir)).unwrap();
        }
        // The real workflow, so its build matrix is the one the shim is checked against,
        // and a shim that maps a platform to each of its targets.
        fs::copy(
            repo().join(".github/workflows/release.yml"),
            work.join(".github/workflows/release.yml"),
        )
        .unwrap();
        f.shim(&TARGETS.map(|(t, ..)| t));
        fs::write(
            work.join("plugin-panel/.claude-plugin/plugin.json"),
            format!("{{\n  \"name\": \"clm-panel\",\n  \"version\": \"{version}\"\n}}\n"),
        )
        .unwrap();
        fs::write(
            work.join("Cargo.toml"),
            format!(
                "[workspace]\nresolver = \"2\"\nmembers = [\"m\"]\n\n\
                 [workspace.package]\nedition = \"2024\"\nversion = \"{version}\"\n\n\
                 [workspace.metadata.fixture]\nversion = \"9.9.9\"\n"
            ),
        )
        .unwrap();
        fs::write(
            work.join("m/Cargo.toml"),
            "[package]\nname = \"m\"\nversion.workspace = true\nedition.workspace = true\n",
        )
        .unwrap();
        fs::write(work.join("m/src/lib.rs"), "#[test]\nfn passes() {}\n").unwrap();
        fs::write(
            work.join("plugin/.claude-plugin/plugin.json"),
            format!(
                "{{\n  \"name\": \"clm\",\n  \"version\": \"{version}\",\n  \"author\": {{ \"name\": \"x\", \"version\": \"keep\" }}\n}}\n"
            ),
        )
        .unwrap();
        fs::write(work.join(".gitignore"), "/target\n").unwrap();
        f.cargo(&["generate-lockfile", "--offline"]);

        let bin = f.bin();
        fs::create_dir_all(&bin).unwrap();
        executable(&bin.join("gh"), FAKE_GH);
        fs::write(bin.join("releases"), "").unwrap();
        fs::write(bin.join("gh.log"), "").unwrap();

        f.git(&work, &["init", "-q", "-b", "main"]);
        f.git(&work, &["add", "-A"]);
        f.git(&work, &["commit", "-q", "-m", "init"]);
        let bare = f.bare();
        f.git(
            f.dir.path(),
            &["init", "-q", "--bare", bare.to_str().unwrap()],
        );
        f.git(&work, &["remote", "add", "origin", bare.to_str().unwrap()]);
        f.git(&work, &["push", "-q", "origin", "main"]);
        f
    }

    fn work(&self) -> PathBuf {
        self.dir.path().join("work")
    }

    /// Writes `plugin/bin/clm` as a shim whose `case` maps a platform to each of `targets`,
    /// each as one `target=<target>` assignment, as the real shim writes them.
    fn shim(&self, targets: &[&str]) {
        let arms: String = targets
            .iter()
            .enumerate()
            .map(|(i, t)| format!("    Platform{i}) target={t} ;;\n"))
            .collect();
        fs::write(
            self.work().join("plugin/bin/clm"),
            format!(
                "#!/bin/sh\ncase \"$(uname -s)-$(uname -m)\" in\n{arms}    *) exit 1 ;;\nesac\n\
                 asset=\"clm-$target.tar.gz\"\n"
            ),
        )
        .unwrap();
    }

    /// Commits the work tree and pushes it to `origin`'s `main`.
    fn commit(&self, message: &str) {
        let work = self.work();
        self.git(&work, &["add", "-A"]);
        self.git(&work, &["commit", "-q", "-m", message]);
        self.git(&work, &["push", "-q", "origin", "main"]);
    }

    fn bare(&self) -> PathBuf {
        self.dir.path().join("remote.git")
    }

    fn bin(&self) -> PathBuf {
        self.dir.path().join("bin")
    }

    fn command(&self, program: impl AsRef<std::ffi::OsStr>, cwd: &Path) -> Command {
        let mut c = Command::new(program);
        c.current_dir(cwd)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("CARGO_TARGET_DIR", self.dir.path().join("target"))
            .env("CARGO_NET_OFFLINE", "true");
        c
    }

    fn git(&self, cwd: &Path, args: &[&str]) -> String {
        let out = self
            .command("git", cwd)
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}: {out:?}");
        String::from_utf8(out.stdout).unwrap()
    }

    fn cargo(&self, args: &[&str]) {
        let out = self
            .command("cargo", &self.work())
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "cargo {args:?}: {out:?}");
    }

    fn releases(&self, tags: &[&str]) {
        fs::write(self.bin().join("releases"), tags.join("\n") + "\n").unwrap();
    }

    fn head(&self) -> String {
        self.git(&self.work(), &["rev-parse", "HEAD"])
            .trim()
            .to_owned()
    }

    fn remote_refs(&self) -> String {
        self.git(&self.work(), &["ls-remote", self.bare().to_str().unwrap()])
    }

    fn run_on(&self, git_ref: &str, version: &str) -> Run {
        let out_file = self.dir.path().join("github_output");
        fs::write(&out_file, "").unwrap();
        let refs_before = self.remote_refs();
        let path = format!(
            "{}:{}",
            self.bin().display(),
            std::env::var("PATH").unwrap()
        );
        let out = self
            .command(script(), &self.work())
            .arg(version)
            .env("PATH", path)
            .env("GITHUB_REF", git_ref)
            .env("GITHUB_SHA", self.head())
            .env("GITHUB_OUTPUT", &out_file)
            .output()
            .unwrap();
        let outputs = fs::read_to_string(&out_file)
            .unwrap()
            .lines()
            .filter_map(|l| l.split_once('='))
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        Run {
            ok: out.status.success(),
            stderr: String::from_utf8(out.stderr).unwrap(),
            outputs,
            gh_log: fs::read_to_string(self.bin().join("gh.log")).unwrap(),
            refs_before,
            refs_after: self.remote_refs(),
        }
    }

    fn run(&self, version: &str) -> Run {
        self.run_on("refs/heads/main", version)
    }
}

/// Fails, publishes nothing and pushes nothing.
fn assert_refused(r: &Run) {
    assert!(!r.ok, "expected failure; stderr: {}", r.stderr);
    assert!(!r.stderr.trim().is_empty(), "a failure names its reason");
    assert!(!r.gh_log.contains("release create"), "{}", r.gh_log);
    assert_eq!(r.refs_before, r.refs_after, "pushed something");
}

#[test]
fn lower_version_fails_and_pushes_nothing() {
    // req:ci/release@dc99ac5d
    let f = Fixture::new("0.2.0");
    assert_refused(&f.run("0.1.0"));
    let f = Fixture::new("0.10.0");
    assert_refused(&f.run("0.9.0"));
}

#[test]
fn existing_release_fails_and_pushes_nothing() {
    // req:ci/release@dc99ac5d
    let f = Fixture::new("0.1.0");
    f.releases(&["v0.1.0", "v0.2.0"]);
    let r = f.run("0.2.0");
    assert_refused(&r);
    assert!(r.gh_log.contains("release list"), "{}", r.gh_log);
}

#[test]
fn failing_gh_fails_closed() {
    // req:ci/release@dc99ac5d
    let f = Fixture::new("0.1.0");
    fs::write(f.bin().join("gh-fails"), "").unwrap();
    assert_refused(&f.run("0.1.0"));
}

#[test]
fn existing_tag_fails_and_pushes_nothing() {
    // req:ci/release@dc99ac5d
    let f = Fixture::new("0.1.0");
    let bare = f.bare();
    f.git(&bare, &["tag", "v0.2.0", "main"]);
    assert_refused(&f.run("0.2.0"));

    let f = Fixture::new("0.1.0");
    f.git(&f.work(), &["tag", "v0.2.0"]);
    assert_refused(&f.run("0.2.0"));
}

#[test]
fn only_main_can_release() {
    // req:ci/release@dc99ac5d
    let f = Fixture::new("0.1.0");
    assert_refused(&f.run_on("refs/heads/feature", "0.1.0"));
    assert_refused(&f.run_on("refs/tags/v0.1.0", "0.1.0"));
}

#[test]
fn version_must_be_major_minor_patch() {
    // req:ci/release@dc99ac5d
    for bad in [
        "1.2",
        "v1.2.3",
        "1.2.3-rc1",
        "1.2.3.4",
        "",
        "0.2.0+1",
        "0.01.0",
        "00.2.0",
    ] {
        let f = Fixture::new("0.1.0");
        let r = f.run(bad);
        assert_refused(&r);
        assert!(
            r.stderr.contains("MAJOR.MINOR.PATCH"),
            "{bad}: {}",
            r.stderr
        );
    }
}

#[test]
fn same_version_releases_the_dispatched_commit() {
    // req:ci/release@dc99ac5d
    let f = Fixture::new("0.1.0");
    f.releases(&["v0.0.9", "v0.1.0-rc1"]);
    let r = f.run("0.1.0");
    assert!(r.ok, "{}", r.stderr);
    assert_eq!(r.outputs.get("sha"), Some(&f.head()));
    assert_eq!(r.outputs.get("bumped").map(String::as_str), Some("false"));
    assert_eq!(r.refs_before, r.refs_after);
}

#[test]
fn shim_naming_an_unbuilt_asset_fails_naming_it() {
    // req:ci/release@dc99ac5d
    let f = Fixture::new("0.1.0");
    let mut targets = TARGETS.map(|(t, ..)| t).to_vec();
    targets[0] = "x86_64-unknown-linux-gnu";
    f.shim(&targets);
    f.commit("shim asks for a -gnu asset");
    let r = f.run("0.2.0");
    assert_refused(&r);
    assert!(
        r.stderr.contains("clm-x86_64-unknown-linux-gnu.tar.gz"),
        "{}",
        r.stderr
    );
    // It fails before asking about releases, so nothing is built for it either.
    assert!(!r.gh_log.contains("release list"), "{}", r.gh_log);
    // The same version fails too: there is nothing to bump, but the assets are still wrong.
    assert_refused(&f.run("0.1.0"));
}

#[test]
fn shim_with_no_targets_fails() {
    // req:ci/release@dc99ac5d
    let f = Fixture::new("0.1.0");
    f.shim(&[]);
    f.commit("shim maps nothing");
    let r = f.run("0.1.0");
    assert_refused(&r);
    assert!(r.stderr.contains("plugin/bin/clm"), "{}", r.stderr);
}

#[test]
fn shim_matching_the_matrix_passes_whatever_its_order() {
    // req:ci/release@dc99ac5d
    let f = Fixture::new("0.1.0");
    // Windows on Arm runs the x86_64 binary: two platforms, one target.
    let mut targets = TARGETS.map(|(t, ..)| t).to_vec();
    targets.reverse();
    targets.push("x86_64-pc-windows-msvc");
    f.shim(&targets);
    f.commit("reorder");
    let r = f.run("0.1.0");
    assert!(r.ok, "{}", r.stderr);
}

/// A file as the remote's `release/v0.2.0` holds it.
fn show_at(f: &Fixture, path: &str) -> String {
    f.git(&f.bare(), &["show", &format!("release/v0.2.0:{path}")])
}

/// `[workspace.package] version` and the metadata version that must not change.
fn cargo_versions(toml: &str) -> Vec<&str> {
    toml.lines()
        .filter_map(|l| l.strip_prefix("version = "))
        .collect()
}

#[test]
fn higher_version_pushes_one_bump_commit_to_a_release_branch() {
    // req:ci/release@dc99ac5d
    let f = Fixture::new("0.1.0");
    f.releases(&["v0.1.0", "v0.2.0-rc1"]);
    let main_before = f.head();
    let r = f.run("0.2.0");
    assert!(r.ok, "{}", r.stderr);
    assert_eq!(r.outputs.get("bumped").map(String::as_str), Some("true"));

    let bare = f.bare();
    let main = f.git(&bare, &["rev-parse", "main"]).trim().to_owned();
    assert_eq!(main, main_before, "main moved");
    let branch = f
        .git(&bare, &["rev-parse", "release/v0.2.0"])
        .trim()
        .to_owned();
    assert_eq!(r.outputs.get("sha"), Some(&branch));
    let ahead = f.git(&bare, &["rev-list", "--count", "main..release/v0.2.0"]);
    assert_eq!(ahead.trim(), "1");
    assert_eq!(
        f.git(&bare, &["log", "-1", "--format=%s|%an", "release/v0.2.0"])
            .trim(),
        "Release v0.2.0|github-actions[bot]"
    );
    let files = f.git(&bare, &["diff", "--name-only", "main", "release/v0.2.0"]);
    let mut files: Vec<_> = files.lines().collect();
    files.sort();
    assert_eq!(
        files,
        [
            "Cargo.lock",
            "Cargo.toml",
            "plugin-panel/.claude-plugin/plugin.json",
            "plugin/.claude-plugin/plugin.json",
        ]
    );
    let panel: serde_json::Value =
        serde_json::from_str(&show_at(&f, "plugin-panel/.claude-plugin/plugin.json")).unwrap();
    assert_eq!(panel["version"], "0.2.0");

    let show = |path: &str| f.git(&bare, &["show", &format!("release/v0.2.0:{path}")]);
    assert_eq!(
        cargo_versions(&show("Cargo.toml")),
        ["\"0.2.0\"", "\"9.9.9\""]
    );
    assert!(
        show("Cargo.lock").contains("name = \"m\"\nversion = \"0.2.0\""),
        "{}",
        show("Cargo.lock")
    );
    let plugin: serde_json::Value =
        serde_json::from_str(&show("plugin/.claude-plugin/plugin.json")).unwrap();
    assert_eq!(plugin["version"], "0.2.0");
    assert_eq!(plugin["author"]["version"], "keep");
    assert!(!r.gh_log.contains("release create"), "{}", r.gh_log);
}

#[test]
fn failing_tests_push_no_bump() {
    // req:ci/release@dc99ac5d
    let f = Fixture::new("0.1.0");
    fs::write(
        f.work().join("m/src/lib.rs"),
        "#[test]\nfn fails() { panic!(\"broken\") }\n",
    )
    .unwrap();
    f.git(&f.work(), &["commit", "-q", "-am", "break"]);
    f.git(&f.work(), &["push", "-q", "origin", "main"]);
    let r = f.run("0.2.0");
    assert_refused(&r);
    assert!(r.stderr.contains("test failed"), "{}", r.stderr);
}

#[test]
fn bump_replaces_an_earlier_failed_runs_branch() {
    // req:ci/release@dc99ac5d
    let f = Fixture::new("0.1.0");
    fs::write(f.work().join("stray"), "left by an earlier run\n").unwrap();
    f.git(&f.work(), &["add", "stray"]);
    f.git(&f.work(), &["commit", "-q", "-m", "stray"]);
    f.git(
        &f.work(),
        &["push", "-q", "origin", "HEAD:refs/heads/release/v0.2.0"],
    );
    f.git(&f.work(), &["reset", "-q", "--hard", "HEAD~1"]);

    let r = f.run("0.2.0");
    assert!(r.ok, "{}", r.stderr);
    let bare = f.bare();
    let ahead = f.git(&bare, &["rev-list", "--count", "main..release/v0.2.0"]);
    assert_eq!(ahead.trim(), "1");
    let files = f.git(&bare, &["ls-tree", "--name-only", "release/v0.2.0"]);
    assert!(!files.lines().any(|l| l == "stray"), "{files}");
}

#[test]
fn a_version_edit_that_changes_nothing_fails() {
    // req:ci/release@dc99ac5d
    let f = Fixture::new("0.1.0");
    fs::write(
        f.work().join("plugin/.claude-plugin/plugin.json"),
        "{ \"name\": \"clm\", \"version\" : \"0.1.0\" }\n",
    )
    .unwrap();
    f.git(&f.work(), &["commit", "-q", "-am", "reformat"]);
    f.git(&f.work(), &["push", "-q", "origin", "main"]);
    let r = f.run("0.2.0");
    assert_refused(&r);
    assert!(r.stderr.contains("plugin.json"), "{}", r.stderr);
}

#[test]
fn versions_compare_as_numbers() {
    // req:ci/release@dc99ac5d
    let f = Fixture::new("0.9.0");
    let r = f.run("0.10.0");
    assert!(r.ok, "{}", r.stderr);
    assert_eq!(r.outputs.get("bumped").map(String::as_str), Some("true"));
}

#[test]
fn release_is_the_only_workflow() {
    // req:ci/release@dc99ac5d
    let names: Vec<_> = fs::read_dir(repo().join(".github/workflows"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names, ["release.yml"]);
}

/// Runs the `merge into main` step's `run` as GitHub would (`bash -eo pipefail`) in a fresh
/// clone of a remote holding `release/v0.2.0`, one commit `Release v0.2.0` on `main`; with
/// `moved`, `main` gained a commit after the branch was cut. Returns the subject of the
/// remote `main`'s tip and its parents.
fn merge_release_branch(moved: bool) -> (String, Vec<String>) {
    let wf = workflow();
    let i = step_running(&wf, "publish", "git push origin main");
    let run = steps(&wf, "publish")[i]["run"].as_str().unwrap().to_owned();
    let f = Fixture::new("0.1.0");
    let work = f.work();
    f.git(&work, &["checkout", "-q", "-b", "release/v0.2.0"]);
    fs::write(work.join("bump"), "0.2.0\n").unwrap();
    f.git(&work, &["add", "-A"]);
    f.git(&work, &["commit", "-q", "-m", "Release v0.2.0"]);
    f.git(&work, &["push", "-q", "origin", "release/v0.2.0"]);
    f.git(&work, &["checkout", "-q", "main"]);
    if moved {
        fs::write(work.join("other"), "x\n").unwrap();
        f.git(&work, &["add", "-A"]);
        f.git(&work, &["commit", "-q", "-m", "landed meanwhile"]);
        f.git(&work, &["push", "-q", "origin", "main"]);
    }
    let clone = f.dir.path().join("runner");
    let bare = f.bare();
    f.git(
        f.dir.path(),
        &[
            "clone",
            "-q",
            bare.to_str().unwrap(),
            clone.to_str().unwrap(),
        ],
    );
    let out = f
        .command("bash", &clone)
        .args(["-eo", "pipefail", "-c", &run])
        .env("VERSION", "0.2.0")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let log = f.git(&bare, &["log", "-1", "--format=%s%n%P", "main"]);
    let mut lines = log.lines();
    let subject = lines.next().unwrap().to_owned();
    let parents = lines
        .next()
        .unwrap_or("")
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    (subject, parents)
}

#[test]
fn merge_into_main_is_titled_release_when_main_moved() {
    // req:ci/release@dc99ac5d
    let (subject, parents) = merge_release_branch(true);
    assert!(subject.starts_with("Release v"), "{subject}");
    assert_eq!(parents.len(), 2, "not a merge: {parents:?}");
}

#[test]
fn merge_into_main_fast_forwards_to_the_bump() {
    // req:ci/release@dc99ac5d
    let (subject, parents) = merge_release_branch(false);
    assert_eq!(subject, "Release v0.2.0");
    assert_eq!(parents.len(), 1, "{parents:?}");
}

#[test]
fn every_cargo_job_installs_the_pinned_toolchain() {
    // req:ci/release@dc99ac5d
    // Jobs install what `rust-toolchain.toml` names (`pinned-rust-toolchain`) with `rustup
    // show`, always, before any step that runs cargo itself or through one of the scripts.
    let wf = workflow();
    // What `dtolnay/rust-toolchain` used to set: no dev debuginfo, colour, short backtraces.
    let env = &wf["env"];
    assert_eq!(env["CARGO_PROFILE_DEV_DEBUG"].as_u64(), Some(0));
    assert_eq!(env["CARGO_TERM_COLOR"].as_str(), Some("always"));
    assert_eq!(env["RUST_BACKTRACE"].as_str(), Some("short"));
    let runs_cargo = |s: &Value| {
        let run = s["run"].as_str().unwrap_or("");
        run.split(|c: char| !(c.is_alphanumeric() || "-_.".contains(c)))
            .any(|w| w == "cargo")
            || ["release-prepare.sh", "detector-tools.sh"]
                .iter()
                .any(|n| run.contains(n))
    };
    let uses = |s: &Value, action: &str| s["uses"].as_str().is_some_and(|u| u.starts_with(action));
    for (job, body) in wf["jobs"].as_mapping().unwrap() {
        let job = job.as_str().unwrap();
        let steps = body["steps"].as_sequence().unwrap();
        for s in steps {
            assert!(!uses(s, "dtolnay/rust-toolchain"), "`{job}`: {s:?}");
        }
        let Some(cargo) = steps.iter().position(runs_cargo) else {
            continue;
        };
        let show = steps
            .iter()
            .position(|s| s["run"].as_str().map(str::trim) == Some("rustup show"))
            .filter(|&show| show < cargo)
            .unwrap_or_else(|| panic!("`{job}` runs cargo without `rustup show` before it"));
        let step = &steps[show];
        assert!(
            step["if"].is_null() && step["continue-on-error"].is_null(),
            "`{job}`: `rustup show` must always run: {step:?}"
        );
    }
}

/// `<name>:<line>` for each line of a workflow file `name` holding a `git push` to `main`: by
/// refspec (`main`, `+main`, `refs/heads/main` or a `:main` / `:refs/heads/main` suffix),
/// `--all` or `--mirror`. Every line of `release.yml`'s step named `merge into main` is exempt:
/// the release's merge of its bump, the one push to `main` `clm ledger verify` accepts.
fn pushes_to_main(name: &str, text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let indent = |l: &str| l.len() - l.trim_start().len();
    let named = |kv: &str| {
        kv.trim()
            .strip_prefix("name:")
            .is_some_and(|v| v.trim().trim_matches(['"', '\'']) == "merge into main")
    };
    let mut exempt = vec![false; lines.len()];
    if name == "release.yml" {
        for (i, line) in lines.iter().enumerate() {
            let Some(first) = line.trim_start().strip_prefix('-') else {
                continue;
            };
            if !first.starts_with(' ') {
                continue;
            }
            let dash = indent(line);
            // The item's keys sit where its first key does, however far past the dash.
            let keys_at = line.len() - first.trim_start().len();
            // A list item runs until the next non-blank line indented no deeper than its dash.
            let end = (i + 1..lines.len())
                .find(|&j| !lines[j].trim().is_empty() && indent(lines[j]) <= dash)
                .unwrap_or(lines.len());
            let keys = &lines[i + 1..end];
            if named(first) || keys.iter().any(|l| indent(l) == keys_at && named(l)) {
                exempt[i..end].fill(true);
            }
        }
    }
    // A line a backslash continues is read with the lines it continues onto.
    let mut found = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let start = i;
        let mut command = String::new();
        loop {
            let line = lines[i].trim_end();
            i += 1;
            match line.strip_suffix('\\') {
                Some(head) if i < lines.len() => command.push_str(head),
                _ => {
                    command.push_str(line);
                    break;
                }
            }
            command.push(' ');
        }
        if !exempt[start] && pushes_main(&command) {
            found.push(format!("{name}:{}", start + 1));
        }
    }
    found
}

/// Whether a command line runs (or quotes) a `git push` whose destination is `main`. Quotes
/// are dropped; a push's arguments end at a shell separator (`;`, `&`, `|`, a bracket or a
/// backtick) or a comment, so `git push origin x && git checkout main` doesn't count.
fn pushes_main(line: &str) -> bool {
    let mut spaced = String::new();
    for c in line.chars() {
        match c {
            '"' | '\'' => {}
            ';' | '&' | '|' | '(' | ')' | '`' => spaced.extend([' ', c, ' ']),
            _ => spaced.push(c),
        }
    }
    let words: Vec<&str> = spaced.split_whitespace().collect();
    let ends = |w: &str| w.starts_with('#') || [";", "&", "|", "(", ")", "`"].contains(&w);
    (0..words.len()).filter(|&i| words[i] == "git").any(|i| {
        // Skip git's own options, such as `-C <dir>`, to reach its subcommand.
        let mut j = i + 1;
        while words.get(j).is_some_and(|w| w.starts_with('-')) {
            j += if matches!(words[j], "-C" | "-c") {
                2
            } else {
                1
            };
        }
        if words.get(j) != Some(&"push") {
            return false;
        }
        words[j + 1..].iter().take_while(|w| !ends(w)).any(|arg| {
            matches!(
                *arg,
                "--all" | "--mirror" | "main" | "+main" | "refs/heads/main" | "+refs/heads/main"
            ) || arg.ends_with(":main")
                || arg.ends_with(":refs/heads/main")
        })
    })
}

#[test]
fn workflow_pushing_main_fails_naming_file_and_line() {
    // req:ci/no-workflow-pushes-main@da2686d6
    let wf = "name: X\non: workflow_dispatch\njobs:\n  x:\n    steps:\n      - run: git push origin HEAD:main\n";
    assert_eq!(pushes_to_main("x.yml", wf), ["x.yml:6"]);
    for push in [
        "git push origin main",
        "git push origin HEAD:main",
        "git push origin refs/heads/main",
        "git push origin +main",
        "git push origin feature:main",
        "git push origin HEAD:refs/heads/main",
        "git push --all origin",
        "git push --mirror origin",
        "git push -f origin \"main\"",
        "echo \"  git push origin main\" >&2",
        "git fetch && git push origin 'HEAD:main'",
        "git push origin HEAD:\"main\"",
        "git -C repo push origin main",
        "git -c http.extraheader=x --no-pager push origin main",
        "out=$(git push origin HEAD:main 2>&1)",
        "git push origin main|| true",
        "git push origin \\\n  HEAD:main",
    ] {
        assert_eq!(
            pushes_to_main("x.yml", &format!("run: {push}\n")),
            ["x.yml:1"],
            "{push}"
        );
    }
    for push in [
        "git push origin --delete \"$branch\"",
        "git push origin feature",
        "git push origin feat; echo main",
        "git push origin feat && git checkout main",
        "git push origin feat&& git checkout main",
        "git push origin feature  # never main",
        "git push origin mainline",
        "git checkout main",
    ] {
        assert!(
            pushes_to_main("x.yml", &format!("run: {push}\n")).is_empty(),
            "{push}"
        );
    }
}

#[test]
fn only_the_merge_into_main_step_of_release_yml_is_exempt() {
    // req:ci/no-workflow-pushes-main@da2686d6
    let wf = "\
jobs:
  publish:
    steps:
      - name: merge into main
        run: |
          git push origin main
          echo \"  git push origin main\" >&2
      - name: after
        run: git push origin main
  other:
    steps:
      - run: git push origin main
";
    assert_eq!(
        pushes_to_main("release.yml", wf),
        ["release.yml:9", "release.yml:12"]
    );
    assert_eq!(
        pushes_to_main("other.yml", wf),
        ["other.yml:6", "other.yml:7", "other.yml:9", "other.yml:12"]
    );
    // The name needn't be the step's first key, nor its keys two columns past the dash.
    let later = "\
steps:
  -   if: always()
      name: merge into main
      run: git push origin main
  - run: git push origin main
";
    assert_eq!(pushes_to_main("release.yml", later), ["release.yml:5"]);
}

#[test]
fn only_the_release_merge_pushes_main() {
    // req:ci/no-workflow-pushes-main@da2686d6
    for entry in fs::read_dir(repo().join(".github/workflows")).unwrap() {
        let path = entry.unwrap().path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().unwrap().to_str().unwrap().to_owned();
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
        let at = pushes_to_main(&name, &text);
        assert!(at.is_empty(), "pushes to main: {}", at.join(", "));
    }
    // The exemption is what lets the release merge through.
    let text = fs::read_to_string(repo().join(".github/workflows/release.yml")).unwrap();
    assert!(!pushes_to_main("other.yml", &text).is_empty());
}
