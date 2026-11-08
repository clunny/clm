#![allow(dead_code)]
//! A clone `work` of a bare `origin` on `main`, whose `clm.toml` holds a `[verify]` table
//! naming a fake check that logs what it ran on, and lands that are plain commits pushed to
//! `main`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clm_ledger::{Body, Record, VerifyMode};
use clm_verify::{Outcome, Request};
use tempfile::TempDir;

#[path = "../../../clm-git/tests/exec/mod.rs"]
mod exec;
pub use exec::write_exec;

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n";

fn component(title: &str, paths: &str, requirements: &str) -> String {
    format!(
        "---\ntitle: {title}\nstatus: active\nowners: []\npaths: [\"{paths}\"]\n---\n\n\
         ## Contract\nc\n\n## Design\nd\n\n## Requirements\n{requirements}"
    )
}

/// A requirement no test cites, so it is pending.
pub const UNBUILT: &str = "\n### Requirement: thing\nIt SHALL do the thing.\n\n\
    #### Scenario: doing\n- GIVEN a\n- WHEN b\n- THEN c\n";

/// Logs the directory and head it ran on and what `CLM_SELECTION` held; sleeps for the
/// seconds in `sleep` when it exists; fails when `check-fails` exists or when the path
/// `fail-path` names exists in the tree it ran on.
const CHECK: &str = r#"
pwd -P >> "$DIR/check-pwd.log"
git rev-parse HEAD >> "$DIR/check-head.log"
if [ -n "${CLM_SELECTION-}" ]; then
  cp "$CLM_SELECTION" "$DIR/selection-copy"
  echo set >> "$DIR/check-selection.log"
else
  echo unset >> "$DIR/check-selection.log"
fi
echo "start" >> "$DIR/check-order.log"
if [ -e "$DIR/sleep" ]; then sleep "$(cat "$DIR/sleep")"; fi
echo "end" >> "$DIR/check-order.log"
if [ -e "$DIR/fail-path" ] && [ -e "$(cat "$DIR/fail-path")" ]; then exit 1; fi
[ ! -e "$DIR/check-fails" ]
"#;

pub struct Fixture {
    dir: TempDir,
}

impl Fixture {
    /// `verify` is the body of the `[verify]` table (its `check` line is added); `None`
    /// leaves the table out.
    pub fn new(verify: Option<&str>) -> Self {
        let f = Fixture {
            dir: TempDir::new().unwrap(),
        };
        let d = f.dir.path();
        let check = f.path("bin/check");
        fs::create_dir_all(check.parent().unwrap()).unwrap();
        write_exec(
            &check,
            &format!("#!/bin/sh\nDIR='{}'\n{CHECK}", d.display()),
        );
        fs::create_dir_all(f.path("tmp")).unwrap();
        fs::create_dir_all(f.work()).unwrap();
        f.git(&f.work(), &["init", "-q", "-b", "main"]);
        f.git(&f.work(), &["config", "user.name", "Christopher Lunny"]);
        f.git(&f.work(), &["config", "user.email", "t@example.com"]);
        let toml = match verify {
            Some(body) => format!("[verify]\ncheck = \"{}\"\n{body}\n", check.display()),
            None => String::new(),
        };
        let pending = UNBUILT.to_string();
        let a = component("A", "crates/a/**", &pending);
        let b = component("B", "crates/b/**", "");
        f.commit(
            &[
                ("clm.toml", &toml),
                (".gitignore", "clm.local.toml\n.clm/\n"),
                ("specs/SPEC.md", ROOT),
                ("specs/a/SPEC.md", &a),
                ("specs/b/SPEC.md", &b),
            ],
            "init",
        );
        let bare = f.path("origin.git");
        f.git(
            d,
            &["init", "-q", "--bare", "-b", "main", bare.to_str().unwrap()],
        );
        f.git(
            &f.work(),
            &["remote", "add", "origin", bare.to_str().unwrap()],
        );
        f.git(&f.work(), &["push", "-q", "origin", "main"]);
        f
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    pub fn work(&self) -> PathBuf {
        self.path("work")
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

    /// Commits `files` on the work clone's `main`, not pushed.
    pub fn commit(&self, files: &[(&str, &str)], message: &str) -> String {
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

    /// A land: a commit of `files` on `main`, pushed to `origin`. Returns its full ID.
    pub fn land(&self, files: &[(&str, &str)], message: &str) -> String {
        let sha = self.commit(files, message);
        self.git(&self.work(), &["push", "-q", "origin", "main"]);
        sha
    }

    /// The commit `n` lands before the head of `main`.
    pub fn back(&self, n: usize) -> String {
        self.git(&self.work(), &["rev-parse", &format!("main~{n}")])
    }

    /// The variables `config` and `clm-verify` read.
    pub fn env(&self) -> impl Fn(&str) -> Option<String> + use<> {
        let map: BTreeMap<String, String> = [
            ("HOME", self.dir.path().display().to_string()),
            ("TMPDIR", self.path("tmp").display().to_string()),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
        move |key: &str| map.get(key).cloned()
    }

    pub fn run(&self, request: &Request) -> Outcome {
        clm_verify::verify(&self.work(), request, &self.env()).unwrap()
    }

    pub fn due(&self) -> Outcome {
        self.run(&Request::Due {
            work_done: false,
            dry_run: false,
        })
    }

    pub fn dry(&self, work_done: bool) -> Outcome {
        self.run(&Request::Due {
            work_done,
            dry_run: true,
        })
    }

    /// A `verified` record for `commit` in the work clone's ledger.
    pub fn verified(&self, commit: &str, passed: bool) -> Record {
        let body = Body::Verified {
            commit: commit.to_string(),
            passed,
            since: None,
            lands: vec![commit.to_string()],
            culprit: None,
            components: if passed { vec![] } else { vec!["b".to_string()] },
            mode: VerifyMode::Local,
            run: None,
        };
        clm_ledger::append(&self.work(), body, &self.env())
            .unwrap()
            .record
    }

    /// The `verified` records the work clone sees, oldest first, as their bodies.
    pub fn records(&self) -> Vec<Body> {
        self.git(&self.work(), &["fetch", "-q", "origin"]);
        let fold = clm_ledger::fold(&self.work()).unwrap();
        let mut all: Vec<Record> = fold.verifications.into_values().flatten().collect();
        all.sort_by(|a, b| a.at.cmp(&b.at));
        all.into_iter().map(|r| r.body).collect()
    }

    pub fn verify_tree(&self) -> PathBuf {
        fs::canonicalize(self.work()).unwrap().join(".clm/verify-tree")
    }
}
