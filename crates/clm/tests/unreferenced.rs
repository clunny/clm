//! `clm unreferenced`: public items of a workspace's libraries that nothing else uses, as a
//! command and as a `clm check` detector.

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
fn repo(files: &[(&str, &str)]) -> TempDir {
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

const A_MANIFEST: &str = "[package]\nname = \"a\"\nversion = \"0.1.0\"\n";
const TAIL: &str = "is public but nothing outside its library uses it";

#[test]
fn scenario_findings() {
    // req:check.unreferenced/report@27b2d762
    let lib = format!(
        "mod x;\n\npub fn f() {{}}\n{}pub fn z() {{}}\n",
        "\n".repeat(6)
    );
    let x = format!("{}pub fn h() {{}}\n", "\n".repeat(11));
    let dir = repo(&[
        ("crates/a/Cargo.toml", A_MANIFEST),
        ("crates/a/src/lib.rs", &lib),
        ("crates/a/src/x.rs", &x),
    ]);
    let out = clm(dir.path(), &["unreferenced"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!(
            "crates/a/src/lib.rs:3: a::f {TAIL}\ncrates/a/src/lib.rs:10: a::z {TAIL}\ncrates/a/src/x.rs:12: a::h {TAIL}\n"
        )
    );
}

#[test]
fn scenario_clean_or_no_rust() {
    // req:check.unreferenced/report@27b2d762
    let used = repo(&[
        ("crates/a/Cargo.toml", A_MANIFEST),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
        (
            "crates/b/Cargo.toml",
            "[package]\nname = \"b\"\nversion = \"0.1.0\"\n",
        ),
        ("crates/b/src/main.rs", "fn main() {\n    a::f();\n}\n"),
    ]);
    let packages_used = repo(&[
        ("ts/package.json", r#"{"name": "@acme/ts"}"#),
        ("ts/index.ts", "export function tsUsed() {}\n"),
        ("py/acme_py/__init__.py", "def py_used(): ...\n"),
        ("app/main.ts", "tsUsed()\n"),
        ("app/main.py", "py_used()\n"),
    ]);
    let nothing = repo(&[
        ("README.md", "# Nothing in Rust\n"),
        ("scripts/run.py", "def h(): ...\n"),
        ("web/app.ts", "export function e() {}\n"),
    ]);
    for dir in [&used, &packages_used, &nothing] {
        let out = clm(dir.path(), &["unreferenced"]);
        assert_eq!(out.status.code(), Some(0), "{out:?}");
        assert!(out.stdout.is_empty(), "{out:?}");
    }
}

#[test]
fn scenario_broken_manifest() {
    // req:check.unreferenced/report@27b2d762
    for (path, text) in [
        ("crates/a/Cargo.toml", "[package\n"),
        ("web/package.json", "{\"name\": \n"),
        ("svc/pyproject.toml", "[project\n"),
    ] {
        let dir = repo(&[(path, text)]);
        let out = clm(dir.path(), &["unreferenced"]);
        assert_eq!(out.status.code(), Some(2), "{out:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert_eq!(stderr.lines().count(), 1, "{stderr}");
        assert!(stderr.contains(path), "{stderr}");
    }
}

#[test]
fn package_findings_are_worded_for_packages() {
    // req:check.unreferenced/report@27b2d762
    let dir = repo(&[
        ("crates/a/Cargo.toml", A_MANIFEST),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
        ("ts/package.json", r#"{"name": "@acme/ts"}"#),
        ("ts/index.ts", "export function g() {}\n"),
        ("py/acme_py/__init__.py", "def h(): ...\n"),
    ]);
    let out = clm(dir.path(), &["unreferenced"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let package = "is exported but nothing outside its package uses it";
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!(
            "crates/a/src/lib.rs:1: a::f {TAIL}\n\
             py/acme_py/__init__.py:1: acme_py::h {package}\n\
             ts/index.ts:1: @acme/ts::g {package}\n"
        )
    );
}

#[test]
fn scenario_allowed_package_and_item() {
    // req:check.unreferenced/allow-list@8bbdff3e
    let dir = repo(&[
        ("packages/core/package.json", r#"{"name": "@acme/core"}"#),
        (
            "packages/core/src/index.ts",
            "export function f() {}\nexport function g() {}\n",
        ),
        (
            "packages/py/acme_core/__init__.py",
            "def h(): ...\ndef k(): ...\n",
        ),
    ]);
    let out = clm(
        dir.path(),
        &[
            "unreferenced",
            "--allow",
            "@acme/core::f",
            "--allow",
            "acme_core",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "packages/core/src/index.ts:2: @acme/core::g is exported but nothing outside its package uses it\n"
    );
}

#[test]
fn one_name_for_a_crate_and_a_package_skips_both() {
    // req:check.unreferenced/allow-list@8bbdff3e
    let dir = repo(&[
        (
            "crates/acme/Cargo.toml",
            "[package]\nname = \"acme-core\"\nversion = \"0.1.0\"\n",
        ),
        ("crates/acme/src/lib.rs", "pub fn f() {}\n"),
        ("py/acme_core/__init__.py", "def h(): ...\n"),
    ]);
    let out = clm(dir.path(), &["unreferenced", "--allow", "acme_core"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
    // Stale only when it skips nothing in either language.
    let dir = repo(&[("py/acme_core/__init__.py", "def h(): ...\n")]);
    let out = clm(dir.path(), &["unreferenced", "--allow", "acme_core"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let dir = repo(&[("py/acme_core/__init__.py", "")]);
    let out = clm(dir.path(), &["unreferenced", "--allow", "acme_core"]);
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "clm.toml: allow acme_core skips no finding\n"
    );
}

#[test]
fn scenario_from_a_subdirectory_of_a_project_without_clm() {
    // req:check.unreferenced/report@27b2d762
    let dir = repo(&[
        ("crates/a/Cargo.toml", A_MANIFEST),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
    ]);
    let out = clm(&dir.path().join("crates/a/src"), &["unreferenced"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("crates/a/src/lib.rs:1: a::f {TAIL}\n")
    );
}

#[test]
fn root_option_names_the_repository() {
    // req:check.unreferenced/report@27b2d762
    let dir = repo(&[
        ("crates/a/Cargo.toml", A_MANIFEST),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
    ]);
    let elsewhere = TempDir::new().unwrap();
    let root = dir.path().to_str().unwrap();
    let out = clm(elsewhere.path(), &["unreferenced", "--root", root]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(String::from_utf8(out.stdout).unwrap().contains("a::f"));
}

#[test]
fn scenario_allowed_item_and_crate() {
    // req:check.unreferenced/allow-list@8bbdff3e
    let dir = repo(&[
        ("crates/a/Cargo.toml", A_MANIFEST),
        ("crates/a/src/lib.rs", "pub fn f() {}\npub fn g() {}\n"),
        (
            "crates/b/Cargo.toml",
            "[package]\nname = \"b\"\nversion = \"0.1.0\"\n",
        ),
        ("crates/b/src/lib.rs", "pub fn h() {}\n"),
    ]);
    let out = clm(
        dir.path(),
        &["unreferenced", "--allow", "a::f", "--allow", "b"],
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("crates/a/src/lib.rs:2: a::g {TAIL}\n")
    );
}

#[test]
fn scenario_stale_entries() {
    // req:check.unreferenced/allow-list@8bbdff3e
    let dir = repo(&[
        ("crates/a/Cargo.toml", A_MANIFEST),
        ("crates/a/src/lib.rs", "pub fn used() {}\n"),
        (
            "crates/b/Cargo.toml",
            "[package]\nname = \"b\"\nversion = \"0.1.0\"\n",
        ),
        ("crates/b/src/main.rs", "fn main() {\n    a::used();\n}\n"),
    ]);
    let out = clm(
        dir.path(),
        &[
            "unreferenced",
            "--allow",
            "a::gone",
            "--allow",
            "zz",
            "--allow",
            "a::used",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "clm.toml: allow a::gone skips no finding\n\
         clm.toml: allow zz skips no finding\n\
         clm.toml: allow a::used skips no finding\n"
    );
}

#[test]
fn scenario_entry_given_as_an_argument() {
    // req:check.unreferenced/entry-attributes@69e092f6
    let dir = repo(&[
        ("crates/a/Cargo.toml", A_MANIFEST),
        (
            "crates/a/src/lib.rs",
            "#[event(fetch)]\npub async fn fetch() {}\n#[worker::event(scheduled)]\npub async fn tick() {}\n",
        ),
    ]);
    let out = clm(dir.path(), &["unreferenced"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("crates/a/src/lib.rs:2: a::fetch {TAIL}\ncrates/a/src/lib.rs:4: a::tick {TAIL}\n")
    );
    let out = clm(dir.path(), &["unreferenced", "--entry", "event"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
}

#[test]
fn scenario_stale_entry() {
    // req:check.unreferenced/entry-attributes@69e092f6
    // req:check.unreferenced/report@27b2d762
    let dir = repo(&[
        ("crates/a/Cargo.toml", A_MANIFEST),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
    ]);
    let out = clm(
        dir.path(),
        &[
            "unreferenced",
            "--entry",
            "napi",
            "--allow",
            "zz",
            "--allow",
            "a::f",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "clm.toml: allow zz skips no finding\nclm.toml: entry napi marks no item\n"
    );
}

#[test]
fn scenario_named_as_a_detector() {
    // req:check.unreferenced/report@27b2d762
    let clm_toml = format!(
        "[[detectors]]\nname = \"unreferenced-public-items\"\ncommand = [{:?}, \"unreferenced\"]\n",
        env!("CARGO_BIN_EXE_clm")
    );
    let dir = repo(&[
        (
            "specs/SPEC.md",
            "---\ntitle: R\nstatus: active\nowners: []\n---\n",
        ),
        ("clm.toml", &clm_toml),
        ("crates/a/Cargo.toml", A_MANIFEST),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
    ]);
    let out = clm(dir.path(), &["check"]);
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(1), "{stdout}{stderr}");
    assert!(
        stdout
            .lines()
            .any(|l| l == "clm.toml: check/detectors: unreferenced-public-items failed (exit 1)"),
        "{stdout}"
    );
    assert!(
        stderr.contains(&format!("crates/a/src/lib.rs:1: a::f {TAIL}")),
        "{stderr}"
    );
}
