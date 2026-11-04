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
const INSTALL: &str = "install --locked --path crates/clm --target-dir target/install";

/// Prints a line, records each run (`<cwd>|<args>`) and
/// installs a fake `clm` where `cargo install` would.
const STUB_CARGO: &str = r#"#!/bin/sh
echo "stub cargo output"
printf '%s|%s\n' "$PWD" "$*" >> "$STUB_RECORD"
mkdir -p "$CARGO_HOME/bin"
printf '#!/bin/sh\necho "clm 0.7.0 (stub)"\n' > "$CARGO_HOME/bin/clm"
chmod +x "$CARGO_HOME/bin/clm"
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
        r#"{ "name": "clm", "plugins": [] }"#,
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
    fs::create_dir_all(root.join("home")).unwrap();
    Clone {
        tmp,
        main,
        linked,
        stub,
    }
}

impl Clone {
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
            .output()
            .unwrap()
    }

    /// The script with the stub `cargo` first on the real `PATH`.
    fn run(&self, cwd: &Path, args: &[&str]) -> Output {
        let path = format!("{}:{}", self.stub.display(), std::env::var("PATH").unwrap());
        self.run_with_path(cwd, args, &path)
    }

    /// A `PATH` of one directory: the stub `cargo` and every program on the real `PATH`
    /// but `jq`.
    fn path_without_jq(&self) -> String {
        let dir = self.root().join("nojq");
        fs::create_dir_all(&dir).unwrap();
        std::os::unix::fs::symlink(self.stub.join("cargo"), dir.join("cargo")).unwrap();
        for part in std::env::var("PATH").unwrap().split(':') {
            let Ok(entries) = fs::read_dir(part) else {
                continue;
            };
            for entry in entries.flatten() {
                let name = entry.file_name();
                let link = dir.join(&name);
                if name != "jq" && fs::symlink_metadata(&link).is_err() {
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
    // req:clm/dev-install@295dabed
    let c = clone();
    let out = c.run(&c.linked, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(c.cargo_runs(), [format!("{}|{INSTALL}", c.main.display())]);
    let hook = fs::read_to_string(c.hook()).unwrap();
    assert!(hook.lines().any(|l| l == MARK), "{hook}");
    assert!(hook.contains(&format!("{}/scripts/dev-install.sh", c.main.display())));
    assert!(hook.contains("--rebuild"));
    assert_ne!(
        fs::metadata(c.hook()).unwrap().permissions().mode() & 0o111,
        0
    );
    let settings = c.settings_json();
    assert_eq!(
        settings["env"]["CLM_BIN"],
        json!(c.cargo_home().join("bin/clm").display().to_string())
    );
    assert_eq!(
        settings["extraKnownMarketplaces"]["clm"],
        json!({ "source": { "source": "directory", "path": c.main.display().to_string() } })
    );
    let printed = text(&out);
    assert!(printed.contains("clm 0.7.0 (stub)"), "{printed}");
    assert!(printed.contains("origin/main"), "{printed}");
}

#[test]
fn existing_settings_are_kept_and_a_second_run_changes_nothing() {
    // req:clm/dev-install@295dabed
    let c = clone();
    write(
        &c.settings(),
        r#"{"enabledPlugins": {"x@y": true}, "env": {"OTHER": "1"}}"#,
    );
    assert_eq!(c.run(&c.main, &[]).status.code(), Some(0));
    let settings = c.settings_json();
    assert_eq!(settings["enabledPlugins"], json!({"x@y": true}));
    assert_eq!(settings["env"]["OTHER"], json!("1"));
    assert!(settings["env"]["CLM_BIN"].is_string());
    assert!(settings["extraKnownMarketplaces"]["clm"].is_object());
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
    // req:clm/dev-install@295dabed
    let c = clone();
    let out = c.run_with_path(&c.main, &[], &c.path_without_jq());
    refuses(&c, out, "jq");
    assert!(!c.hook().exists() && !c.settings().exists());

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
    for bad in ["[]", r#"{"env": "x"}"#] {
        let c = clone();
        write(&c.settings(), bad);
        let out = c.run(&c.main, &[]);
        refuses(&c, out, "settings.local.json");
        assert_eq!(fs::read_to_string(c.settings()).unwrap(), bad);
        assert!(!c.hook().exists());
    }
}

#[test]
fn an_empty_settings_file_is_filled_and_keeps_its_mode() {
    // req:clm/dev-install@295dabed
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
    // req:clm/dev-install@295dabed
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
prev=
for a in "$@"; do
	[ "$prev" = --path ] && src=$a
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
mkdir -p "$CARGO_HOME/bin"
printf '#!/bin/sh\necho "clm 0.7.0 (%s)"\n' "$(echo "$head" | cut -c1-12)" > "$CARGO_HOME/bin/clm"
chmod +x "$CARGO_HOME/bin/clm"
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

    /// An installed fake `clm` whose `--version` names `commit`.
    fn install_clm_at(&self, commit: &str) {
        let bin = self.cargo_home().join("bin/clm");
        write(
            &bin,
            &format!("#!/bin/sh\necho \"clm 0.7.0 ({})\"\n", &commit[..12]),
        );
        executable(&bin);
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
    // req:clm/dev-install-rebuild@1f1802fc
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
        "install --locked --path {}/crates/clm --target-dir {}/target/install|{b}",
        src.display(),
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
    // req:clm/dev-install-rebuild@1f1802fc
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
    // req:clm/dev-install-rebuild@1f1802fc
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
    // req:clm/dev-install-rebuild@1f1802fc
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
    // req:clm/dev-install-rebuild@1f1802fc
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
    // req:clm/dev-install@295dabed
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
    // req:clm/dev-install@295dabed
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
    // req:clm/dev-install-remove@4ecdc62e
    let c = clone();
    write(&c.settings(), r#"{"enabledPlugins": {"x@y": true}}"#);
    assert_eq!(c.run(&c.main, &[]).status.code(), Some(0));
    assert!(c.hook().exists());
    write(&c.post_merge(), &format!("#!/bin/sh\n{MARK}\n"));
    fs::create_dir_all(c.common().join("clm-install-src")).unwrap();
    let out = c.run(&c.linked, &["--remove"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(!c.hook().exists(), "hook left");
    assert!(!c.post_merge().exists(), "post-merge hook left");
    assert_eq!(c.settings_json(), json!({"enabledPlugins": {"x@y": true}}));
    assert!(
        text(&out).contains("claude plugin marketplace add clunny/clm"),
        "{}",
        text(&out)
    );
    assert!(
        c.cargo_home().join("bin/clm").exists(),
        "binary uninstalled"
    );
    assert!(c.common().join("clm-install-src").exists());
    let again = c.run(&c.main, &["--remove"]);
    assert_eq!(again.status.code(), Some(0), "{}", text(&again));
}

#[test]
fn remove_keeps_foreign_hooks() {
    // req:clm/dev-install-remove@4ecdc62e
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
fn the_build_ignores_the_hooks_git_environment() {
    // req:clm/dev-install-rebuild@1f1802fc
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
    // req:clm/dev-install-rebuild@1f1802fc
    let (c, _b) = clone_to_rebuild("crates/clm/src/main.rs");
    let out = c.rebuild(&c.main, &[("STUB_FAIL", "1")]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    c.wait_unlocked();
    assert_eq!(c.cargo_runs().len(), 1, "{:?}", c.cargo_runs());
    let log = fs::read_to_string(c.common().join("clm-install.log")).unwrap();
    assert!(log.contains("exit status 7"), "{log}");
}
