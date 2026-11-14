//! The plugin under `plugin/`: its package, hooks, skills, agent and `bin/clm` shim.
// clm:reads docs/guide/**

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;
use tempfile::TempDir;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn plugin() -> PathBuf {
    repo().join("plugin")
}

fn json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn workspace_version() -> String {
    let text = fs::read_to_string(repo().join("Cargo.toml")).unwrap();
    let mut in_package = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_package = line == "[workspace.package]";
        } else if in_package && let Some(v) = line.strip_prefix("version = ") {
            return v.trim_matches('"').to_owned();
        }
    }
    panic!("no [workspace.package] version");
}

/// A plugin's `plugin.json` names the workspace version, and a failure names both.
fn assert_workspace_version(manifest: &Value, plugin: &str) {
    let (version, workspace) = (&manifest["version"], workspace_version());
    assert!(
        version.as_str() == Some(workspace.as_str()),
        "{plugin}'s plugin.json version {version} differs from the workspace version {workspace}"
    );
}

#[test]
fn package() {
    // req:plugin/package@c9c402a7
    let market = json(&repo().join(".claude-plugin/marketplace.json"));
    let listed = market["plugins"].as_array().unwrap();
    assert_eq!(listed[0]["name"], "clm", "{market}");
    assert_eq!(listed[0]["source"], "./plugin");
    let manifest = json(&plugin().join(".claude-plugin/plugin.json"));
    assert_eq!(manifest["name"], "clm");
    assert_workspace_version(&manifest, "plugin");
}

#[test]
fn codex_package() {
    // req:plugin/codex-package@88f0c401
    // clm:reads .agents/plugins/marketplace.json
    // clm:reads plugin/.codex-plugin/plugin.json
    let market = json(&repo().join(".agents/plugins/marketplace.json"));
    let listed = market["plugins"].as_array().unwrap();
    let clm = listed
        .iter()
        .find(|p| p["name"] == "clm")
        .expect("clm listed");
    assert_eq!(clm["source"]["path"], "./plugin", "{market}");
    let manifest = json(&plugin().join(".codex-plugin/plugin.json"));
    assert_eq!(manifest["name"], "clm");
    assert_workspace_version(&manifest, "codex plugin");
    // One set of skills, and the hooks file the Codex wiring ships.
    assert_eq!(manifest["skills"], "./skills/");
    assert_eq!(manifest["hooks"], "./hooks/codex.json");
    assert!(plugin().join("skills").is_dir());
    // Claude Code loads `skills/` by default, so its manifest names no other directory.
    let claude_manifest = json(&plugin().join(".claude-plugin/plugin.json"));
    assert!(
        claude_manifest["skills"].is_null() || claude_manifest["skills"] == "./skills/",
        "{claude_manifest}"
    );
    // Both packages run the same shim.
    assert!(plugin().join("bin/clm").is_file());
    let claude = fs::read_to_string(plugin().join("hooks/hooks.json")).unwrap();
    assert!(claude.contains("bin/clm"), "{claude}");
}

#[test]
fn manifest_owner() {
    // req:plugin/manifest-owner@17c45976
    let manifest = json(&plugin().join(".claude-plugin/plugin.json"));
    assert_eq!(manifest["author"]["name"], "Christopher Lunny");
    assert_eq!(manifest["license"], "MIT");
    let market = json(&repo().join(".claude-plugin/marketplace.json"));
    assert_eq!(market["owner"]["name"], "Christopher Lunny");
}

#[test]
fn panel_package() {
    // req:plugin.panel/panel-package@286ccb95
    let market = json(&repo().join(".claude-plugin/marketplace.json"));
    let names: Vec<_> = market["plugins"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["name"].as_str().unwrap(), p["source"].as_str().unwrap()))
        .collect();
    assert_eq!(
        names,
        [("clm", "./plugin"), ("clm-panel", "./plugin-panel")],
        "{market}"
    );
    let panel = repo().join("plugin-panel");
    assert_eq!(
        json(&panel.join("hooks/hooks.json")),
        serde_json::json!({ "modules": ["./register.tsx"] })
    );
    assert!(panel.join("hooks/register.tsx").is_file());
    assert!(
        json(&plugin().join("hooks/hooks.json"))
            .get("modules")
            .is_none()
    );
    let manifest = json(&panel.join(".claude-plugin/plugin.json"));
    assert_eq!(manifest["name"], "clm-panel");
    assert_eq!(manifest["dependencies"], serde_json::json!(["clm"]));
    assert_workspace_version(&manifest, "plugin-panel");
    let package = json(&panel.join("package.json"));
    // It runs the tests, after the type-check (`type-check`).
    let test = package["scripts"]["test"].as_str().unwrap_or_default();
    assert!(test.ends_with("vitest run"), "{package}");
    assert!(panel.join("package-lock.json").is_file());
    let npmrc = fs::read_to_string(panel.join(".npmrc")).unwrap();
    assert!(
        npmrc.lines().any(|l| l.trim() == "ignore-scripts=true"),
        "{npmrc}"
    );
}

#[test]
fn hooks_wired() {
    // req:plugin/hooks-wired@fe0e2818
    let hooks = json(&plugin().join("hooks/hooks.json"));
    let commands = |event: &str, matcher: Option<&str>| -> Vec<String> {
        hooks["hooks"][event]
            .as_array()
            .unwrap_or_else(|| panic!("no {event} in {hooks}"))
            .iter()
            .filter(|g| matcher.is_none_or(|m| g["matcher"] == m))
            .flat_map(|g| g["hooks"].as_array().unwrap().iter())
            .map(|h| h["command"].as_str().unwrap().to_owned())
            .collect()
    };
    let launcher = "\"${CLAUDE_PLUGIN_ROOT}/bin/clm\" hook ";
    assert_eq!(
        commands("PreToolUse", Some("Bash")),
        [format!("{launcher}guard")]
    );
    assert_eq!(
        commands("SessionStart", None),
        [format!("{launcher}session-start")]
    );
    assert_eq!(commands("Stop", None), [format!("{launcher}autosync")]);
}

#[test]
fn codex_hooks_wired() {
    // req:plugin/codex-hooks-wired@1b861624
    // clm:reads plugin/hooks/codex.json
    // clm:reads plugin/hooks/hooks.json
    let hooks = json(&plugin().join("hooks/codex.json"));
    let commands = |event: &str, matcher: Option<&str>| -> Vec<String> {
        hooks["hooks"][event]
            .as_array()
            .unwrap_or_else(|| panic!("no {event} in {hooks}"))
            .iter()
            .filter(|g| matcher.is_none_or(|m| g["matcher"] == m))
            .flat_map(|g| g["hooks"].as_array().unwrap().iter())
            .map(|h| h["command"].as_str().unwrap().to_owned())
            .collect()
    };
    let launcher = "\"${PLUGIN_ROOT}/bin/clm\" hook ";
    assert_eq!(
        commands("PreToolUse", Some("Bash")),
        [format!("{launcher}guard --agent-harness codex")]
    );
    assert_eq!(
        commands("SessionStart", None),
        [format!("{launcher}session-start --agent-harness codex")]
    );
    assert_eq!(
        commands("Stop", None),
        [format!("{launcher}autosync --agent-harness codex")]
    );
    assert!(
        commands("PreToolUse", Some("Bash"))[0]
            .ends_with("bin/clm\" hook guard --agent-harness codex")
    );
    // Claude Code's wiring is unchanged.
    let claude = fs::read_to_string(plugin().join("hooks/hooks.json")).unwrap();
    assert!(!claude.contains("--agent-harness"), "{claude}");
}

/// The YAML frontmatter of a markdown file.
fn frontmatter(path: &Path) -> serde_yaml::Value {
    let text = fs::read_to_string(path).unwrap();
    let rest = text.strip_prefix("---\n").expect("frontmatter");
    let (yaml, _) = rest.split_once("\n---\n").expect("frontmatter end");
    serde_yaml::from_str(yaml).unwrap()
}

/// The skills `skills` lists, and no others.
const SKILLS: [&str; 10] = [
    "build",
    "configure",
    "contribution",
    "health",
    "help",
    "init",
    "propose",
    "triage",
    "upgrade",
    "verify",
];

/// What `skills` asks of a plugin's `skills/` directory, one line per gap naming the skill.
fn skill_gaps(dir: &Path) -> Vec<String> {
    let mut gaps = Vec::new();
    for skill in SKILLS {
        let file = dir.join(skill).join("SKILL.md");
        let described = fs::read_to_string(&file).ok().is_some_and(|text| {
            text.strip_prefix("---\n")
                .and_then(|rest| rest.split_once("\n---\n"))
                .and_then(|(yaml, _)| serde_yaml::from_str::<serde_yaml::Value>(yaml).ok())
                .is_some_and(|fm| fm["description"].as_str().is_some_and(|d| !d.is_empty()))
        });
        if !described {
            gaps.push(format!(
                "{skill}: skills/{skill}/SKILL.md has no description"
            ));
        }
    }
    for entry in fs::read_dir(dir).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        if !SKILLS.contains(&name.as_str()) {
            gaps.push(format!("{name}: not one of the plugin's skills"));
        }
    }
    gaps
}

#[test]
fn skills() {
    // req:plugin/skills@69efdd01
    assert_eq!(skill_gaps(&plugin().join("skills")), Vec::<String>::new());
    // skill without a description
    let tmp = TempDir::new().unwrap();
    for skill in SKILLS {
        let dir = tmp.path().join(skill);
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), "---\ndescription: x\n---\nbody\n").unwrap();
    }
    assert!(skill_gaps(tmp.path()).is_empty());
    fs::write(
        tmp.path().join("health/SKILL.md"),
        "---\nname: health\n---\nbody\n",
    )
    .unwrap();
    let gaps = skill_gaps(tmp.path());
    assert!(
        gaps.len() == 1 && gaps[0].starts_with("health:"),
        "{gaps:?}"
    );
    fs::remove_dir_all(tmp.path().join("health")).unwrap();
    let gaps = skill_gaps(tmp.path());
    assert!(
        gaps.len() == 1 && gaps[0].starts_with("health:"),
        "{gaps:?}"
    );
    // an extra directory
    fs::create_dir(tmp.path().join("health")).unwrap();
    fs::write(
        tmp.path().join("health/SKILL.md"),
        "---\ndescription: x\n---\n",
    )
    .unwrap();
    fs::create_dir(tmp.path().join("implement")).unwrap();
    let gaps = skill_gaps(tmp.path());
    assert!(
        gaps.len() == 1 && gaps[0].starts_with("implement:"),
        "{gaps:?}"
    );
}

/// The pages `clm guide` prints: `docs/guide/<page>.md`, `index` for its README, and
/// `spec-format` for `docs/spec-format.md`.
fn guide_pages() -> Vec<String> {
    let mut pages: Vec<String> = fs::read_dir(repo().join("docs/guide"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter_map(|name| name.strip_suffix(".md").map(str::to_owned))
        .map(|page| {
            if page == "README" {
                "index".to_owned()
            } else {
                page
            }
        })
        .collect();
    pages.push("spec-format".to_owned());
    // `constraints` is generated from the carried entries of `ARCHITECTURE.md`
    // (`init/guide-constraints-page`).
    pages.push("constraints".to_owned());
    pages.sort();
    pages
}

/// What `method-pointers` asks of every skill and agent: each `clm guide <page>` it names
/// is one of `pages`. One line per gap naming the file and text.
fn guide_gaps(name: &str, text: &str, pages: &[String]) -> Vec<String> {
    let mut gaps: Vec<String> = Vec::new();
    for (at, needle) in text.match_indices("clm guide ") {
        let page: String = text[at + needle.len()..]
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
            .collect();
        if !page.is_empty() && !pages.contains(&page) {
            gaps.push(format!(
                "{name}: names `clm guide {page}`, no such page {page}"
            ));
        }
    }
    gaps
}

#[test]
fn skills_point_at_the_guide() {
    // req:plugin/method-pointers@3c5ff1a8
    let pages = guide_pages();
    let mut files: Vec<PathBuf> = fs::read_dir(plugin().join("skills"))
        .unwrap()
        .map(|entry| entry.unwrap().path().join("SKILL.md"))
        .collect();
    files.extend(
        fs::read_dir(plugin().join("agents"))
            .unwrap()
            .map(|entry| entry.unwrap().path()),
    );
    assert!(files.len() >= 14, "{files:?}");
    for file in &files {
        let text = fs::read_to_string(file).unwrap();
        assert_eq!(
            guide_gaps(&file.display().to_string(), &text, &pages),
            Vec::<String>::new()
        );
    }
    // a page that doesn't exist
    let gaps = guide_gaps("x", "Run `clm guide no-such-page`.", &pages);
    assert!(
        gaps.len() == 1 && gaps[0].contains("no-such-page"),
        "{gaps:?}"
    );
    assert!(guide_gaps("x", "Run `clm guide <page>`.", &pages).is_empty());
    assert!(guide_gaps("x", "Run `clm guide spec-format`.", &pages).is_empty());
}

#[test]
fn launch_models() {
    // req:plugin/launch-models@e8f4da39
    for agent in ["spec-auditor", "architecture-auditor", "spec-slimmer"] {
        let fm = frontmatter(&plugin().join(format!("agents/{agent}.md")));
        assert_eq!(
            fm["model"], "sonnet",
            "{agent}: frontmatter must set model: sonnet"
        );
    }
}

#[path = "../../clm-git/tests/exec/mod.rs"]
mod exec;
use exec::write_exec as executable;

/// The release targets, with the `uname -s` and `uname -m` that map to each.
const TARGETS: [(&str, &str, &str); 4] = [
    ("Linux", "x86_64", "x86_64-unknown-linux-musl"),
    ("Linux", "aarch64", "aarch64-unknown-linux-musl"),
    ("Darwin", "x86_64", "x86_64-apple-darwin"),
    ("Darwin", "arm64", "aarch64-apple-darwin"),
];

/// Each release target once.
fn release_targets() -> std::collections::BTreeSet<&'static str> {
    TARGETS.iter().map(|(_, _, target)| *target).collect()
}

/// A copy of the shim in a plugin dir of version 9.9.9, a fake `curl` serving the assets in
/// `assets/`, a `gh` that logs its calls, and an empty cache.
struct Shim {
    dir: TempDir,
}

impl Shim {
    fn new() -> Shim {
        let dir = TempDir::new().unwrap();
        let d = dir.path();
        for sub in [
            "plugin/bin",
            "plugin/.claude-plugin",
            "fakebin",
            "assets",
            "cache",
        ] {
            fs::create_dir_all(d.join(sub)).unwrap();
        }
        executable(
            &d.join("plugin/bin/clm"),
            &fs::read_to_string(plugin().join("bin/clm")).unwrap(),
        );
        fs::write(
            d.join("plugin/.claude-plugin/plugin.json"),
            "{\n  \"name\": \"clm\",\n  \"version\": \"9.9.9\"\n}\n",
        )
        .unwrap();
        // `curl -fsSL -o <file> <url>`: logs its arguments, copies the asset the url names, and
        // fails like `-f` does when there is none.
        executable(
            &d.join("fakebin/curl"),
            "#!/bin/sh\necho \"$*\" >> \"$SHIM_TEST/curl.log\"\n\
             while [ $# -gt 1 ]; do [ \"$1\" = -o ] && out=$2; shift; done\n\
             cp \"$SHIM_TEST/assets/$(basename \"$1\")\" \"$out\" 2>/dev/null || exit 22\n",
        );
        // The shim has no use for `gh`: any call is logged, and fails.
        executable(
            &d.join("fakebin/gh"),
            "#!/bin/sh\necho \"$*\" >> \"$SHIM_TEST/gh.log\"\nexit 1\n",
        );
        Shim { dir }
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }

    /// Serves a release whose `clm` records its arguments, with
    /// checksums that match unless `corrupt`.
    fn serve(&self, corrupt: bool) {
        let stage = self.path("stage");
        fs::create_dir_all(&stage).unwrap();
        executable(
            &stage.join("clm"),
            "#!/bin/sh\necho \"cached clm $*\" >> \"$SHIM_TEST/runs.log\"\n",
        );
        for target in release_targets() {
            let asset = format!("clm-{target}.tar.gz");
            let ok = Command::new("tar")
                .args(["czf", &format!("../assets/{asset}"), "clm"])
                .current_dir(&stage)
                .status()
                .unwrap();
            assert!(ok.success());
            let sum = Command::new("sh")
                .args(["-c", &format!("sha256sum {asset} || shasum -a 256 {asset}")])
                .current_dir(self.path("assets"))
                .output()
                .unwrap();
            let mut sum = String::from_utf8(sum.stdout).unwrap();
            if corrupt {
                sum = format!("{}{}", "0".repeat(64), &sum[64..]);
            }
            fs::write(self.path(&format!("assets/{asset}.sha256")), sum).unwrap();
        }
    }

    /// Installs a fake `uname` that reports the platform `system` `machine`.
    fn platform(&self, system: &str, machine: &str) {
        executable(
            &self.path("fakebin/uname"),
            &format!("#!/bin/sh\n[ \"$1\" = -s ] && echo '{system}' || echo '{machine}'\n"),
        );
    }

    fn run(&self, args: &[&str], env: &[(&str, &str)]) -> Output {
        let path = format!(
            "{}:{}",
            self.path("fakebin").display(),
            std::env::var("PATH").unwrap()
        );
        Command::new(self.path("plugin/bin/clm"))
            .args(args)
            .env("PATH", path)
            .env("SHIM_TEST", self.dir.path())
            .env("XDG_CACHE_HOME", self.path("cache"))
            .env_remove("CLM_BIN")
            .envs(env.iter().copied())
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }

    fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.path(rel)).unwrap_or_default()
    }

    /// How many release assets `curl` was asked for.
    fn downloads(&self) -> usize {
        self.read("curl.log")
            .lines()
            .filter(|l| l.ends_with(".tar.gz"))
            .count()
    }

    fn cached(&self) -> bool {
        self.path("cache/clm/9.9.9/clm").exists()
    }
}

#[test]
fn shim_first_use_downloads_once() {
    // req:plugin/binary-shim@38609dbc
    let shim = Shim::new();
    shim.serve(false);
    for _ in 0..2 {
        let out = shim.run(&["status"], &[]);
        assert!(out.status.success(), "{out:?}");
    }
    let curl = shim.read("curl.log");
    let url = "https://github.com/clunny/clm/releases/download/v9.9.9/clm-";
    let asked = |suffix: &str| {
        curl.lines()
            .filter(|l| l.ends_with(&format!(".tar.gz{suffix}")) && l.contains(url))
            .count()
    };
    assert_eq!((asked(""), asked(".sha256")), (1, 1), "{curl}");
    assert_eq!(shim.downloads(), 1, "{curl}");
    assert_eq!(shim.read("gh.log"), "", "the shim needs no gh");
    assert_eq!(
        shim.read("runs.log"),
        "cached clm status\ncached clm status\n"
    );
    assert!(shim.cached());
    let leftovers: Vec<_> = fs::read_dir(shim.path("cache/clm/9.9.9"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(leftovers, ["clm"], "the download's temp dir is cleaned up");
}

#[test]
fn shim_bad_checksum_caches_nothing() {
    // req:plugin/binary-shim@38609dbc
    let shim = Shim::new();
    shim.serve(true);
    let out = shim.run(&["status"], &[]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(!shim.cached());
    assert_eq!(shim.read("runs.log"), "");
}

#[test]
fn shim_without_a_release_says_how_to_install() {
    // req:plugin/binary-shim@38609dbc
    let shim = Shim::new();
    fs::remove_dir(shim.path("assets")).unwrap();
    let out = shim.run(&["status"], &[]);
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("cargo install") && err.contains("v9.9.9"),
        "{err}"
    );
    assert!(!shim.cached());
    // Hooks stay silent instead, so a missing binary never breaks a session, and skip the
    // download for a while rather than retry it on every tool call.
    for _ in 0..2 {
        let out = shim.run(&["hook", "guard"], &[]);
        assert_eq!(
            (out.status.code(), out.stdout.len(), out.stderr.len()),
            (Some(0), 0, 0),
            "{out:?}"
        );
    }
    assert_eq!(shim.downloads(), 2, "status, then one hook");
    shim.run(&["status"], &[]);
    assert_eq!(shim.downloads(), 3, "commands always retry");
    assert!(!shim.cached());
}

/// Runs `hook session-start` on a shim that can't get a binary, and checks it exits 0
/// having cached nothing, with one line saying the guard is off and how to install `tag`.
fn session_start_says_the_guard_is_off(shim: &Shim, tag: &str) {
    let out = shim.run(&["hook", "session-start"], &[]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(!shim.cached());
    let said = String::from_utf8(out.stdout).unwrap();
    let install = format!("cargo install --git https://github.com/clunny/clm --tag {tag} clm");
    for words in [
        "CLM state",
        "guard",
        "`clm approve`",
        "unavailable",
        &install,
    ] {
        assert!(said.contains(words), "{words} missing from {said:?}");
    }
    for retired in ["clm review", "spec-approved"] {
        assert!(!said.contains(retired), "{retired}: {said:?}");
    }
    assert_eq!(said.lines().count(), 1, "{said:?}");
}

#[test]
fn shim_session_start_without_a_binary_says_the_guard_is_off() {
    // req:plugin/binary-shim@38609dbc
    let shim = Shim::new();
    fs::remove_dir(shim.path("assets")).unwrap();
    session_start_says_the_guard_is_off(&shim, "v9.9.9");
    assert_eq!(shim.downloads(), 1);
    // The second run skips the download (`.unavailable`) and still says so.
    session_start_says_the_guard_is_off(&shim, "v9.9.9");
    assert_eq!(shim.downloads(), 1);
}

#[test]
fn shim_without_a_version_fails() {
    // req:plugin/binary-shim@38609dbc
    let shim = Shim::new();
    shim.serve(false);
    fs::write(shim.path("plugin/.claude-plugin/plugin.json"), "{}\n").unwrap();
    let out = shim.run(&["status"], &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("version"));
    // Hooks still never fail, and session start still says the guard is off.
    let out = shim.run(&["hook", "guard"], &[]);
    assert_eq!((out.status.code(), out.stdout.len()), (Some(0), 0));
    session_start_says_the_guard_is_off(&shim, "v<version>");
    assert_eq!(shim.read("curl.log"), "");
}

#[test]
fn shim_downloads_the_target_for_each_platform() {
    // req:plugin/shim-targets@7df28173
    for (system, machine, target) in TARGETS {
        let shim = Shim::new();
        shim.serve(false);
        shim.platform(system, machine);
        for _ in 0..2 {
            let out = shim.run(&["status"], &[]);
            assert!(out.status.success(), "{system} {machine}: {out:?}");
        }
        let curl = shim.read("curl.log");
        assert!(
            curl.lines()
                .any(|l| l.ends_with(&format!("/clm-{target}.tar.gz"))),
            "{system} {machine}: {curl}"
        );
        assert_eq!(shim.downloads(), 1, "{system} {machine}: {curl}");
        assert!(shim.cached(), "{system} {machine}: the binary isn't cached");
        assert_eq!(
            shim.read("runs.log"),
            "cached clm status\ncached clm status\n",
            "{system} {machine}"
        );
    }
}

#[test]
fn shim_maps_each_target_in_one_assignment() {
    // req:plugin/shim-targets@7df28173
    // `ci/release`'s prepare step reads the shim's `target=<target>` assignments: exactly
    // the four targets, and no others.
    let shim = fs::read_to_string(plugin().join("bin/clm")).unwrap();
    for target in release_targets() {
        assert_eq!(
            shim.matches(&format!("target={target}")).count(),
            1,
            "target={target}"
        );
    }
    let assignments = shim
        .lines()
        .filter(|l| l.trim_start().contains(") target=") && !l.contains("target= "))
        .count();
    assert_eq!(assignments, release_targets().len(), "{shim}");
}

/// What `shim-targets` forbids the shim's text to name: a Windows target or `clm.exe`.
fn windows_words(shim: &str) -> Vec<&'static str> {
    let lower = shim.to_lowercase();
    ["windows", "clm.exe"]
        .into_iter()
        .filter(|word| lower.contains(word))
        .collect()
}

#[test]
fn shim_names_no_windows_target() {
    // req:plugin/shim-targets@7df28173
    let shim = fs::read_to_string(plugin().join("bin/clm")).unwrap();
    assert_eq!(windows_words(&shim), Vec::<&str>::new());
    // The text fails the check when a comment names Windows in any case, or `clm.exe`.
    assert_eq!(windows_words("# runs on WINDOWS\n"), ["windows"]);
    assert_eq!(windows_words("exe=clm.exe\n"), ["clm.exe"]);
}

#[test]
fn shim_on_an_unsupported_platform_fails() {
    // req:plugin/binary-shim@38609dbc
    // req:plugin/shim-targets@7df28173
    // Windows's shells (MSYS2, Cygwin and the rest) get no binary either (`no-windows-support`).
    for (system, machine) in [
        ("FreeBSD", "amd64"),
        ("CYGWIN_NT-10.0", "x86_64"),
        ("MINGW64_NT-10.0", "x86_64"),
        ("UCRT64_NT-10.0-22631", "x86_64"),
    ] {
        let shim = Shim::new();
        shim.serve(false);
        shim.platform(system, machine);
        let out = shim.run(&["status"], &[]);
        assert_eq!(out.status.code(), Some(1), "{system}");
        let err = String::from_utf8_lossy(&out.stderr);
        let platform = format!("{system} {machine}");
        for words in ["no prebuilt clm", &platform, "cargo install"] {
            assert!(err.contains(words), "{words} missing from {err}");
        }
        assert_eq!(shim.read("curl.log"), "");
        let out = shim.run(&["hook", "guard"], &[]);
        assert_eq!((out.status.code(), out.stdout.len()), (Some(0), 0));
        session_start_says_the_guard_is_off(&shim, "v9.9.9");
        assert_eq!(shim.read("curl.log"), "");
        assert!(!shim.cached());
    }
}

#[test]
fn shim_prefers_clm_bin() {
    // req:plugin/binary-shim@38609dbc
    let shim = Shim::new();
    let stub = shim.path("own-clm");
    executable(
        &stub,
        "#!/bin/sh\necho \"own $*\" >> \"$SHIM_TEST/runs.log\"\n",
    );
    let out = shim.run(
        &["next", "--expiry", "2"],
        &[("CLM_BIN", stub.to_str().unwrap())],
    );
    assert!(out.status.success(), "{out:?}");
    assert_eq!(shim.read("runs.log"), "own next --expiry 2\n");
    assert_eq!(shim.read("curl.log"), "");
}
