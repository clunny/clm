mod common;

use std::path::Path;
use std::process::Command;

use clm_coverage::{matches, owner, pending};
use common::{ROOT, graph, marker, spec, write};
use tempfile::TempDir;

#[test]
fn scenario_owned_and_unowned() {
    // req:check.coverage/owner@732fc105
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &spec(r#"["crates/a/**"]"#, &[]));
    write(d, "specs/a/b/SPEC.md", &spec(r#"["crates/b/**"]"#, &[]));
    let g = graph(d);
    assert_eq!(owner(&g, Path::new("crates/a/src/lib.rs")), Some("a"));
    assert_eq!(owner(&g, Path::new("crates/b/x.rs")), Some("a.b"));
    assert_eq!(owner(&g, Path::new("tools/x.sh")), None);
}

#[test]
fn the_deepest_node_then_the_first_by_id() {
    // req:check.coverage/owner@732fc105
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &spec(r#"["crates/**"]"#, &[]));
    write(d, "specs/a/b/SPEC.md", &spec(r#"["crates/x/**"]"#, &[]));
    write(d, "specs/a/c/SPEC.md", &spec(r#"["crates/x/**"]"#, &[]));
    let g = graph(d);
    assert_eq!(owner(&g, Path::new("crates/x/t.rs")), Some("a.b"));
    assert_eq!(owner(&g, Path::new("crates/y/t.rs")), Some("a"));
}

#[test]
fn pending_puts_an_unowned_file_on_the_root() {
    // req:check.coverage/owner@732fc105
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &spec(r#"["crates/a/**"]"#, &[]));
    // `g/x` is gone with its node: its markers move to the node owning their file.
    write(d, "changes/r/change.md", "---\nstatus: applied\n---\n");
    write(
        d,
        "changes/r/deltas/g.md",
        "---\ncomponent: g\nremove: true\n---\n## REMOVED Requirements\n\n### Requirement: x <!-- base: 0123456789abcdef -->\n",
    );
    let g = graph(d);
    let found: Vec<(String, String)> = pending(
        &g,
        &[
            marker("g/x", None, "crates/a/t.rs", 1),
            marker("g/x", None, "old/t.rs", 2),
        ],
    )
    .into_iter()
    .map(|p| (p.node.clone(), p.to_string()))
    .collect();
    assert_eq!(
        found,
        [
            ("a".to_string(), "g/x obsolete crates/a/t.rs:1".to_string()),
            ("clm".to_string(), "g/x obsolete old/t.rs:2".to_string()),
        ]
    );
}

#[test]
fn two_components_tests_match_the_first_by_id() {
    // req:check.coverage/owner@732fc105
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    for id in ["b", "a"] {
        write(
            d,
            &format!("specs/{id}/SPEC.md"),
            &spec("[]", &[]).replace("paths: []", "tests: [\"t/**\"]"),
        );
    }
    let g = graph(d);
    assert_eq!(owner(&g, Path::new("t/x.rs")), Some("a"));
}

#[test]
fn scenario_tests_inside_another_components_paths() {
    // req:check.coverage/owner@732fc105
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/harness/SPEC.md", &spec(r#"["tests/**"]"#, &[]));
    write(
        d,
        "specs/a/SPEC.md",
        &spec(r#"["crates/a/**"]"#, &[]).replace(
            "paths: [\"crates/a/**\"]",
            "paths: [\"crates/a/**\"]\ntests: [\"tests/a/**\"]",
        ),
    );
    let g = graph(d);
    assert_eq!(owner(&g, Path::new("tests/a/conftest.py")), Some("a"));
    assert_eq!(owner(&g, Path::new("tests/helpers.py")), Some("harness"));
}

#[test]
fn scenario_one_glob() {
    // req:check.coverage/owner@732fc105
    assert!(matches("crates/*/tests/**", "crates/a/tests/t.rs"));
    assert!(!matches("crates/*/tests/**", "crates/a/src/t.rs"));
}

/// Runs `git` in `dir`, failing the test when it fails, and returns its standard output.
/// Hermetic: no outer repository's variables and no user or system config, so no
/// excludes file hides a fixture.
fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn scenario_globs_as_git_reads_them() {
    // req:check.coverage/owner@732fc105
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    let files = [
        "a.rs",
        "a/b.rs",
        "ab.rs",
        "abc.rs",
        "README.md",
        "crates/a/src/lib.rs",
        "crates/a/tests/t.rs",
        "crates/a/tests/deep/u.rs",
        "crates/ab/tests/t.rs",
        "crates/b/Cargo.toml",
        "crates/lib.rs",
        "docs/x/y/z.md",
        "docs/z.md",
        "tests/t1.py",
        "tests/t12.py",
    ];
    for f in files {
        write(d, f, "x\n");
    }
    git(d, &["init", "-q"]);
    git(d, &["add", "."]);
    let ls = |pathspec: &[&str]| -> Vec<String> {
        let out = git(d, &[&["ls-files", "-z", "--"], pathspec].concat());
        out.split_terminator('\0').map(String::from).collect()
    };
    let tracked = ls(&[]);
    assert_eq!(tracked.len(), files.len());
    for glob in [
        "*.rs",
        "*",
        "crates/*",
        "crates/*/tests/**",
        "crates/*/*.toml",
        "crates/**",
        "crates/**/*.rs",
        "**/t.rs",
        "**/*.md",
        "docs/**/z.md",
        "a?.rs",
        "a?b.rs",
        "?.rs",
        "tests/t?.py",
        "crates/a?/**",
        "crates/a",
        "**",
        // `**` that isn't a whole segment is one `*`.
        "**.rs",
        "crates/**.rs",
        "a**.rs",
        // ...except right after the literal prefix, which git strips before matching.
        "a**",
        "a***",
        "crates/a**",
        "a/b**s",
        // A trailing slash, brackets and escapes.
        "crates/a/",
        "crates/*/",
        "[ab].rs",
        "[!a]*.rs",
        "crates/[a-b]/**",
        "tests/t[[:digit:]].py",
        "a\\?.rs",
    ] {
        let listed = ls(&[&format!(":(glob){glob}")]);
        let matched: Vec<String> = tracked
            .iter()
            .filter(|f| matches(glob, f))
            .cloned()
            .collect();
        assert_eq!(matched, listed, "glob {glob}");
    }
}
