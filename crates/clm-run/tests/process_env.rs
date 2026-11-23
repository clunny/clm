//! The calls of `clm-run` whose tests need the process's own environment
//! (`commands-in-crate`): a fake `claude` or `git` first on `PATH`, or a variable git reads,
//! such as `GIT_ALLOW_PROTOCOL` or `GIT_DIR`, reaches the commands a call runs only that way.
//! Every test of this binary holds `into_process`'s lock for its whole run, so no thread
//! reads the environment while one changes it.

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clm_run::SUPERPOWERS_COMMIT;
use support::common::*;
use support::*;
use tempfile::TempDir;

#[test]
fn scenario_node_prepared() {
    // req:orchestrator.run/run@5ea05da6
    let _env = into_process();
    let (r, f) = (repo(), fake());
    let (launcher, p) = launched_in(&r.work, &r.work, &f, "b", &[]);
    let bundle = clm_context::bundle(&load(&r.work), &r.work, "b").unwrap();
    assert!(!launcher.contains(bundle.trim()), "{launcher}");
    assert!(launcher.contains("before anything else"), "{launcher}");

    let tip = remote_tip(&r, "impl/b").expect("origin/impl/b exists");
    assert_eq!(
        git(&r.origin, &["log", "-1", "--format=%s", &tip]).trim(),
        "Claim b"
    );
    assert_eq!(local_tip(&r, "impl/b").as_deref(), Some(tip.as_str()));
    assert_eq!(
        git(&r.work, &["rev-parse", "--abbrev-ref", "impl/b@{upstream}"]).trim(),
        "origin/impl/b"
    );
    assert!(!checked_out(&r, "impl/b"), "no worktree has impl/b");
    assert!(!ran(&f), "claude must not start");
    assert_eq!(git(&r.work, &["branch", "--show-current"]).trim(), "main");
    assert_eq!(git(&r.work, &["status", "--porcelain"]), "");
    let exclude = fs::read_to_string(r.work.join(".git/info/exclude")).unwrap();
    assert!(exclude.lines().any(|l| l == ".clm/"), "{exclude}");

    let first = p.lines().next().unwrap();
    assert!(first.contains("git switch impl/b"), "{first}");
    for needle in [".clm/plans/b.md", "`b/r` unbuilt"] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
    for s in SKILLS {
        let path = f.superpowers.join(format!("skills/{s}/SKILL.md"));
        assert!(
            p.contains(&format!("`superpowers:{s}`, at `{}`", path.display())),
            "{s} with its path missing from {p}"
        );
    }
    assert!(
        !p.contains("already exists"),
        "a fresh claim isn't a resume: {p}"
    );
    assert!(p.ends_with(&bundle), "{p}");
}

#[test]
fn scenario_claimed_elsewhere() {
    // req:orchestrator.run/run@5ea05da6
    let _env = into_process();
    let (r, f) = (repo(), fake());
    push_claim(&r.work, "b", 1);
    refused(&r, &f, "b", &[], 1);
    assert!(!ran(&f), "claude must not start");
    assert!(local_tip(&r, "impl/b").is_none());
}

#[test]
fn a_skill_whose_fetch_fails_refuses_before_claiming() {
    // req:orchestrator.run/run@5ea05da6
    let _env = into_process();
    let (r, f) = (repo(), fake());
    let cache = TempDir::new().unwrap();
    // No override, so the pinned copy is fetched, and git refuses its https fetch.
    let env = [
        ("CLM_SUPERPOWERS_DIR", Path::new("")),
        ("XDG_CACHE_HOME", cache.path()),
        ("GIT_ALLOW_PROTOCOL", Path::new("file")),
    ];
    refused(&r, &f, "b", &env, 2);
    assert!(remote_tip(&r, "impl/b").is_none(), "no claim");
    assert!(place_names(&r).is_empty(), "no place");
}

#[cfg(unix)]
#[test]
fn scenario_worktree_git_cant_remove() {
    // req:orchestrator.run/run-finish@efe0dbf8
    let _env = into_process();
    let (r, f) = (repo(), fake());
    let wt = landed_build(&r, &f);
    // A `git` whose `worktree remove` refuses, leaving the worktree listed and on disk.
    // Pinned to the argv `git::remove_worktree` runs: `git worktree remove ...`.
    write_exec(
        &f.bin.join("git"),
        "#!/bin/sh\n\
         real=$(PATH=\"$FAKE_REAL_PATH\" command -v git)\n\
         if [ \"$1\" = worktree ] && [ \"$2\" = remove ]; then echo refused >&2; exit 1; fi\n\
         exec \"$real\" \"$@\"\n",
    );
    let real = std::env::var("PATH").unwrap();
    let path = format!("{}:{real}", f.bin.display());
    let line = ["--finish", "b", "--root", r.work.to_str().unwrap()];
    let ran = called(
        |env, out, err| clm_run::run_command(line, Path::new("."), &r.work, env, out, err),
        &env_of(&[
            ("PATH", path.as_str()),
            ("FAKE_REAL_PATH", &real),
            (
                "XDG_CONFIG_HOME",
                r.work.join(".no-config").to_str().unwrap(),
            ),
        ]),
    );
    assert_eq!(ran.status.code(), Some(0));
    let out = String::from_utf8_lossy(&ran.stdout).into_owned();
    assert!(place_names(&r).is_empty());
    assert!(wt.exists(), "worktree should be left on disk");
    assert!(
        git(&r.work, &["worktree", "list", "--porcelain"]).contains(&wt.display().to_string()),
        "worktree should still be listed"
    );
    assert!(
        out.contains("freed b") && out.contains("landed impl/b"),
        "{out}"
    );
    let left = out.lines().find(|l| l.starts_with("left ")).expect(&out);
    assert!(left.contains(&wt.display().to_string()), "{left}");
    assert!(left.contains("git worktree prune"), "{left}");
    assert!(!out.contains("removed"), "{out}");
}

/// A `git` first on PATH that fails the fetches of `origin` its `FAKE_FETCH_FAILS` counts
/// down (`always` for every one) as a lost ref race does, and runs the real `git` otherwise.
fn fake_git(f: &Fake) {
    write_exec(
        &f.bin.join("git"),
        "#!/bin/sh\n\
         real=$(PATH=\"$FAKE_REAL_PATH\" command -v git)\n\
         if [ \"$1\" = fetch ]; then for a in \"$@\"; do if [ \"$a\" = origin ]; then\n\
           echo fetch >> \"$FAKE_OUT/fetches\"\n\
           n=$(wc -l < \"$FAKE_OUT/fetches\")\n\
           if [ \"$FAKE_FETCH_FAILS\" = always ] || [ \"$n\" -le \"$FAKE_FETCH_FAILS\" ]; then\n\
             echo \"error: cannot lock ref 'refs/remotes/origin/impl/a'\" >&2; exit 1\n\
           fi\n\
         fi; done; fi\n\
         exec \"$real\" \"$@\"\n",
    );
}

#[test]
fn scenario_a_run_whose_fetch_loses_a_ref_race() {
    // req:orchestrator.run/run-fetch-retries-race@901c36c0
    let _env = into_process();
    let (r, f) = (repo(), fake());
    fake_git(&f);
    let real = PathBuf::from(std::env::var("PATH").unwrap_or_default());
    let env = [
        ("FAKE_REAL_PATH", &*real),
        ("FAKE_FETCH_FAILS", Path::new("1")),
    ];
    let (launcher, _) = launched_in(&r.work, &r.work, &f, "b", &env);
    assert!(launcher.contains("git switch impl/b"), "{launcher}");
    assert!(remote_tip(&r, "impl/b").is_some(), "claimed");
    assert!(local_tip(&r, "impl/b").is_some());
    assert!(r.work.join(".git/clm/builds/b").exists(), "place b");
}

#[test]
fn scenario_a_run_whose_fetch_keeps_failing() {
    // req:orchestrator.run/run-fetch-retries-race@901c36c0
    let _env = into_process();
    let (r, f) = (repo(), fake());
    fake_git(&f);
    let real = PathBuf::from(std::env::var("PATH").unwrap_or_default());
    let env = [
        ("FAKE_REAL_PATH", &*real),
        ("FAKE_FETCH_FAILS", Path::new("always")),
    ];
    let stderr = refused(&r, &f, "b", &env, 2);
    assert!(stderr.contains("cannot lock ref"), "{stderr}");
    assert!(remote_tip(&r, "impl/b").is_none(), "no claim");
    assert!(!r.work.join(".git/clm/builds/b").exists(), "no place b");
}

#[test]
fn scenario_a_finish_whose_fetch_loses_a_ref_race() {
    // req:orchestrator.run/run-fetch-retries-race@901c36c0
    let _env = into_process();
    let (r, f) = (repo(), fake());
    fake_git(&f);
    let real = PathBuf::from(std::env::var("PATH").unwrap_or_default());
    let env = [
        ("FAKE_REAL_PATH", &*real),
        ("FAKE_FETCH_FAILS", Path::new("1")),
    ];
    let ran = clm_run_with(&r.work, &r.work, &f, &["--finish", "b"], &env);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
}

#[test]
fn pinned_copy() {
    // req:orchestrator.run/superpowers-path@27168d3f
    let _env = into_process();
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

    // `XDG_CACHE_HOME` set to the empty string, and `HOME` holding the copy under `.cache/`.
    let home = TempDir::new().unwrap();
    let skill = with_skill(
        &home
            .path()
            .join(".cache/clm/superpowers")
            .join(SUPERPOWERS_COMMIT),
    );
    let o = clm_superpowers(
        "brainstorming",
        &[
            ("XDG_CACHE_HOME", "".as_ref()),
            ("HOME", home.path().as_os_str()),
            ("PATH", empty.path().as_os_str()),
        ],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(stdout(&o), format!("{}\n", skill.display()));

    // Neither set: 1 naming both, and no path.
    let o = clm_superpowers("brainstorming", &[("PATH", empty.path().as_os_str())]);
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
    let err = stderr(&o);
    assert!(
        err.contains("XDG_CACHE_HOME") && err.contains("HOME"),
        "{err}"
    );
}

#[test]
fn bad_name_is_refused_before_fetching() {
    // req:orchestrator.run/superpowers-path@27168d3f
    let _env = into_process();
    let cache = TempDir::new().unwrap();
    let o = clm_superpowers(
        "../brainstorming",
        &[
            ("XDG_CACHE_HOME", cache.path().as_os_str()),
            ("GIT_ALLOW_PROTOCOL", "file".as_ref()),
        ],
    );
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    assert_eq!(stdout(&o), "", "no path");
    assert!(!cache.path().join("clm").exists());
}

#[test]
fn fetch_fails() {
    // req:orchestrator.run/superpowers-path@27168d3f
    let _env = into_process();
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
    // req:orchestrator.run/superpowers-path@27168d3f
    let _env = into_process();
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
        clm_superpowers_in(
            &std::env::current_dir().unwrap(),
            args,
            &[
                ("CLM_SUPERPOWERS_DIR", dir.path().as_os_str()),
                ("PATH", bin.path().as_os_str()),
            ],
        )
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
    // req:orchestrator.run/superpowers-path@27168d3f
    let _env = into_process();
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
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
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

#[test]
fn scenario_setup_gets_the_projects_variables() {
    // req:orchestrator.run/setup-command@357d0a31
    let _env = into_process();
    let r = setup_repo(
        "[build]\nsetup = \"echo $RELAY_SERVER_IMAGE > seen\"\n[env]\nRELAY_SERVER_IMAGE = \"relay:amd64\"\n",
    );
    fs::write(
        r.path().join("clm.local.toml"),
        "[env]\nRELAY_SERVER_IMAGE = \"relay:arm64\"\n",
    )
    .unwrap();
    let out = clm_setup_with(
        r.path(),
        &[
            ("XDG_CONFIG_HOME", r.path().as_os_str()),
            ("RELAY_SERVER_IMAGE", "relay:shell".as_ref()),
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(
        fs::read_to_string(r.path().join("seen")).unwrap().trim(),
        "relay:arm64"
    );
}
