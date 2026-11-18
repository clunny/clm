#![allow(dead_code)]
//! A repository whose `origin` is a local bare repository, for `clm approve`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clm_ledger::Record;
use tempfile::TempDir;

pub struct Fixture {
    dir: TempDir,
}

impl Fixture {
    /// A clone `work` (git user `Christopher Lunny`) of a bare `origin`, on `main`, whose
    /// `clm.toml` holds `settings`.
    pub fn new(settings: &str) -> Self {
        let f = Fixture {
            dir: TempDir::new().unwrap(),
        };
        let d = f.dir.path();
        let work = f.work();
        fs::create_dir_all(&work).unwrap();
        f.git(&work, &["init", "-q", "-b", "main"]);
        f.user(&work, "Christopher Lunny");
        f.commit_files(&[("a.txt", "a\n"), ("clm.toml", settings)], "init");
        let bare = f.path("origin.git");
        f.git(
            d,
            &["init", "-q", "--bare", "-b", "main", bare.to_str().unwrap()],
        );
        f.git(&work, &["remote", "add", "origin", bare.to_str().unwrap()]);
        f.git(&work, &["push", "-q", "origin", "main"]);
        f.git(&work, &["fetch", "-q", "origin"]);
        f.git(d, &["clone", "-q", bare.to_str().unwrap(), "other"]);
        f.user(&f.path("other"), "Other");
        f
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    pub fn work(&self) -> PathBuf {
        self.path("work")
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
    pub fn commit_files(&self, files: &[(&str, &str)], message: &str) {
        let work = self.work();
        for (name, text) in files {
            let path = work.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        self.git(&work, &["add", "-A"]);
        self.git(&work, &["commit", "-q", "-m", message]);
    }

    /// A branch off `main` committing `files`, pushed; the work clone is back on `main` after.
    pub fn branch(&self, name: &str, files: &[(&str, &str)]) {
        let work = self.work();
        self.git(&work, &["checkout", "-q", "-b", name, "main"]);
        self.commit_files(files, name);
        self.git(&work, &["push", "-q", "origin", name]);
        self.git(&work, &["checkout", "-q", "main"]);
    }

    /// Moves `origin`'s `main` on by a commit writing `file`, from the `other` clone.
    pub fn advance_main(&self, file: &str, text: &str) {
        let other = self.path("other");
        self.git(&other, &["pull", "-q", "--ff-only", "origin", "main"]);
        let path = other.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
        self.git(&other, &["add", "-A"]);
        self.git(&other, &["commit", "-q", "-m", "main moved on"]);
        self.git(&other, &["push", "-q", "origin", "HEAD:main"]);
    }

    /// The environment `config` and `clm-approval` read: `HOME` here, and `vars`.
    pub fn env(&self, vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let mut map = std::collections::BTreeMap::new();
        map.insert("HOME".to_string(), self.dir.path().display().to_string());
        for (k, v) in vars {
            map.insert(k.to_string(), v.to_string());
        }
        move |key: &str| map.get(key).cloned()
    }

    /// `env`, with git's global config isolated from the user's own.
    pub fn isolated_env(&self) -> impl Fn(&str) -> Option<String> + use<> {
        let global = self.path("gitconfig").display().to_string();
        self.env(&[("GIT_CONFIG_GLOBAL", &global)])
    }

    /// Runs `call` with the work clone's `Repo`, its settings loaded under `isolated_env`.
    pub fn with_repo<T>(&self, call: impl FnOnce(&clm_approval::Repo) -> T) -> T {
        let env = self.isolated_env();
        let root = fs::canonicalize(self.work()).unwrap();
        let settings = clm_config::load(Some(&root), &env).unwrap();
        call(&clm_approval::Repo::new(&root, &env, &settings))
    }

    /// The fingerprint of `branch`'s diff against `origin/main`.
    pub fn fingerprint(&self, branch: &str) -> String {
        let work = self.work();
        self.git(&work, &["checkout", "-q", branch]);
        let fp = clm_gate::fingerprint(&work, "origin/main").unwrap();
        self.git(&work, &["checkout", "-q", "main"]);
        fp
    }

    /// Every record the work clone sees, after fetching.
    pub fn records(&self) -> Vec<Record> {
        self.git(&self.work(), &["fetch", "-q", "origin"]);
        clm_ledger::fold(&self.work()).unwrap().records
    }
}

/// A fresh ed25519 key pair at `dir/<name>`: the private key's path and the public key.
pub fn keygen(dir: &Path, name: &str) -> (PathBuf, String) {
    fs::create_dir_all(dir).unwrap();
    let key = dir.join(name);
    let ok = Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-C", name, "-f"])
        .arg(&key)
        .status()
        .unwrap()
        .success();
    assert!(ok, "ssh-keygen -t ed25519");
    let public = fs::read_to_string(key.with_extension("pub")).unwrap();
    (key, public.trim().to_string())
}

/// A record's line: `id`, `writer`, `at` and `body`'s fields, signed under the namespace
/// with the private key given, when one is.
pub fn line(
    id: &str,
    writer: &str,
    at: &str,
    body: clm_ledger::Body,
    key: Option<(&Path, &str)>,
) -> String {
    let record = Record {
        id: id.into(),
        writer: writer.into(),
        at: at.into(),
        body,
    };
    let serde_json::Value::Object(mut object) = serde_json::to_value(&record).unwrap() else {
        unreachable!("a record is an object")
    };
    if let Some((key, namespace)) = key {
        use std::io::Write as _;
        let message = clm_ledger::signed_message(&object);
        let mut child = Command::new("ssh-keygen")
            .args(["-Y", "sign", "-n", namespace, "-f"])
            .arg(key)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(message.as_bytes()).unwrap();
        drop(stdin);
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success(), "ssh-keygen -Y sign");
        let sig = String::from_utf8(out.stdout).unwrap().trim().to_string();
        object.insert("sig".into(), serde_json::Value::String(sig));
    }
    serde_json::Value::Object(object).to_string()
}

/// The fold over `lines`, as one ledger's `records.jsonl`.
pub fn fold(lines: &[String]) -> clm_ledger::Fold {
    let text = lines.join("\n");
    clm_ledger::fold_records(&[("test", &text)])
}

/// The message of a refusal.
pub fn refused<T: std::fmt::Debug>(result: Result<T, clm_approval::Error>) -> String {
    match result {
        Err(clm_approval::Error::Refused(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}
