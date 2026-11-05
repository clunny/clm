//! `clm init`: writes CLM's files into a git repository and prints the human's next steps.

use std::fs;
use std::path::{Path, PathBuf};

use clm_config::Host;
use clm_git::{REMOTE, git};

const ARCHITECTURE: &str = include_str!("../templates/architecture.md");
const CAPABILITIES: &str = include_str!("../templates/capabilities.md");
const ROOT_SPEC: &str = include_str!("../templates/spec.md");
const CLAUDE: &str = include_str!("../templates/claude.md");

/// What `clm land` runs before `clm check`, written by `clm init` for the project to edit.
const CHECK_SCRIPT: &str = "scripts/clm-check.sh";

/// The code files `clm init` writes, which the root node it writes owns (its `paths`).
const ROOT_PATHS: [&str; 4] = [SETTINGS, ".gitignore", "clm.toml", CHECK_SCRIPT];

/// Claude Code's project settings, where the allow rules for unattended sessions go
/// (`init-seeds-permissions`).
const SETTINGS: &str = ".claude/settings.json";

/// The same, as one clone keeps it, which Claude Code merges with [`SETTINGS`].
const LOCAL_SETTINGS: &str = ".claude/settings.local.json";

/// The allow rules for the `clm` commands unattended sessions run that write to the branch,
/// `origin` or the backlog: for each of ten subcommands, the bare form then the one with
/// arguments. `clm approve` is never among them, and neither is any `git` command.
pub const UNATTENDED_RULES: [&str; 20] = [
    "Bash(clm apply)",
    "Bash(clm apply *)",
    "Bash(clm branch)",
    "Bash(clm branch *)",
    "Bash(clm check)",
    "Bash(clm check *)",
    "Bash(clm claim)",
    "Bash(clm claim *)",
    "Bash(clm defect)",
    "Bash(clm defect *)",
    "Bash(clm gate)",
    "Bash(clm gate *)",
    "Bash(clm land)",
    "Bash(clm land *)",
    "Bash(clm pr)",
    "Bash(clm pr *)",
    "Bash(clm rebase)",
    "Bash(clm rebase *)",
    "Bash(clm workaround)",
    "Bash(clm workaround *)",
];

/// What `clm init` was asked for.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// The `--lang` values, in order; repeats count once.
    pub langs: Vec<String>,
    /// False with `--no-detectors`.
    pub detectors: bool,
    /// `--host`, when given; otherwise read from [`REMOTE`]'s URL.
    pub host: Option<Host>,
}

/// A language `--lang` accepts: its detector and its build line.
struct Lang {
    name: &'static str,
    detector: &'static str,
    command: &'static str,
    hint: &'static str,
    /// The manifest the detector's `requires` names.
    manifest: &'static str,
    /// The shell test the build line runs under (`init-without-manifest`).
    present: &'static str,
    build: &'static str,
}

const LANGS: [Lang; 3] = [
    Lang {
        name: "rust",
        detector: "rust-unused-dependencies",
        command: r#"["cargo-machete"]"#,
        hint: "cargo install --locked cargo-machete",
        manifest: "Cargo.toml",
        present: "[ -f Cargo.toml ]",
        build: "cargo test",
    },
    Lang {
        name: "typescript",
        detector: "typescript-dead-code",
        command: r#"["node_modules/.bin/knip"]"#,
        hint: "npm ci (or npm install --save-dev knip when package.json doesn't list it)",
        manifest: "package.json",
        present: "[ -f package.json ]",
        build: "npm ci && npm test",
    },
    Lang {
        name: "python",
        detector: "python-dead-code",
        command: r#"["vulture", "."]"#,
        hint: "pip install vulture",
        manifest: "pyproject.toml",
        present: "[ -f pyproject.toml ] || [ -f setup.py ]",
        build: "pytest",
    },
];

/// Sets up CLM's tooling in the CLM repository found from `cwd`, or at the top of the git
/// repository holding it when there is none (`init-root`). Returns the report to print, or
/// why nothing was written. Calls no host: every file is the project's once written.
pub fn init(cwd: &Path, opts: &Options) -> Result<String, String> {
    let langs = langs(&opts.langs)?;
    let top = top_level(cwd)?;
    // `git` prints the top with symlinks resolved; so are the directories searched.
    let cwd = fs::canonicalize(cwd).map_err(|e| format!("{}: {e}", cwd.display()))?;
    let root = clm_root(&cwd, &top).unwrap_or(&top).to_path_buf();
    let host = opts.host.unwrap_or_else(|| host_of(&root));
    // Rules are seeded on the first run only: a rerun leaves the settings as they are.
    let first_run = !root.join("specs/SPEC.md").exists();
    let mut files = Files {
        root: &root,
        out: String::new(),
        paths: Vec::new(),
    };

    let toml = clm_toml(host, if opts.detectors { &langs } else { &[] });
    files.write("clm.toml", &toml)?;
    if files.write(CHECK_SCRIPT, &check_script(&langs))? {
        make_executable(&root.join(CHECK_SCRIPT)).map_err(|e| format!("{CHECK_SCRIPT}: {e}"))?;
    }
    files.write("ARCHITECTURE.md", ARCHITECTURE)?;
    files.write("CAPABILITIES.md", CAPABILITIES)?;
    // The root spec's `outside` reads the files the bootstrap commit will hold, which depend
    // on the `.gitignore` as `clm init` leaves it (`init-root-ownership`).
    files.ignore_session_files()?;
    if first_run {
        files.seed_permissions()?;
    }
    // The root spec lists the index, so only when it will be written.
    let spec = if first_run {
        root_spec(&root)?
    } else {
        String::new()
    };
    files.write("specs/SPEC.md", &spec)?;
    files.write("CLAUDE.md", CLAUDE)?;

    let mut out = files.out;
    let steps = next_steps(
        &root,
        &files.paths,
        configured_host(&root, host),
        permission_step(&root),
    );
    if !steps.is_empty() {
        out.push_str("\nNext steps (yours):\n");
        for step in steps {
            out.push_str(&format!("- {step}\n"));
        }
    }
    Ok(out)
}

/// Writes CLM's files into `root`, noting each as `wrote` or `kept` in `out` (or, for
/// `.gitignore`, `added`).
struct Files<'a> {
    root: &'a Path,
    out: String,
    /// Every file's path, written or kept.
    paths: Vec<&'static str>,
}

impl Files<'_> {
    /// Writes `rel` when it's missing; returns whether it did.
    fn write(&mut self, rel: &'static str, content: &str) -> Result<bool, String> {
        self.paths.push(rel);
        let path = self.root.join(rel);
        if path.exists() {
            self.out.push_str(&format!("kept {rel}\n"));
            return Ok(false);
        }
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        fs::write(&path, content).map_err(|e| format!("{rel}: {e}"))?;
        self.out.push_str(&format!("wrote {rel}\n"));
        Ok(true)
    }

    /// Adds the [`UNATTENDED_RULES`] the settings lack (`init-seeds-permissions`), creating
    /// the file, `permissions` and `allow` when missing. Every other key and rule is kept,
    /// in order. A file that isn't a JSON object, or whose `permissions` or `allow` has
    /// another type, is left as it is: the next steps name the rules then.
    fn seed_permissions(&mut self) -> Result<(), String> {
        let path = self.root.join(SETTINGS);
        let existing = match fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(format!("{SETTINGS}: {e}")),
        };
        let mut doc = match &existing {
            Some(bytes) if !bytes.iter().all(u8::is_ascii_whitespace) => {
                match serde_json::from_slice::<serde_json::Value>(bytes) {
                    Ok(doc) => doc,
                    Err(_) => return Ok(()),
                }
            }
            _ => serde_json::json!({}),
        };
        let Some(top) = doc.as_object_mut() else {
            return Ok(());
        };
        let permissions = top
            .entry("permissions")
            .or_insert_with(|| serde_json::json!({}));
        let Some(permissions) = permissions.as_object_mut() else {
            return Ok(());
        };
        let allow = permissions
            .entry("allow")
            .or_insert_with(|| serde_json::json!([]));
        let Some(allow) = allow.as_array_mut() else {
            return Ok(());
        };
        let before = allow.len();
        for rule in UNATTENDED_RULES {
            if !allow.iter().any(|r| r.as_str() == Some(rule)) {
                allow.push(rule.into());
            }
        }
        self.paths.push(SETTINGS);
        if existing.is_some() && allow.len() == before {
            self.out.push_str(&format!("kept {SETTINGS}\n"));
            return Ok(());
        }
        let mut text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
        text.push('\n');
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        fs::write(&path, text).map_err(|e| format!("{SETTINGS}: {e}"))?;
        let verb = if existing.is_some() {
            "added rules to"
        } else {
            "wrote"
        };
        self.out.push_str(&format!("{verb} {SETTINGS}\n"));
        Ok(())
    }

    /// Makes `.gitignore` list each of [`IGNORED`] (`init-ignores-session-files`): writes it
    /// when missing, appends each entry git doesn't already ignore, in order, noting `added
    /// <entry> to .gitignore` for each, and leaves every other byte as it was. The file is
    /// read as bytes, so one that isn't UTF-8 is still kept or appended to, and a CRLF file
    /// gains CRLF lines. Git decides what is ignored, and a git failure fails `clm init`
    /// before the file is touched.
    fn ignore_session_files(&mut self) -> Result<(), String> {
        let rel = ".gitignore";
        let path = self.root.join(rel);
        let mut bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let lines = IGNORED.map(|(entry, _)| format!("{entry}\n")).concat();
                self.write(rel, &lines)?;
                return Ok(());
            }
            Err(e) => return Err(format!("{rel}: {e}")),
        };
        self.paths.push(rel);
        let mut missing = Vec::new();
        for (entry, probe) in IGNORED {
            if !ignored(self.root, probe)? {
                missing.push(entry);
            }
        }
        if missing.is_empty() {
            self.out.push_str(&format!("kept {rel}\n"));
            return Ok(());
        }
        let eol: &[u8] = if bytes.windows(2).any(|w| w == b"\r\n") {
            b"\r\n"
        } else {
            b"\n"
        };
        if !bytes.is_empty() && !bytes.ends_with(b"\n") {
            bytes.extend_from_slice(eol);
        }
        for entry in &missing {
            bytes.extend_from_slice(entry.as_bytes());
            bytes.extend_from_slice(eol);
        }
        fs::write(&path, bytes).map_err(|e| format!("{rel}: {e}"))?;
        for entry in missing {
            self.out.push_str(&format!("added {entry} to {rel}\n"));
        }
        Ok(())
    }
}

/// The entries `.gitignore` must list, in the order they're written, each with the path
/// git is asked about: one clone's settings (`local-settings`), a session's scratch files
/// and the session worktrees.
const IGNORED: [(&str, &str); 3] = [
    ("clm.local.toml", "clm.local.toml"),
    (".clm/", ".clm/pr-body.md"),
    (".claude/worktrees/", ".claude/worktrees/x/f"),
];

/// Whether git ignores `probe` in `root`: `git check-ignore -q --no-index` exits 0 when it
/// does and 1 when it doesn't; anything else is git's error. `--no-index` reads the patterns
/// whatever the index holds, so a tracked `clm.local.toml` the file lists still counts. One
/// path a call, since `-q` with several can't tell them apart.
fn ignored(root: &Path, probe: &str) -> Result<bool, String> {
    match git(root, &["check-ignore", "-q", "--no-index", probe]) {
        Ok(_) => Ok(true),
        Err(clm_git::Error::Failed { status, .. }) if status.code() == Some(1) => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

/// The languages named, each once, in the order first given; an unknown one is an error.
fn langs(names: &[String]) -> Result<Vec<&'static Lang>, String> {
    let mut found: Vec<&'static Lang> = Vec::new();
    for name in names {
        let Some(lang) = LANGS.iter().find(|l| l.name == name) else {
            let known: Vec<&str> = LANGS.iter().map(|l| l.name).collect();
            return Err(format!(
                "unknown language `{name}`; known: {}",
                known.join(", ")
            ));
        };
        if !found.iter().any(|l| l.name == lang.name) {
            found.push(lang);
        }
    }
    Ok(found)
}

/// The first of `cwd` and its ancestors with `specs/SPEC.md`, looking no higher than the
/// first that holds `.git` (a directory, or a worktree's file): `clm-hooks`' rule. Nor
/// higher than `top`, for a work tree whose git directory is elsewhere.
fn clm_root<'a>(cwd: &'a Path, top: &Path) -> Option<&'a Path> {
    for dir in cwd.ancestors() {
        if dir.join("specs/SPEC.md").is_file() {
            return Some(dir);
        }
        if dir == top || dir.join(".git").exists() {
            return None;
        }
    }
    None
}

/// The top level of the git repository holding `cwd`.
fn top_level(cwd: &Path) -> Result<PathBuf, String> {
    match git(cwd, &["rev-parse", "--show-toplevel"]) {
        // In the canonical form `root` takes, so the two compare equal on every platform
        // (git prints `C:/...` on Windows, where `canonicalize` gives `\\?\C:\...`).
        Ok(top) => fs::canonicalize(&top).map_err(|e| format!("{top}: {e}")),
        Err(clm_git::Error::Failed { stderr, .. }) => {
            Err(format!("not in a git repository: {stderr}"))
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Where [`REMOTE`] lives, from its URL, or `none` without one. Asks no host.
fn host_of(root: &Path) -> Host {
    let Ok(url) = git(root, &["remote", "get-url", REMOTE]) else {
        return Host::None;
    };
    let url = url.to_ascii_lowercase();
    if url.contains("github.com") {
        Host::Github
    } else {
        Host::Git
    }
}

/// The repository's `host`: what a `clm.toml` it kept sets, else `written`, the one it wrote.
fn configured_host(root: &Path, written: Host) -> Host {
    match clm_config::load(Some(root), &clm_config::process_env) {
        Ok(settings) if matches!(settings.host.layer, clm_config::Layer::File(_)) => {
            settings.host.value
        }
        _ => written,
    }
}

/// The [`UNATTENDED_RULES`] neither settings file allows, compared as exact strings, as the
/// union of both files' `permissions.allow` (Claude Code merges them). A file that can't be
/// read as that shape allows nothing.
fn missing_rules(root: &Path) -> Vec<&'static str> {
    let mut allowed = Vec::new();
    for rel in [SETTINGS, LOCAL_SETTINGS] {
        let Ok(bytes) = fs::read(root.join(rel)) else {
            continue;
        };
        let Ok(doc) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            continue;
        };
        if let Some(rules) = doc["permissions"]["allow"].as_array() {
            allowed.extend(rules.iter().filter_map(|r| r.as_str().map(str::to_owned)));
        }
    }
    UNATTENDED_RULES
        .into_iter()
        .filter(|rule| !allowed.iter().any(|a| a == rule))
        .collect()
}

/// The step naming the allow rules still missing, if any.
fn permission_step(root: &Path) -> Option<String> {
    let missing = missing_rules(root);
    (!missing.is_empty()).then(|| {
        format!(
            "Add these allow rules to `permissions.allow` in `{SETTINGS}`, so unattended \
             sessions can run `clm`: {}.",
            missing
                .iter()
                .map(|r| format!("`{r}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

/// `clm init --check-permissions`: writes nothing. Returns what to print, empty when every
/// rule is allowed; a non-empty text is the heading and each missing rule, one a line
/// (`init-check-permissions`).
pub fn check_permissions(cwd: &Path) -> Result<String, String> {
    let top = top_level(cwd)?;
    let cwd = fs::canonicalize(cwd).map_err(|e| format!("{}: {e}", cwd.display()))?;
    let root = clm_root(&cwd, &top).unwrap_or(&top);
    let missing = missing_rules(root);
    if missing.is_empty() {
        return Ok(String::new());
    }
    let mut text = format!("missing allow rules in {SETTINGS}:\n");
    for rule in missing {
        text.push_str(rule);
        text.push('\n');
    }
    Ok(text)
}

/// Sets the check script's mode so it runs.
fn make_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
}

/// The steps left for the human: the `permissions` step, then the push, skipped once `git`
/// shows it done and nothing else is left to commit.
/// Whether the host protects `main` would take asking it, so that step stays, as a
/// conditional. `paths` are all of CLM's files, written or kept.
fn next_steps(root: &Path, paths: &[&str], host: Host, permissions: Option<String>) -> Vec<String> {
    let mut steps: Vec<String> = permissions.into_iter().collect();
    // Done: nothing to commit, and `REMOTE/main` (as last fetched) has the same files, or
    // there's no remote to push to.
    let mut status = vec!["status", "--porcelain", "--"];
    status.extend(paths);
    let remote_main = format!("{REMOTE}/main");
    let mut diff = vec!["diff", "--quiet", remote_main.as_str(), "--"];
    diff.extend(paths);
    let no_remote = git(root, &["remote", "get-url", REMOTE]).is_err();
    let pushed = git(root, &status).is_ok_and(|s| s.trim().is_empty())
        && (no_remote || git(root, &diff).is_ok());
    if !pushed || !steps.is_empty() {
        steps.push(
            "Commit these files and push them to `main` yourself; from then on every branch \
             lands through `clm land`."
                .into(),
        );
    }
    if host != Host::None {
        steps.push(
            "If the host protects `main`, let the identity that runs `clm land` push it (a \
             bypass or an allowed pusher)."
                .into(),
        );
    }
    steps
}

fn clm_toml(host: Host, langs: &[&Lang]) -> String {
    let mut text = format!(
        "# CLM settings for this repository (`clm config get <key>` shows each).\n\
         \n\
         # Where the remote lives: `none`, `git` or `github`.\n\
         host = \"{host}\"\n\
         # Whose approval of a spec change counts: `any` writer's, the author's included; set\n\
         # `other` to count only a person's other than the branch's commit authors.\n\
         approval = \"any\"\n\
         \n\
         # What `clm land` runs in the tree it lands, before `clm check`.\n\
         [land]\n\
         check = \"{CHECK_SCRIPT}\"\n"
    );
    if !langs.is_empty() {
        text.push_str(
            "\n# `clm check` runs these detectors of dead code and unused dependencies \
             (`no-dead-code`).\n",
        );
    }
    for lang in langs {
        text.push_str(&format!(
            "\n[[detectors]]\nname = \"{}\"\ncommand = {}\nhint = \"{}\"\nrequires = \"{}\"\n",
            lang.detector, lang.command, lang.hint, lang.manifest
        ));
    }
    text
}

fn check_script(langs: &[&Lang]) -> String {
    let mut text = String::from(
        "#!/usr/bin/env bash\n\
         # Builds and tests this repository. `clm land` runs it before `clm check`; edit it to\n\
         # suit.\n\
         set -euo pipefail\n\
         # Run from the repository's root, wherever this is run from.\n\
         cd \"$(dirname \"${BASH_SOURCE[0]}\")/..\"\n",
    );
    // Each language's line runs once its manifest exists, so a repository with no code yet
    // passes.
    for lang in langs {
        text.push_str(&format!(
            "if {}; then\n    {}\nfi\n",
            lang.present, lang.build
        ));
    }
    text
}

/// The root node, titled after the repository's directory, owning the files `clm init`
/// writes and leaving the repository's other code `outside` until its nodes take it
/// (`init-root-ownership`).
fn root_spec(root: &Path) -> Result<String, String> {
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "project".into());
    // `outside` last, so no file name it holds is taken for a placeholder.
    Ok(ROOT_SPEC
        .replace("{{title}}", &yaml_string(&name))
        .replace("{{name}}", &name)
        .replace("{{outside}}", &outside(root)?))
}

/// The root's `outside`, as a YAML flow list: the top-level entry of each code file the
/// bootstrap commit will hold (`clm-git`'s `files_to_commit`), other than those the root owns, as `<dir>/**` or the file's name, each once, sorted
/// by bytes.
fn outside(root: &Path) -> Result<String, String> {
    let entries: std::collections::BTreeSet<String> = crate::survey::code_only(
        crate::survey::lossy(clm_git::files_to_commit(root).map_err(|e| e.to_string())?),
    )
    .into_iter()
    .filter(|f| !ROOT_PATHS.contains(&f.as_str()))
    // A staged file removed from disk isn't one `git add -A` commits.
    .filter(|f| root.join(f).symlink_metadata().is_ok())
    .map(|f| match f.split_once('/') {
        Some((dir, _)) => format!("{dir}/**"),
        None => f,
    })
    .collect();
    let quoted: Vec<String> = entries
        .iter()
        .map(|e| format!("\"{}\"", yaml_string(e)))
        .collect();
    Ok(format!("[{}]", quoted.join(", ")))
}

/// `text` escaped for a YAML double-quoted string.
fn yaml_string(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}
