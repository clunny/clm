//! `clm superpowers <skill>`: the path of a skill in the pinned superpowers copy builds
//! follow.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use clm_run::SUPERPOWERS_COMMIT;
use tempfile::TempDir;

/// Runs `clm superpowers <skill>` with `env` on top of an environment without
/// `CLM_SUPERPOWERS_DIR`, so the host's override never leaks in.
fn clm_superpowers(skill: &str, env: &[(&str, &std::ffi::OsStr)]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(["superpowers", skill])
        .env_remove("CLM_SUPERPOWERS_DIR")
        .envs(env.iter().copied())
        .output()
        .unwrap()
}

/// A directory holding `skills/brainstorming/SKILL.md`; returns that file's path.
fn with_skill(dir: &Path) -> PathBuf {
    let skill = dir.join("skills/brainstorming/SKILL.md");
    fs::create_dir_all(skill.parent().unwrap()).unwrap();
    fs::write(&skill, "# brainstorming\n").unwrap();
    skill
}

/// An `XDG_CACHE_HOME` holding the pinned copy; returns it and the skill's path.
fn pinned_cache() -> (TempDir, PathBuf) {
    let cache = TempDir::new().unwrap();
    let skill = with_skill(
        &cache
            .path()
            .join("clm/superpowers")
            .join(SUPERPOWERS_COMMIT),
    );
    (cache, skill)
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn pinned_copy() {
    // req:orchestrator.run/superpowers-path@411708d0
    let (cache, skill) = pinned_cache();
    // No git on PATH: any attempt to fetch would fail.
    let empty = TempDir::new().unwrap();
    let o = clm_superpowers(
        "brainstorming",
        &[
            ("XDG_CACHE_HOME", cache.path().as_os_str()),
            ("PATH", empty.path().as_os_str()),
        ],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(stdout(&o), format!("{}\n", skill.display()));
}

#[test]
fn override_dir() {
    // req:orchestrator.run/superpowers-path@411708d0
    let dir = TempDir::new().unwrap();
    let skill = with_skill(dir.path());
    let o = clm_superpowers(
        "brainstorming",
        &[("CLM_SUPERPOWERS_DIR", dir.path().as_os_str())],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(stdout(&o), format!("{}\n", skill.display()));
}

#[test]
fn relative_override_prints_an_absolute_path() {
    // req:orchestrator.run/superpowers-path@411708d0
    let dir = TempDir::new().unwrap();
    let skill = with_skill(&dir.path().canonicalize().unwrap().join("sp"));
    let o = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(["superpowers", "brainstorming"])
        .current_dir(dir.path())
        .env("CLM_SUPERPOWERS_DIR", "sp")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(stdout(&o), format!("{}\n", skill.display()));
}

#[test]
fn empty_override_counts_as_unset() {
    // req:orchestrator.run/superpowers-path@411708d0
    let (cache, skill) = pinned_cache();
    let o = clm_superpowers(
        "brainstorming",
        &[
            ("XDG_CACHE_HOME", cache.path().as_os_str()),
            ("CLM_SUPERPOWERS_DIR", "".as_ref()),
        ],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(stdout(&o), format!("{}\n", skill.display()));
}

#[test]
fn unknown_skill() {
    // req:orchestrator.run/superpowers-path@411708d0
    let dir = TempDir::new().unwrap();
    with_skill(dir.path());
    // Only the name check keeps `skills/brainstorming` and `../brainstorming` from
    // reaching these files.
    with_skill(&dir.path().join("skills"));
    fs::create_dir_all(dir.path().join("brainstorming")).unwrap();
    fs::write(dir.path().join("brainstorming/SKILL.md"), "").unwrap();
    fs::write(dir.path().join("skills/SKILL.md"), "").unwrap();
    for skill in [
        "no-such-skill",
        "../brainstorming",
        "",
        "skills/brainstorming",
        ".",
    ] {
        let o = clm_superpowers(skill, &[("CLM_SUPERPOWERS_DIR", dir.path().as_os_str())]);
        assert_eq!(o.status.code(), Some(1), "{skill}: {}", stderr(&o));
        assert!(stderr(&o).contains(&format!("`{skill}`")), "{}", stderr(&o));
        assert_eq!(stdout(&o), "", "{skill}");
    }
}

#[test]
fn skill_dir_without_skill_md() {
    // req:orchestrator.run/superpowers-path@411708d0
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("skills/brainstorming")).unwrap();
    let o = clm_superpowers(
        "brainstorming",
        &[("CLM_SUPERPOWERS_DIR", dir.path().as_os_str())],
    );
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    assert!(stderr(&o).contains("`brainstorming`"), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
}

#[test]
fn bad_name_is_refused_before_fetching() {
    // req:orchestrator.run/superpowers-path@411708d0
    let cache = TempDir::new().unwrap();
    let o = clm_superpowers(
        "../brainstorming",
        &[
            ("XDG_CACHE_HOME", cache.path().as_os_str()),
            ("GIT_ALLOW_PROTOCOL", "file".as_ref()),
        ],
    );
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    assert!(!cache.path().join("clm").exists());
}

#[test]
fn symlinked_cache_prints_the_path_run_loads() {
    // req:orchestrator.run/superpowers-path@411708d0
    let (cache, _) = pinned_cache();
    let links = TempDir::new().unwrap();
    let link = links.path().join("cache");
    std::os::unix::fs::symlink(cache.path(), &link).unwrap();
    let o = clm_superpowers("brainstorming", &[("XDG_CACHE_HOME", link.as_os_str())]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(
        stdout(&o),
        format!(
            "{}\n",
            link.join("clm/superpowers")
                .join(SUPERPOWERS_COMMIT)
                .join("skills/brainstorming/SKILL.md")
                .display()
        )
    );
}

#[test]
fn fetch_fails() {
    // req:orchestrator.run/superpowers-path@411708d0
    let cache = TempDir::new().unwrap();
    // Git refuses the https fetch, so this needs no network.
    let o = clm_superpowers(
        "brainstorming",
        &[
            ("XDG_CACHE_HOME", cache.path().as_os_str()),
            ("GIT_ALLOW_PROTOCOL", "file".as_ref()),
        ],
    );
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(!stderr(&o).trim().is_empty());
    assert_eq!(stdout(&o), "");
    assert!(
        !cache
            .path()
            .join("clm/superpowers")
            .join(SUPERPOWERS_COMMIT)
            .exists()
    );
}

#[test]
fn no_reserved_words() {
    // req:orchestrator.run/superpowers-path@411708d0
    let dir = TempDir::new().unwrap();
    with_skill(dir.path());
    // `claude` and `git` that leave a mark when they run, and nothing else on `PATH`.
    let bin = TempDir::new().unwrap();
    let ran = bin.path().join("ran");
    for tool in ["claude", "git"] {
        let fake = bin.path().join(tool);
        fs::write(
            &fake,
            format!("#!/bin/sh\necho {tool} >> '{}'\n", ran.display()),
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_clm"))
            .arg("superpowers")
            .args(args)
            .env("CLM_SUPERPOWERS_DIR", dir.path())
            .env("PATH", bin.path())
            .output()
            .unwrap()
    };
    let o = run(&["upgrade"]);
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    assert!(stderr(&o).contains("`upgrade`"), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
    let o = run(&["eval", "out"]);
    assert!(!o.status.success(), "{}", stdout(&o));
    assert!(stderr(&o).contains("unexpected argument"), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
    assert!(!ran.exists(), "{}", fs::read_to_string(&ran).unwrap());
}

/// Every file under `dir` with its bytes, keyed by relative path.
fn snapshot(dir: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(base).unwrap().to_path_buf(),
                    fs::read(&path).unwrap(),
                );
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

#[test]
fn a_clone_the_environment_names_is_left_alone() {
    // req:orchestrator.run/superpowers-path@411708d0
    let cache = TempDir::new().unwrap();
    let root = TempDir::new().unwrap();
    let (w, g) = (root.path().join("w"), root.path().join("g"));
    fs::create_dir(&w).unwrap();
    let git = |args: &[&str]| {
        let o = Command::new("git")
            .args(args)
            .current_dir(&w)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", stderr(&o));
    };
    git(&["init", "-q", "--separate-git-dir", g.to_str().unwrap()]);
    fs::write(w.join("f"), "x").unwrap();
    git(&["add", "f"]);
    // `maintenance.auto=false`: newer git detaches auto-maintenance after a
    // commit, and its `objects/maintenance.lock` comes and goes under the
    // snapshots.
    git(&[
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "-c",
        "maintenance.auto=false",
        "commit",
        "-q",
        "-m",
        "c",
    ]);
    let (before_g, before_w) = (snapshot(&g), snapshot(&w));
    let o = clm_superpowers(
        "brainstorming",
        &[
            ("XDG_CACHE_HOME", cache.path().as_os_str()),
            ("GIT_ALLOW_PROTOCOL", "file".as_ref()),
            ("GIT_DIR", g.as_os_str()),
            ("GIT_WORK_TREE", w.as_os_str()),
        ],
    );
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert_eq!(snapshot(&g), before_g);
    assert_eq!(snapshot(&w), before_w);
}
