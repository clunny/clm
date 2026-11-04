//! `linguist_excluded` decides as `git check-attr` does (`linguist-attributes`).

use std::fs;
use std::process::Command;

use tempfile::TempDir;

/// What `git check-attr` says of each path in a repository holding `attributes`
/// (`(directory, text)`) and no other attribute source: excluded or not.
fn git_says(attributes: &[(&str, &str)], paths: &[&str]) -> Vec<bool> {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    for (d, text) in attributes {
        let file = root.join(d).join(".gitattributes");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    }
    let git = |args: &[&str]| {
        Command::new("git")
            .args([
                "-c",
                "core.attributesFile=/dev/null",
                "-c",
                "core.quotePath=false",
            ])
            .args(args)
            .current_dir(root)
            .env("HOME", root)
            .env("XDG_CONFIG_HOME", root)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap()
    };
    assert!(git(&["init", "-q"]).status.success());
    let mut args = vec![
        "check-attr",
        "linguist-generated",
        "linguist-vendored",
        "--",
    ];
    args.extend(paths);
    let out = git(&args);
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    paths
        .iter()
        .map(|p| {
            text.lines().any(|l| {
                l.strip_prefix(&format!("{p}: "))
                    .is_some_and(|rest| rest.ends_with(": set") || rest.ends_with(": true"))
            })
        })
        .collect()
}

fn ours(attributes: &[(&str, &str)], paths: &[&str]) -> Vec<bool> {
    paths
        .iter()
        .map(|p| clm_check::linguist_excluded(attributes, p))
        .collect()
}

fn agree(attributes: &[(&str, &str)], paths: &[&str]) -> Vec<bool> {
    let git = git_says(attributes, paths);
    assert_eq!(ours(attributes, paths), git, "paths {paths:?}");
    git
}

#[test]
fn agrees_with_git() {
    // req:check/linguist-attributes@63cc344c
    let attributes = [
        (
            "",
            "[attr]gen linguist-generated\nCargo.lock gen\nvendor/** linguist-vendored\n*.min.js linguist-generated\n",
        ),
        (
            "web",
            "package-lock.json linguist-generated=true\nkeep.min.js -linguist-generated\n",
        ),
    ];
    let paths = [
        "Cargo.lock",
        "crates/x/Cargo.lock",
        "vendor/a/b.rs",
        "web/package-lock.json",
        "web/keep.min.js",
        "web/app.min.js",
        "src/main.rs",
    ];
    assert_eq!(
        agree(&attributes, &paths),
        [true, true, true, true, false, true, false]
    );
}

#[test]
fn unsetting_and_values() {
    // req:check/linguist-attributes@63cc344c
    let attributes = [
        (
            "",
            "# a comment\n\
             *.lock linguist-generated\n\
             a.lock !linguist-generated\n\
             b.lock linguist-generated=false\n\
             c.lock linguist-generated=yes\n\
             d.lock linguist-vendored -linguist-vendored\n\
             e.lock -linguist-generated linguist-generated\n",
        ),
        ("sub", "*.lock -linguist-generated\n"),
    ];
    let paths = [
        "x.lock",
        "a.lock",
        "b.lock",
        "c.lock",
        "d.lock",
        "e.lock",
        "sub/x.lock",
    ];
    agree(&attributes, &paths);
}

#[test]
fn anchors_directories_and_stars() {
    // req:check/linguist-attributes@63cc344c
    let attributes = [
        (
            "",
            "/top.rs linguist-generated\n\
             gen/ linguist-generated\n\
             out linguist-vendored\n\
             **/build/*.js linguist-generated\n\
             docs/*.rs linguist-vendored\n\
             a/**/z.rs linguist-generated\n\
             [xy].c linguist-generated\n",
        ),
        (
            "lib",
            "/only.rs linguist-generated\nnested/*.rs linguist-vendored\n",
        ),
    ];
    let paths = [
        "top.rs",
        "x/top.rs",
        "gen",
        "gen/a.rs",
        "out",
        "out/a.rs",
        "p/out",
        "build/a.js",
        "p/q/build/a.js",
        "build/s/a.js",
        "docs/a.rs",
        "docs/s/a.rs",
        "x/docs/a.rs",
        "a/z.rs",
        "a/b/c/z.rs",
        "x.c",
        "s/y.c",
        "z.c",
        "lib/only.rs",
        "lib/s/only.rs",
        "only.rs",
        "lib/nested/a.rs",
        "nested/a.rs",
    ];
    agree(&attributes, &paths);
}

#[test]
fn quoted_patterns_and_posix_classes() {
    // req:check/linguist-attributes@63cc344c
    let attributes = [(
        "./",
        "\"caf\\303\\251.min.js\" linguist-generated\n\
         \"a b.rs\" linguist-vendored\n\
         v[[:digit:]].lock linguist-generated\n\
         w[![:alpha:]].lock linguist-generated\n",
    )];
    let paths = [
        "café.min.js",
        "s/a b.rs",
        "v1.lock",
        "vx.lock",
        "w1.lock",
        "wx.lock",
    ];
    assert_eq!(
        agree(&attributes, &paths),
        [true, true, true, false, true, false]
    );
}

#[test]
fn macros() {
    // req:check/linguist-attributes@63cc344c
    let attributes = [
        (
            "",
            "[attr]gen linguist-generated linguist-vendored\n\
             [attr]both gen\n\
             a.rs gen\n\
             b.rs -gen\n\
             c.rs gen -linguist-generated -linguist-vendored\n\
             d.rs -linguist-generated gen\n\
             e.rs both\n\
             f.rs binary\n\
             g.rs gen=false\n",
        ),
        ("s", "[attr]sub linguist-generated\nh.rs sub\n"),
    ];
    let paths = [
        "a.rs", "b.rs", "c.rs", "d.rs", "e.rs", "f.rs", "g.rs", "s/h.rs",
    ];
    agree(&attributes, &paths);
}

#[test]
fn no_attributes() {
    // req:check/linguist-attributes@63cc344c
    let none: [(&str, &str); 0] = [];
    assert!(!clm_check::linguist_excluded(&none, "Cargo.lock"));
}
