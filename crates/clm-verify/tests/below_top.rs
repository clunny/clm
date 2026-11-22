//! A CLM root that isn't the top of its git repository: the files `clm verify` names carry
//! the root's place before `clm.toml` (verify/settings-from-main, verify/verify-env).

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clm_verify::Request;
use tempfile::TempDir;

const ROOT_SPEC: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n";

const VERIFY: &str = "[verify]\ncheck = \"true\"\nmode = \"local\"\ntriggers = []\n";

fn git(cwd: &Path, args: &[&str]) -> String {
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

/// A clone `work` of a bare `origin`, whose CLM root is its directory `sub/` holding
/// `clm.toml` with `toml`.
struct Below {
    dir: TempDir,
}

impl Below {
    fn new(toml: &str) -> Self {
        let b = Below {
            dir: TempDir::new().unwrap(),
        };
        let work = b.work();
        fs::create_dir_all(&work).unwrap();
        git(&work, &["init", "-q", "-b", "main"]);
        git(&work, &["config", "user.name", "Christopher Lunny"]);
        git(&work, &["config", "user.email", "t@example.com"]);
        b.land(
            &[
                (".gitignore", "clm.local.toml\n.clm/\n"),
                ("sub/clm.toml", toml),
                ("sub/specs/SPEC.md", ROOT_SPEC),
            ],
            "init",
        );
        let bare = b.dir.path().join("origin.git");
        git(
            b.dir.path(),
            &["init", "-q", "--bare", "-b", "main", bare.to_str().unwrap()],
        );
        git(&work, &["remote", "add", "origin", bare.to_str().unwrap()]);
        git(&work, &["push", "-q", "origin", "main"]);
        b
    }

    fn work(&self) -> PathBuf {
        self.dir.path().join("work")
    }

    fn root(&self) -> PathBuf {
        self.work().join("sub")
    }

    /// Commits `files`, pushing `main` once `origin` exists. Returns the commit's full ID.
    fn land(&self, files: &[(&str, &str)], message: &str) -> String {
        let work = self.work();
        for (name, text) in files {
            let path = work.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        git(&work, &["add", "-A"]);
        git(&work, &["commit", "-q", "-m", message]);
        if !git(&work, &["remote"]).is_empty() {
            git(&work, &["push", "-q", "origin", "main"]);
        }
        git(&work, &["rev-parse", "HEAD"])
    }

    fn env(&self) -> impl Fn(&str) -> Option<String> + use<> {
        let home = self.dir.path().display().to_string();
        move |key: &str| (key == "HOME").then(|| home.clone())
    }
}

#[test]
fn a_clm_root_below_the_top() {
    // req:verify/settings-from-main@35114bba
    let b = Below::new("");
    let out = common::verify(
        &b.root(),
        &Request::Due {
            work_done: false,
            dry_run: false,
        },
        &b.env(),
    )
    .unwrap();
    assert_eq!(
        (out.line.as_str(), out.code),
        ("verify off: no [verify] in origin/main:sub/clm.toml", 0)
    );
}

#[test]
fn a_table_that_cant_be_used_is_named_below_the_top() {
    // req:verify/verify-env@877031f2
    let b = Below::new(VERIFY);
    let bad = b.land(
        &[("sub/clm.toml", &format!("{VERIFY}[env]\nCLM_X = \"1\"\n"))],
        "bad",
    );
    b.land(&[("sub/clm.toml", VERIFY)], "after");
    let err = common::verify(&b.root(), &Request::Commit(bad.clone()), &b.env()).unwrap_err();
    assert!(matches!(err, clm_verify::Error::Refused(_)), "{err}");
    let message = err.to_string();
    assert!(
        message.contains(&format!("{}:sub/clm.toml", &bad[..7])),
        "{message}"
    );
    assert!(message.contains("env"), "{message}");
}
