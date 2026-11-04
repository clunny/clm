//! `git-host`: CLM with GitHub, a bare git server or no host at all.

use std::fs;
use std::os::unix::fs::PermissionsExt;

use crate::{Repo, landed, printed};

#[test]
fn bare_server() {
    // req:capability/git-host@4c4fceda
    let repo = Repo::init();
    // `origin` is a bare git server.
    repo.land_on_origin("true");

    // A `gh` on `PATH` that notes any call, so a host call shows.
    let bin = repo.root.parent().unwrap().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let called = bin.join("gh-called");
    let gh = bin.join("gh");
    fs::write(
        &gh,
        format!("#!/bin/sh\necho \"$@\" >> '{}'\nexit 1\n", called.display()),
    )
    .unwrap();
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let run = |args: &[&str]| repo.clm_env(args, "", &[("PATH", &path)]);
    let clm = |args: &[&str]| {
        let out = run(args);
        assert!(out.status.success(), "clm {args:?}: {}", printed(&out));
        String::from_utf8_lossy(&out.stdout).into_owned()
    };

    // Propose: a change that needs review, checked on its own branch, with its page.
    repo.git(&["switch", "-q", "-c", "spec/20261030-anywhere"]);
    let capabilities = fs::read_to_string(repo.root.join("CAPABILITIES.md")).unwrap();
    repo.write(
        "CAPABILITIES.md",
        &capabilities.replace("A person can c.", "A person can c, anywhere."),
    );
    repo.write(
        "changes/20261030-anywhere/change.md",
        "---\nstatus: applied\n---\n\n## Intent\n\nc, anywhere.\n",
    );
    repo.commit("c, anywhere");
    clm(&["check"]);
    let gate = run(&["gate", "--base", "origin/main"]);
    let first = String::from_utf8_lossy(&gate.stdout).into_owned();
    assert_eq!(
        first.lines().next(),
        Some("spec review"),
        "{}",
        printed(&gate)
    );
    repo.git(&["push", "-q", "-u", "origin", "spec/20261030-anywhere"]);
    repo.git(&["switch", "-q", "main"]);
    let body = repo.root.parent().unwrap().join("body.md");
    fs::write(&body, "## Intent\n\nc, anywhere.\n").unwrap();
    let page = clm(&[
        "pr",
        "open",
        "spec/20261030-anywhere",
        "--title",
        "c, anywhere",
        "--body-file",
        body.to_str().unwrap(),
    ]);
    assert_eq!(page.trim(), "none");

    // Approve, then land.
    clm(&["approve", "spec/20261030-anywhere"]);
    let commit = landed(&run(&["land", "spec/20261030-anywhere"]));
    assert_eq!(repo.origin_main(), commit);
    let landed = repo.git(&["show", &format!("{commit}:CAPABILITIES.md")]);
    assert!(landed.contains("A person can c, anywhere."), "{landed}");

    // Nothing needed a host.
    assert!(
        !called.exists(),
        "gh was called: {}",
        fs::read_to_string(&called).unwrap()
    );
}
