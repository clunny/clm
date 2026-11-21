//! `clm unreferenced` as a call: the command's arguments, its streams and its exit status,
//! in-process.

use std::fs;
use std::path::Path;
use std::process::Command;

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

/// What a run of the command gave: its exit status and what it wrote to each stream.
#[derive(Debug)]
struct Out {
    status: Status,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

#[derive(Debug)]
struct Status(u8);

impl Status {
    fn code(&self) -> Option<i32> {
        Some(self.0.into())
    }
}

/// `clm <args>` as `crates/clm` routes it, with `dir` as the root it found.
fn clm(dir: &Path, args: &[&str]) -> Out {
    let (name, rest) = args.split_first().unwrap();
    assert_eq!(*name, "unreferenced");
    // The binary ran with git's own config isolated; the call's `git` inherits this process's
    // environment, so the test sets it here. Safe: the tests run one per process
    // (`ci/nextest-runner`), and every call sets the same values.
    unsafe {
        std::env::set_var("GIT_CONFIG_GLOBAL", "/dev/null");
        std::env::set_var("GIT_CONFIG_NOSYSTEM", "1");
    }
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let status = clm_unreferenced::command(
        rest.iter().copied(),
        dir,
        &|key| std::env::var(key).ok(),
        &mut stdout,
        &mut stderr,
    );
    Out {
        status: Status(status),
        stdout,
        stderr,
    }
}

const A_MANIFEST: &str = "[package]\nname = \"a\"\nversion = \"0.1.0\"\n";
const TAIL: &str = "is public but nothing outside its library uses it";

#[test]
fn a_command_called_in_a_test() {
    // req:check.unreferenced/commands-in-crate@215013cd
    let dir = repo(&[
        ("crates/a/Cargo.toml", A_MANIFEST),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
    ]);
    let out = clm(dir.path(), &["unreferenced"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(out.stderr.is_empty(), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("crates/a/src/lib.rs:1: a::f {TAIL}\n")
    );

    // An error is one line on the error stream, with the command's prefix, and status 2.
    let broken = repo(&[("crates/a/Cargo.toml", "[package\n")]);
    let out = clm(broken.path(), &["unreferenced"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.starts_with("clm unreferenced: crates/a/Cargo.toml"),
        "{stderr}"
    );
    assert_eq!(stderr.lines().count(), 1, "{stderr}");

    // Nothing to report is status 0 with nothing written.
    let clean = repo(&[("README.md", "# none\n")]);
    let out = clm(clean.path(), &["unreferenced"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{out:?}");
}

#[test]
fn arguments_it_cannot_parse() {
    // req:check.unreferenced/commands-in-crate@215013cd
    let dir = repo(&[("README.md", "# none\n")]);
    let out = clm(dir.path(), &["unreferenced", "--bogus"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("--bogus"), "{stderr}");
    assert!(stderr.contains("Usage: clm unreferenced"), "{stderr}");
    // The call returned rather than ending the process, so the test goes on.
    let out = clm(dir.path(), &["unreferenced", "--allow"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
}

#[test]
fn scenario_findings() {
    // req:check.unreferenced/report@849ec044
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
    // req:check.unreferenced/report@849ec044
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
    // req:check.unreferenced/report@849ec044
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
    // req:check.unreferenced/report@849ec044
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
fn root_option_names_the_repository() {
    // req:check.unreferenced/report@849ec044
    // req:check.unreferenced/commands-in-crate@215013cd
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
    // req:check.unreferenced/report@849ec044
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
fn stale_entries_follow_in_the_order_given() {
    // req:check.unreferenced/report@849ec044
    let dir = repo(&[
        ("crates/a/Cargo.toml", A_MANIFEST),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
    ]);
    let out = clm(
        dir.path(),
        &[
            "unreferenced",
            "--entry",
            "zeta",
            "--allow",
            "a::f",
            "--entry",
            "alpha",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "clm.toml: entry zeta marks no item\nclm.toml: entry alpha marks no item\n"
    );
}

#[test]
fn runs_no_cargo_npm_node_or_python() {
    // req:check.unreferenced/report@849ec044
    use std::os::unix::fs::PermissionsExt;
    let dir = repo(&[
        ("Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n"),
        ("crates/a/Cargo.toml", A_MANIFEST),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
        (
            "packages/core/package.json",
            "{\"name\": \"@acme/core\", \"main\": \"index.ts\"}\n",
        ),
        ("packages/core/index.ts", "export function g() {}\n"),
        ("py/pyproject.toml", "[project]\nname = \"acme-py\"\n"),
        ("py/acme_py/__init__.py", "def h():\n    pass\n"),
    ]);
    // Stand-ins that record their own run, ahead of everything else on `PATH`; `git` stays
    // reachable behind them. Safe: the tests run one per process (`ci/nextest-runner`).
    let bin = TempDir::new().unwrap();
    let ran = bin.path().join("ran");
    for tool in ["cargo", "npm", "npx", "node", "python", "python3"] {
        let path = bin.path().join(tool);
        let script = format!("#!/bin/sh\necho {tool} >> '{}'\n", ran.display());
        fs::write(&path, script).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = format!(
        "{}:{}",
        bin.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    unsafe { std::env::set_var("PATH", path) };
    let out = clm(dir.path(), &["unreferenced"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(!ran.exists(), "ran {:?}", fs::read_to_string(&ran));
}

/// The call run with `dir` as the root, `args` after the command's name, and `env` as its
/// environment lookup, writing to `out` and to a buffer for the error stream.
fn call(
    dir: &Path,
    args: &[&str],
    env: &dyn Fn(&str) -> Option<String>,
    out: &mut dyn std::io::Write,
) -> (u8, Vec<u8>) {
    // Safe: the tests run one per process (`ci/nextest-runner`), and every call sets the same
    // values.
    unsafe {
        std::env::set_var("GIT_CONFIG_GLOBAL", "/dev/null");
        std::env::set_var("GIT_CONFIG_NOSYSTEM", "1");
    }
    let mut err = Vec::new();
    let status = clm_unreferenced::command(args.iter().copied(), dir, env, out, &mut err);
    (status, err)
}

const HELP_FIRST_LINE: &str = "List the public items of the workspace's Rust libraries and TypeScript and Python packages that nothing outside their own library or package uses; exit 1 when it lists any";

#[test]
fn help_asked_of_the_call() {
    // req:check.unreferenced/commands-in-crate@01c980d6
    let dir = repo(&[
        ("crates/a/Cargo.toml", A_MANIFEST),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
    ]);
    for flag in ["--help", "-h"] {
        let mut out = Vec::new();
        let (status, err) = call(dir.path(), &[flag], &|_| None, &mut out);
        assert_eq!(status, 0, "{flag}");
        assert!(err.is_empty(), "{flag}: {}", String::from_utf8_lossy(&err));
        let out = String::from_utf8(out).unwrap();
        let mut lines = out.lines();
        assert_eq!(lines.next(), Some(HELP_FIRST_LINE), "{flag}: {out}");
        assert_eq!(lines.next(), Some(""), "{flag}: {out}");
        assert_eq!(
            lines.next(),
            Some("Usage: clm unreferenced [OPTIONS]"),
            "{flag}: {out}"
        );
    }
}

/// A stream whose every write fails.
struct Refusing;

impl std::io::Write for Refusing {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("refused"))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::other("refused"))
    }
}

#[test]
fn a_stream_that_refuses_writes() {
    // req:check.unreferenced/commands-in-crate@01c980d6
    let dir = repo(&[
        ("crates/a/Cargo.toml", A_MANIFEST),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
    ]);
    let (status, _) = call(dir.path(), &[], &|_| None, &mut Refusing);
    assert_eq!(status, 2);
    let (status, _) = call(dir.path(), &["--help"], &|_| None, &mut Refusing);
    assert_eq!(status, 2);
}

#[test]
fn an_environment_it_does_not_read() {
    // req:check.unreferenced/commands-in-crate@01c980d6
    let dir = repo(&[
        ("crates/a/Cargo.toml", A_MANIFEST),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
    ]);
    let mut out = Vec::new();
    let (status, err) = call(
        dir.path(),
        &[],
        &|key| panic!("the call read {key} from the environment"),
        &mut out,
    );
    assert_eq!(status, 1, "{}", String::from_utf8_lossy(&err));
    assert_eq!(
        String::from_utf8(out).unwrap(),
        format!("crates/a/src/lib.rs:1: a::f {TAIL}\n")
    );
    // The caller's process goes on.
}

#[test]
fn scenario_odd_manifests_and_unreadable_files() {
    // req:check.unreferenced/report@1a2b2341
    // req:check.unreferenced/public-items@32359404
    use std::os::unix::fs::PermissionsExt;
    // A `[package]` without a string `name`.
    for manifest in [
        "[package]\nversion = \"0.1.0\"\n",
        "[package]\nname = 1\nversion = \"0.1.0\"\n",
    ] {
        let dir = repo(&[("crates/a/Cargo.toml", manifest)]);
        let out = clm(dir.path(), &["unreferenced"]);
        assert_eq!(out.status.code(), Some(2), "{out:?}");
        assert!(out.stdout.is_empty(), "{out:?}");
        assert_eq!(
            String::from_utf8(out.stderr).unwrap(),
            "clm unreferenced: crates/a/Cargo.toml: [package] has no name\n"
        );
    }

    // No `[package]` table, a `package` key that isn't one, and a nameless `package.json`.
    let dir = repo(&[
        ("Cargo.toml", "[workspace]\n"),
        ("crates/b/Cargo.toml", "package = 1\n"),
        ("package.json", r#"{"private": true}"#),
    ]);
    let out = clm(dir.path(), &["unreferenced"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{out:?}");

    // A `[lib] name` that isn't a string leaves the package's name.
    let dir = repo(&[
        (
            "crates/a/Cargo.toml",
            "[package]\nname = \"a-b\"\nversion = \"0.1.0\"\n\n[lib]\nname = 1\n",
        ),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
    ]);
    let out = clm(dir.path(), &["unreferenced"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("crates/a/src/lib.rs:1: a_b::f {TAIL}\n")
    );

    // A tracked source the process can't read.
    let dir = repo(&[
        ("crates/a/Cargo.toml", A_MANIFEST),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
    ]);
    let lib = dir.path().join("crates/a/src/lib.rs");
    fs::set_permissions(&lib, fs::Permissions::from_mode(0o000)).unwrap();
    // Running as root reads it anyway, and then there is no unreadable file to show.
    if fs::read(&lib).is_err() {
        let out = clm(dir.path(), &["unreferenced"]);
        fs::set_permissions(&lib, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(out.status.code(), Some(2), "{out:?}");
        assert!(out.stdout.is_empty(), "{out:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            stderr.starts_with("clm unreferenced: crates/a/src/lib.rs: "),
            "{stderr}"
        );
        assert_eq!(stderr.lines().count(), 1, "{stderr}");
    }

    // `--root` naming a directory in no git repository.
    let outside = TempDir::new().unwrap();
    let elsewhere = repo(&[("README.md", "# none\n")]);
    let root = outside.path().to_str().unwrap();
    let out = clm(elsewhere.path(), &["unreferenced", "--root", root]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.starts_with("clm unreferenced: "), "{stderr}");
    assert!(stderr.contains("git"), "{stderr}");
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
}
