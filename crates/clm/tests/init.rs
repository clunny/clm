//! `clm init` as the binary runs it, with a fake `gh` on `PATH` that it must never call: the
//! root's tests of its values, and `init`'s tests that run another component's command on
//! what it wrote. The rest are `init`'s, in `crates/clm-init/tests/init.rs`.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::{PoisonError, RwLock};

use tempfile::TempDir;

/// A process forked while another thread holds a just-written executable open for writing
/// inherits that descriptor, and running the file then fails with "text file busy". Writing
/// executables holds this lock exclusively, and every spawn shares it.
static FORKS: RwLock<()> = RwLock::new(());

fn git(dir: &Path, args: &[&str]) -> String {
    let out = {
        let _shared = FORKS.read().unwrap_or_else(PoisonError::into_inner);
        Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@example.com")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@example.com")
            .output()
            .unwrap()
    };
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn read(root: &Path, rel: &str) -> String {
    fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// An empty git repository in a directory named `shop`.
fn repo() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("shop");
    fs::create_dir(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    (tmp, dir)
}

/// A directory holding a fake `gh` that appends its arguments as one line to `log` and
/// fails: `clm init` never calls it.
fn fake_hosts() -> TempDir {
    let dir = TempDir::new().unwrap();
    let _exclusive = FORKS.write().unwrap_or_else(PoisonError::into_inner);
    for host in ["gh", "az"] {
        let script = format!(
            "#!/bin/sh\nprintf '{host} %s\\n' \"$*\" >> '{log}'\nexit 1\n",
            log = dir.path().join("log").display(),
        );
        let path = dir.path().join(host);
        fs::write(&path, script).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    dir
}

/// Every call the fake `gh` and `az` saw.
fn host_calls(hosts: &TempDir) -> String {
    fs::read_to_string(hosts.path().join("log")).unwrap_or_default()
}

fn path_with(hosts: &TempDir) -> String {
    format!(
        "{}:{}",
        hosts.path().display(),
        std::env::var("PATH").unwrap_or_default()
    )
}

/// Runs `clm <args>` in `cwd` with `path` as `PATH` and no user settings.
fn clm(cwd: &Path, args: &[&str], path: &str) -> Output {
    let config = TempDir::new().unwrap();
    let _shared = FORKS.read().unwrap_or_else(PoisonError::into_inner);
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .current_dir(cwd)
        .env("PATH", path)
        .env("XDG_CONFIG_HOME", config.path())
        .env_remove("CLM_AUTOSYNC")
        .env_remove("CLM_DEFECTS")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap()
}

/// Runs `clm init <args>` in `cwd`, expecting exit 0 and no call to `gh`; returns
/// stdout.
fn init(cwd: &Path, args: &[&str], hosts: &TempDir) -> String {
    let mut all = vec!["init"];
    all.extend(args);
    let out = clm(cwd, &all, &path_with(hosts));
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(host_calls(hosts), "", "clm init called a host");
    String::from_utf8(out.stdout).unwrap()
}

const FILES: [&str; 7] = [
    "clm.toml",
    "scripts/clm-check.sh",
    "ARCHITECTURE.md",
    "CAPABILITIES.md",
    "specs/SPEC.md",
    "AGENTS.md",
    "CLAUDE.md",
];

/// Sets `origin` to `url`.
fn origin(dir: &Path, url: &str) {
    git(dir, &["remote", "add", "origin", url]);
}

/// The `host` line of `clm.toml`.
fn host_line(dir: &Path) -> String {
    let toml = read(dir, "clm.toml");
    toml.lines()
        .find(|l| l.starts_with("host = "))
        .unwrap_or_else(|| panic!("no host: {toml}"))
        .to_owned()
}

#[test]
fn empty_repository() {
    // req:clm/init-result-accepted@f3e67ac2
    // req:init/init-files@2e91537b
    let (_tmp, dir) = repo();
    origin(&dir, "https://github.com/o/r.git");
    let hosts = fake_hosts();
    let out = init(&dir, &["--lang", "rust"], &hosts);
    for file in FILES {
        assert!(dir.join(file).is_file(), "{file} missing");
        assert!(out.contains(&format!("wrote {file}\n")), "{out}");
    }
    assert!(!dir.join(".github").exists(), "{out}");
    let check = dir.join("scripts/clm-check.sh");
    assert_eq!(
        fs::metadata(&check).unwrap().permissions().mode() & 0o111,
        0o111
    );
    let script = read(&dir, "scripts/clm-check.sh");
    assert!(script.starts_with("#!/usr/bin/env bash\n"), "{script}");
    assert!(script.contains("cargo test"), "{script}");
    let toml = read(&dir, "clm.toml");
    assert!(toml.contains("command = [\"cargo-machete\"]"), "{toml}");
    assert!(toml.lines().any(|l| l == "host = \"github\""), "{toml}");
    assert!(toml.lines().any(|l| l == "approval = \"any\""), "{toml}");
    assert!(toml.contains("`other`"), "{toml}");
    assert!(
        toml.contains("\n[land]\ncheck = \"scripts/clm-check.sh\"\n"),
        "{toml}"
    );
    assert_eq!(read(&dir, "CLAUDE.md"), "@AGENTS.md\n");
    let spec = read(&dir, "specs/SPEC.md");
    assert!(spec.contains("title: \"shop\"\n"), "{spec}");
    assert!(spec.contains("status: unbuilt\n"), "{spec}");
    let check = clm(&dir, &["check"], &path_with(&hosts));
    assert_eq!(check.status.code(), Some(0), "{check:?}");
    // `clm` reads the settings back as written.
    assert_eq!(config(&dir, "host"), "github");
    assert_eq!(config(&dir, "approval"), "any");
    assert_eq!(config(&dir, "land"), "check=scripts/clm-check.sh");
}

/// The element names `clm hashes <layer>` prints in `dir`.
fn layer_entries(dir: &Path, layer: &str, hosts: &TempDir) -> Vec<String> {
    let out = clm(dir, &["hashes", layer], &path_with(hosts));
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| l.split(' ').next().unwrap().to_string())
        .collect()
}

#[test]
fn human_layers() {
    // req:clm/init-result-accepted@f3e67ac2
    let (_tmp, dir) = repo();
    let hosts = fake_hosts();
    init(&dir, &[], &hosts);
    for layer in ["architecture", "capability"] {
        assert_eq!(
            layer_entries(&dir, layer, &hosts),
            Vec::<String>::new(),
            "{layer}"
        );
    }
    let pending = clm(&dir, &["pending"], &path_with(&hosts));
    assert_eq!(pending.status.code(), Some(0), "{pending:?}");
    assert_eq!(String::from_utf8_lossy(&pending.stdout), "", "{pending:?}");
}

#[test]
fn init_writes_capabilities() {
    // req:init/init-files@2e91537b
    let (_tmp, dir) = repo();
    let hosts = fake_hosts();
    let out = init(&dir, &["--lang", "rust"], &hosts);
    assert!(out.contains("wrote CAPABILITIES.md\n"), "{out}");
    let caps = read(&dir, "CAPABILITIES.md");
    assert!(caps.contains("\n## Capabilities\n"), "{caps}");
    // No entry the project's own tests would have to prove.
    for layer in ["capability", "architecture"] {
        assert_eq!(layer_entries(&dir, layer, &hosts), Vec::<String>::new());
    }
    let pending = clm(&dir, &["pending"], &path_with(&hosts));
    assert_eq!(String::from_utf8_lossy(&pending.stdout), "", "{pending:?}");
    // CLM's carried constraints are named by where they are, outside `## Constraints`.
    let arch = read(&dir, "ARCHITECTURE.md");
    let before = &arch[..arch.find("\n## Constraints\n").expect(&arch)];
    assert!(before.contains("`clm guide constraints`"), "{arch}");
    let check = clm(&dir, &["check"], &path_with(&hosts));
    assert_eq!(check.status.code(), Some(0), "{check:?}");

    // An existing CAPABILITIES.md is the project's.
    let (_tmp2, other) = repo();
    write(&other, "CAPABILITIES.md", "# Ours\n");
    let out = init(&other, &["--lang", "rust"], &hosts);
    assert_eq!(read(&other, "CAPABILITIES.md"), "# Ours\n");
    assert!(out.contains("kept CAPABILITIES.md\n"), "{out}");
}

#[test]
fn template_says_architectural_constraint() {
    // req:clm/init-result-accepted@f3e67ac2
    // req:init/template-says-architectural-constraint@edbeb5d2
    let (_tmp, dir) = repo();
    let hosts = fake_hosts();
    init(&dir, &[], &hosts);
    let arch = read(&dir, "ARCHITECTURE.md");
    let claude = read(&dir, "AGENTS.md");
    assert!(
        arch.contains("\nThe architectural constraints:"),
        "preamble: {arch}"
    );
    assert!(arch.contains("\n## Constraints\n"), "{arch}");
    assert!(!arch.contains("## Requirements"), "{arch}");
    assert!(
        claude.contains("architectural constraints apply to all work"),
        "{claude}"
    );
    assert_eq!(
        layer_entries(&dir, "architecture", &hosts),
        Vec::<String>::new()
    );
    let old = |text: &str| {
        let words: Vec<String> = text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(str::to_lowercase)
            .collect();
        words
            .windows(2)
            .any(|w| w[0] == "architecture" && w[1].starts_with("requirement"))
    };
    assert!(!old(&arch) && !old(&claude));
    let templates = Path::new(env!("CARGO_MANIFEST_DIR")).join("../clm-init/templates");
    for entry in fs::read_dir(templates).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap();
        assert!(!old(&text), "{} holds the old term", path.display());
    }
    let check = clm(&dir, &["check"], &path_with(&hosts));
    assert_eq!(check.status.code(), Some(0), "{check:?}");
}

#[test]
fn templates_say_new_words() {
    // req:clm/init-result-accepted@f3e67ac2
    // req:init/templates-say-new-words@4e249586
    let (_tmp, dir) = repo();
    let hosts = fake_hosts();
    init(&dir, &[], &hosts);
    assert!(read(&dir, "specs/SPEC.md").contains("\nstatus: unbuilt\n"));
    // No template holds a word `plain-terms-renames` replaced.
    let templates = Path::new(env!("CARGO_MANIFEST_DIR")).join("../clm-init/templates");
    for entry in fs::read_dir(templates).unwrap() {
        let path = entry.unwrap().path();
        for line in fs::read_to_string(&path).unwrap().lines() {
            assert!(
                !line.contains("specd")
                    && line.trim() != "kind: human"
                    && !line.contains("kind: friction")
                    && !line
                        .split(|c: char| !c.is_alphanumeric() && c != '_')
                        .any(|w| w == "burdens"),
                "{}: {line}",
                path.display()
            );
        }
    }
    // The model loads the tree with no violation.
    let check = clm(&dir, &["check"], &path_with(&hosts));
    assert_eq!(check.status.code(), Some(0), "{check:?}");
}

#[test]
fn architecture_template_proof() {
    // req:clm/init-result-accepted@f3e67ac2
    // req:init/architecture-template-proof@54ed99e1
    let (_tmp, dir) = repo();
    let hosts = fake_hosts();
    init(&dir, &[], &hosts);
    let arch = read(&dir, "ARCHITECTURE.md");
    // The preamble says how an entry is proved, and that no test cites one.
    let preamble = &arch[..arch.find("\n## Constraints\n").expect(&arch)];
    for needle in [
        "under `#### Enforced by`",
        "under `#### Untested`, or both",
        "A test never cites an entry",
        "`clm guide constraints`",
        "`#### Replaces`",
    ] {
        assert!(preamble.contains(needle), "{needle}: {preamble}");
    }
    assert!(!arch.contains(concat!("req", ":architecture/")), "{arch}");
    // No carried constraint is stated in the file: they come from the `clm` that runs.
    assert!(!arch.contains("## Enforced by clm"), "{arch}");
    let check = clm(&dir, &["check"], &path_with(&hosts));
    assert_eq!(check.status.code(), Some(0), "{check:?}");
}

#[test]
fn host_azure_given() {
    // req:clm/init-host-values@404984ac
    let (_tmp, dir) = repo();
    origin(&dir, "https://github.com/o/r.git");
    init(&dir, &["--host", "azure"], &fake_hosts());
    assert_eq!(host_line(&dir), "host = \"azure\"");
}

#[test]
fn host_unknown_refused() {
    // req:clm/init-host-values@404984ac
    let (_tmp, dir) = repo();
    let out = clm(
        &dir,
        &["init", "--host", "gitlab"],
        &path_with(&fake_hosts()),
    );
    assert_eq!(out.status.code(), Some(2), "clap refuses it: {out:?}");
    assert!(!dir.join("clm.toml").exists());
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 1, "only .git");
}

/// `clm config get <key>`'s value in `dir`.
fn config(dir: &Path, key: &str) -> String {
    let out = clm(dir, &["config", "get", key], "/usr/bin:/bin");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let line = String::from_utf8(out.stdout).unwrap();
    line.split('\t').next().unwrap().to_owned()
}

#[test]
fn typescript_project_sets_build_setup() {
    // req:clm/init-result-accepted@f3e67ac2
    // req:init/init-build-setup@aeb0d060
    let (_tmp, dir) = repo();
    init(&dir, &["--lang", "typescript"], &fake_hosts());
    let out = clm(&dir, &["config", "get", "build"], "/usr/bin:/bin");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let line = String::from_utf8(out.stdout).unwrap();
    assert!(
        line.starts_with("setup=test ! -f package.json || npm ci\t")
            && line.trim_end().ends_with("clm.toml"),
        "{line}"
    );
    // No package.json yet: the setup passes without calling npm.
    let failing = stubs(&["npm"], 1);
    let setup = clm(&dir, &["setup"], &path_with(&failing));
    assert_eq!(setup.status.code(), Some(0), "{setup:?}");
    assert_eq!(host_calls(&failing), "", "npm ran");
    let check = clm(&dir, &["check"], &path_with(&failing));
    assert_eq!(check.status.code(), Some(0), "{check:?}");
}

#[test]
fn setup_installs_when_package_json_exists() {
    // req:clm/init-result-accepted@f3e67ac2
    // req:init/init-build-setup@aeb0d060
    let (_tmp, dir) = repo();
    init(&dir, &["--lang", "typescript"], &fake_hosts());
    write(&dir, "package.json", "{}\n");
    let recorder = stubs(&["npm"], 0);
    let setup = clm(&dir, &["setup"], &path_with(&recorder));
    assert_eq!(setup.status.code(), Some(0), "{setup:?}");
    assert_eq!(host_calls(&recorder), "npm ci\n");
}

/// The twenty-six allow rules `clm init` seeds, in order.
fn rules() -> Vec<String> {
    let mut rules = Vec::new();
    for sub in [
        "apply",
        "branch",
        "build",
        "check",
        "claim",
        "defect",
        "gate",
        "land",
        "pr",
        "rebase",
        "setup",
        "verify",
        "workaround",
    ] {
        rules.push(format!("Bash(clm {sub})"));
        rules.push(format!("Bash(clm {sub} *)"));
    }
    rules
}

/// `permissions.allow` of `.claude/settings.json` in `dir`.
fn allowed(dir: &Path) -> Vec<String> {
    let json: serde_json::Value =
        serde_json::from_str(&read(dir, ".claude/settings.json")).unwrap();
    json["permissions"]["allow"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn init_seeds_the_allow_rules() {
    // req:clm/init-result-accepted@f3e67ac2
    // req:init/init-seeds-permissions@bda454a5
    let (_tmp, dir) = repo();
    let hosts = fake_hosts();
    init(&dir, &[], &hosts);
    assert_eq!(allowed(&dir), rules());
    assert_eq!(clm_init::UNATTENDED_RULES.to_vec(), rules());
    assert_eq!(allowed(&dir).len(), 26);
    assert!(allowed(&dir).contains(&"Bash(clm build)".to_owned()));
    assert!(allowed(&dir).contains(&"Bash(clm build *)".to_owned()));
    assert!(allowed(&dir).contains(&"Bash(clm setup)".to_owned()));
    assert!(allowed(&dir).contains(&"Bash(clm setup *)".to_owned()));
    assert!(allowed(&dir).contains(&"Bash(clm verify *)".to_owned()));
    assert!(allowed(&dir).contains(&"Bash(clm land *)".to_owned()));
    assert!(
        allowed(&dir)
            .iter()
            .all(|r| !r.contains("approve") && !r.contains("git"))
    );
    let text = read(&dir, ".claude/settings.json");
    assert!(
        text.ends_with("}\n") && text.contains("\n  \"permissions\""),
        "{text}"
    );
    let check = clm(&dir, &["check"], &path_with(&hosts));
    assert_eq!(check.status.code(), Some(0), "{check:?}");
}

#[test]
fn init_keeps_the_settings_it_finds() {
    // req:clm/init-result-accepted@f3e67ac2
    // req:init/init-seeds-permissions@bda454a5
    let (_tmp, dir) = repo();
    write(
        &dir,
        ".claude/settings.json",
        r#"{"permissions":{"deny":["x"],"allow":["Bash(make *)","Bash(clm land *)"]},"enabledPlugins":{"clm@clm":true}}"#,
    );
    let hosts = fake_hosts();
    init(&dir, &[], &hosts);
    let json: serde_json::Value =
        serde_json::from_str(&read(&dir, ".claude/settings.json")).unwrap();
    assert_eq!(json["enabledPlugins"]["clm@clm"], true);
    assert_eq!(json["permissions"]["deny"][0], "x");
    let allow = allowed(&dir);
    assert_eq!(allow[..2], ["Bash(make *)", "Bash(clm land *)"]);
    for rule in rules() {
        assert_eq!(allow.iter().filter(|r| **r == rule).count(), 1, "{rule}");
    }
    let keys: Vec<&String> = json.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["permissions", "enabledPlugins"]);
    let check = clm(&dir, &["check"], &path_with(&hosts));
    assert_eq!(check.status.code(), Some(0), "{check:?}");
}

/// A directory holding executables `names`, each appending its name and arguments as a line
/// to `log` and exiting `code`.
fn stubs(names: &[&str], code: i32) -> TempDir {
    let dir = TempDir::new().unwrap();
    let _exclusive = FORKS.write().unwrap_or_else(PoisonError::into_inner);
    for name in names {
        let script = format!(
            "#!/bin/sh\nprintf '{name} %s\\n' \"$*\" >> '{log}'\nexit {code}\n",
            log = dir.path().join("log").display(),
        );
        let path = dir.path().join(name);
        fs::write(&path, script).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    dir
}

/// Runs `scripts/clm-check.sh` of `dir` from `cwd` with `stubs` first on `PATH`.
fn run_check_script(dir: &Path, cwd: &Path, stubs: &TempDir) -> Output {
    let _shared = FORKS.read().unwrap_or_else(PoisonError::into_inner);
    Command::new("bash")
        .arg(dir.join("scripts/clm-check.sh"))
        .current_dir(cwd)
        .env("PATH", path_with(stubs))
        .output()
        .unwrap()
}

#[test]
fn empty_repository_every_language() {
    // req:clm/init-result-accepted@f3e67ac2
    // req:init/init-without-manifest@30800928
    let (_tmp, dir) = repo();
    let failing = stubs(&["cargo", "npm", "pytest"], 1);
    init(
        &dir,
        &["--lang", "rust", "--lang", "typescript", "--lang", "python"],
        &fake_hosts(),
    );
    let toml = read(&dir, "clm.toml");
    assert!(
        toml.contains("command = [\"node_modules/.bin/knip\"]"),
        "{toml}"
    );
    let check = clm(&dir, &["check"], &path_with(&failing));
    assert_eq!(check.status.code(), Some(0), "{check:?}");
    let err = String::from_utf8_lossy(&check.stderr);
    for (detector, manifest) in [
        ("rust-unused-dependencies", "Cargo.toml"),
        ("typescript-dead-code", "package.json"),
        ("python-dead-code", "pyproject.toml"),
    ] {
        // Each detector's own table names its manifest.
        let table = toml
            .split("[[detectors]]")
            .find(|t| t.contains(&format!("name = \"{detector}\"")))
            .unwrap_or_else(|| panic!("{detector}: {toml}"));
        assert!(
            table.contains(&format!("requires = \"{manifest}\"\n")),
            "{table}"
        );
        assert!(
            err.lines()
                .any(|l| l.starts_with(&format!("{detector} skipped"))),
            "{err}"
        );
    }
    // From the root and from elsewhere, the script finds nothing to run and passes.
    fs::create_dir(dir.join("sub")).unwrap();
    for cwd in [&dir, &dir.join("sub")] {
        let script = run_check_script(&dir, cwd, &failing);
        assert_eq!(script.status.code(), Some(0), "{script:?}");
    }
    assert_eq!(host_calls(&failing), "", "a stub ran");
}

#[test]
fn dependencies_not_installed() {
    // req:clm/init-result-accepted@f3e67ac2
    // req:init/init-without-manifest@30800928
    let (_tmp, dir) = repo();
    write(&dir, "package.json", "{}\n");
    init(&dir, &["--lang", "typescript"], &fake_hosts());
    let check = clm(&dir, &["check"], &path_with(&fake_hosts()));
    assert_eq!(check.status.code(), Some(1), "{check:?}");
    // Violations print on stdout.
    let violations = String::from_utf8_lossy(&check.stdout);
    let line = violations
        .lines()
        .find(|l| l.starts_with("clm.toml: check/detectors: typescript-dead-code could not run:"))
        .unwrap_or_else(|| panic!("{violations}"));
    assert!(
        line.ends_with(
            "; install with: npm ci (or npm install --save-dev knip when package.json doesn't list it)"
        ),
        "{line}"
    );
}
