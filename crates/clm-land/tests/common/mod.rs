#![allow(dead_code)]
//! A repository whose `origin` is a local bare repository, with fake gate, check and
//! `clm check` commands that log what they ran on.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clm_ledger::{Body, Record};
use tempfile::TempDir;

#[path = "../../../clm-git/tests/exec/mod.rs"]
mod exec;
pub use exec::write_exec;

/// Logs its args and directory, prints `affected-out` when it exists (`readers-out` when
/// given `--readers`, `overlap-out` when given `--overlap`), and fails with a reason on stderr when `affected-fails` exists.
const FAKE_AFFECTED: &str = r#"
echo "$*" >> "$DIR/affected.log"
pwd -P >> "$DIR/affected-pwd.log"
if [ -e "$DIR/affected-fails" ]; then
  printf '\n   \n  no graph here  \nmore\n' >&2
  exit 1
fi
if [[ " $* " == *" --why "* ]]; then
  if [ -e "$DIR/why-fails" ]; then echo "no chain here" >&2; exit 1; fi
  if [ -e "$DIR/why-out" ]; then cat "$DIR/why-out"; fi
elif [[ " $* " == *" --overlap "* ]]; then
  if [ -e "$DIR/overlap-out" ]; then cat "$DIR/overlap-out"; fi
elif [[ " $* " == *" --readers "* ]]; then
  if [ -e "$DIR/readers-out" ]; then cat "$DIR/readers-out"; fi
elif [ -e "$DIR/affected-out" ]; then cat "$DIR/affected-out"; fi
"#;

/// Logs HEAD and its args, prints `gate-out`. With `needs-approval` present it blocks with
/// `gate/approval` unless given `--approved`.
const FAKE_GATE: &str = r#"
echo "$(git rev-parse HEAD) $*" >> "$DIR/gate.log"
lock="$DIR/work/.clm/land.lock"; [ -d "$DIR/work/proj" ] && lock="$DIR/work/proj/.clm/land.lock"
if [ -e "$lock" ] && ! flock -n "$lock" true; then echo held; else echo free; fi >> "$DIR/gate-lock.log"
echo gate >> "$DIR/order.log"
cat "$DIR/gate-out"
if [ -e "$DIR/needs-approval" ] && [[ " $* " != *" --approved"* ]]; then
  echo "gate/approval: this change needs review"
  exit 1
fi
exit 0
"#;

/// Logs the tree it ran on. With `move-main` present, the first run pushes a new commit to
/// `origin`'s `main` from the `other` clone; with `move-head` naming a branch, pushes a new
/// commit to that branch on `origin`. Fails if `check-fails` exists.
const FAKE_CHECK: &str = r#"
git rev-parse 'HEAD^{tree}' >> "$DIR/check.log"
basename "$PWD" >> "$DIR/check-dir.log"
pwd -P >> "$DIR/check-pwd.log"
echo "CI=${CI-unset} CARGO_TARGET_DIR=${CARGO_TARGET_DIR-unset}" >> "$DIR/check-env.log"
echo "CARGO_INCREMENTAL=${CARGO_INCREMENTAL-unset}" >> "$DIR/check-incremental.log"
echo "${CLM_SLOT-unset}" >> "$DIR/check-slot.log"
echo "${CLM_LAND_DIR-}" >> "$DIR/check-land-dir.log"
echo "CLM_SELECTION=${CLM_SELECTION-unset}" >> "$DIR/check-selection.log"
if [ -n "${CLM_SELECTION-}" ]; then cp "$CLM_SELECTION" "$DIR/selection-copy"; fi
if [ -e "$DIR/move-main" ] && { [ ! -e "$DIR/moved" ] || grep -q always "$DIR/move-main"; }; then
  touch "$DIR/moved"
  (
    cd "$DIR/other"
    git fetch -q origin main
    git reset -q --hard origin/main
    echo "moved $(wc -l < "$DIR/check.log")" > moved.txt
    git add -A
    git commit -q -m "main moved"
    git push -q origin HEAD:main
    git rev-parse HEAD > "$DIR/moved"
  )
fi
if [ -e "$DIR/move-head" ]; then
  branch=$(cat "$DIR/move-head")
  rm "$DIR/move-head"
  (
    cd "$DIR/other"
    git fetch -q origin "$branch"
    git checkout -q --detach FETCH_HEAD
    echo moved > head-moved.txt
    git add -A
    git commit -q -m "head moved"
    git push -q origin "HEAD:refs/heads/$branch"
    git rev-parse HEAD > "$DIR/head-moved"
  )
fi
lock="$DIR/work/.clm/land.lock"; [ -d "$DIR/work/proj" ] && lock="$DIR/work/proj/.clm/land.lock"
if [ -e "$lock" ] && ! flock -n "$lock" true; then echo held; else echo free; fi >> "$DIR/check-lock.log"
if [ -e "$DIR/check-deletes" ]; then rm -f "$(cat "$DIR/check-deletes")"; fi
tree=$(git rev-parse 'HEAD^{tree}')
echo "start $tree" >> "$DIR/check-order.log"
if [ -e "$DIR/hold" ]; then
  for _ in $(seq 600); do [ -e "$DIR/release" ] && break; sleep 0.1; done
fi
echo "end $tree" >> "$DIR/check-order.log"
echo "check $(git log -1 --format=%s 'HEAD^2')" >> "$DIR/order.log"
if [ -e "$DIR/check-sleeps" ]; then sleep "$(cat "$DIR/check-sleeps")"; fi
if [ -e "$DIR/check-prints" ]; then bash "$DIR/check-prints"; fi
if [ -e "$DIR/check-builds" ]; then mkdir -p "$CARGO_TARGET_DIR" && touch "$CARGO_TARGET_DIR/built"; fi
if [ -e "$DIR/check-sparse" ]; then mkdir -p "$CARGO_TARGET_DIR" && truncate -s "$(cat "$DIR/check-sparse")" "$CARGO_TARGET_DIR/big"; fi
[ ! -e "$DIR/check-fails" ]
"#;

/// Logs the tree and its args.
const FAKE_CLM_CHECK: &str = r#"
echo "$(git rev-parse 'HEAD^{tree}') $*" >> "$DIR/clm-check.log"
pwd -P >> "$DIR/clm-check-pwd.log"
lock="$DIR/work/.clm/land.lock"; [ -d "$DIR/work/proj" ] && lock="$DIR/work/proj/.clm/land.lock"
if [ -e "$lock" ] && ! flock -n "$lock" true; then echo held; else echo free; fi >> "$DIR/clm-check-lock.log"
[ ! -e "$DIR/clm-check-fails" ]
"#;

pub struct Fixture {
    dir: TempDir,
    /// Whether the CLM repository is `proj/` in the git repository `work`.
    nested: bool,
}

impl Fixture {
    /// A clone `work` (git user `Christopher Lunny`) of a bare `origin`, on `main`, whose
    /// `clm.toml` holds `settings` and `[land] check` naming the fake check.
    pub fn new(settings: &str) -> Self {
        Self::make(settings, false, false)
    }

    /// As `new`, with the CLM repository in `work/proj/`.
    pub fn nested() -> Self {
        Self::make("", false, true)
    }

    /// A repository `work` with `host = "none"` whose `origin` is the bare repository
    /// `keep_remote` made, reached as `origin.git` like the others.
    pub fn local() -> Self {
        Self::local_with("")
    }

    /// As `local`, with `settings` in `clm.toml` too.
    pub fn local_with(settings: &str) -> Self {
        Self::make(&format!("host = \"none\"\n{settings}"), true, false)
    }

    fn make(settings: &str, kept: bool, nested: bool) -> Self {
        let f = Fixture {
            dir: TempDir::new().unwrap(),
            nested,
        };
        let d = f.dir.path();
        for (name, body) in [
            ("gate", FAKE_GATE),
            ("check", FAKE_CHECK),
            ("clm-check", FAKE_CLM_CHECK),
            ("affected", FAKE_AFFECTED),
        ] {
            let path = f.path("bin").join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let text = format!("#!/usr/bin/env bash\nDIR='{}'\n{body}", d.display());
            write_exec(&path, &text);
        }
        f.write("gate-out", "code\n");
        fs::create_dir_all(f.tmpdir()).unwrap();
        let work = f.work();
        fs::create_dir_all(&work).unwrap();
        f.git(&f.path("work"), &["init", "-q", "-b", "main"]);
        f.user(&work, "Christopher Lunny");
        let toml = format!(
            "{settings}\n[land]\ncheck = \"{}\"\n",
            f.path("bin/check").display()
        );
        // What `clm land` keeps is under `.clm/`; these are the lines `clm init` ignores.
        let files = [
            ("a.txt", "a\n"),
            ("clm.toml", &toml),
            (".gitignore", "clm.local.toml\n.clm/\n.claude/worktrees/\n"),
        ];
        f.commit_files(&files, "init");
        let bare = f.path("origin.git");
        if kept {
            let made = clm_host::keep_remote(&work, clm_config::Host::None)
                .unwrap()
                .expect("a clone with no origin gets one");
            std::os::unix::fs::symlink(made, &bare).unwrap();
            f.git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        } else {
            f.git(
                d,
                &["init", "-q", "--bare", "-b", "main", bare.to_str().unwrap()],
            );
            f.git(&work, &["remote", "add", "origin", bare.to_str().unwrap()]);
        }
        f.git(&work, &["push", "-q", "origin", "main"]);
        f.git(&work, &["fetch", "-q", "origin"]);
        f.git(d, &["clone", "-q", bare.to_str().unwrap(), "other"]);
        f.user(&f.path("other"), "Other");
        f
    }

    /// Commits `line` at the end of `clm.toml`'s `[land]` table on `main`, pushed.
    pub fn land_setting(&self, line: &str) {
        let path = self.work().join("clm.toml");
        let text = fs::read_to_string(&path).unwrap();
        self.commit_files(&[("clm.toml", &format!("{text}{line}\n"))], "land setting");
        self.git(&self.work(), &["push", "-q", "origin", "main"]);
    }

    /// The `TMPDIR` `env` gives, where `clm land` makes its temporary worktrees.
    pub fn tmpdir(&self) -> PathBuf {
        self.path("tmp")
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    pub fn work(&self) -> PathBuf {
        match self.nested {
            true => self.path("work/proj"),
            false => self.path("work"),
        }
    }

    pub fn write(&self, name: &str, text: &str) {
        fs::write(self.path(name), text).unwrap();
    }

    pub fn lines(&self, name: &str) -> Vec<String> {
        fs::read_to_string(self.path(name))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    pub fn git(&self, cwd: &Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    }

    fn user(&self, dir: &Path, name: &str) {
        self.git(dir, &["config", "user.name", name]);
        self.git(dir, &["config", "user.email", "t@example.com"]);
    }

    /// Commits `files` on the work clone's current branch.
    pub fn commit_files(&self, files: &[(&str, &str)], message: &str) -> String {
        let work = self.work();
        for (name, text) in files {
            let path = work.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        self.git(&work, &["add", "-A"]);
        self.git(&work, &["commit", "-q", "-m", message]);
        self.git(&work, &["rev-parse", "HEAD"])
    }

    /// A branch off `main` committing `files`, pushed when there is an `origin`; the work
    /// clone is back on `main` after. Returns its head.
    pub fn branch(&self, name: &str, files: &[(&str, &str)]) -> String {
        let work = self.work();
        self.git(&work, &["checkout", "-q", "-b", name, "main"]);
        let head = self.commit_files(files, name);
        self.git(&work, &["push", "-q", "origin", name]);
        self.git(&work, &["checkout", "-q", "main"]);
        head
    }

    /// Moves `origin`'s `main` on by a commit writing `file`, from the `other` clone.
    pub fn advance_main(&self, file: &str, text: &str) -> String {
        let other = self.path("other");
        self.git(&other, &["pull", "-q", "--ff-only", "origin", "main"]);
        let path = other.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
        self.git(&other, &["add", "-A"]);
        self.git(&other, &["commit", "-q", "-m", "main moved on"]);
        self.git(&other, &["push", "-q", "origin", "HEAD:main"]);
        self.git(&other, &["rev-parse", "HEAD"])
    }

    /// A linked worktree `name` of the work clone with `branch` checked out.
    pub fn linked(&self, name: &str, branch: &str) -> PathBuf {
        let path = self.path(name);
        let at = path.to_str().unwrap();
        self.git(&self.work(), &["worktree", "add", "-q", at, branch]);
        path
    }

    /// A build place `name` of the clone with `slot`, naming `branch`: the file
    /// `orchestrator`'s `place_slot` reads, in the clone's git common directory.
    pub fn place(&self, name: &str, slot: u64, branch: &str) {
        let builds = self.path("work/.git/clm/builds");
        fs::create_dir_all(&builds).unwrap();
        let text =
            format!("kind build\ntaken 2030-01-01T00:00:00Z\nslot {slot}\nbranch {branch}\n");
        fs::write(builds.join(name), text).unwrap();
    }

    /// Gives `origin` a `pre-receive` hook running `body` (bash, with `DIR` the fixture's
    /// directory and each `<old> <new> <ref>` line on stdin), or removes it given `None`.
    pub fn hook(&self, body: Option<&str>) {
        let path = self.path("origin.git/hooks/pre-receive");
        match body {
            Some(body) => {
                let d = self.dir.path().display();
                write_exec(&path, &format!("#!/usr/bin/env bash\nDIR='{d}'\n{body}"));
            }
            None => fs::remove_file(path).unwrap(),
        }
    }

    /// `rev` on `origin`.
    pub fn remote(&self, rev: &str) -> String {
        self.git(&self.path("origin.git"), &["rev-parse", rev])
    }

    /// `rev` in the work clone.
    pub fn rev(&self, rev: &str) -> String {
        self.git(&self.work(), &["rev-parse", rev])
    }

    /// The parents of `commit`, in `repo`.
    pub fn parents(&self, repo: &Path, commit: &str) -> Vec<String> {
        self.git(repo, &["rev-list", "--parents", "-n1", commit])
            .split_whitespace()
            .skip(1)
            .map(str::to_owned)
            .collect()
    }

    pub fn tree(&self, repo: &Path, commit: &str) -> String {
        self.git(repo, &["rev-parse", &format!("{commit}^{{tree}}")])
    }

    /// The environment `config` and `clm-land` read: `HOME` here, the fake gate,
    /// `clm check` and `clm affected`, and `vars`.
    pub fn env(&self, vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let map = self.vars(vars);
        move |key: &str| map.get(key).cloned()
    }

    /// The variables `env` gives, for a `clm` process.
    pub fn vars(&self, vars: &[(&str, &str)]) -> BTreeMap<String, String> {
        let mut map: BTreeMap<String, String> = BTreeMap::new();
        map.insert("HOME".into(), self.dir.path().display().to_string());
        map.insert("TMPDIR".into(), self.tmpdir().display().to_string());
        map.insert(
            "CLM_GATE".into(),
            self.path("bin/gate").display().to_string(),
        );
        map.insert(
            "CLM_CHECK".into(),
            self.path("bin/clm-check").display().to_string(),
        );
        map.insert(
            "CLM_AFFECTED".into(),
            self.path("bin/affected").display().to_string(),
        );
        for (k, v) in vars {
            map.insert(k.to_string(), v.to_string());
        }
        map
    }

    /// The fingerprint of `branch`'s diff against `origin/main`.
    pub fn fingerprint(&self, branch: &str) -> String {
        let work = self.work();
        self.git(&work, &["checkout", "-q", branch]);
        let base = "origin/main";
        let fp = clm_gate::fingerprint(&work, base).unwrap();
        self.git(&work, &["checkout", "-q", "main"]);
        fp
    }

    /// Appends `body` to the ledger of `repo` (whose git user names the writer).
    pub fn append(&self, repo: &Path, body: Body) -> Record {
        clm_ledger::append(repo, body, &self.env(&[]))
            .unwrap()
            .record
    }

    /// Makes `records` (each a record's JSON object) the whole ledger of `writer` on
    /// `origin`, as the web app writes one, with whatever `writer` and `at` they hold.
    pub fn web_ledger(&self, writer: &str, records: &[serde_json::Value]) {
        let dir = self.path(&format!("ledger-{}", writer.replace('/', "-")));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        self.git(&dir, &["init", "-q"]);
        self.user(&dir, "Web");
        let text: String = records.iter().map(|r| format!("{r}\n")).collect();
        fs::write(dir.join("records.jsonl"), text).unwrap();
        self.git(&dir, &["add", "-A"]);
        self.git(&dir, &["commit", "-q", "-m", "ledger"]);
        let bare = self.path("origin.git");
        let to = format!("HEAD:refs/heads/clm/ledger/{writer}");
        self.git(&dir, &["push", "-q", "-f", bare.to_str().unwrap(), &to]);
    }

    /// Another clone of `origin`, its git user `user`.
    pub fn clone_as(&self, name: &str, user: &str) -> PathBuf {
        let bare = self.path("origin.git");
        self.git(
            self.dir.path(),
            &["clone", "-q", bare.to_str().unwrap(), name],
        );
        let clone = self.path(name);
        self.user(&clone, user);
        clone
    }

    /// Every record the work clone sees, after fetching.
    pub fn records(&self) -> Vec<Record> {
        self.git(&self.work(), &["fetch", "-q", "origin"]);
        clm_ledger::fold(&self.work()).unwrap().records
    }

    pub fn checked(&self) -> Vec<String> {
        self.records()
            .into_iter()
            .filter_map(|r| match r.body {
                Body::Checked { code_key, .. } => Some(code_key),
                _ => None,
            })
            .collect()
    }

    pub fn landed(&self) -> Vec<Body> {
        self.records()
            .into_iter()
            .filter(|r| matches!(r.body, Body::Landed { .. }))
            .map(|r| r.body)
            .collect()
    }

    /// The clone's land tree: `.clm/land-tree` in the CLM root of the main worktree, as git
    /// names it.
    pub fn land_tree(&self) -> PathBuf {
        fs::canonicalize(self.work())
            .unwrap()
            .join(".clm/land-tree")
    }

    /// The worktrees git lists for the work clone, the main one first.
    pub fn worktrees(&self) -> Vec<PathBuf> {
        self.git(&self.work(), &["worktree", "list", "--porcelain"])
            .lines()
            .filter_map(|l| l.strip_prefix("worktree "))
            .map(PathBuf::from)
            .collect()
    }

    /// No temporary worktree is left in `TMPDIR` or in git's list: none but the main one
    /// and the land tree.
    pub fn assert_no_scratch(&self) {
        let list = self.worktrees();
        let land_tree = self.land_tree();
        assert!(
            list[1..].iter().all(|w| *w == land_tree),
            "{list:?} besides the main worktree and {}",
            land_tree.display()
        );
        let left: Vec<_> = fs::read_dir(self.tmpdir()).unwrap().collect();
        assert!(left.is_empty(), "left in TMPDIR: {left:?}");
    }

    /// A `checked` record for the code key of `rev` with the fixture's check command, and
    /// that key.
    pub fn mark_checked(&self, rev: &str) -> String {
        let key = clm_ledger::code_key(&self.work(), rev).unwrap();
        self.append(
            &self.work(),
            Body::Checked {
                code_key: key.clone(),
                command: self.check_command(),
                basis: None,
            },
        );
        key
    }

    /// The command `[land] check` names.
    pub fn check_command(&self) -> String {
        self.path("bin/check").display().to_string()
    }
}

/// The message of a refusal.
pub fn refused(result: Result<clm_land::Output, clm_land::Error>) -> String {
    match result {
        Err(clm_land::Error::Refused(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}
