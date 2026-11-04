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

/// The message of a refusal.
pub fn refused(result: Result<clm_approval::Output, clm_approval::Error>) -> String {
    match result {
        Err(clm_approval::Error::Refused(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}
