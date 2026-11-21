//! `clm init`: setting up a repository's CLM tooling, through `clm-init`'s call, with a fake
//! `gh` on `PATH` that it must never call.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::{PoisonError, RwLock};

use tempfile::TempDir;

mod common;

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

/// Calls `clm <args>` in `cwd` as the binary ran: with `path` as `PATH`, an empty
/// `XDG_CONFIG_HOME` and git's own config isolated, on this process for the call, since `git` and the tools a command starts inherit
/// them, and no user settings.
fn clm(cwd: &Path, args: &[&str], path: &str) -> Output {
    let config = TempDir::new().unwrap();
    let config_home = config.path().to_str().unwrap().to_owned();
    let env = |key: &str| match key {
        "PATH" => Some(path.to_owned()),
        "XDG_CONFIG_HOME" => Some(config_home.clone()),
        "CLM_AUTOSYNC" | "CLM_DEFECTS" => None,
        "GIT_CONFIG_GLOBAL" => Some("/dev/null".to_owned()),
        "GIT_CONFIG_NOSYSTEM" => Some("1".to_owned()),
        _ => std::env::var(key).ok(),
    };
    let _shared = FORKS.read().unwrap_or_else(PoisonError::into_inner);
    let [global, nosystem] = common::ISOLATED_GIT;
    common::call(
        cwd,
        args,
        &env,
        &[
            ("PATH", path),
            ("XDG_CONFIG_HOME", &config_home),
            global,
            nosystem,
        ],
    )
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
fn template_says_pull_request() {
    // req:init/template-says-pull-request@c09d31bb
    let (_tmp, dir) = repo();
    // No file it writes calls a host's pull request a page, as only the guide has pages.
    for (path, text) in written_for_every_language(&dir) {
        assert!(!text.contains("page's"), "{path}: {text}");
    }
}

/// The frontmatter line of `specs/SPEC.md` that starts with `key: `.
fn spec_key(dir: &Path, key: &str) -> String {
    let spec = read(dir, "specs/SPEC.md");
    spec.lines()
        .find(|l| l.starts_with(&format!("{key}: ")))
        .unwrap_or_else(|| panic!("no {key}: {spec}"))
        .to_owned()
}

const ROOT_PATHS: &str =
    r#"paths: [".claude/settings.json", ".gitignore", "clm.toml", "scripts/clm-check.sh"]"#;

#[test]
fn root_owns_what_init_writes_and_leaves_the_code_outside() {
    // req:init/init-root-ownership@e3d39505
    let (_tmp, dir) = repo();
    for file in [
        "src/main.rs",
        "src/lib/x.rs",
        "scripts/build.sh",
        "README.md",
        "docs/a.md",
    ] {
        write(&dir, file, "x\n");
    }
    git(&dir, &["add", "src", "scripts", "README.md", "docs"]);
    init(&dir, &[], &fake_hosts());
    assert_eq!(spec_key(&dir, "paths"), ROOT_PATHS);
    assert_eq!(
        spec_key(&dir, "outside"),
        r#"outside: ["README.md", "scripts/**", "src/**"]"#
    );
}

#[test]
fn root_outside_counts_the_untracked_files_the_bootstrap_commits() {
    // req:init/init-root-ownership@e3d39505
    let (_tmp, dir) = repo();
    write(&dir, "src/main.rs", "x\n");
    write(&dir, ".gitignore", "build/\n");
    git(&dir, &["add", "src", ".gitignore"]);
    for file in [
        ".claude/settings.json",
        ".mcp.json",
        "build/out.o",
        ".clm/pr-body.md",
    ] {
        write(&dir, file, "x\n");
    }
    init(&dir, &[], &fake_hosts());
    assert_eq!(
        spec_key(&dir, "outside"),
        r#"outside: [".mcp.json", "src/**"]"#
    );
}

#[test]
fn root_outside_in_an_empty_repository() {
    // req:init/init-root-ownership@e3d39505
    let (_tmp, dir) = repo();
    init(&dir, &[], &fake_hosts());
    assert_eq!(spec_key(&dir, "paths"), ROOT_PATHS);
    assert_eq!(spec_key(&dir, "outside"), "outside: []");
}

#[test]
fn root_outside_skips_clm_files_and_escapes() {
    // req:init/init-root-ownership@e3d39505
    // CLM's own files are the root's or spec paths; a name with `"` or `\` is escaped as YAML.
    let (_tmp, dir) = repo();
    for file in [
        "clm.toml",
        ".gitignore",
        "scripts/clm-check.sh",
        "ARCHITECTURE.md",
        "CAPABILITIES.md",
        "CLAUDE.md",
        "AGENTS.md",
        "specs/x/SPEC.md",
        "changes/c/change.md",
        "backlog/b.md",
        "a\"b\\c.txt",
        "Zed",
    ] {
        write(&dir, file, "x\n");
    }
    git(&dir, &["add", "-A"]);
    init(&dir, &[], &fake_hosts());
    assert_eq!(
        spec_key(&dir, "outside"),
        r#"outside: ["Zed", "a\"b\\c.txt"]"#
    );
}

#[test]
fn no_remote() {
    // req:init/init-files@86bfd1b0
    let (_tmp, dir) = repo();
    init(&dir, &[], &fake_hosts());
    assert_eq!(host_line(&dir), "host = \"none\"");
}

#[test]
fn the_host_comment() {
    // req:init/init-files@86bfd1b0
    let (_tmp, dir) = repo();
    init(&dir, &[], &fake_hosts());
    let toml = read(&dir, "clm.toml");
    let lines: Vec<&str> = toml.lines().collect();
    let at = lines.iter().position(|l| *l == "host = \"none\"").unwrap();
    let comment = lines[at - 1];
    assert!(comment.starts_with('#'), "{toml}");
    for host in ["`none`", "`git`", "`github`", "`azure`"] {
        assert!(comment.contains(host), "{host}: {comment}");
    }
}

#[test]
fn host_given() {
    // req:init/init-files@86bfd1b0
    let (_tmp, dir) = repo();
    origin(&dir, "https://github.com/o/r.git");
    init(&dir, &["--host", "git"], &fake_hosts());
    assert_eq!(host_line(&dir), "host = \"git\"");
    for host in ["none", "github"] {
        let (_tmp, dir) = repo();
        init(&dir, &["--host", host], &fake_hosts());
        assert_eq!(host_line(&dir), format!("host = \"{host}\""));
    }
}

#[test]
fn azure_remote() {
    // req:init/init-files@86bfd1b0
    let (_tmp, dir) = repo();
    origin(&dir, "https://o@dev.azure.com/o/p/_git/r");
    let hosts = fake_hosts();
    init(&dir, &[], &hosts);
    assert_eq!(host_line(&dir), "host = \"azure\"");
    assert_eq!(host_calls(&hosts), "", "clm init called a host");
}

#[test]
fn host_from_origin() {
    // req:init/init-files@86bfd1b0
    for (url, host) in [
        ("git@github.com:o/r.git", "github"),
        ("https://GitHub.com/o/r", "github"),
        ("https://o@dev.azure.com/o/p/_git/r", "azure"),
        ("https://example.com/o/r.git", "git"),
        ("ssh://git@example.com/srv/r.git", "git"),
    ] {
        let (_tmp, dir) = repo();
        origin(&dir, url);
        init(&dir, &[], &fake_hosts());
        assert_eq!(host_line(&dir), format!("host = \"{host}\""), "{url}");
    }
}

#[test]
fn project_files_kept() {
    // req:init/init-files@86bfd1b0
    let (_tmp, dir) = repo();
    write(&dir, "CLAUDE.md", "# Our rules\n");
    write(&dir, "clm.toml", "defects = \"off\"\n");
    let out = init(&dir, &["--lang", "rust"], &fake_hosts());
    assert_eq!(read(&dir, "CLAUDE.md"), "# Our rules\n");
    assert_eq!(read(&dir, "clm.toml"), "defects = \"off\"\n");
    assert!(out.contains("kept CLAUDE.md\n"), "{out}");
    assert!(out.contains("kept clm.toml\n"), "{out}");
    assert!(out.contains("wrote AGENTS.md\n"), "{out}");
    assert!(
        out.lines()
            .any(|l| l.contains("add `@AGENTS.md` to `CLAUDE.md`")),
        "{out}"
    );
    // An `AGENTS.md` of the project's own is kept.
    let (_tmp, dir) = repo();
    write(&dir, "AGENTS.md", "# Mine\n");
    let out = init(&dir, &[], &fake_hosts());
    assert_eq!(read(&dir, "AGENTS.md"), "# Mine\n");
    assert!(out.contains("kept AGENTS.md\n"), "{out}");
    // A CLAUDE.md that already imports it needs no such line.
    let (_tmp, dir) = repo();
    write(&dir, "CLAUDE.md", "# Ours\n@AGENTS.md\n");
    let out = init(&dir, &[], &fake_hosts());
    assert!(out.contains("kept CLAUDE.md\n"), "{out}");
    assert!(!out.contains("add `@AGENTS.md`"), "{out}");
    assert!(out.contains("wrote ARCHITECTURE.md\n"), "{out}");
}

/// `clm config get <key>`'s value in `dir`.
fn config(dir: &Path, key: &str) -> String {
    let out = clm(dir, &["config", "get", key], "/usr/bin:/bin");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let line = String::from_utf8(out.stdout).unwrap();
    line.split('\t').next().unwrap().to_owned()
}

#[test]
fn typescript_without_detectors_still_sets_build_setup() {
    // req:init/init-build-setup@8f81ad45
    let (_tmp, dir) = repo();
    init(
        &dir,
        &["--lang", "typescript", "--no-detectors"],
        &fake_hosts(),
    );
    assert!(
        config(&dir, "build").starts_with("setup=test ! -f package.json || npm ci"),
        "{}",
        read(&dir, "clm.toml")
    );
}

#[test]
fn other_projects_have_no_build_table() {
    // req:init/init-build-setup@8f81ad45
    let hosts = fake_hosts();
    for langs in [&["--lang", "rust"][..], &["--lang", "python"], &[]] {
        let (_tmp, dir) = repo();
        init(&dir, langs, &hosts);
        let toml = read(&dir, "clm.toml");
        assert!(!toml.contains("[build]"), "{langs:?}: {toml}");
    }
}

#[test]
fn two_languages() {
    // req:init/init-detectors@5fd8dbae
    let (_tmp, dir) = repo();
    init(&dir, &["--lang", "rust", "--lang", "python"], &fake_hosts());
    assert_eq!(
        config(&dir, "detectors"),
        "rust-unused-dependencies,python-dead-code"
    );
    let toml = read(&dir, "clm.toml");
    assert_eq!(toml.matches("\nhint = ").count(), 2, "{toml}");
}

#[test]
fn existing_codebase() {
    // req:init/init-detectors@5fd8dbae
    let (_tmp, dir) = repo();
    write(&dir, "src/main.rs", "fn main() {}\n");
    init(&dir, &["--lang", "rust", "--no-detectors"], &fake_hosts());
    assert_eq!(config(&dir, "detectors"), "");
    // The check script still builds the language.
    let script = read(&dir, "scripts/clm-check.sh");
    assert!(script.contains("cargo test"), "{script}");
}

#[test]
fn repeated_and_unknown_languages() {
    // req:init/init-detectors@5fd8dbae
    let (_tmp, dir) = repo();
    let hosts = fake_hosts();
    init(&dir, &["--lang", "rust", "--lang", "rust"], &hosts);
    assert_eq!(config(&dir, "detectors"), "rust-unused-dependencies");

    let (_tmp, dir) = repo();
    let out = clm(&dir, &["init", "--lang", "cobol"], &path_with(&hosts));
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("cobol") && err.contains("rust, typescript, python"),
        "{err}"
    );
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 1, "only .git");
}

/// Commits everything in `dir`.
fn commit_all(dir: &Path) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "x"]);
}

#[test]
fn upgrade() {
    // req:init/init-rerun@6580fa19
    // Initialised by an earlier `clm`, which also wrote a workflow; the project edited its
    // check script and settings since.
    let (_tmp, dir) = repo();
    origin(&dir, "https://github.com/o/r.git");
    let hosts = fake_hosts();
    init(&dir, &["--lang", "rust"], &hosts);
    let check = format!("{}make lint\n", read(&dir, "scripts/clm-check.sh"));
    write(&dir, "scripts/clm-check.sh", &check);
    write(&dir, "clm.toml", "host = \"git\"\n");
    write(
        &dir,
        ".github/workflows/clm.yml",
        "# Generated by clm init (clm 0.2.0).\n",
    );
    commit_all(&dir);

    let out = init(&dir, &["--lang", "rust", "--lang", "python"], &hosts);
    assert!(out.contains("kept scripts/clm-check.sh\n"), "{out}");
    assert!(out.contains("kept clm.toml\n"), "{out}");
    assert!(out.contains("kept .gitignore\n"), "{out}");
    assert_eq!(
        git(&dir, &["status", "--porcelain", "--untracked-files=all"]),
        ""
    );
}

#[test]
fn upgrade_adds_capabilities() {
    // req:init/init-rerun@6580fa19
    let (_tmp, dir) = repo();
    let hosts = fake_hosts();
    init(&dir, &["--lang", "rust"], &hosts);
    fs::remove_file(dir.join("CAPABILITIES.md")).unwrap();
    commit_all(&dir);
    let out = init(&dir, &["--lang", "rust"], &hosts);
    assert!(out.contains("wrote CAPABILITIES.md\n"), "{out}");
    assert_eq!(
        git(&dir, &["status", "--porcelain", "--untracked-files=all"]),
        "?? CAPABILITIES.md\n"
    );
}

#[test]
fn idempotent() {
    // req:init/init-rerun@6580fa19
    let (_tmp, dir) = repo();
    let hosts = fake_hosts();
    init(&dir, &["--lang", "rust", "--lang", "typescript"], &hosts);
    let first: Vec<(String, Vec<u8>)> = FILES
        .iter()
        .chain(&[".gitignore"])
        .map(|f| (f.to_string(), fs::read(dir.join(f)).unwrap()))
        .collect();
    commit_all(&dir);
    init(&dir, &["--lang", "rust", "--lang", "typescript"], &hosts);
    assert_eq!(
        git(&dir, &["status", "--porcelain", "--untracked-files=all"]),
        ""
    );

    // The same inputs in another repository give the same bytes.
    let (_tmp2, other) = repo();
    init(&other, &["--lang", "rust", "--lang", "typescript"], &hosts);
    for (file, bytes) in first {
        assert_eq!(fs::read(other.join(&file)).unwrap(), bytes, "{file}");
    }
}

/// The text after `Next steps (yours):`.
fn next_steps(out: &str) -> String {
    let (_, steps) = out
        .split_once("Next steps (yours):\n")
        .unwrap_or_else(|| panic!("no next steps: {out}"));
    steps.to_owned()
}

/// Asserts the steps name nothing of hosted CI.
fn no_ci(steps: &str) {
    for word in [
        "CLM_RELEASES_TOKEN",
        "auto-merge",
        "Actions",
        "status check",
    ] {
        assert!(!steps.contains(word), "{word}: {steps}");
    }
}

#[test]
fn next_steps_name_the_push_and_clm_land() {
    // req:init/init-next-steps@1a95eded
    let (_tmp, dir) = repo();
    let steps = next_steps(&init(&dir, &["--lang", "rust"], &fake_hosts()));
    assert!(steps.contains("push them to `main`"), "{steps}");
    assert!(steps.contains("`clm land`"), "{steps}");
    no_ci(&steps);

    // On a host, `main` may be protected: the identity that runs `clm land` must get through.
    let (_tmp, dir) = repo();
    origin(&dir, "https://github.com/o/r.git");
    let steps = next_steps(&init(&dir, &[], &fake_hosts()));
    let given = clm_host::setup_steps(clm_config::Host::Github);
    assert!(!given.is_empty());
    for line in given {
        assert!(steps.contains(&line), "{line}: {steps}");
    }
    no_ci(&steps);
}

#[test]
fn azure_next_steps() {
    // req:init/init-next-steps@1a95eded
    let (_tmp, dir) = repo();
    let steps = next_steps(&init(&dir, &["--host", "azure"], &fake_hosts()));
    let given = clm_host::setup_steps(clm_config::Host::Azure);
    assert!(!given.is_empty());
    // Each line whole, after the push step, in order.
    let lines: Vec<&str> = steps.lines().map(|l| l.trim_start_matches("- ")).collect();
    let push = step(&steps, &["push them to `main`"]);
    for (i, line) in given.iter().enumerate() {
        assert_eq!(lines.get(push + 1 + i), Some(&line.as_str()), "{steps}");
    }
    assert!(steps.contains("az login"), "{steps}");
    assert!(steps.contains("Bypass policies when pushing"), "{steps}");
    no_ci(&steps);
}

#[test]
fn next_steps_skip_what_is_done() {
    // req:init/init-next-steps@1a95eded
    // The push is left after a local commit, and not once `origin/main` has the files; the
    // host's setup steps stay, since `clm init` can't ask the host.
    let (tmp, dir) = repo();
    let hosts = fake_hosts();
    let steps = next_steps(&init(&dir, &[], &hosts));
    assert_eq!(steps.lines().count(), 1, "{steps}");
    commit_all(&dir);
    // A `none` clone gets its own remote before the command runs
    // (`clm/none-remote-before-commands`), so the push is left; the kept `clm.toml` still
    // says `none`, so no protection step.
    let steps = next_steps(&init(&dir, &[], &hosts));
    assert!(steps.contains("push them to `main`"), "{steps}");
    assert!(!steps.contains("protects `main`"), "{steps}");
    git(&dir, &["remote", "set-url", "origin", "/nowhere.git"]);
    let steps = next_steps(&init(&dir, &[], &hosts));
    assert!(steps.contains("push them to `main`"), "{steps}");
    assert!(!steps.contains("protects `main`"), "{steps}");
    git(&dir, &["remote", "remove", "origin"]);

    write(
        &dir,
        "clm.toml",
        "host = \"git\"\n[land]\ncheck = \"scripts/clm-check.sh\"\n",
    );
    commit_all(&dir);
    let origin_dir = tmp.path().join("origin.git");
    git(tmp.path(), &["init", "-q", "--bare", "origin.git"]);
    origin(&dir, origin_dir.to_str().unwrap());
    git(&dir, &["push", "-q", "origin", "main"]);
    let steps = next_steps(&init(&dir, &[], &hosts));
    assert!(!steps.contains("push them to `main`"), "{steps}");
    assert_eq!(steps.lines().count(), 1, "{steps}");
    assert!(steps.contains("protects `main`"), "{steps}");
}

/// The index of the first step line holding every one of `words`.
fn step(steps: &str, words: &[&str]) -> usize {
    steps
        .lines()
        .position(|l| words.iter().all(|w| l.contains(w)))
        .unwrap_or_else(|| panic!("no step naming {words:?}: {steps}"))
}

#[test]
fn an_old_clm_toml_gets_no_upgrade_step() {
    // req:init/init-next-steps@1a95eded
    let (_tmp, dir) = repo();
    write(&dir, "clm.toml", "approval = \"any\"\n");
    commit_all(&dir);
    let out = init(&dir, &[], &fake_hosts());
    assert_eq!(read(&dir, "clm.toml"), "approval = \"any\"\n");
    let steps = next_steps(&out);
    for word in ["clm.toml", "host", "[land]"] {
        assert!(!steps.contains(word), "{word}: {steps}");
    }
    no_ci(&steps);
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
fn init_writes_whitespace_settings_new_and_keeps_complete_ones() {
    // req:init/init-seeds-permissions@ccc79706
    let (_tmp, dir) = repo();
    write(&dir, ".claude/settings.json", "  \n");
    init(&dir, &[], &fake_hosts());
    assert_eq!(allowed(&dir), rules());
    let (_tmp, dir) = repo();
    let compact = serde_json::json!({"permissions": {"allow": rules()}}).to_string();
    write(&dir, ".claude/settings.json", &compact);
    init(&dir, &[], &fake_hosts());
    assert_eq!(read(&dir, ".claude/settings.json"), compact);
}

#[test]
fn init_leaves_settings_that_arent_json() {
    // req:init/init-seeds-permissions@ccc79706
    for text in [
        "{",
        "[]",
        r#"{"permissions":"x"}"#,
        r#"{"permissions":{"allow":{}}}"#,
    ] {
        let (_tmp, dir) = repo();
        write(&dir, ".claude/settings.json", text);
        let steps = next_steps(&init(&dir, &[], &fake_hosts()));
        assert_eq!(read(&dir, ".claude/settings.json"), text);
        step(&steps, &[".claude/settings.json", "Bash(clm land *)"]);
    }
}

#[test]
fn a_rerun_leaves_the_settings_alone() {
    // req:init/init-seeds-permissions@ccc79706
    let (_tmp, dir) = repo();
    let hosts = fake_hosts();
    init(&dir, &[], &hosts);
    fs::remove_file(dir.join(".claude/settings.json")).unwrap();
    commit_all(&dir);
    let steps = next_steps(&init(&dir, &[], &hosts));
    assert_eq!(
        git(&dir, &["status", "--porcelain", "--untracked-files=all"]),
        ""
    );
    let at = step(
        &steps,
        &[
            ".claude/settings.json",
            "Bash(clm land *)",
            "Bash(clm branch *)",
        ],
    );
    assert!(at < step(&steps, &["push them to `main`"]), "{steps}");
}

#[test]
fn a_rerun_with_the_rules_names_nothing() {
    // req:init/init-seeds-permissions@ccc79706
    let (_tmp, dir) = repo();
    let hosts = fake_hosts();
    init(&dir, &[], &hosts);
    commit_all(&dir);
    let out = init(&dir, &[], &hosts);
    assert_eq!(git(&dir, &["status", "--porcelain"]), "");
    assert!(!out.contains(".claude/settings.json"), "{out}");
}

/// Runs `clm init --check-permissions` in `dir`.
fn check_permissions(dir: &Path) -> Output {
    clm(
        dir,
        &["init", "--check-permissions"],
        &path_with(&fake_hosts()),
    )
}

const HEADING: &str = "missing allow rules in .claude/settings.json:\n";

#[test]
fn check_permissions_all_allowed() {
    // req:init/init-check-permissions@7521e60b
    let (_tmp, dir) = repo();
    init(&dir, &[], &fake_hosts());
    commit_all(&dir);
    let out = check_permissions(&dir);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
    assert_eq!(git(&dir, &["status", "--porcelain"]), "");
}

#[test]
fn check_permissions_reads_both_files() {
    // req:init/init-check-permissions@7521e60b
    let (_tmp, dir) = repo();
    let rest: Vec<String> = rules()
        .into_iter()
        .filter(|r| !r.starts_with("Bash(clm land"))
        .collect();
    write(
        &dir,
        ".claude/settings.json",
        &serde_json::json!({"permissions": {"allow": rest}}).to_string(),
    );
    write(
        &dir,
        ".claude/settings.local.json",
        r#"{"permissions":{"allow":["Bash(clm land *)"]}}"#,
    );
    let out = check_permissions(&dir);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("{HEADING}Bash(clm land)\n")
    );
}

#[test]
fn check_permissions_without_settings() {
    // req:init/init-check-permissions@7521e60b
    let (_tmp, dir) = repo();
    write(&dir, "specs/SPEC.md", "x\n");
    let out = check_permissions(&dir);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let want: String = rules().iter().map(|r| format!("{r}\n")).collect();
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("{HEADING}{want}")
    );
    assert!(!dir.join(".claude").exists());
}

#[test]
fn check_permissions_wants_the_exact_rule() {
    // req:init/init-check-permissions@7521e60b
    let (_tmp, dir) = repo();
    let mut allow: Vec<String> = rules()
        .into_iter()
        .filter(|r| !r.starts_with("Bash(clm land"))
        .collect();
    allow.push("Bash(clm land:*)".into());
    write(
        &dir,
        ".claude/settings.json",
        &serde_json::json!({"permissions": {"allow": allow}}).to_string(),
    );
    let out = check_permissions(&dir);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("{HEADING}Bash(clm land)\nBash(clm land *)\n")
    );
    // A file that isn't that shape allows nothing.
    write(&dir, ".claude/settings.json", "{");
    let out = check_permissions(&dir);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().lines().count(),
        rules().len() + 1
    );
}

#[test]
fn check_permissions_outside_a_repository_and_with_other_options() {
    // req:init/init-check-permissions@7521e60b
    let plain = TempDir::new().unwrap();
    let out = check_permissions(plain.path());
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("not in a git repository"));
    let (_tmp, dir) = repo();
    for other in [
        &["--lang", "rust"][..],
        &["--host", "git"],
        &["--no-detectors"],
    ] {
        let mut args = vec!["init", "--check-permissions"];
        args.extend(other);
        let out = clm(&dir, &args, &path_with(&fake_hosts()));
        assert_eq!(out.status.code(), Some(2), "{out:?}");
    }
    assert!(!dir.join("clm.toml").exists());
}

#[test]
fn rerun_from_a_subdirectory() {
    // req:init/init-root@57c73388
    let (_tmp, dir) = repo();
    let hosts = fake_hosts();
    init(&dir, &["--lang", "rust"], &hosts);
    write(&dir, "src/main.rs", "fn main() {}\n");
    commit_all(&dir);
    let out = init(&dir.join("src"), &["--lang", "rust"], &hosts);
    assert!(out.contains("kept specs/SPEC.md\n"), "{out}");
    assert_eq!(git(&dir, &["status", "--porcelain"]), "");
    assert_eq!(fs::read_dir(dir.join("src")).unwrap().count(), 1);
}

#[test]
fn clm_repository_in_a_subdirectory() {
    // req:init/init-root@57c73388
    // `tool/` holds files an earlier `clm` wrote, its workflow among them; the git
    // repository's top has none. The workflow is the project's now and stays as it is, and
    // a clean `git status` (untracked files included) shows nothing written outside `tool/`.
    let hosts = fake_hosts();
    let (_tmp0, model) = repo();
    init(&model, &["--lang", "rust"], &hosts);
    let (_tmp, dir) = repo();
    let tool = dir.join("tool");
    // An earlier `clm` wrote other bytes than this one, and the project has edited them.
    for file in FILES {
        write(&tool, file, &format!("{}\n# edited\n", read(&model, file)));
    }
    write(
        &tool,
        ".github/workflows/clm.yml",
        "# Generated by clm init (clm 0.2.0).\n",
    );
    write(&tool, ".gitignore", IGNORED);
    fs::create_dir_all(tool.join("changes")).unwrap();
    write(&dir, "README.md", "# shop\n");
    commit_all(&dir);

    let out = init(&tool.join("changes"), &["--lang", "rust"], &hosts);
    for file in FILES {
        assert!(out.contains(&format!("kept {file}\n")), "{out}");
    }
    assert!(!out.contains("wrote .github"), "{out}");
    assert_eq!(
        git(&dir, &["status", "--porcelain", "--untracked-files=all"]),
        ""
    );
}

#[test]
fn no_clm_repository() {
    // req:init/init-root@57c73388
    let (_tmp, dir) = repo();
    write(&dir, "src/main.rs", "fn main() {}\n");
    init(&dir.join("src"), &["--lang", "rust"], &fake_hosts());
    for file in FILES {
        assert!(dir.join(file).is_file(), "{file} missing");
    }
    assert_eq!(fs::read_dir(dir.join("src")).unwrap().count(), 1);
}

#[test]
fn search_stops_at_the_git_repository() {
    // req:init/init-root@57c73388
    // A git repository nested in a CLM repository is a repository of its own.
    let (_tmp, outer) = repo();
    let hosts = fake_hosts();
    init(&outer, &[], &hosts);
    let inner = outer.join("vendor/lib");
    fs::create_dir_all(&inner).unwrap();
    git(&inner, &["init", "-q", "-b", "main"]);
    init(&inner, &[], &hosts);
    for file in FILES {
        assert!(inner.join(file).is_file(), "{file} missing");
    }
}

#[test]
fn search_stops_at_the_top_without_a_git_entry() {
    // req:init/init-root@57c73388
    // A work tree with its git directory elsewhere has no `.git`: the search still ends at
    // its top, not at a CLM repository above it.
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "specs/SPEC.md", "outside\n");
    let tree = tmp.path().join("tree");
    fs::create_dir(&tree).unwrap();
    let git_dir = TempDir::new().unwrap();
    git(git_dir.path(), &["init", "-q", "--bare", "-b", "main", "."]);
    let hosts = fake_hosts();
    let config = TempDir::new().unwrap();
    let out = {
        let _exclusive = FORKS.write().unwrap_or_else(PoisonError::into_inner);
        let path = path_with(&hosts);
        let (git_dir, tree_dir) = (git_dir.path().to_str().unwrap(), tree.to_str().unwrap());
        let config_home = config.path().to_str().unwrap();
        let vars = [
            ("PATH", path.as_str()),
            ("XDG_CONFIG_HOME", config_home),
            ("GIT_DIR", git_dir),
            ("GIT_WORK_TREE", tree_dir),
            ("GIT_CONFIG_GLOBAL", "/dev/null"),
            ("GIT_CONFIG_NOSYSTEM", "1"),
        ];
        let env = |key: &str| match key {
            "XDG_CONFIG_HOME" => Some(config_home.to_owned()),
            _ => vars
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_owned())
                .or_else(|| std::env::var(key).ok()),
        };
        common::call(&tree, &["init", "--host", "git"], &env, &vars)
    };
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(tree.join("specs/SPEC.md").is_file(), "{out:?}");
    assert!(!tmp.path().join("clm.toml").exists(), "{out:?}");
}

#[test]
fn not_a_repository() {
    // req:init/init-files@86bfd1b0
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("plain");
    fs::create_dir(&dir).unwrap();
    let hosts = fake_hosts();
    let config = TempDir::new().unwrap();
    let out = {
        let _exclusive = FORKS.write().unwrap_or_else(PoisonError::into_inner);
        let path = path_with(&hosts);
        let ceiling = tmp.path().to_str().unwrap();
        let config_home = config.path().to_str().unwrap();
        let vars = [
            ("PATH", path.as_str()),
            ("XDG_CONFIG_HOME", config_home),
            ("GIT_CEILING_DIRECTORIES", ceiling),
        ];
        let env = |key: &str| match key {
            "XDG_CONFIG_HOME" => Some(config_home.to_owned()),
            _ => vars
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_owned())
                .or_else(|| std::env::var(key).ok()),
        };
        common::call(&dir, &["init", "--lang", "rust"], &env, &vars)
    };
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("git"),
        "{out:?}"
    );
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 0, "nothing written");
    assert_eq!(host_calls(&hosts), "");
}

/// The `.gitignore` `clm init` writes when there is none.
const IGNORED: &str = "clm.local.toml\n.clm/\n.claude/worktrees/\n";

/// The three paths the scenarios ask git about, and what `git check-ignore` prints for them.
const SESSION_PATHS: [&str; 3] = ["clm.local.toml", ".clm/pr-body.md", ".claude/worktrees/x/f"];
const ALL_IGNORED: &str = "clm.local.toml\n.clm/pr-body.md\n.claude/worktrees/x/f\n";

/// `git check-ignore` of the session paths in `dir`.
fn check_ignore(dir: &Path) -> Output {
    let config = TempDir::new().unwrap();
    let _shared = FORKS.read().unwrap_or_else(PoisonError::into_inner);
    Command::new("git")
        .arg("check-ignore")
        .args(SESSION_PATHS)
        .current_dir(dir)
        .env("XDG_CONFIG_HOME", config.path())
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap()
}

/// Runs `clm init` in a fresh repository whose `.gitignore` is `before`, checks git then
/// ignores every session path, and returns the file's bytes after and the output.
fn init_with_ignore(before: &[u8]) -> (Vec<u8>, String) {
    let (_tmp, dir) = repo();
    fs::write(dir.join(".gitignore"), before).unwrap();
    let out = init(&dir, &[], &fake_hosts());
    let check = check_ignore(&dir);
    assert_eq!(
        check.stdout,
        ALL_IGNORED.as_bytes(),
        "{before:?}: {check:?}"
    );
    (fs::read(dir.join(".gitignore")).unwrap(), out)
}

#[test]
fn ignore_file_written_in_a_new_repository() {
    // req:init/init-ignores-local-settings@77443238
    let (_tmp, dir) = repo();
    let out = init(&dir, &[], &fake_hosts());
    assert_eq!(read(&dir, ".gitignore"), IGNORED);
    assert!(out.contains("wrote .gitignore\n"), "{out}");
    let check = check_ignore(&dir);
    assert_eq!(check.status.code(), Some(0), "{check:?}");
    assert_eq!(check.stdout, ALL_IGNORED.as_bytes(), "{check:?}");
}

#[test]
fn empty_ignore_file() {
    // req:init/init-ignores-local-settings@77443238
    let (after, out) = init_with_ignore(b"");
    assert_eq!(after, IGNORED.as_bytes());
    assert!(
        out.contains(
            "added clm.local.toml to .gitignore\nadded .clm/ to .gitignore\n\
             added .claude/worktrees/ to .gitignore\n"
        ),
        "{out}"
    );
}

#[test]
fn ignore_lines_appended() {
    // req:init/init-ignores-local-settings@77443238
    // Each keeps its bytes and gains the missing entries, in order, each on a line of its own.
    for (before, after) in [
        (
            "node_modules/",
            "node_modules/\nclm.local.toml\n.clm/\n.claude/worktrees/\n",
        ),
        (
            "# ours\n/target/\n",
            "# ours\n/target/\nclm.local.toml\n.clm/\n.claude/worktrees/\n",
        ),
        (
            "clm.local.toml.bak\n.clm-old/\n",
            "clm.local.toml.bak\n.clm-old/\nclm.local.toml\n.clm/\n.claude/worktrees/\n",
        ),
        // A Windows-authored file gains CRLF lines.
        (
            "target/\r\n/clm.local.toml\r\n",
            "target/\r\n/clm.local.toml\r\n.clm/\r\n.claude/worktrees/\r\n",
        ),
        // Some already listed, a directory without its final `/`.
        (
            "/clm.local.toml\n/.clm\n",
            "/clm.local.toml\n/.clm\n.claude/worktrees/\n",
        ),
        // Git reads the leading spaces as part of the pattern, so it matches nothing.
        (
            "  clm.local.toml\n.clm/\n.claude/worktrees/\n",
            "  clm.local.toml\n.clm/\n.claude/worktrees/\nclm.local.toml\n",
        ),
        // A later negation un-ignores it; the appended line goes after it, so it wins.
        (
            "clm.local.toml\n.clm/\n.claude/worktrees/\n!clm.local.toml\n",
            "clm.local.toml\n.clm/\n.claude/worktrees/\n!clm.local.toml\nclm.local.toml\n",
        ),
    ] {
        let (bytes, out) = init_with_ignore(before.as_bytes());
        assert_eq!(String::from_utf8(bytes).unwrap(), after, "{before:?}");
        assert!(!out.contains("kept .gitignore"), "{out}");
    }
    let (_, out) = init_with_ignore(b"/clm.local.toml\n/.clm\n");
    assert!(
        out.contains("added .claude/worktrees/ to .gitignore\n") && !out.contains("added .clm/"),
        "{out}"
    );
    for before in [
        "  clm.local.toml\n.clm/\n.claude/worktrees/\n",
        "clm.local.toml\n.clm/\n.claude/worktrees/\n!clm.local.toml\n",
    ] {
        let (_, out) = init_with_ignore(before.as_bytes());
        assert!(
            out.contains("added clm.local.toml to .gitignore\n")
                && out.matches("added ").count() == 1,
            "{out}"
        );
    }

    // A file that isn't UTF-8 keeps its bytes.
    let (bytes, _) = init_with_ignore(b"# caf\xe9\n");
    assert_eq!(
        bytes,
        b"# caf\xe9\nclm.local.toml\n.clm/\n.claude/worktrees/\n"
    );
}

#[test]
fn ignore_lines_already_listed() {
    // req:init/init-ignores-local-settings@77443238
    for before in [
        "/clm.local.toml\n.clm/\n/.claude/worktrees\n",
        "node_modules/\nclm.local.toml  \n/.clm/  \n.claude/worktrees",
        "clm.local.toml\r\n.clm\r\n.claude/worktrees/\r\n",
        // A broader pattern is a listing.
        "clm.local.toml\n.clm/\n.claude/\n",
        "/.clm/*\n.claude/\nclm.local.toml\n",
    ] {
        let (after, out) = init_with_ignore(before.as_bytes());
        assert_eq!(String::from_utf8(after).unwrap(), before);
        assert!(out.contains("kept .gitignore\n"), "{out}");
        assert!(!out.contains("added"), "{out}");
    }
}

#[test]
fn tracked_local_settings_kept() {
    // req:init/init-ignores-local-settings@77443238
    // Git reports a tracked file as not ignored unless told to read the patterns alone.
    let (_tmp, dir) = repo();
    let before = "clm.local.toml\n.clm/\n.claude/worktrees/\n";
    write(&dir, ".gitignore", before);
    // Every probe path tracked, so none of the three probes may lean on the index.
    for path in SESSION_PATHS {
        write(&dir, path, "");
    }
    git(&dir, &["add", "-f", ".gitignore"]);
    git(
        &dir,
        &["add", "-f"]
            .into_iter()
            .chain(SESSION_PATHS)
            .collect::<Vec<_>>(),
    );
    git(&dir, &["commit", "-q", "-m", "x"]);
    let hosts = fake_hosts();
    for run in 1..=2 {
        let out = init(&dir, &[], &hosts);
        assert_eq!(read(&dir, ".gitignore"), before, "run {run}");
        assert!(out.contains("kept .gitignore\n"), "run {run}: {out}");
        assert!(!out.contains("added"), "run {run}: {out}");
    }
}

#[test]
fn ignore_check_failure_leaves_the_file() {
    // req:init/init-ignores-local-settings@77443238
    // Git refuses a probe path beyond a symbolic link: `clm init` fails with git's message.
    let (_tmp, dir) = repo();
    let elsewhere = TempDir::new().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), dir.join(".clm")).unwrap();
    write(&dir, ".gitignore", "node_modules/\n");
    let hosts = fake_hosts();
    let out = clm(&dir, &["init"], &path_with(&hosts));
    assert_ne!(out.status.code(), Some(0), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    // The command and git's own words.
    assert!(
        stderr.contains("check-ignore") && stderr.contains("symbolic link"),
        "{stderr}"
    );
    assert_eq!(read(&dir, ".gitignore"), "node_modules/\n");
}

#[test]
fn ignore_file_in_a_parent_counts() {
    // req:init/init-ignores-local-settings@77443238
    // An ignore file elsewhere counts as git counts it.
    let (_tmp, dir) = repo();
    write(&dir, ".gitignore", "clm.local.toml\n");
    write(&dir, "tool/specs/SPEC.md", "");
    write(&dir, "tool/.gitignore", ".clm/\n");
    let out = init(&dir.join("tool"), &[], &fake_hosts());
    assert_eq!(read(&dir, "tool/.gitignore"), ".clm/\n.claude/worktrees/\n");
    assert!(
        out.contains("added .claude/worktrees/ to .gitignore\n")
            && out.matches("added ").count() == 1,
        "{out}"
    );
}

#[test]
fn ignore_file_in_a_subdirectory_clm_repository() {
    // req:init/init-ignores-local-settings@77443238
    let hosts = fake_hosts();
    let (_tmp0, model) = repo();
    init(&model, &[], &hosts);
    let (_tmp, dir) = repo();
    let tool = dir.join("tool");
    for file in FILES {
        write(&tool, file, &read(&model, file));
    }
    let out = init(&tool, &[], &hosts);
    assert!(out.contains("wrote .gitignore\n"), "{out}");
    assert_eq!(read(&tool, ".gitignore"), IGNORED);
    assert!(!dir.join(".gitignore").exists());
}

#[test]
fn upgrade_adds_the_ignore_lines() {
    // req:init/init-rerun@6580fa19
    for (before, after) in [
        (
            "/target/\n",
            "/target/\nclm.local.toml\n.clm/\n.claude/worktrees/\n",
        ),
        // The one-line file an earlier `clm init` wrote.
        ("clm.local.toml\n", IGNORED),
    ] {
        let (_tmp, dir) = repo();
        let hosts = fake_hosts();
        init(&dir, &["--lang", "rust"], &hosts);
        write(&dir, ".gitignore", before);
        commit_all(&dir);
        init(&dir, &["--lang", "rust"], &hosts);
        assert_eq!(
            git(&dir, &["status", "--porcelain", "--untracked-files=all"]),
            " M .gitignore\n"
        );
        assert_eq!(read(&dir, ".gitignore"), after);
    }
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
fn manifest_added_later() {
    // req:init/init-without-manifest@dc17c7c8
    let (_tmp, dir) = repo();
    init(
        &dir,
        &["--lang", "typescript", "--lang", "python"],
        &fake_hosts(),
    );
    write(&dir, "package.json", "{}\n");
    let recorder = stubs(&["npm", "pytest"], 0);
    fs::create_dir(dir.join("sub")).unwrap();
    let out = run_check_script(&dir, &dir.join("sub"), &recorder);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(host_calls(&recorder), "npm ci\nnpm test\n");
    // `setup.py` alone counts for Python.
    write(&dir, "setup.py", "");
    let out = run_check_script(&dir, &dir, &recorder);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(host_calls(&recorder).ends_with("pytest \n"));
}

#[test]
fn rust_and_python_manifests() {
    // req:init/init-without-manifest@dc17c7c8
    let (_tmp, dir) = repo();
    init(&dir, &["--lang", "rust", "--lang", "python"], &fake_hosts());
    write(&dir, "Cargo.toml", "");
    write(&dir, "pyproject.toml", "");
    let recorder = stubs(&["cargo", "pytest"], 0);
    let out = run_check_script(&dir, &dir, &recorder);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(host_calls(&recorder), "cargo test\npytest \n");
}

#[test]
fn survey_counts_agents_md_as_a_spec_file() {
    // req:init/init-root-ownership@e3d39505
    let (_tmp, dir) = repo();
    write(&dir, "AGENTS.md", "x\n");
    write(&dir, "src/a.rs", "fn a() {}\n");
    git(&dir, &["add", "-A"]);
    init(&dir, &[], &fake_hosts());
    assert_eq!(spec_key(&dir, "outside"), r#"outside: ["src/**"]"#);
}

/// What `clm init` wrote, as `clm-model` loads it: the entries of both human layers and the
/// violations. The other commands' view of the result is the root's (`clm/init-result-accepted`).
fn loaded(dir: &Path) -> (Vec<String>, Vec<String>) {
    let (graph, violations) = clm_model::load(dir);
    let entries = graph
        .layers
        .values()
        .flatten()
        .map(|r| r.id.clone())
        .collect();
    let violations = violations.iter().map(|v| format!("{v:?}")).collect();
    (entries, violations)
}

#[test]
fn empty_repository() {
    // req:init/init-files@86bfd1b0
    let (_tmp, dir) = repo();
    origin(&dir, "https://github.com/o/r.git");
    let out = init(&dir, &["--lang", "rust"], &fake_hosts());
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
    assert!(
        toml.contains("\n[land]\ncheck = \"scripts/clm-check.sh\"\n"),
        "{toml}"
    );
    assert_eq!(read(&dir, "CLAUDE.md"), "@AGENTS.md\n");
    // `config` reads the settings back as written.
    assert_eq!(config(&dir, "host"), "github");
    assert_eq!(config(&dir, "approval"), "any");
    assert_eq!(config(&dir, "land"), "check=scripts/clm-check.sh");
}

#[test]
fn human_layers() {
    // req:init/init-files@86bfd1b0
    let (_tmp, dir) = repo();
    init(&dir, &[], &fake_hosts());
    let caps = read(&dir, "CAPABILITIES.md");
    assert!(caps.contains("\n## Capabilities\n"), "{caps}");
    // No entry the project's own tests would have to prove.
    let (entries, violations) = loaded(&dir);
    assert_eq!(entries, Vec::<String>::new());
    assert_eq!(violations, Vec::<String>::new());
    // CLM's carried constraints are named by where they are, outside `## Constraints`.
    let arch = read(&dir, "ARCHITECTURE.md");
    let before = &arch[..arch.find("\n## Constraints\n").expect(&arch)];
    assert!(before.contains("`clm guide constraints`"), "{arch}");

    // An existing CAPABILITIES.md is the project's.
    let (_tmp2, other) = repo();
    write(&other, "CAPABILITIES.md", "# Ours\n");
    let out = init(&other, &[], &fake_hosts());
    assert_eq!(read(&other, "CAPABILITIES.md"), "# Ours\n");
    assert!(out.contains("kept CAPABILITIES.md\n"), "{out}");
}

/// Whether `text` holds the old term: the word `architecture` followed by `requirement(s)`.
fn holds_old_term(text: &str) -> bool {
    let words: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    words
        .windows(2)
        .any(|w| w[0] == "architecture" && w[1].starts_with("requirement"))
}

/// Every file `clm init --lang rust --lang typescript --lang python` wrote in `dir`, as
/// (path, text).
fn written_for_every_language(dir: &Path) -> Vec<(String, String)> {
    init(dir, &EVERY_LANGUAGE, &fake_hosts());
    let written = common::written_files(dir);
    assert!(written.len() >= 9, "{written:?}");
    written
}

const EVERY_LANGUAGE: [&str; 6] = ["--lang", "rust", "--lang", "typescript", "--lang", "python"];

#[test]
fn template_says_architectural_constraint() {
    // req:init/template-says-architectural-constraint@1fc53af0
    let (_tmp, dir) = repo();
    let written = written_for_every_language(&dir);
    let arch = read(&dir, "ARCHITECTURE.md");
    let agents = read(&dir, "AGENTS.md");
    assert!(
        arch.contains("\nThe architectural constraints:"),
        "preamble: {arch}"
    );
    assert!(arch.contains("\n## Constraints\n"), "{arch}");
    assert!(!arch.contains("## Requirements"), "{arch}");
    assert!(
        agents.contains("architectural constraints apply to all work"),
        "{agents}"
    );
    for (path, text) in &written {
        assert!(!holds_old_term(text), "{path} holds the old term");
    }
    // A word is a run of letters and digits, compared in any case.
    assert!(holds_old_term("the Architecture-Requirements here"));
    assert!(holds_old_term("architecture_requirement"));
    assert!(!holds_old_term("architectures requirement"));
}

#[test]
fn templates_say_new_words() {
    // req:init/templates-say-new-words@8d1bce25
    let (_tmp, dir) = repo();
    let written = written_for_every_language(&dir);
    assert!(read(&dir, "specs/SPEC.md").contains("\nstatus: unbuilt\n"));
    // No file it wrote holds a word `plain-terms-renames` replaced.
    for (path, text) in &written {
        for line in text.lines() {
            assert!(
                !line.contains("specd")
                    && line.trim() != "kind: human"
                    && !line.contains("kind: friction")
                    && !line
                        .split(|c: char| !c.is_alphanumeric() && c != '_')
                        .any(|w| w == "burdens"),
                "{path}: {line}"
            );
        }
    }
    // `clm-model` loads the tree `clm init` wrote with no violation.
    assert_eq!(loaded(&dir).1, Vec::<String>::new());
}

#[test]
fn architecture_template_proof() {
    // req:init/architecture-template-proof@c54b4702
    let (_tmp, dir) = repo();
    init(&dir, &[], &fake_hosts());
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
}

#[test]
fn typescript_project_sets_build_setup() {
    // req:init/init-build-setup@8f81ad45
    let (_tmp, dir) = repo();
    init(&dir, &["--lang", "typescript"], &fake_hosts());
    let toml: toml::Table = read(&dir, "clm.toml").parse().unwrap();
    assert_eq!(
        toml["build"]["setup"].as_str(),
        Some("test ! -f package.json || npm ci"),
        "{toml}"
    );
}

#[test]
fn init_seeds_the_allow_rules() {
    // req:init/init-seeds-permissions@ccc79706
    let (_tmp, dir) = repo();
    init(&dir, &[], &fake_hosts());
    let allow = allowed(&dir);
    assert_eq!(allow, rules());
    assert_eq!(clm_init::UNATTENDED_RULES.to_vec(), rules());
    for rule in [
        "Bash(clm build)",
        "Bash(clm build *)",
        "Bash(clm land)",
        "Bash(clm land *)",
        "Bash(clm setup)",
        "Bash(clm setup *)",
        "Bash(clm verify *)",
    ] {
        assert!(allow.contains(&rule.to_owned()), "{rule}");
    }
    assert!(
        allow
            .iter()
            .all(|r| !r.contains("approve") && !r.contains("git"))
    );
    let text = read(&dir, ".claude/settings.json");
    assert!(
        text.ends_with("}\n") && text.contains("\n  \"permissions\""),
        "{text}"
    );
}

#[test]
fn init_keeps_the_settings_it_finds() {
    // req:init/init-seeds-permissions@ccc79706
    let (_tmp, dir) = repo();
    write(
        &dir,
        ".claude/settings.json",
        r#"{"permissions":{"deny":["x"],"allow":["Bash(make *)","Bash(clm land *)"]},"enabledPlugins":{"clm@clm":true}}"#,
    );
    init(&dir, &[], &fake_hosts());
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
}

#[test]
fn empty_repository_every_language() {
    // req:init/init-without-manifest@dc17c7c8
    let (_tmp, dir) = repo();
    let failing = stubs(&["cargo", "npm", "pytest"], 1);
    init(
        &dir,
        &["--lang", "rust", "--lang", "typescript", "--lang", "python"],
        &fake_hosts(),
    );
    let toml = read(&dir, "clm.toml");
    let table = |detector: &str| {
        toml.split("[[detectors]]")
            .find(|t| t.contains(&format!("name = \"{detector}\"")))
            .unwrap_or_else(|| panic!("{detector}: {toml}"))
    };
    // Each detector's own table names its manifest.
    for (detector, manifest) in [
        ("rust-unused-dependencies", "Cargo.toml"),
        ("typescript-dead-code", "package.json"),
        ("python-dead-code", "pyproject.toml"),
    ] {
        let table = table(detector);
        assert!(
            table.contains(&format!("requires = \"{manifest}\"\n")),
            "{table}"
        );
    }
    let typescript = table("typescript-dead-code");
    assert!(
        typescript.contains("command = [\"node_modules/.bin/knip\"]"),
        "{typescript}"
    );
    assert!(
        typescript.contains(
            "hint = \"npm ci (or npm install --save-dev knip when package.json doesn't list it)\"\n"
        ),
        "{typescript}"
    );
    // From the root and from a subdirectory, the script finds nothing to run and passes.
    fs::create_dir(dir.join("sub")).unwrap();
    for cwd in [&dir, &dir.join("sub")] {
        let script = run_check_script(&dir, cwd, &failing);
        assert_eq!(script.status.code(), Some(0), "{script:?}");
    }
    assert_eq!(host_calls(&failing), "", "a stub ran");
}
