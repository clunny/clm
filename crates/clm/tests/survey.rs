//! `clm survey`: a map of a repository's code, and `--components` checking a component map against it.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

/// A git repository holding `files`, all added to the index.
fn repo(files: &[(&str, &[u8])]) -> TempDir {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"]);
    for (rel, content) in files {
        let path = dir.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    git(dir.path(), &["add", "-A"]);
    dir
}

fn clm(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap()
}

fn survey(dir: &Path) -> String {
    let out = clm(dir, &["survey"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    String::from_utf8(out.stdout).unwrap()
}

/// The lines of `out`'s section headed `# <name>`.
fn section<'a>(out: &'a str, name: &str) -> Vec<&'a str> {
    out.lines()
        .skip_while(|l| *l != format!("# {name}"))
        .skip(1)
        .take_while(|l| !l.starts_with("# "))
        .filter(|l| !l.is_empty())
        .collect()
}

const TWO_CRATES: &[(&str, &[u8])] = &[
    (
        "Cargo.toml",
        b"[workspace]\nresolver = \"2\"\nmembers = [\"crates/*\"]\n",
    ),
    (
        "crates/a/Cargo.toml",
        b"[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\nb = { path = \"../b\" }\n",
    ),
    (
        "crates/a/src/lib.rs",
        b"mod parse;\nuse crate::util::helper;\n\npub fn a() -> u32 {\n    helper() + b::b()\n}\n",
    ),
    ("crates/a/src/parse.rs", b"pub fn parse() {}\n"),
    ("crates/a/src/util/mod.rs", b"pub fn helper() -> u32 {\n    1\n}\n"),
    ("crates/a/tests/a.rs", b"#[test]\nfn t() {}\n"),
    (
        "crates/b/Cargo.toml",
        b"[package]\nname = \"b\"\nversion = \"0.1.0\"\n",
    ),
    ("crates/b/src/lib.rs", b"pub fn b() -> u32 {\n    2\n}\n"),
    ("README.md", b"# Two crates\n"),
    ("docs/decisions/use-rust.md", b"# Use Rust\n"),
    ("docs/decisions/two-crates.md", b"# Two crates\n"),
];

#[test]
fn two_crates() {
    // req:init/survey-map@3138d8a2
    let repo = repo(TWO_CRATES);
    let out = survey(repo.path());
    let dirs = section(&out, "directories");
    assert!(dirs.contains(&"crates/a  toml 1 file, 6 lines"), "{out}");
    assert!(
        dirs.contains(&"crates/a/src  rust 2 files, 7 lines"),
        "{out}"
    );
    assert!(
        dirs.contains(&"crates/a/src/util  rust 1 file, 3 lines"),
        "{out}"
    );
    assert!(
        dirs.contains(&"crates/b/src  rust 1 file, 3 lines"),
        "{out}"
    );
    // Depth-first: a directory's subdirectories come before its next sibling.
    let pos = |d: &str| {
        dirs.iter()
            .position(|l| l.starts_with(&format!("{d}  ")))
            .unwrap()
    };
    assert!(pos(".") < pos("crates/a"));
    assert!(pos("crates/a/src/util") < pos("crates/a/tests"));
    assert!(pos("crates/a/tests") < pos("crates/b"));
    assert_eq!(
        section(&out, "manifests"),
        [
            "Cargo.toml  workspace: crates/*",
            "crates/a/Cargo.toml",
            "crates/b/Cargo.toml"
        ]
    );
    assert_eq!(section(&out, "tests"), ["crates/a/tests"]);
    assert_eq!(
        section(&out, "docs"),
        ["README.md", "docs/decisions  2 ADRs"]
    );
    assert_eq!(
        section(&out, "edges"),
        [
            "crates/a -> crates/b  1",
            "crates/a/src -> crates/a/src/util  1"
        ]
    );
}

#[test]
fn typescript_import() {
    // req:init/survey-map@3138d8a2
    let repo = repo(&[
        ("web/package.json", b"{ \"name\": \"web\" }\n"),
        (
            "web/src/app.ts",
            b"import { util } from \"./lib/util\";\nexport * from \"./lib/more.js\";\nimport { shared } from \"@x/shared\";\nimport React from \"react\";\n",
        ),
        ("web/src/lib/util.ts", b"export const util = 1;\n"),
        ("web/src/lib/more.ts", b"export const more = 2;\n"),
        ("web/src/app.test.ts", b"import { util } from \"./lib\";\n"),
        ("web/src/lib/index.ts", b"export * from \"./util\";\n"),
        ("pkg/shared/package.json", b"{ \"name\": \"@x/shared\" }\n"),
        ("pkg/shared/index.ts", b"export const shared = 3;\n"),
    ]);
    let out = survey(repo.path());
    assert_eq!(
        section(&out, "edges"),
        ["web/src -> pkg/shared  1", "web/src -> web/src/lib  3"]
    );
    assert_eq!(section(&out, "tests"), ["web/src"]);
    assert!(
        section(&out, "manifests").contains(&"web/package.json"),
        "{out}"
    );
}

#[test]
fn python_import() {
    // req:init/survey-map@3138d8a2
    let repo = repo(&[
        (
            "app/main.py",
            b"import os\nimport app.db\nfrom app.models import user\nfrom .util import fmt\n",
        ),
        ("app/__init__.py", b""),
        ("app/db.py", b"x = 1\n"),
        ("app/util.py", b"def fmt(): pass\n"),
        ("app/models/__init__.py", b""),
        (
            "app/models/user.py",
            b"from ..util import fmt\nfrom . import base\n",
        ),
        ("app/models/base.py", b""),
        ("tests/test_user.py", b"from app.models.user import *\n"),
        ("pyproject.toml", b"[project]\nname = \"app\"\n"),
    ]);
    let out = survey(repo.path());
    assert_eq!(
        section(&out, "edges"),
        [
            "app -> app/models  1",
            "app/models -> app  1",
            "tests -> app/models  1"
        ]
    );
    assert_eq!(section(&out, "tests"), ["tests"]);
    assert_eq!(section(&out, "manifests"), ["pyproject.toml"]);
}

#[test]
fn skipped_and_counted() {
    // req:init/survey-map@3138d8a2
    let repo = repo(&[
        ("specs/SPEC.md", b"---\ntitle: T\n---\n"),
        ("changes/x/change.md", b"x\n"),
        ("backlog/y.md", b"y\n"),
        (".github/workflows/ci.yml", b"name: CI\n"),
        ("assets/logo.dat", b"PNG\0\x01\x02"),
        ("notes/todo.txt", b"one\ntwo"),
        ("notes/empty.txt", b""),
    ]);
    let out = survey(repo.path());
    assert_eq!(
        section(&out, "directories"),
        ["notes  text 2 files, 2 lines"]
    );
    assert!(!out.contains("specs") && !out.contains("backlog"), "{out}");
}

#[test]
fn rust_crate_roots_and_targets() {
    // req:init/survey-map@3138d8a2
    // `mod x;` in a test, binary or example crate root resolves beside it, and a
    // target-specific path dependency is an edge too.
    let repo = repo(&[
        (
            "x/Cargo.toml",
            b"[package]\nname = \"x\"\n\n[target.'cfg(unix)'.dev-dependencies]\ny = { path = \"../y\" }\n",
        ),
        ("x/src/lib.rs", b""),
        ("x/src/bin/tool.rs", b"mod cli;\nfn main() {}\n"),
        ("x/src/bin/cli/mod.rs", b""),
        ("x/tests/api.rs", b"mod common;\n"),
        ("x/tests/common/mod.rs", b""),
        ("x/examples/demo.rs", b"mod util;\nfn main() {}\n"),
        ("x/examples/util/mod.rs", b""),
        ("y/Cargo.toml", b"[package]\nname = \"y\"\n"),
    ]);
    assert_eq!(
        section(&survey(repo.path()), "edges"),
        [
            "x -> y  1",
            "x/examples -> x/examples/util  1",
            "x/src/bin -> x/src/bin/cli  1",
            "x/tests -> x/tests/common  1"
        ]
    );
}

#[test]
fn python_project_layouts() {
    // req:init/survey-map@3138d8a2
    // Absolute imports resolve from a Python project's directory and its `src/`, not only
    // from the repository's top.
    let repo = repo(&[
        ("backend/pyproject.toml", b"[project]\nname = \"app\"\n"),
        ("backend/app/__init__.py", b""),
        ("backend/app/models.py", b"class User: pass\n"),
        (
            "backend/app/api/routes.py",
            b"from app.models import User\n",
        ),
        ("lib/setup.py", b""),
        ("lib/src/pkg/a.py", b"import pkg.sub.c\n"),
        ("lib/src/pkg/sub/c.py", b""),
    ]);
    assert_eq!(
        section(&survey(repo.path()), "edges"),
        [
            "backend/app/api -> backend/app  1",
            "lib/src/pkg -> lib/src/pkg/sub  1"
        ]
    );
}

#[test]
fn from_a_subdirectory() {
    // req:init/survey-map@3138d8a2
    // req:clm/root-from-cwd@839330d9
    // Before `clm init` there's no CLM root; the survey still maps the whole repository.
    let repo = repo(TWO_CRATES);
    let out = clm(&repo.path().join("crates/a"), &["survey"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8(out.stdout).unwrap(), survey(repo.path()));
}

/// Covers the files outside the crates.
const TOP: &str = "  - { id: top, paths: [\"Cargo.toml\", \"README.md\", \"docs/**\"] }\n";

#[test]
fn components_key() {
    // req:init/survey-components@368a5dcd
    let map = format!(
        "components:\n  - {{ id: a, paths: [\"crates/a/**\"], depends_on: [b] }}\n  - {{ id: b, paths: [\"crates/b/**\"] }}\n{TOP}"
    );
    let repo = repo(TWO_CRATES);
    fs::write(repo.path().join("map.yaml"), map).unwrap();
    let out = clm(repo.path(), &["survey", "--components", "map.yaml"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{out:?}");
}

/// Runs `clm survey --components map.yaml` on `files` with `map` written beside them.
fn components(files: &[(&str, &[u8])], map: &str) -> Output {
    let repo = repo(files);
    fs::write(repo.path().join("map.yaml"), map).unwrap();
    clm(repo.path(), &["survey", "--components", "map.yaml"])
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

const A_AND_B: &str = "  - { id: a, paths: [\"crates/a/**\"], depends_on: [DEPS] }\n  - { id: b, paths: [\"crates/b/**\"] }\n";

fn a_and_b(deps: &str) -> String {
    A_AND_B.replace("DEPS", deps)
}

#[test]
fn uncovered_file() {
    // req:init/survey-components@368a5dcd
    let map = "components:\n  - { id: a, paths: [\"crates/a/**\"] }\n";
    let out = components(TWO_CRATES, map);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        stdout(&out).contains("survey/uncovered")
            && stdout(&out).contains("crates/b/")
            && !stdout(&out).contains("crates/a/"),
        "{out:?}"
    );
}

#[test]
fn undeclared_edge() {
    // req:init/survey-components@368a5dcd
    let map = format!("components:\n{}{TOP}", a_and_b(""));
    let out = components(TWO_CRATES, &map);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let text = stdout(&out);
    assert!(
        text.contains("survey/undeclared-edge") && text.contains("`a`") && text.contains("`b`"),
        "{text}"
    );
}

#[test]
fn good_map() {
    // req:init/survey-components@368a5dcd
    let map = format!("components:\n{}{TOP}", a_and_b("b"));
    let out = components(TWO_CRATES, &map);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{out:?}");
}

#[test]
fn part_left_for_later() {
    // req:init/survey-components@368a5dcd
    let map = format!(
        "components:\n  - {{ id: a, paths: [\"crates/a/**\"], depends_on: [] }}\n{TOP}outside: [\"crates/b/**\"]\n"
    );
    let out = components(TWO_CRATES, &map);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{out:?}");
}

#[test]
fn outside_glob_that_exempts_nothing() {
    // req:init/survey-components@368a5dcd
    let map = format!(
        "components:\n{}{TOP}outside: [\"crates/c/**\"]\n",
        a_and_b("b")
    );
    let out = components(TWO_CRATES, &map);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let text = stdout(&out);
    assert!(
        text.contains("survey/stale-outside") && text.contains("crates/c/**"),
        "{text}"
    );
}

#[test]
fn an_outside_glob_over_a_covered_file_exempts_nothing() {
    // req:init/survey-components@368a5dcd
    let map = format!(
        "components:\n{}{TOP}outside: [\"crates/b/**\"]\n",
        a_and_b("b")
    );
    let out = components(TWO_CRATES, &map);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(stdout(&out).contains("survey/stale-outside"), "{out:?}");
}

#[test]
fn survey_reads_the_index_alone() {
    // req:init/init-root-ownership@6b897376
    let repo = repo(TWO_CRATES);
    fs::write(repo.path().join("untracked.txt"), "x\n").unwrap();
    assert!(!survey(repo.path()).contains("untracked.txt"));
    let map = format!("components:\n{}", a_and_b("b"));
    fs::write(repo.path().join("map.yml"), map).unwrap();
    let out = clm(repo.path(), &["survey", "--components", "map.yml"]);
    assert!(!stdout(&out).contains("untracked.txt"), "{out:?}");
}

#[test]
fn the_code_files_check_reads() {
    // req:init/survey-components@368a5dcd
    let mut files = TWO_CRATES.to_vec();
    files.push(("docs/notes.md", b"notes\n"));
    files.push((".github/workflows/ci.yml", b"name: ci\n"));
    let top = "  - { id: top, paths: [\"Cargo.toml\", \"README.md\"] }\n";
    let map = format!("components:\n{}{top}", a_and_b("b"));
    let out = components(&files, &map);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let text = stdout(&out);
    assert!(
        text.contains("survey/uncovered") && text.contains(".github/workflows/ci.yml"),
        "{text}"
    );
    assert!(!text.contains("docs/notes.md"), "{text}");
}

#[test]
fn outside_that_is_not_a_list_of_globs_is_refused() {
    // req:init/survey-components@368a5dcd
    for outside in ["outside: x\n", "outside: [\"\"]\n", "outside: [1]\n"] {
        let map = format!("components:\n{}{TOP}{outside}", a_and_b("b"));
        let out = components(TWO_CRATES, &map);
        let unparsed = components(TWO_CRATES, "components: 3\n");
        assert_eq!(
            out.status.code(),
            unparsed.status.code(),
            "{outside}: {out:?}"
        );
        assert_eq!(out.status.code(), Some(2), "{outside}: {out:?}");
        assert!(out.stdout.is_empty(), "{out:?}");
    }
}

#[test]
fn a_file_two_components_cover_overlaps() {
    // req:init/survey-components@368a5dcd
    let map = format!(
        "components:\n{}  - {{ id: c, paths: [\"crates/a/src/lib.rs\"] }}\n{TOP}",
        a_and_b("b")
    );
    let out = components(TWO_CRATES, &map);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(stdout(&out).contains("survey/overlap"), "{out:?}");
}

#[test]
fn a_depends_on_cycle_is_reported() {
    // req:init/survey-components@368a5dcd
    let map = format!(
        "components:\n  - {{ id: a, paths: [\"crates/a/**\"], depends_on: [b] }}\n  - {{ id: b, paths: [\"crates/b/**\"], depends_on: [a] }}\n{TOP}"
    );
    let out = components(TWO_CRATES, &map);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        stdout(&out).contains("survey/cycle") && stdout(&out).contains("a -> b -> a"),
        "{out:?}"
    );
}

#[test]
fn the_report_lines_name_what_they_found() {
    // req:init/survey-components@368a5dcd
    let overlap = format!(
        "components:\n{}  - {{ id: c, paths: [\"crates/a/src/lib.rs\"] }}\n{TOP}",
        a_and_b("b")
    );
    let text = stdout(&components(TWO_CRATES, &overlap));
    assert!(
        text.contains("crates/a/src/lib.rs is in the paths of `a` and `c`"),
        "{text}"
    );
    let stale = format!(
        "components:\n{}{TOP}outside: [\"crates/c/**\"]\n",
        a_and_b("b")
    );
    let text = stdout(&components(TWO_CRATES, &stale));
    assert!(
        text.contains("outside glob crates/c/** matches no file outside every component"),
        "{text}"
    );
}

#[test]
fn binary_files_count_and_spec_files_do_not() {
    // req:init/survey-components@368a5dcd
    let mut files = TWO_CRATES.to_vec();
    files.push(("assets/logo.bin", b"\0\x01\x02"));
    for f in [
        "specs/SPEC.md",
        "changes/x/change.md",
        "backlog/y.md",
        "ARCHITECTURE.md",
        "CAPABILITIES.md",
        "CLAUDE.md",
        "AGENTS.md",
    ] {
        files.push((f, b"x\n"));
    }
    let map = format!("components:\n{}{TOP}", a_and_b("b"));
    let text = stdout(&components(&files, &map));
    assert!(
        text.contains("assets/logo.bin is in no component's paths"),
        "{text}"
    );
    for f in [
        "specs/",
        "changes/",
        "backlog/",
        "ARCHITECTURE",
        "CAPABILITIES",
        "CLAUDE",
        "AGENTS",
    ] {
        assert!(!text.contains(f), "{f}: {text}");
    }
}

#[test]
fn a_star_does_not_cross_a_slash() {
    // req:init/survey-components@368a5dcd
    let map = "components:\n  - { id: a, paths: [\"crates/*\"] }\n  - { id: t, paths: [\"Cargo.toml\", \"README.md\"] }\n";
    let text = stdout(&components(TWO_CRATES, map));
    assert!(
        text.contains("crates/a/src/lib.rs is in no component's paths"),
        "{text}"
    );
}

#[test]
fn a_duplicate_component_exits_1() {
    // req:init/says-component@c8e61ff0
    let repo = repo(TWO_CRATES);
    let map = "components:\n  - { id: a, paths: [] }\n  - { id: a, paths: [] }\n";
    fs::write(repo.path().join("map.yaml"), map).unwrap();
    let out = clm(repo.path(), &["survey", "--components", "map.yaml"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("component `a` appears twice"), "{text}");
}

#[test]
fn the_old_key() {
    // req:init/survey-components@368a5dcd
    let repo = repo(TWO_CRATES);
    let map = format!("nodes:\n{}{TOP}", a_and_b("b"));
    fs::write(repo.path().join("map.yaml"), map).unwrap();
    let out = clm(repo.path(), &["survey", "--components", "map.yaml"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("map.yaml") && text.contains("`components`") && text.contains("`nodes`"),
        "{text}"
    );
    let flag = clm(repo.path(), &["survey", "--nodes", "map.yaml"]);
    assert_eq!(flag.status.code(), Some(2), "{flag:?}");
    assert!(
        String::from_utf8_lossy(&flag.stderr).contains("unexpected argument"),
        "{flag:?}"
    );
}

const CARVED: &[(&str, &[u8])] = &[
    ("app/mod.py", b"x = 1\n"),
    ("feat/lib.py", b"y = 1\n"),
    ("tests/conftest.py", b""),
    ("tests/feat/test_x.py", b"import app.mod\n"),
];

fn carved_map(feat: &str) -> String {
    format!(
        "components:\n  - {{ id: app, paths: [\"app/**\"], depends_on: [feat] }}\n  - {{ id: feat, paths: [\"feat/**\"], {feat} }}\n  - {{ id: harness, paths: [\"tests/**\"] }}\n"
    )
}

#[test]
fn tests_carved_out_of_a_suite() {
    // req:init/survey-components@368a5dcd
    let map = carved_map("tests: [\"tests/feat/**\"]");
    let out = components(CARVED, &map);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let text = stdout(&out);
    assert!(
        text.contains("survey/undeclared-edge")
            && text.contains("tests/feat/test_x.py (component `feat`)")
            && text.contains("`app`"),
        "{out:?}"
    );
    assert!(!text.contains("survey/overlap"), "{out:?}");
    assert!(!text.contains("survey/cycle"), "{out:?}");
}

#[test]
fn an_exempt_edge_is_not_reported() {
    // req:init/survey-components@368a5dcd
    let map = carved_map(
        "tests: [\"tests/feat/**\"], exemptions: [{ rule: undeclared-imports, files: [\"tests/feat/**\"], until: backlog/x }]",
    );
    let out = components(CARVED, &map);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "", "{out:?}");
}

#[test]
fn an_exemption_covers_only_its_rule_and_files() {
    // req:init/survey-components@368a5dcd
    for exemption in [
        "{ rule: skipped-tests, files: [\"tests/feat/**\"], until: backlog/x }",
        "{ rule: undeclared-imports, files: [\"tests/other/**\"], until: backlog/x }",
    ] {
        let map = carved_map(&format!(
            "tests: [\"tests/feat/**\"], exemptions: [{exemption}]"
        ));
        let out = components(CARVED, &map);
        assert_eq!(out.status.code(), Some(1), "{exemption}: {out:?}");
        assert!(
            stdout(&out).contains("survey/undeclared-edge"),
            "{exemption}: {out:?}"
        );
    }
}

#[test]
fn an_exemption_the_model_would_refuse_is_refused() {
    // req:init/survey-components@368a5dcd
    let unparsed = components(CARVED, "components: 3\n");
    for exemption in [
        "3",
        "{ files: [\"a\"], until: backlog/x }",
        "{ rule: undeclared-imports, until: backlog/x }",
        "{ rule: undeclared-imports, files: [\"a\"] }",
        "{ rule: paths-disjoint, files: [\"a\"], until: backlog/x }",
        "{ rule: undeclared-imports, files: [], until: backlog/x }",
        "{ rule: undeclared-imports, files: [\"\"], until: backlog/x }",
        "{ rule: undeclared-imports, files: \"a\", until: backlog/x }",
        "{ rule: undeclared-imports, files: [\"a\"], until: x }",
        "{ rule: undeclared-imports, files: [\"a\"], until: backlog/x, extra: 1 }",
    ] {
        let map = carved_map(&format!("exemptions: [{exemption}]"));
        let out = components(CARVED, &map);
        assert_eq!(
            out.status.code(),
            unparsed.status.code(),
            "{exemption}: {out:?}"
        );
        assert!(out.stdout.is_empty(), "{exemption}: {out:?}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("exemptions"), "{exemption}: {err}");
    }
    let ok = carved_map(
        "exemptions: [{ rule: skipped-tests, files: [\"a\"], until: backlog/x, why: \"y\" }]",
    );
    assert_ne!(components(CARVED, &ok).status.code(), Some(2));
}

#[test]
fn a_map_that_sets_test_depends_on_is_refused() {
    // req:init/survey-components@368a5dcd
    let map = carved_map("tests: [\"tests/feat/**\"], test_depends_on: [app]");
    let out = components(CARVED, &map);
    let unparsed = components(CARVED, "components: 3\n");
    assert_eq!(out.status.code(), unparsed.status.code(), "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
}

#[test]
fn two_components_tests_overlap() {
    // req:init/survey-components@368a5dcd
    let map = format!(
        "{}  - {{ id: other, paths: [\"other/**\"], tests: [\"tests/feat/**\"] }}\n",
        carved_map("tests: [\"tests/feat/**\"]")
    );
    let mut files = CARVED.to_vec();
    files.push(("other/x.py", b""));
    let out = components(&files, &map);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let text = stdout(&out);
    assert!(
        text.contains("survey/overlap") && text.contains("tests/feat/test_x.py"),
        "{out:?}"
    );
}

#[test]
fn stable() {
    // req:init/survey-map@3138d8a2
    // Twenty crates, each depending on the next two, in files written in reverse: any
    // ordering that isn't by path shows up as a difference between runs or as unsorted edges.
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for n in (0..20).rev() {
        let deps: String = (n + 1..20)
            .take(2)
            .map(|d| format!("c{d:02} = {{ path = \"../c{d:02}\" }}\n"))
            .collect();
        files.push((
            format!("crates/c{n:02}/Cargo.toml"),
            format!("[package]\nname = \"c{n:02}\"\n\n[dependencies]\n{deps}").into_bytes(),
        ));
        files.push((
            format!("crates/c{n:02}/src/lib.rs"),
            b"mod inner;\nuse crate::inner::deep;\n".to_vec(),
        ));
        files.push((
            format!("crates/c{n:02}/src/inner/mod.rs"),
            b"pub mod deep;\n".to_vec(),
        ));
        files.push((format!("crates/c{n:02}/src/inner/deep.rs"), Vec::new()));
    }
    let files: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(p, c)| (p.as_str(), c.as_slice()))
        .collect();
    let repo = repo(&files);
    let first = survey(repo.path());
    assert_eq!(first, survey(repo.path()));
    let edges = section(&first, "edges");
    assert_eq!(edges.len(), 20 + 19 + 18, "{first}");
    let mut sorted = edges.clone();
    sorted.sort_by_key(|l| {
        let (from, rest) = l.split_once(" -> ").unwrap();
        let to = rest.split_once("  ").unwrap().0;
        (from.to_owned(), to.to_owned())
    });
    assert_eq!(edges, sorted);
}

#[test]
fn components_flag_checks_the_map() {
    // req:clm/component-flags@4d6d18e7
    let repo = repo(TWO_CRATES);
    fs::write(repo.path().join("nodes.yaml"), "components: []\n").unwrap();
    let new = clm(repo.path(), &["survey", "--components", "nodes.yaml"]);
    assert_eq!(new.status.code(), Some(1), "{new:?}");
    let help = String::from_utf8(clm(repo.path(), &["survey", "--help"]).stdout).unwrap();
    assert!(help.contains("--components"), "{help}");
}
