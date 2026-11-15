#![cfg(unix)]

mod common;

use clm_git::remove_worktree;
use common::{repo, sh};
use std::os::unix::fs::PermissionsExt;

#[test]
fn git_leaves_the_directory() {
    // req:vcs/remove-worktree@ca5e9717
    // Alone in its file: it puts a fake `git` first on this process's `PATH`.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("r");
    std::fs::create_dir(&root).unwrap();
    repo(&root, &[("a.txt", "a\n"), (".gitignore", "out\n")]);
    let w = dir.path().join("w");
    sh(
        &root,
        &["worktree", "add", "-q", "-b", "wb", w.to_str().unwrap()],
    );
    let d = dir.path().join("d");
    std::fs::create_dir(&d).unwrap();
    std::fs::write(d.join("f"), "x").unwrap();
    let bin = dir.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    let real_git = String::from_utf8(
        std::process::Command::new("which")
            .arg("git")
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let real_git = real_git.trim();
    let script = format!(
        "#!/bin/sh\n\
         if [ \"$1\" = worktree ] && [ \"$2\" = remove ]; then\n\
         {git} \"$@\"\n\
         mkdir -p '{w}'\n\
         echo x > '{w}/f'\n\
         ln -s '{d}' '{w}/out'\n\
         exit 1\n\
         fi\n\
         exec {git} \"$@\"\n",
        git = real_git,
        w = w.display(),
        d = d.display()
    );
    let fake = bin.join("git");
    std::fs::write(&fake, script).unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = std::env::var_os("PATH").unwrap();
    let mut dirs = vec![bin];
    dirs.extend(std::env::split_paths(&path));
    // SAFETY: the only test in this process.
    unsafe { std::env::set_var("PATH", std::env::join_paths(dirs).unwrap()) };
    remove_worktree(&root, &w).unwrap();
    assert!(!w.exists());
    assert!(!sh(&root, &["worktree", "list"]).contains("/w "));
    assert!(d.join("f").exists());
}
