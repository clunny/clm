//! `scripts/dev-install.sh` run against a scratch clone with a stub `cargo` first on `PATH`
//! (clm/dev-install, clm/dev-install-rebuild, clm/dev-install-remove).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tempfile::TempDir;

const MARK: &str = "# clm dev-install";

/// Prints a line, records each run (`<cwd>|<args>`) and installs a fake `clm` under the
/// `--root` it is given, as `cargo install` would; `uninstall` is only recorded.
const STUB_CARGO: &str = r#"#!/bin/sh
echo "stub cargo output"
printf '%s|%s\n' "$PWD" "$*" >> "$STUB_RECORD"
[ "$1" != uninstall ] || exit 0
root=$CARGO_HOME
prev=
for a in "$@"; do
	[ "$prev" = --root ] && root=$a
	prev=$a
done
mkdir -p "$root/bin"
printf '#!/bin/sh\necho "clm 0.7.0 (stub)"\n' > "$root/bin/clm"
chmod +x "$root/bin/clm"
"#;

/// Records each run (`<cwd>|<args>`) in `$STUB_CLAUDE_RECORD`. `plugin marketplace list --json`
/// prints `$STUB_CLAUDE_LIST` and `plugin list --json` prints `$STUB_CLAUDE_PLUGINS` (`[]` when
/// unset); `plugin marketplace remove <name>` deletes the `<plugin>@<name>` keys of `enabledPlugins`
/// and `extraKnownMarketplaces.<name>` from the `.claude/settings.json` of the directory it runs
/// in, when there is one, as Claude Code does; a command whose arguments hold
/// `$STUB_CLAUDE_FAIL` fails, and every `plugin` command fails when `$STUB_CLAUDE_FAIL_ALL` is set.
const STUB_CLAUDE: &str = r#"#!/bin/sh
printf '%s|%s\n' "$PWD" "$*" >> "$STUB_CLAUDE_RECORD"
[ -z "${STUB_CLAUDE_FAIL_ALL:-}" ] || exit 1
case "$*" in *"${STUB_CLAUDE_FAIL:-@@none@@}"*) echo "stub claude failing" >&2; exit 1 ;; esac
case "$*" in
"plugin marketplace remove "*)
	name=$4
	f=$PWD/.claude/settings.json
	if [ -f "$f" ]; then
		jq --arg n "$name" '(if .enabledPlugins then .enabledPlugins |= with_entries(select(.key | endswith("@" + $n) | not)) else . end) | del(.extraKnownMarketplaces[$n])' "$f" > "$f.tmp" && mv "$f.tmp" "$f"
	fi ;;
esac
case "$*" in
"plugin marketplace list --json") printf '%s\n' "${STUB_CLAUDE_LIST:-[]}" ;;
"plugin list --json") printf '%s\n' "${STUB_CLAUDE_PLUGINS:-[]}" ;;
esac
"#;

struct Clone {
    tmp: TempDir,
    /// The clone's main worktree.
    main: PathBuf,
    /// A linked worktree of it, on branch `side`.
    linked: PathBuf,
    /// The stub `cargo`'s directory.
    stub: PathBuf,
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn executable(path: &Path) {
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// A clone on `main` with a `clm` marketplace and the script, and a linked worktree.
fn clone() -> Clone {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let main = root.join("main");
    fs::create_dir_all(&main).unwrap();
    git(&main, &["init", "-q", "-b", "main"]);
    git(&main, &["config", "user.name", "t"]);
    git(&main, &["config", "user.email", "t@example.com"]);
    write(
        &main.join(".claude-plugin/marketplace.json"),
        r#"{ "name": "clm", "plugins": [
            { "name": "clm", "source": "./plugin" },
            { "name": "clm-panel", "source": "./plugin-panel" } ] }"#,
    );
    write(&main.join("plugin/.claude-plugin/plugin.json"), "{}\n");
    write(
        &main.join("plugin-panel/.claude-plugin/plugin.json"),
        "{}\n",
    );
    let script = main.join("scripts/dev-install.sh");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/dev-install.sh");
    write(&script, &fs::read_to_string(source).unwrap());
    executable(&script);
    write(&main.join("crates/clm/src/main.rs"), "fn main() {}\n");
    git(&main, &["add", "-A"]);
    git(&main, &["commit", "-qm", "init"]);
    let linked = root.join("linked");
    git(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "side",
            linked.to_str().unwrap(),
        ],
    );
    let stub = root.join("stub");
    write(&stub.join("cargo"), STUB_CARGO);
    executable(&stub.join("cargo"));
    write(&stub.join("claude"), STUB_CLAUDE);
    executable(&stub.join("claude"));
    fs::create_dir_all(root.join("home")).unwrap();
    Clone {
        tmp,
        main,
        linked,
        stub,
    }
}

impl Clone {
    /// The main worktree tracks a `.claude/settings.json` that enables `clm@clm` and lists
    /// `extraKnownMarketplaces.clm`.
    fn track_settings(&self) {
        write(
            &self.main.join(".claude/settings.json"),
            &json!({
                "enabledPlugins": { "clm@clm": true },
                "extraKnownMarketplaces": { "clm": { "source": {
                    "source": "github", "repo": "clunny/clm" } } }
            })
            .to_string(),
        );
        git(&self.main, &["add", ".claude/settings.json"]);
        git(&self.main, &["commit", "-qm", "track settings"]);
    }

    /// `git status --porcelain --untracked-files=no` in the main worktree.
    fn tracked_changes(&self) -> String {
        git(
            &self.main,
            &["status", "--porcelain", "--untracked-files=no"],
        )
    }

    fn root(&self) -> PathBuf {
        self.tmp.path().canonicalize().unwrap()
    }

    fn cargo_home(&self) -> PathBuf {
        self.root().join("cargo-home")
    }

    fn record(&self) -> PathBuf {
        self.root().join("cargo-record")
    }

    fn hook(&self) -> PathBuf {
        self.main.join(".git/hooks/reference-transaction")
    }

    /// `<common>/clm-dev`, where the script installs everything.
    fn dev(&self) -> PathBuf {
        self.main.join(".git/clm-dev")
    }

    fn claude_record(&self) -> PathBuf {
        self.root().join("claude-record")
    }

    /// The stub `claude`'s runs, as `<cwd>|<args>` lines.
    fn claude_runs(&self) -> Vec<String> {
        fs::read_to_string(self.claude_record())
            .map(|t| t.lines().map(str::to_string).collect())
            .unwrap_or_default()
    }

    /// The stub `claude`'s runs of `plugin` commands other than the marketplace listing.
    fn claude_changes(&self) -> Vec<String> {
        self.claude_runs()
            .into_iter()
            .filter(|r| !r.ends_with("|plugin marketplace list --json"))
            .collect()
    }

    /// The stub `claude` lists `clm` as a directory source at `path`.
    fn lists_clm_at(&self, path: &Path) -> (String, String) {
        (
            "STUB_CLAUDE_LIST".to_string(),
            json!([{ "name": "clm", "source": "directory", "path": path.display().to_string() }])
                .to_string(),
        )
    }

    /// `$CARGO_HOME/.crates.toml` recording `clm` from `source`.
    fn record_cargo_install(&self, source: &str) {
        write(
            &self.cargo_home().join(".crates.toml"),
            &format!("[v1]\n\"clm 0.7.0 ({source})\" = [\"clm\"]\n"),
        );
    }

    fn settings(&self) -> PathBuf {
        self.main.join(".claude/settings.local.json")
    }

    /// The stub `cargo`'s runs, as `<cwd>|<args>` lines.
    fn cargo_runs(&self) -> Vec<String> {
        fs::read_to_string(self.record())
            .map(|t| t.lines().map(str::to_string).collect())
            .unwrap_or_default()
    }

    /// `scripts/dev-install.sh <args>` in `cwd`, with `path` as `PATH`.
    fn run_with_path(&self, cwd: &Path, args: &[&str], path: &str) -> Output {
        let mut cmd = Command::new("sh");
        cmd.arg(self.main.join("scripts/dev-install.sh")).args(args);
        self.in_env(cmd, cwd, path)
    }

    fn in_env(&self, mut cmd: Command, cwd: &Path, path: &str) -> Output {
        cmd.current_dir(cwd)
            .env("PATH", path)
            .env("HOME", self.root().join("home"))
            .env("CARGO_HOME", self.cargo_home())
            .env_remove("CARGO_INSTALL_ROOT")
            .env("STUB_RECORD", self.record())
            .env("STUB_CLAUDE_RECORD", self.claude_record())
            .output()
            .unwrap()
    }

    /// The script with the stubs first on the real `PATH`.
    fn run(&self, cwd: &Path, args: &[&str]) -> Output {
        self.run_env(cwd, args, &[])
    }

    /// `run` with more variables set.
    fn run_env(&self, cwd: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
        let path = format!("{}:{}", self.stub.display(), std::env::var("PATH").unwrap());
        let mut cmd = Command::new("sh");
        cmd.arg(self.main.join("scripts/dev-install.sh")).args(args);
        for (k, v) in env {
            cmd.env(k, v);
        }
        self.in_env(cmd, cwd, &path)
    }

    /// A `PATH` of one directory: the stubs and every program on the real `PATH` but `skip`.
    fn path_without(&self, skip: &str) -> String {
        let dir = self.root().join(format!("no-{skip}"));
        fs::create_dir_all(&dir).unwrap();
        for stub in ["cargo", "claude"] {
            if stub != skip {
                std::os::unix::fs::symlink(self.stub.join(stub), dir.join(stub)).unwrap();
            }
        }
        for part in std::env::var("PATH").unwrap().split(':') {
            let Ok(entries) = fs::read_dir(part) else {
                continue;
            };
            for entry in entries.flatten() {
                let name = entry.file_name();
                let link = dir.join(&name);
                if name != skip && fs::symlink_metadata(&link).is_err() {
                    let _ = std::os::unix::fs::symlink(entry.path(), link);
                }
            }
        }
        dir.display().to_string()
    }

    fn settings_json(&self) -> Value {
        serde_json::from_str(&fs::read_to_string(self.settings()).unwrap()).unwrap()
    }
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn a_fresh_clone_is_set_up_from_a_linked_worktree() {
    // req:clm/dev-install@97cbd5cb
    let c = clone();
    let out = c.run(&c.linked, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let dev = c.dev();
    assert_eq!(
        c.cargo_runs(),
        [format!(
            "{}|install --locked --path crates/clm --root {} --target-dir target/install",
            c.main.display(),
            dev.display()
        )]
    );
    let hook = fs::read_to_string(c.hook()).unwrap();
    assert!(hook.lines().any(|l| l == MARK), "{hook}");
    assert!(hook.contains(&format!("{}/scripts/dev-install.sh", c.main.display())));
    assert!(hook.contains("--rebuild"));
    assert_ne!(
        fs::metadata(c.hook()).unwrap().permissions().mode() & 0o111,
        0
    );
    let manifest = c.main.join(".clm/dev-marketplace.json");
    let marketplace: Value = serde_json::from_str(&fs::read_to_string(&manifest).unwrap()).unwrap();
    let tracked: Value = serde_json::from_str(
        &fs::read_to_string(c.main.join(".claude-plugin/marketplace.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(marketplace["name"], json!("clm-dev"));
    assert_eq!(marketplace["plugins"], tracked["plugins"]);
    assert!(!dev.join("marketplace").exists(), "old marketplace left");
    let main = c.main.display();
    assert_eq!(
        c.claude_changes(),
        [
            format!("{main}|plugin marketplace add {}", manifest.display()),
            format!("{main}|plugin install clm@clm-dev --scope local"),
            format!("{main}|plugin install clm-panel@clm-dev --scope local"),
            format!("{main}|plugin list --json"),
        ]
    );
    let settings = c.settings_json();
    assert_eq!(
        settings["env"]["CLM_BIN"],
        json!(dev.join("bin/clm").display().to_string())
    );
    assert_eq!(settings["enabledPlugins"]["clm@clm-dev"], json!(true));
    assert_eq!(settings["enabledPlugins"]["clm@clm"], json!(false));
    assert_eq!(settings["enabledPlugins"]["clm-panel@clm-dev"], json!(true));
    assert_eq!(settings["enabledPlugins"]["clm-panel@clm"], json!(false));
    assert!(settings.get("extraKnownMarketplaces").is_none());
    let printed = text(&out);
    assert!(printed.contains("clm 0.7.0 (stub)"), "{printed}");
    assert!(printed.contains("origin/main"), "{printed}");
    assert!(printed.contains("restarted"), "{printed}");
    assert!(printed.contains("plain `clm`"), "{printed}");
}

#[test]
fn an_earlier_symlinked_marketplace_is_replaced() {
    // req:clm/dev-install@97cbd5cb
    let c = clone();
    let old = c.dev().join("marketplace");
    write(
        &old.join(".claude-plugin/marketplace.json"),
        r#"{ "name": "clm-dev", "plugins": [{ "name": "clm", "source": "./plugin" }] }"#,
    );
    std::os::unix::fs::symlink(c.main.join("plugin"), old.join("plugin")).unwrap();
    let out = c.run(&c.main, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(!old.exists(), "the earlier marketplace is left");
    assert!(c.dev().join("bin/clm").exists());
    let manifest = c.main.join(".clm/dev-marketplace.json");
    let runs = c.claude_runs();
    assert!(
        runs.contains(&format!(
            "{}|plugin marketplace add {}",
            c.main.display(),
            manifest.display()
        )),
        "{runs:?}"
    );
    assert!(runs.iter().all(|r| !r.contains("marketplace remove")));
}

/// `plugin list --json` output holding `clm@clm-dev` with `errors` for the project at `path`.
fn plugin_list(path: &Path, errors: &[&str]) -> String {
    json!([{ "id": "clm@clm-dev", "projectPath": path.display().to_string(), "errors": errors }])
        .to_string()
}

#[test]
fn a_plugin_that_fails_to_load_stops_it() {
    // req:clm/dev-install@97cbd5cb
    let errors = ["Path escapes plugin directory"];
    for with_settings in [false, true] {
        let c = clone();
        if with_settings {
            write(&c.settings(), r#"{"permissions": {"allow": ["x"]}}"#);
        }
        let before = fs::read(c.settings()).ok();
        let list = plugin_list(&c.main, &errors);
        let out = c.run_env(&c.main, &[], &[("STUB_CLAUDE_PLUGINS", &list)]);
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert!(text(&out).contains("clm@clm-dev"), "{}", text(&out));
        assert!(
            text(&out).contains("Path escapes plugin directory"),
            "{}",
            text(&out)
        );
        assert_eq!(fs::read(c.settings()).ok(), before);
    }

    let c = clone();
    let list = plugin_list(&c.root().join("elsewhere"), &errors);
    let out = c.run_env(&c.main, &[], &[("STUB_CLAUDE_PLUGINS", &list)]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));

    // An entry for a plugin the marketplace doesn't list is not this script's to judge.
    let c = clone();
    let list = plugin_list(&c.main, &errors).replace("clm@clm-dev", "other@clm-dev");
    let out = c.run_env(&c.main, &[], &[("STUB_CLAUDE_PLUGINS", &list)]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));

    let c = clone();
    let list = plugin_list(&c.main, &[]);
    let out = c.run_env(&c.main, &[], &[("STUB_CLAUDE_PLUGINS", &list)]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));

    // A listing that fails, or isn't a JSON array, stops it too.
    for (k, v) in [
        ("STUB_CLAUDE_FAIL", "plugin list"),
        ("STUB_CLAUDE_PLUGINS", "{}"),
    ] {
        let c = clone();
        let out = c.run_env(&c.main, &[], &[(k, v)]);
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert!(text(&out).contains("plugin list"), "{}", text(&out));
        assert!(!c.settings().exists());
    }
}

#[test]
fn nothing_outside_the_clone_holds_the_build() {
    // req:clm/dev-install@97cbd5cb
    let c = clone();
    c.record_cargo_install("registry+https://example.com");
    let out = c.run(&c.main, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(
        !c.cargo_home().join("bin").exists(),
        "wrote under CARGO_HOME/bin"
    );
    for run in c.claude_runs() {
        assert!(!run.contains("marketplace remove"), "{run}");
        assert!(!run.contains("--scope user"), "{run}");
    }
}

#[test]
fn an_earlier_setup_moves() {
    // req:clm/dev-install@97cbd5cb
    let c = clone();
    let old_bin = c.cargo_home().join("bin/clm").display().to_string();
    write(
        &c.settings(),
        &json!({
            "env": { "CLM_BIN": old_bin },
            "extraKnownMarketplaces": { "clm": { "source": { "source": "directory",
                "path": c.main.display().to_string() } } }
        })
        .to_string(),
    );
    c.record_cargo_install("path+file:///somewhere/clm");
    let (k, v) = c.lists_clm_at(&c.main);
    let out = c.run_env(&c.linked, &[], &[(&k, &v)]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let changes = c.claude_changes();
    let main = c.main.display();
    assert_eq!(changes[0], "/|plugin marketplace remove clm");
    assert_eq!(
        changes[1],
        format!("{main}|plugin marketplace add clunny/clm")
    );
    assert!(
        c.cargo_runs().iter().any(|r| r.ends_with("|uninstall clm")),
        "{:?}",
        c.cargo_runs()
    );
    let settings = c.settings_json();
    assert_eq!(
        settings["env"]["CLM_BIN"],
        json!(c.dev().join("bin/clm").display().to_string())
    );
    assert!(
        settings.get("extraKnownMarketplaces").is_none(),
        "{settings}"
    );
    assert!(text(&out).contains("installs it again"), "{}", text(&out));
}

#[test]
fn a_release_install_is_left_in_place_and_named() {
    // req:clm/dev-install@97cbd5cb
    let c = clone();
    c.record_cargo_install("git+https://github.com/clunny/clm#abc");
    let release = c.root().join("release/bin");
    write(&release.join("clm"), "#!/bin/sh\n");
    executable(&release.join("clm"));
    let path = format!(
        "{}:{}:{}",
        c.stub.display(),
        release.display(),
        std::env::var("PATH").unwrap()
    );
    let out = c.run_with_path(&c.main, &[], &path);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(
        c.cargo_runs().iter().all(|r| !r.contains("uninstall")),
        "{:?}",
        c.cargo_runs()
    );
    let printed = text(&out);
    assert!(
        printed.contains(&release.join("clm").display().to_string()),
        "{printed}"
    );
    assert!(printed.contains("ahead of the plugin's shim"), "{printed}");
}

#[test]
fn another_clones_marketplace_record_is_kept() {
    // req:clm/dev-install@97cbd5cb
    let other = json!([{ "name": "clm", "source": "github", "repo": "clunny/clm" }]).to_string();
    let c = clone();
    let out = c.run_env(&c.main, &[], &[("STUB_CLAUDE_LIST", &other)]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(
        c.claude_runs()
            .iter()
            .all(|r| !r.contains("marketplace remove"))
    );

    let c = clone();
    let (k, v) = c.lists_clm_at(&c.root().join("elsewhere"));
    let out = c.run_env(&c.main, &[], &[(&k, &v)]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(
        c.claude_runs()
            .iter()
            .all(|r| !r.contains("marketplace remove"))
    );
}

#[test]
fn existing_settings_are_kept_and_a_second_run_changes_nothing() {
    // req:clm/dev-install@97cbd5cb
    let c = clone();
    write(
        &c.settings(),
        r#"{"permissions": {"allow": ["x"]}, "enabledPlugins": {"x@y": true}, "env": {"OTHER": "1"}}"#,
    );
    assert_eq!(c.run(&c.main, &[]).status.code(), Some(0));
    let settings = c.settings_json();
    assert_eq!(settings["permissions"], json!({"allow": ["x"]}));
    assert_eq!(settings["enabledPlugins"]["x@y"], json!(true));
    assert_eq!(settings["enabledPlugins"]["clm@clm-dev"], json!(true));
    assert_eq!(settings["env"]["OTHER"], json!("1"));
    assert!(settings["env"]["CLM_BIN"].is_string());
    let (first, hook) = (fs::read(c.settings()).unwrap(), fs::read(c.hook()).unwrap());
    assert_eq!(c.run(&c.linked, &[]).status.code(), Some(0));
    assert_eq!(fs::read(c.settings()).unwrap(), first);
    assert_eq!(fs::read(c.hook()).unwrap(), hook);
}

/// Runs the script in a refusing state and checks it changed nothing.
fn refuses(c: &Clone, out: Output, reason: &str) {
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(text(&out).contains(reason), "{}", text(&out));
    assert!(c.cargo_runs().is_empty(), "cargo ran");
}

#[test]
fn refuses_before_changing_anything() {
    // req:clm/dev-install@97cbd5cb
    for skip in ["jq", "claude"] {
        let c = clone();
        let out = c.run_with_path(&c.main, &[], &c.path_without(skip));
        refuses(&c, out, skip);
        assert!(!c.hook().exists() && !c.settings().exists() && !c.dev().exists());
    }

    let c = clone();
    git(&c.main, &["switch", "-q", "-c", "topic"]);
    let out = c.run(&c.linked, &[]);
    refuses(&c, out, "is on topic, not main");
    assert!(!c.hook().exists() && !c.settings().exists());

    let c = clone();
    write(&c.hook(), "#!/bin/sh\necho mine\n");
    let out = c.run(&c.main, &[]);
    refuses(&c, out, "reference-transaction");
    assert_eq!(
        fs::read_to_string(c.hook()).unwrap(),
        "#!/bin/sh\necho mine\n"
    );
    assert!(!c.settings().exists());

    let c = clone();
    write(&c.settings(), "{");
    let out = c.run(&c.main, &[]);
    refuses(&c, out, "settings.local.json");
    assert_eq!(fs::read_to_string(c.settings()).unwrap(), "{");
    assert!(!c.hook().exists());

    // Valid JSON that isn't an object to edit is refused before anything changes too.
    for bad in [
        "[]",
        "null",
        r#"{"env": "x"}"#,
        r#"{"env": false}"#,
        r#"{"enabledPlugins": []}"#,
    ] {
        let c = clone();
        write(&c.settings(), bad);
        let out = c.run(&c.main, &[]);
        refuses(&c, out, "settings.local.json");
        assert_eq!(fs::read_to_string(c.settings()).unwrap(), bad);
        assert!(!c.hook().exists());
    }

    // A marketplace plugin whose source isn't inside the checkout.
    let c = clone();
    write(
        &c.main.join(".claude-plugin/marketplace.json"),
        r#"{ "name": "clm", "plugins": [{ "name": "x", "source": "../x" }] }"#,
    );
    let out = c.run(&c.main, &[]);
    refuses(&c, out, "source");
    assert!(!c.hook().exists() && !c.dev().exists());

    // Claude Code's marketplace listing fails, or isn't a JSON array.
    for (k, v) in [("STUB_CLAUDE_FAIL_ALL", "1"), ("STUB_CLAUDE_LIST", "{}")] {
        let c = clone();
        let out = c.run_env(&c.main, &[], &[(k, v)]);
        refuses(&c, out, "marketplace list");
        assert!(!c.hook().exists() && !c.dev().exists());
    }
}

#[test]
fn a_failing_claude_command_stops_it() {
    // req:clm/dev-install@97cbd5cb
    let c = clone();
    write(&c.settings(), r#"{"env": {"OTHER": "1"}}"#);
    let before = fs::read(c.settings()).unwrap();
    let out = c.run_env(&c.main, &[], &[("STUB_CLAUDE_FAIL", "plugin install")]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(text(&out).contains("plugin install"), "{}", text(&out));
    assert_eq!(fs::read(c.settings()).unwrap(), before);
}

#[test]
fn an_empty_settings_file_is_filled_and_keeps_its_mode() {
    // req:clm/dev-install@97cbd5cb
    let c = clone();
    write(&c.settings(), "");
    fs::set_permissions(c.settings(), fs::Permissions::from_mode(0o644)).unwrap();
    let out = c.run(&c.main, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(c.settings_json()["env"]["CLM_BIN"].is_string());
    assert_eq!(
        fs::metadata(c.settings()).unwrap().permissions().mode() & 0o777,
        0o644
    );
}

#[test]
fn a_foreign_hook_is_replaced_with_force() {
    // req:clm/dev-install@97cbd5cb
    let c = clone();
    write(&c.hook(), "#!/bin/sh\necho mine\n");
    let out = c.run(&c.main, &["--force"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let hook = fs::read_to_string(c.hook()).unwrap();
    assert!(hook.lines().any(|l| l == MARK), "{hook}");
}

/// A stub `cargo` for rebuilds: records `<cwd>|<args>|<commit of the source it builds>`,
/// moves `refs/remotes/origin/main` to `$STUB_MOVE_TO` on its first run when that is set, and
/// installs a fake `clm` whose `--version` names the commit it was built from.
const STUB_REBUILD_CARGO: &str = r#"#!/bin/sh
[ -z "${STUB_SLEEP:-}" ] || sleep "$STUB_SLEEP"
echo "stub cargo output"
src=
root=$CARGO_HOME
prev=
for a in "$@"; do
	[ "$prev" = --path ] && src=$a
	[ "$prev" = --root ] && root=$a
	prev=$a
done
head=$(git -C "${src%/crates/clm}" rev-parse HEAD)
if [ -n "${STUB_FAIL:-}" ]; then
	printf '%s|%s|%s\n' "$PWD" "$*" "$head" >> "$STUB_RECORD"
	echo "stub failing" >&2
	exit 7
fi
if [ -n "${STUB_MOVE_TO:-}" ] && [ ! -e "$STUB_RECORD" ]; then
	git -C "$STUB_MAIN" update-ref refs/remotes/origin/main "$STUB_MOVE_TO"
fi
printf '%s|%s|%s\n' "$PWD" "$*" "$head" >> "$STUB_RECORD"
mkdir -p "$root/bin"
printf '#!/bin/sh\necho "clm 0.7.0 (%s)"\n' "$(echo "$head" | cut -c1-12)" > "$root/bin/clm"
chmod +x "$root/bin/clm"
"#;

impl Clone {
    fn common(&self) -> PathBuf {
        self.main.join(".git")
    }

    fn lock(&self) -> PathBuf {
        self.common().join("clm-install.lock")
    }

    fn post_merge(&self) -> PathBuf {
        self.common().join("hooks/post-merge")
    }

    /// Commits `path` changed on `side`, leaving `main` where it is, and returns the commit.
    fn commit_on_side(&self, path: &str) -> String {
        write(&self.linked.join(path), &format!("changed {path}\n"));
        git(&self.linked, &["add", "-A"]);
        git(&self.linked, &["commit", "-qm", "change"]);
        git(&self.linked, &["rev-parse", "HEAD"])
    }

    fn set_origin_main(&self, commit: &str) {
        git(
            &self.main,
            &["update-ref", "refs/remotes/origin/main", commit],
        );
    }

    /// An installed fake `clm` whose `--version` names `commit`, where the script installs it.
    fn install_clm_at(&self, commit: &str) {
        self.install_clm_in(&self.dev().join("bin/clm"), commit);
    }

    fn install_clm_in(&self, bin: &Path, commit: &str) {
        write(
            bin,
            &format!("#!/bin/sh\necho \"clm 0.7.0 ({})\"\n", &commit[..12]),
        );
        executable(bin);
    }

    /// `--rebuild` in `cwd` with the rebuild stub `cargo` first on `PATH`.
    fn rebuild(&self, cwd: &Path, env: &[(&str, &str)]) -> Output {
        let dir = self.root().join("stub-rebuild");
        write(&dir.join("cargo"), STUB_REBUILD_CARGO);
        executable(&dir.join("cargo"));
        let path = format!("{}:{}", dir.display(), std::env::var("PATH").unwrap());
        let mut cmd = Command::new("sh");
        cmd.arg(self.main.join("scripts/dev-install.sh"))
            .arg("--rebuild")
            .env("STUB_MAIN", &self.main);
        for (k, v) in env {
            cmd.env(k, v);
        }
        self.in_env(cmd, cwd, &path)
    }

    /// Waits up to twenty seconds for the rebuild's lock to be gone.
    fn wait_unlocked(&self) {
        let start = Instant::now();
        while self.lock().exists() && start.elapsed() < Duration::from_secs(20) {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(!self.lock().exists(), "the rebuild never finished");
    }
}

/// A clone whose installed `clm` names `A` and whose `origin/main` is at `B`, a later commit
/// changing `path`; returns the clone and `B`.
fn clone_to_rebuild(path: &str) -> (Clone, String) {
    let c = clone();
    let a = git(&c.main, &["rev-parse", "HEAD"]);
    c.install_clm_at(&a);
    let b = c.commit_on_side(path);
    c.set_origin_main(&b);
    (c, b)
}

#[test]
fn a_rust_change_on_origin_main_is_built_in_the_background() {
    // req:clm/dev-install-rebuild@a526b301
    let (c, b) = clone_to_rebuild("crates/clm/src/main.rs");
    let head = git(&c.main, &["rev-parse", "HEAD"]);
    let start = Instant::now();
    let out = c.rebuild(&c.linked, &[("STUB_SLEEP", "2")]);
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "the script waited for cargo"
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let log = c.common().join("clm-install.log");
    assert!(
        text(&out).contains(&log.display().to_string()),
        "{}",
        text(&out)
    );
    c.wait_unlocked();
    let src = c.common().join("clm-install-src");
    let runs = c.cargo_runs();
    assert_eq!(runs.len(), 1, "{runs:?}");
    let want = format!(
        "install --locked --path {}/crates/clm --root {} --target-dir {}/target/install|{b}",
        src.display(),
        c.dev().display(),
        c.main.display()
    );
    assert!(runs[0].ends_with(&want), "{runs:?}");
    assert_eq!(git(&src, &["rev-parse", "HEAD"]), b);
    assert!(
        fs::read_to_string(&log)
            .unwrap()
            .contains("stub cargo output")
    );
    assert_eq!(git(&c.main, &["rev-parse", "HEAD"]), head);
    assert!(git(&c.main, &["status", "--porcelain"]).is_empty());
}

#[test]
fn nothing_to_rebuild() {
    // req:clm/dev-install-rebuild@a526b301
    // Only a spec changed between the installed commit and origin/main.
    let (c, _b) = clone_to_rebuild("specs/SPEC.md");
    assert_eq!(c.rebuild(&c.main, &[]).status.code(), Some(0));
    c.wait_unlocked();
    assert!(c.cargo_runs().is_empty(), "cargo ran for a spec change");

    // The installed binary already names origin/main's commit.
    let (c, b) = clone_to_rebuild("crates/clm/src/main.rs");
    c.install_clm_at(&b);
    assert_eq!(c.rebuild(&c.main, &[]).status.code(), Some(0));
    c.wait_unlocked();
    assert!(c.cargo_runs().is_empty(), "cargo ran for a current binary");

    // There is no origin/main.
    let c = clone();
    assert_eq!(c.rebuild(&c.main, &[]).status.code(), Some(0));
    c.wait_unlocked();
    assert!(c.cargo_runs().is_empty(), "cargo ran with no origin/main");
}

#[test]
fn one_rebuild_at_a_time() {
    // req:clm/dev-install-rebuild@a526b301
    // A running process holds the lock.
    let (c, _b) = clone_to_rebuild("crates/clm/src/main.rs");
    let mut sleeper = Command::new("sleep").arg("30").spawn().unwrap();
    write(&c.lock().join("pid"), &sleeper.id().to_string());
    let out = c.rebuild(&c.main, &[]);
    let _ = sleeper.kill();
    let _ = sleeper.wait();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(c.cargo_runs().is_empty(), "cargo ran under a held lock");
    assert_eq!(
        fs::read_to_string(c.lock().join("pid")).unwrap(),
        sleeper.id().to_string()
    );

    // A lock with no pid file is just being taken.
    let (c, _b) = clone_to_rebuild("crates/clm/src/main.rs");
    fs::create_dir_all(c.lock()).unwrap();
    let out = c.rebuild(&c.main, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    // A build started in its place would have run by now.
    std::thread::sleep(Duration::from_secs(1));
    assert!(c.cargo_runs().is_empty(), "cargo ran under a held lock");
    assert!(c.lock().is_dir(), "the lock was taken from its holder");
    assert!(
        !c.lock().join("pid").exists(),
        "the lock was taken from its holder"
    );
}

#[test]
fn a_dead_rebuilds_lock_is_replaced() {
    // req:clm/dev-install-rebuild@a526b301
    let (c, _b) = clone_to_rebuild("crates/clm/src/main.rs");
    let mut done = Command::new("true").spawn().unwrap();
    done.wait().unwrap();
    write(&c.lock().join("pid"), &done.id().to_string());
    let out = c.rebuild(&c.main, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    c.wait_unlocked();
    assert_eq!(c.cargo_runs().len(), 1, "{:?}", c.cargo_runs());
}

#[test]
fn a_move_during_a_rebuild_is_built_too() {
    // req:clm/dev-install-rebuild@a526b301
    let (c, b) = clone_to_rebuild("crates/clm/src/main.rs");
    let next = c.commit_on_side("Cargo.lock");
    let out = c.rebuild(&c.main, &[("STUB_MOVE_TO", &next)]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    c.wait_unlocked();
    let heads: Vec<String> = c
        .cargo_runs()
        .iter()
        .map(|r| r.rsplit('|').next().unwrap().to_string())
        .collect();
    assert_eq!(heads, [b, next]);
}

#[test]
fn the_hook_fires_on_a_lands_push_only() {
    // req:clm/dev-install@97cbd5cb
    let c = clone();
    assert_eq!(c.run(&c.main, &[]).status.code(), Some(0));
    let record = c.root().join("hook-record");
    let script = c.main.join("scripts/dev-install.sh");
    write(
        &script,
        &format!("#!/bin/sh\necho \"$*\" >> '{}'\n", record.display()),
    );
    executable(&script);
    let bare = c.root().join("origin.git");
    git(&c.root(), &["init", "-q", "--bare", bare.to_str().unwrap()]);
    git(
        &c.main,
        &["remote", "add", "origin", bare.to_str().unwrap()],
    );
    let commit = c.commit_on_side("crates/clm/src/main.rs");
    git(
        &c.linked,
        &["push", "-q", "origin", &format!("{commit}:refs/heads/main")],
    );
    git(&c.linked, &["switch", "-q", "-c", "topic"]);
    write(&c.linked.join("topic.txt"), "x\n");
    git(&c.linked, &["add", "-A"]);
    git(&c.linked, &["commit", "-qm", "topic"]);
    git(&c.linked, &["fetch", "-q", "origin"]);
    assert_eq!(fs::read_to_string(&record).unwrap(), "--rebuild\n");
}

#[test]
fn an_earlier_post_merge_hook_is_removed_only_when_ours() {
    // req:clm/dev-install@97cbd5cb
    let c = clone();
    write(&c.post_merge(), &format!("#!/bin/sh\n{MARK}\nexit 0\n"));
    let out = c.run(&c.main, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(!c.post_merge().exists(), "our earlier hook left");

    let c = clone();
    write(&c.post_merge(), "#!/bin/sh\necho mine\n");
    let out = c.run(&c.main, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(
        fs::read_to_string(c.post_merge()).unwrap(),
        "#!/bin/sh\necho mine\n"
    );
}

#[test]
fn remove_undoes_the_setup() {
    // req:clm/dev-install-remove@2899ca2f
    let c = clone();
    write(&c.settings(), r#"{"permissions": {"allow": ["x"]}}"#);
    assert_eq!(c.run(&c.main, &[]).status.code(), Some(0));
    assert!(c.hook().exists() && c.dev().exists());
    write(&c.post_merge(), &format!("#!/bin/sh\n{MARK}\n"));
    fs::create_dir_all(c.common().join("clm-install-src")).unwrap();
    let before = c.claude_runs().len();
    let out = c.run(&c.linked, &["--remove"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(!c.hook().exists(), "hook left");
    assert!(!c.post_merge().exists(), "post-merge hook left");
    assert!(!c.dev().exists(), "clm-dev left");
    assert!(
        !c.main.join(".clm/dev-marketplace.json").exists(),
        "manifest left"
    );
    assert_eq!(c.settings_json(), json!({"permissions": {"allow": ["x"]}}));
    let main = c.main.display();
    let after: Vec<String> = c.claude_changes().split_off(4);
    assert_eq!(
        after,
        [
            format!("{main}|plugin uninstall clm@clm-dev --scope local"),
            format!("{main}|plugin uninstall clm-panel@clm-dev --scope local"),
            "/|plugin marketplace remove clm-dev".to_string(),
        ]
    );
    assert!(c.claude_runs().len() > before);
    assert!(text(&out).contains("release"), "{}", text(&out));
    assert!(c.common().join("clm-install-src").exists());
    let again = c.run(&c.main, &["--remove"]);
    assert_eq!(again.status.code(), Some(0), "{}", text(&again));
}

#[test]
fn remove_reads_an_earlier_setups_marketplace_when_the_manifest_is_missing() {
    // req:clm/dev-install-remove@2899ca2f
    let c = clone();
    let old = c.dev().join("marketplace/.claude-plugin/marketplace.json");
    write(
        &old,
        r#"{ "name": "clm-dev", "plugins": [{ "name": "old-one", "source": "./x" }] }"#,
    );
    write(&c.settings(), r#"{"env": {"CLM_BIN": "x/bin/clm"}}"#);
    let out = c.run(&c.main, &["--remove"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let main = c.main.display();
    assert_eq!(
        c.claude_changes(),
        [
            format!("{main}|plugin uninstall old-one@clm-dev --scope local"),
            "/|plugin marketplace remove clm-dev".to_string(),
        ]
    );
    assert!(!c.dev().exists());

    // With only the manifest file left and claude present, it is read and deleted.
    let c = clone();
    write(
        &c.main.join(".clm/dev-marketplace.json"),
        r#"{ "name": "clm-dev", "plugins": [{ "name": "only", "source": "./x" }] }"#,
    );
    let out = c.run(&c.main, &["--remove"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(!c.main.join(".clm/dev-marketplace.json").exists());
    assert!(
        c.claude_changes()[0].ends_with("plugin uninstall only@clm-dev --scope local"),
        "{:?}",
        c.claude_changes()
    );

    // With only the manifest file left and no claude, it refuses and changes nothing.
    let c = clone();
    write(
        &c.main.join(".clm/dev-marketplace.json"),
        r#"{ "name": "clm-dev", "plugins": [] }"#,
    );
    let out = c.run_with_path(&c.main, &["--remove"], &c.path_without("claude"));
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(c.main.join(".clm/dev-marketplace.json").exists());
}

#[test]
fn an_earlier_setup_is_undone() {
    // req:clm/dev-install-remove@2899ca2f
    let c = clone();
    let old_bin = c.cargo_home().join("bin/clm").display().to_string();
    write(
        &c.settings(),
        &json!({
            "env": { "CLM_BIN": old_bin },
            "extraKnownMarketplaces": { "clm": { "source": { "source": "directory",
                "path": c.main.display().to_string() } } }
        })
        .to_string(),
    );
    c.record_cargo_install("path+file:///somewhere/clm");
    let (k, v) = c.lists_clm_at(&c.main);
    let out = c.run_env(&c.main, &["--remove"], &[(&k, &v)]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let main = c.main.display();
    assert_eq!(
        c.claude_changes(),
        [
            "/|plugin marketplace remove clm".to_string(),
            format!("{main}|plugin marketplace add clunny/clm"),
        ]
    );
    assert!(c.cargo_runs().iter().any(|r| r.ends_with("|uninstall clm")));
    assert_eq!(c.settings_json(), json!({}));
}

#[test]
fn remove_goes_on_when_claude_code_has_already_forgotten() {
    // req:clm/dev-install-remove@2899ca2f
    let c = clone();
    assert_eq!(c.run(&c.main, &[]).status.code(), Some(0));
    let out = c.run_env(&c.main, &["--remove"], &[("STUB_CLAUDE_FAIL_ALL", "1")]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(!c.dev().exists());
    assert_eq!(c.settings_json(), json!({}));
}

#[test]
fn remove_without_claude_refuses_when_clm_dev_exists() {
    // req:clm/dev-install-remove@2899ca2f
    let c = clone();
    assert_eq!(c.run(&c.main, &[]).status.code(), Some(0));
    let settings = fs::read(c.settings()).unwrap();
    let out = c.run_with_path(&c.main, &["--remove"], &c.path_without("claude"));
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(text(&out).contains("claude"), "{}", text(&out));
    assert!(c.hook().exists() && c.dev().exists());
    assert_eq!(fs::read(c.settings()).unwrap(), settings);
}

#[test]
fn remove_keeps_foreign_hooks() {
    // req:clm/dev-install-remove@2899ca2f
    let c = clone();
    write(&c.hook(), "#!/bin/sh\necho mine\n");
    write(&c.post_merge(), "#!/bin/sh\necho theirs\n");
    let out = c.run(&c.main, &["--remove"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(
        fs::read_to_string(c.hook()).unwrap(),
        "#!/bin/sh\necho mine\n"
    );
    assert_eq!(
        fs::read_to_string(c.post_merge()).unwrap(),
        "#!/bin/sh\necho theirs\n"
    );
}

#[test]
fn a_clone_set_up_before_moving_is_rebuilt_where_it_was() {
    // req:clm/dev-install-rebuild@a526b301
    let c = clone();
    let a = git(&c.main, &["rev-parse", "HEAD"]);
    let old_bin = c.cargo_home().join("bin/clm");
    c.install_clm_in(&old_bin, &a);
    write(
        &c.settings(),
        &json!({ "env": { "CLM_BIN": old_bin.display().to_string() } }).to_string(),
    );
    let b = c.commit_on_side("crates/clm/src/main.rs");
    c.set_origin_main(&b);
    let out = c.rebuild(&c.main, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    c.wait_unlocked();
    let runs = c.cargo_runs();
    assert_eq!(runs.len(), 1, "{runs:?}");
    assert!(
        runs[0].contains(&format!("--root {} ", c.cargo_home().display())),
        "{runs:?}"
    );
    assert!(!c.dev().exists());
}

#[test]
fn the_build_ignores_the_hooks_git_environment() {
    // req:clm/dev-install-rebuild@a526b301
    let (c, b) = clone_to_rebuild("crates/clm/src/main.rs");
    let head = git(&c.main, &["rev-parse", "HEAD"]);
    let git_dir = c.common().display().to_string();
    let out = c.rebuild(&c.main, &[("GIT_DIR", &git_dir)]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    c.wait_unlocked();
    assert_eq!(git(&c.main, &["rev-parse", "HEAD"]), head);
    let src = c.common().join("clm-install-src");
    assert_eq!(git(&src, &["rev-parse", "HEAD"]), b);
    let runs = c.cargo_runs();
    assert_eq!(runs.len(), 1, "{runs:?}");
    assert!(runs[0].ends_with(&format!("|{b}")), "{runs:?}");
}

#[test]
fn a_failed_build_frees_the_lock_and_is_not_repeated() {
    // req:clm/dev-install-rebuild@a526b301
    let (c, _b) = clone_to_rebuild("crates/clm/src/main.rs");
    let out = c.rebuild(&c.main, &[("STUB_FAIL", "1")]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    c.wait_unlocked();
    assert_eq!(c.cargo_runs().len(), 1, "{:?}", c.cargo_runs());
    let log = fs::read_to_string(c.common().join("clm-install.log")).unwrap();
    assert!(log.contains("exit status 7"), "{log}");
}

// req:clm/dev-install@97cbd5cb
#[test]
fn the_tracked_settings_are_left_alone() {
    let c = clone();
    c.track_settings();
    let old_bin = c.cargo_home().join("bin/clm").display().to_string();
    write(
        &c.settings(),
        &json!({
            "env": { "CLM_BIN": old_bin },
            "extraKnownMarketplaces": { "clm": { "source": { "source": "directory",
                "path": c.main.display().to_string() } } }
        })
        .to_string(),
    );
    c.record_cargo_install("path+file:///somewhere/clm");
    let (k, v) = c.lists_clm_at(&c.main);
    let out = c.run_env(&c.linked, &[], &[(&k, &v)]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(c.claude_changes()[0], "/|plugin marketplace remove clm");
    assert_eq!(c.tracked_changes(), "", "{}", text(&out));
    // Run again on the clone set up: still nothing tracked changes.
    let again = c.run(&c.linked, &[]);
    assert_eq!(again.status.code(), Some(0), "{}", text(&again));
    assert_eq!(c.tracked_changes(), "");
}

// req:clm/dev-install-remove@2899ca2f
#[test]
fn remove_leaves_the_tracked_settings_alone() {
    // The clone of `undone`.
    let c = clone();
    c.track_settings();
    assert_eq!(c.run(&c.main, &[]).status.code(), Some(0));
    let out = c.run(&c.linked, &["--remove"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(
        c.claude_changes()
            .contains(&"/|plugin marketplace remove clm-dev".to_string()),
        "{:?}",
        c.claude_changes()
    );
    assert_eq!(c.tracked_changes(), "", "{}", text(&out));

    // The earlier setup of `an earlier setup undone`.
    let c = clone();
    c.track_settings();
    let old_bin = c.cargo_home().join("bin/clm").display().to_string();
    write(
        &c.settings(),
        &json!({
            "env": { "CLM_BIN": old_bin },
            "extraKnownMarketplaces": { "clm": { "source": { "source": "directory",
                "path": c.main.display().to_string() } } }
        })
        .to_string(),
    );
    let (k, v) = c.lists_clm_at(&c.main);
    let out = c.run_env(&c.main, &["--remove"], &[(&k, &v)]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(c.claude_changes()[0], "/|plugin marketplace remove clm");
    assert_eq!(c.tracked_changes(), "", "{}", text(&out));
}
