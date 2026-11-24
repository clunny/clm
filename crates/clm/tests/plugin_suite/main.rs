//! The plugin under `plugin/`: its package, hooks, skills, agent and `bin/clm` shim.
// clm:reads plugin/skills/**
// clm:reads plugin/commands/**
// clm:reads plugin/bin/clm
// clm:reads plugin/hooks/hooks.json
// clm:reads .claude-plugin/marketplace.json

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
    // req:plugin/package@ce03107e
    let market = json(&repo().join(".claude-plugin/marketplace.json"));
    // two plugins: `clm`, then `clm-panel`, and no other.
    let listed: Vec<(&str, &str)> = market["plugins"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["name"].as_str().unwrap(), p["source"].as_str().unwrap()))
        .collect();
    assert_eq!(
        listed,
        [("clm", "./plugin"), ("clm-panel", "./plugin-panel")],
        "{market}"
    );
    let manifest = json(&plugin().join(".claude-plugin/plugin.json"));
    assert_eq!(manifest["name"], "clm");
    assert_workspace_version(&manifest, "plugin");
}

#[test]
fn codex_package() {
    // req:plugin/codex-package@32fd657b
    // clm:reads .agents/plugins/marketplace.json
    // clm:reads plugin/.codex-plugin/plugin.json
    let market = json(&repo().join(".agents/plugins/marketplace.json"));
    let listed = market["plugins"].as_array().unwrap();
    let clm = listed
        .iter()
        .find(|p| p["name"] == "clm")
        .expect("clm listed");
    assert_eq!(clm["source"]["source"], "local", "{market}");
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
    for hooks in ["hooks/hooks.json", "hooks/codex.json"] {
        let wired = json(&plugin().join(hooks));
        let commands: Vec<&str> = wired["hooks"]
            .as_object()
            .unwrap()
            .values()
            .flat_map(|groups| groups.as_array().unwrap())
            .flat_map(|g| g["hooks"].as_array().unwrap())
            .map(|h| h["command"].as_str().unwrap())
            .collect();
        assert!(!commands.is_empty(), "{hooks}");
        for command in commands {
            assert!(command.contains("/bin/clm\" hook "), "{hooks}: {command}");
        }
    }
}

#[test]
fn manifest_owner() {
    // req:plugin/manifest-owner@6e020597
    let manifest = json(&plugin().join(".claude-plugin/plugin.json"));
    assert_eq!(manifest["author"]["name"], "Christopher Lunny");
    assert_eq!(manifest["license"], "MIT");
    let market = json(&repo().join(".claude-plugin/marketplace.json"));
    assert_eq!(market["owner"]["name"], "Christopher Lunny");
}

#[test]
fn hooks_wired() {
    // req:plugin/hooks-wired@f14fccf0
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
    // autosync bounded
    let stop: Vec<&Value> = hooks["hooks"]["Stop"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["hooks"].as_array().unwrap().iter())
        .collect();
    assert_eq!(stop.len(), 1, "{hooks}");
    assert_eq!(stop[0]["timeout"], 30, "{hooks}");
    // no hooks module in clm
    assert!(hooks.get("modules").is_none(), "{hooks}");
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

/// Whether a markdown file exists with a non-empty `description` in its frontmatter.
fn described(file: &Path) -> bool {
    fs::read_to_string(file).ok().is_some_and(|text| {
        text.strip_prefix("---\n")
            .and_then(|rest| rest.split_once("\n---\n"))
            .and_then(|(yaml, _)| serde_yaml::from_str::<serde_yaml::Value>(yaml).ok())
            .is_some_and(|fm| fm["description"].as_str().is_some_and(|d| !d.is_empty()))
    })
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
        if !described(&dir.join(skill).join("SKILL.md")) {
            gaps.push(format!(
                "{skill}: skills/{skill}/SKILL.md has no description"
            ));
        }
    }
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        if !entry.file_type().unwrap().is_dir() {
            continue;
        }
        let name = entry.file_name().into_string().unwrap();
        if !SKILLS.contains(&name.as_str()) {
            gaps.push(format!("{name}: not one of the plugin's skills"));
        }
    }
    gaps
}

#[test]
fn skills() {
    // req:plugin/skills@43f0b337
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

/// What `view-command` asks of a plugin's `commands/` directory, one line per gap naming
/// the command.
fn command_gaps(dir: &Path) -> Vec<String> {
    if described(&dir.join("view.md")) {
        Vec::new()
    } else {
        vec!["view: commands/view.md has no description".to_owned()]
    }
}

#[test]
fn view_command() {
    // req:plugin/view-command@3a0c7ead
    assert_eq!(
        command_gaps(&plugin().join("commands")),
        Vec::<String>::new()
    );
    // command without a description: missing, then without one, then with an empty one.
    let tmp = TempDir::new().unwrap();
    let gap = |dir: &Path| {
        let gaps = command_gaps(dir);
        assert!(gaps.len() == 1 && gaps[0].starts_with("view:"), "{gaps:?}");
    };
    gap(tmp.path());
    fs::write(
        tmp.path().join("view.md"),
        "---\nargument-hint: x\n---\nbody\n",
    )
    .unwrap();
    gap(tmp.path());
    fs::write(
        tmp.path().join("view.md"),
        "---\ndescription: \"\"\n---\nbody\n",
    )
    .unwrap();
    gap(tmp.path());
    fs::write(
        tmp.path().join("view.md"),
        "---\ndescription: x\n---\nbody\n",
    )
    .unwrap();
    assert!(command_gaps(tmp.path()).is_empty());
}

#[test]
fn launch_models() {
    // clm:reads plugin/agents/**
    // req:plugin/launch-models@e8f4da39
    for agent in ["spec-auditor", "architecture-auditor", "spec-slimmer"] {
        let fm = frontmatter(&plugin().join(format!("agents/{agent}.md")));
        assert_eq!(
            fm["model"], "sonnet",
            "{agent}: frontmatter must set model: sonnet"
        );
    }
}

#[path = "../../../clm-git/tests/exec/mod.rs"]
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
        self.run_unset(args, env, &[])
    }

    /// Runs the shim with `env` set and the variables `unset` removed, after the harness's own.
    fn run_unset(&self, args: &[&str], env: &[(&str, &str)], unset: &[&str]) -> Output {
        let path = format!(
            "{}:{}",
            self.path("fakebin").display(),
            std::env::var("PATH").unwrap()
        );
        let mut command = Command::new(self.path("plugin/bin/clm"));
        command
            .args(args)
            .env("PATH", path)
            .env("SHIM_TEST", self.dir.path())
            .env("XDG_CACHE_HOME", self.path("cache"))
            .env_remove("CLM_BIN")
            .envs(env.iter().copied());
        for name in unset {
            command.env_remove(name);
        }
        command.stdin(Stdio::null()).output().unwrap()
    }

    /// The version's cache directory.
    fn version_cache(&self) -> PathBuf {
        self.path("cache/clm/9.9.9")
    }

    /// Ages the record of a failed download to `minutes` ago.
    fn age_mark(&self, minutes: u64) {
        let ago = std::time::SystemTime::now() - std::time::Duration::from_secs(minutes * 60);
        fs::File::open(self.version_cache().join(".unavailable"))
            .unwrap()
            .set_modified(ago)
            .unwrap();
    }

    /// When the record of a failed download was last written.
    fn mark_time(&self) -> std::time::SystemTime {
        fs::metadata(self.version_cache().join(".unavailable"))
            .unwrap()
            .modified()
            .unwrap()
    }

    /// The names the version's cache directory holds, sorted.
    fn cache_entries(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.version_cache())
            .map(|dir| {
                dir.map(|e| e.unwrap().file_name().into_string().unwrap())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
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
    // req:plugin/binary-shim@48d0adf8
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
    // req:plugin/binary-shim@48d0adf8
    let shim = Shim::new();
    shim.serve(true);
    let out = shim.run(&["status"], &[]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(!shim.cached());
    assert_eq!(shim.read("runs.log"), "");
}

#[test]
fn shim_without_a_release_says_how_to_install() {
    // req:plugin/binary-shim@48d0adf8
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
    assert!(!shim.cached());
    says_the_guard_is_off(out, tag);
}

/// Checks a session start's output: exit 0, and on stdout one line saying the guard is off
/// and how to install `tag`, with nothing on stderr.
fn says_the_guard_is_off(out: Output, tag: &str) {
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stderr), "", "{out:?}");
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
    // req:plugin/binary-shim@48d0adf8
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
    // req:plugin/binary-shim@48d0adf8
    let shim = Shim::new();
    shim.serve(false);
    fs::write(shim.path("plugin/.claude-plugin/plugin.json"), "{}\n").unwrap();
    let out = shim.run(&["status"], &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("can't read the version"),
        "{out:?}"
    );
    // Hooks still never fail, and session start still says the guard is off.
    let out = shim.run(&["hook", "guard"], &[]);
    assert_eq!(
        (out.status.code(), out.stdout.len(), out.stderr.len()),
        (Some(0), 0, 0),
        "{out:?}"
    );
    session_start_says_the_guard_is_off(&shim, "v<version>");
    assert_eq!(shim.read("curl.log"), "");
}

#[test]
fn shim_downloads_the_target_for_each_platform() {
    // req:plugin/shim-targets@ea4177ee
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

/// The targets `text` assigns, as `ci/release`'s prepare step matches them:
/// `(^|[^[:alnum:]_$])target=[[:alnum:]_.-]+`, one entry per match.
fn target_assignments(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    for (at, needle) in text.match_indices("target=") {
        let before = text[..at].chars().next_back();
        if before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$') {
            continue;
        }
        let rest = &text[at + needle.len()..];
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-')))
            .unwrap_or(rest.len());
        if end > 0 {
            found.push(&rest[..end]);
        }
    }
    found
}

#[test]
fn shim_maps_each_target_in_one_assignment() {
    // req:plugin/shim-targets@ea4177ee
    // no Windows target in the shim: the targets matched are exactly the four, each once.
    let shim = fs::read_to_string(plugin().join("bin/clm")).unwrap();
    let mut matched = target_assignments(&shim);
    matched.sort_unstable();
    let mut expected: Vec<&str> = release_targets().into_iter().collect();
    expected.sort_unstable();
    assert_eq!(matched, expected, "{shim}");
    // The pattern as the prepare step reads it.
    assert_eq!(
        target_assignments("a) target=x-y.z_1 ;;\nb=$target=no\nmy_target=no\n*) target= ;;"),
        ["x-y.z_1"]
    );
}

#[test]
fn shim_on_an_unsupported_platform_fails() {
    // req:plugin/binary-shim@48d0adf8
    // req:plugin/shim-targets@ea4177ee
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
    // req:plugin/binary-shim@48d0adf8
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
    // Its exit status is the binary's own.
    let fails = shim.path("failing-clm");
    executable(&fails, "#!/bin/sh\nexit 101\n");
    let out = shim.run(
        &["next", "--expiry", "2"],
        &[("CLM_BIN", fails.to_str().unwrap())],
    );
    assert_eq!(out.status.code(), Some(101), "{out:?}");
    // A missing file exits 127, and one that isn't executable 126, hooks included.
    let plain = shim.path("plain-clm");
    fs::write(&plain, "#!/bin/sh\nexit 0\n").unwrap();
    let missing = shim.path("no-such-clm");
    for (bin, code) in [(&missing, 127), (&plain, 126)] {
        for args in [&["next", "--expiry", "2"][..], &["hook", "guard"]] {
            let out = shim.run(args, &[("CLM_BIN", bin.to_str().unwrap())]);
            assert_eq!(out.status.code(), Some(code), "{args:?} {out:?}");
        }
    }
    assert_eq!(shim.read("curl.log"), "");
    // An empty `CLM_BIN` downloads and runs the cached binary, as when it's unset.
    shim.serve(false);
    let out = shim.run(&["next", "--expiry", "2"], &[("CLM_BIN", "")]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(shim.downloads(), 1);
    assert!(shim.cached());
    assert_eq!(
        shim.read("runs.log"),
        "own next --expiry 2\ncached clm next --expiry 2\n"
    );
}

#[test]
fn shim_hook_after_a_failed_download() {
    // req:plugin/binary-shim@48d0adf8
    let shim = Shim::new();
    fs::remove_dir(shim.path("assets")).unwrap();
    let out = shim.run(&["hook", "guard"], &[]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(shim.downloads(), 1);
    // The failure was 59 minutes ago: the hooks download nothing, and leave its time alone.
    shim.age_mark(59);
    let marked = shim.mark_time();
    let out = shim.run(&["hook", "guard"], &[]);
    assert_eq!(
        (out.status.code(), out.stdout.len(), out.stderr.len()),
        (Some(0), 0, 0),
        "{out:?}"
    );
    says_the_guard_is_off(shim.run(&["hook", "session-start"], &[]), "v9.9.9");
    assert_eq!(shim.downloads(), 1);
    assert_eq!(shim.mark_time(), marked, "a hook that skips moves no mark");
    // Any other command still downloads.
    let out = shim.run(&["status"], &[]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(shim.downloads(), 2);
    // Two minutes older than recorded, the hour is over, and the hook downloads again.
    fs::File::open(shim.version_cache().join(".unavailable"))
        .unwrap()
        .set_modified(shim.mark_time() - std::time::Duration::from_secs(120))
        .unwrap();
    let out = shim.run(&["hook", "guard"], &[]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(shim.downloads(), 3);
}

#[test]
fn shim_without_a_cache_directory() {
    // req:plugin/binary-shim@48d0adf8
    type Case<'a> = (&'a [(&'a str, &'a str)], &'a [&'a str]);
    let cases: [Case; 3] = [
        (&[], &["HOME", "XDG_CACHE_HOME"]),
        (&[("XDG_CACHE_HOME", "")], &["HOME"]),
        (&[("XDG_CACHE_HOME", ""), ("HOME", "")], &[]),
    ];
    for (env, unset) in cases {
        let shim = Shim::new();
        shim.serve(false);
        let out = shim.run_unset(&["hook", "guard"], env, unset);
        assert_eq!(
            (out.status.code(), out.stdout.len(), out.stderr.len()),
            (Some(0), 0, 0),
            "{env:?} {unset:?}: {out:?}"
        );
        says_the_guard_is_off(
            shim.run_unset(&["hook", "session-start"], env, unset),
            "v9.9.9",
        );
        let out = shim.run_unset(&["status"], env, unset);
        assert_eq!(out.status.code(), Some(1), "{env:?} {unset:?}: {out:?}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains("XDG_CACHE_HOME") && err.contains("HOME"),
            "{err}"
        );
        assert_eq!(shim.read("curl.log"), "", "{env:?} {unset:?}");
    }
}

#[test]
fn shim_says_each_message_where_it_belongs() {
    // req:plugin/binary-shim@48d0adf8
    let shim = Shim::new();
    fs::remove_dir(shim.path("assets")).unwrap();
    // The session start's line on stdout, one line, nothing on stderr.
    says_the_guard_is_off(shim.run(&["hook", "session-start"], &[]), "v9.9.9");
    // A command's message on stderr, nothing on stdout, naming `CLM_BIN`.
    let out = shim.run(&["status"], &[]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(out.stdout.len(), 0, "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("CLM_BIN"), "{err}");
    // With no plugin.json to read, a hook still says nothing.
    fs::remove_file(shim.path("plugin/.claude-plugin/plugin.json")).unwrap();
    let out = shim.run(&["hook", "guard"], &[]);
    assert_eq!(
        (out.status.code(), out.stdout.len(), out.stderr.len()),
        (Some(0), 0, 0),
        "{out:?}"
    );
}

/// Whether the tests run as root, for whom no permission bit refuses a write.
fn is_root() -> bool {
    // SAFETY: geteuid has no preconditions and can't fail.
    unsafe { libc::geteuid() == 0 }
}

/// Makes its directories writable again when dropped, so a failing test leaves none behind.
struct Writable(Vec<PathBuf>);

impl Drop for Writable {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        for dir in &self.0 {
            let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o755));
        }
    }
}

/// Sets `path`'s permission bits to `mode`.
fn chmod(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

#[test]
fn shim_with_a_cache_it_cannot_write() {
    // req:plugin/binary-shim@48d0adf8
    if is_root() {
        return;
    }
    for case in ["unwritable", "read-only mark", "linked mark"] {
        let shim = Shim::new();
        fs::remove_dir(shim.path("assets")).unwrap();
        let cache = shim.version_cache();
        fs::create_dir_all(&cache).unwrap();
        let mark = cache.join(".unavailable");
        let _restore = Writable(vec![cache.clone(), shim.path("closed")]);
        match case {
            "unwritable" => chmod(&cache, 0o555),
            "read-only mark" => {
                fs::write(&mark, "").unwrap();
                shim.age_mark(120);
                chmod(&mark, 0o444);
            }
            _ => {
                let closed = shim.path("closed");
                fs::create_dir(&closed).unwrap();
                chmod(&closed, 0o555);
                std::os::unix::fs::symlink(closed.join("mark"), &mark).unwrap();
                let aged = Command::new("touch")
                    .args(["-h", "-t", "200001010000"])
                    .arg(&mark)
                    .status()
                    .unwrap();
                assert!(aged.success());
            }
        }
        let quiet = |out: Output| {
            assert_eq!(
                (out.status.code(), out.stdout.len(), out.stderr.len()),
                (Some(0), 0, 0),
                "{case}: {out:?}"
            );
        };
        quiet(shim.run(&["hook", "guard"], &[]));
        says_the_guard_is_off(shim.run(&["hook", "session-start"], &[]), "v9.9.9");
        let before = shim.downloads();
        quiet(shim.run(&["hook", "guard"], &[]));
        if case != "unwritable" {
            assert_eq!(
                before, 1,
                "{case}: the old mark let the first hook download"
            );
            assert_eq!(
                shim.downloads(),
                1,
                "{case}: the second hook downloads nothing"
            );
        }
        let out = shim.run(&["status"], &[]);
        assert_eq!(out.status.code(), Some(1), "{case}: {out:?}");
        if case == "unwritable" {
            let err = String::from_utf8_lossy(&out.stderr);
            assert!(err.contains("can't write to"), "{case}: {err}");
        }
        assert!(!shim.cached(), "{case}");
    }
}

/// A directory of links to the tools the shim runs, with no `curl` among them.
fn path_without_curl(shim: &Shim) -> PathBuf {
    let dir = shim.path("nocurl");
    fs::create_dir(&dir).unwrap();
    for tool in [
        "sh",
        "sed",
        "head",
        "uname",
        "dirname",
        "mkdir",
        "mktemp",
        "find",
        "rm",
        "tar",
        "gzip",
        "sha256sum",
        "shasum",
        "chmod",
        "mv",
        "touch",
    ] {
        let found = Command::new("sh")
            .args(["-c", &format!("command -v {tool}")])
            .output()
            .unwrap();
        let at = String::from_utf8(found.stdout).unwrap();
        if !at.trim().is_empty() {
            std::os::unix::fs::symlink(at.trim(), dir.join(tool)).unwrap();
        }
    }
    dir
}

#[test]
fn shim_leaves_nothing_behind() {
    // req:plugin/binary-shim@48d0adf8
    // No `curl` on the PATH.
    let shim = Shim::new();
    shim.serve(false);
    let nocurl = path_without_curl(&shim);
    let out = shim.run(&["status"], &[("PATH", nocurl.to_str().unwrap())]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("with curl"),
        "the failure is curl's: {out:?}"
    );
    assert_eq!(shim.cache_entries(), Vec::<String>::new());
    // An archive holding no `clm`, with a matching checksum.
    let shim = Shim::new();
    let stage = shim.path("stage");
    fs::create_dir(&stage).unwrap();
    fs::write(stage.join("other"), "x").unwrap();
    for target in release_targets() {
        let asset = format!("clm-{target}.tar.gz");
        let ok = Command::new("sh")
            .args([
                "-c",
                &format!(
                    "tar czf ../assets/{asset} other && cd ../assets && \
                     (sha256sum {asset} || shasum -a 256 {asset}) > {asset}.sha256"
                ),
            ])
            .current_dir(&stage)
            .status()
            .unwrap();
        assert!(ok.success());
    }
    let out = shim.run(&["status"], &[]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("holds no clm"),
        "{out:?}"
    );
    assert_eq!(shim.cache_entries(), Vec::<String>::new());
    // A download that succeeds after a hook's failed one leaves only `clm`.
    let shim = Shim::new();
    fs::remove_dir(shim.path("assets")).unwrap();
    shim.run(&["hook", "guard"], &[]);
    assert_eq!(shim.cache_entries(), [".unavailable"]);
    fs::create_dir(shim.path("assets")).unwrap();
    shim.serve(false);
    let out = shim.run(&["status"], &[]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(shim.cache_entries(), ["clm"]);
}

#[test]
fn shim_bounds_each_download() {
    // req:plugin/binary-shim@48d0adf8
    let shim = Shim::new();
    shim.serve(false);
    let out = shim.run(&["status"], &[]);
    assert!(out.status.success(), "{out:?}");
    let curl = shim.read("curl.log");
    assert_eq!(curl.lines().count(), 2, "{curl}");
    for call in curl.lines() {
        let words: Vec<&str> = call.split(' ').collect();
        for (flag, value) in [("--connect-timeout", "10"), ("--max-time", "120")] {
            assert!(
                words.windows(2).any(|w| w == [flag, value]),
                "{flag} {value} missing from {call}"
            );
        }
    }
}

#[test]
fn shim_caches_under_xdg_or_home() {
    // req:plugin/binary-shim@48d0adf8
    // `XDG_CACHE_HOME` set is the harness's default.
    let shim = Shim::new();
    shim.serve(false);
    assert!(shim.run(&["status"], &[]).status.success());
    assert!(shim.cached());
    // `XDG_CACHE_HOME` unset or empty: under `HOME`.
    for xdg in [None, Some("")] {
        let shim = Shim::new();
        shim.serve(false);
        let home = shim.path("home");
        fs::create_dir(&home).unwrap();
        let mut env = vec![("HOME", home.to_str().unwrap())];
        let mut unset = vec![];
        match xdg {
            Some(value) => env.push(("XDG_CACHE_HOME", value)),
            None => unset.push("XDG_CACHE_HOME"),
        }
        let out = shim.run_unset(&["status"], &env, &unset);
        assert!(out.status.success(), "{xdg:?}: {out:?}");
        assert!(home.join(".cache/clm/9.9.9/clm").is_file(), "{xdg:?}");
        assert!(!shim.cached(), "{xdg:?}");
    }
}
