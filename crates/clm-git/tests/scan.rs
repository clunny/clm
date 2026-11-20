//! No crate but `clm-git` starts `git`, `gh` or `az`, and none but six starts any process
//! at all (vcs/one-git-layer).

use std::path::{Path, PathBuf};

/// The crates that may start a process: `clm-git`, and those that run programs the project
/// names (a detector, a land check, `clm`, `sh`).
const MAY_START: [&str; 6] = [
    "clm-git",
    "clm-check",
    "clm-land",
    "clm-loop",
    "clm-run",
    "clm-verify",
];

/// Calls that start `git`, `gh` or `az`, without their closing bracket.
const TOOLS: [&str; 3] = [
    "Command::new(\"git\"",
    "Command::new(\"gh\"",
    "Command::new(\"az\"",
];

#[derive(Debug, Default)]
struct Scan {
    /// Each crate read, by its directory's name.
    crates: Vec<String>,
    /// `<file>:<line>` of each match, and `<crate>: ...` for a crate with no Rust source.
    findings: Vec<String>,
}

/// Reads `build.rs` and every `.rs` file under `src/` of each directory under `crates/`
/// that holds a `Cargo.toml`.
fn scan(workspace: &Path) -> Scan {
    let mut scan = Scan::default();
    let crates = sorted(&workspace.join("crates"));
    for krate in crates.iter().filter(|d| d.join("Cargo.toml").is_file()) {
        let name = krate.file_name().unwrap().to_string_lossy().into_owned();
        let sources = rust_files(&krate.join("src"));
        if sources.is_empty() {
            scan.findings
                .push(format!("crates/{name}: no .rs file under src/"));
        }
        let build = krate.join("build.rs");
        for file in build.is_file().then_some(build).into_iter().chain(sources) {
            let shown = file.strip_prefix(workspace).unwrap().display().to_string();
            for line in matches(&name, &read(&file)) {
                scan.findings.push(format!("{shown}:{line}"));
            }
        }
        scan.crates.push(name);
    }
    scan
}

/// The lines, each once, on which a string `krate` may not hold starts, read from `text`
/// with every whitespace character removed.
fn matches(krate: &str, text: &str) -> Vec<usize> {
    let mut squeezed = String::new();
    // The line of each byte of `squeezed`.
    let mut lines = Vec::new();
    let mut line = 1;
    for c in text.chars() {
        if c == '\n' {
            line += 1;
        }
        if !c.is_whitespace() {
            squeezed.push(c);
            lines.extend(std::iter::repeat_n(line, c.len_utf8()));
        }
    }
    let s = squeezed.as_str();
    let mut at: Vec<usize> = Vec::new();
    if krate != "clm-git" {
        for tool in TOOLS {
            at.extend(s.match_indices(tool).map(|(i, _)| i));
        }
    }
    if !MAY_START.contains(&krate) {
        at.extend(s.match_indices("process::Command").map(|(i, _)| i));
        at.extend(s.match_indices("process::{").filter_map(|(i, _)| {
            group(&s[i + "process::".len()..])
                .contains("Command")
                .then_some(i)
        }));
        at.extend(
            s.match_indices("Command::new(")
                .filter(|(i, _)| !s[..*i].ends_with("clap::"))
                .map(|(i, _)| i),
        );
    }
    let mut found: Vec<usize> = at.into_iter().map(|i| lines[i]).collect();
    found.sort_unstable();
    found.dedup();
    found
}

/// The text of the brace group `s` starts with, up to the brace that closes it.
fn group(s: &str) -> &str {
    let mut depth = 0;
    for (i, c) in s.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            return &s[..i];
        }
    }
    s
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for path in sorted(dir) {
        if path.is_dir() {
            out.extend(rust_files(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out
}

fn sorted(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<_> = entries.map(|e| e.unwrap().path()).collect();
    paths.sort();
    paths
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// A workspace holding, for each `(name, source)`, a crate `crates/<name>` whose
/// `src/lib.rs` is `source`.
fn fixture(crates: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write(
        &dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/*\"]\n",
    );
    for (name, source) in crates {
        let krate = dir.path().join("crates").join(name);
        write(
            &krate.join("Cargo.toml"),
            &format!("[package]\nname = \"{name}\"\n\n[dependencies]\nclm-git.workspace = true\n"),
        );
        write(&krate.join("src/lib.rs"), source);
    }
    dir
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

const RUNS_GIT: &str = "fn f() {\n    std::process::Command::new(\"git\");\n}\n";

#[test]
fn a_wrapper_comes_back() {
    // req:vcs/one-git-layer@672cd7c7
    let ws = fixture(&[("clm-land", "mod m;\n")]);
    write(
        &ws.path().join("crates/clm-land/src/m/n.rs"),
        "\n\nlet _ = Command::new(\"gh\");\n\n#[cfg(test)]\nmod tests {\n    fn t() { Command::new(\"git\"); }\n}\n",
    );
    assert_eq!(
        scan(ws.path()).findings,
        [
            "crates/clm-land/src/m/n.rs:3",
            "crates/clm-land/src/m/n.rs:7"
        ]
    );
}

#[test]
fn a_crate_that_has_not_migrated() {
    // req:vcs/one-git-layer@672cd7c7
    let ws = fixture(&[("a", RUNS_GIT)]);
    write(
        &ws.path().join("crates/a/Cargo.toml"),
        "[package]\nname = \"a\"\n",
    );
    assert_eq!(scan(ws.path()).findings, ["crates/a/src/lib.rs:2"]);
}

#[test]
fn clean_dependants() {
    // req:vcs/one-git-layer@672cd7c7
    let ws = fixture(&[("a", "fn f() {}\n"), ("b", "mod m;\n")]);
    // A crate's `tests/` and `benches/` are outside the scan.
    write(&ws.path().join("crates/a/tests/t.rs"), RUNS_GIT);
    write(&ws.path().join("crates/a/benches/b.rs"), RUNS_GIT);
    let found = scan(ws.path());
    assert!(found.findings.is_empty(), "{found:?}");
    assert_eq!(found.crates, ["a", "b"]);
}

#[test]
fn the_azure_cli() {
    // req:vcs/one-git-layer@672cd7c7
    let source = "fn f() {\n    Command::new(\"az\").arg(\"repos\");\n}\n";
    let ws = fixture(&[("a", source), ("clm-git", source), ("clm-land", source)]);
    assert_eq!(
        scan(ws.path()).findings,
        ["crates/a/src/lib.rs:2", "crates/clm-land/src/lib.rs:2"]
    );
}

#[test]
fn a_wrapper_that_takes_the_program() {
    // req:vcs/one-git-layer@672cd7c7
    let ws = fixture(&[
        (
            "a",
            "fn f(program: &str) {\n    std::process::Command::new(program);\n}\n",
        ),
        ("b", "use std::fs;\nuse std::process::{Child, Command};\n"),
        ("c", "\nuse std::os::unix::process::CommandExt;\n"),
        (
            "d",
            "use std::{\n    process::{Child, ExitStatus},\n    fs,\n};\nuse std::process::{\n    io::{Read},\n    Command,\n};\n",
        ),
        ("clm-run", "use std::process::{Child, Command};\n"),
    ]);
    assert_eq!(
        scan(ws.path()).findings,
        [
            "crates/a/src/lib.rs:2",
            "crates/b/src/lib.rs:2",
            "crates/c/src/lib.rs:2",
            "crates/d/src/lib.rs:5"
        ]
    );
}

#[test]
fn a_call_split_across_lines() {
    // req:vcs/one-git-layer@672cd7c7
    let ws = fixture(&[(
        "clm-land",
        "fn f() {\n    let _ = Command::new(\n        \"gh\",\n    )\n    .output();\n}\n",
    )]);
    assert_eq!(scan(ws.path()).findings, ["crates/clm-land/src/lib.rs:2"]);
}

#[test]
fn a_crate_that_runs_other_programs() {
    // req:vcs/one-git-layer@672cd7c7
    let ws = fixture(&[
        ("clm-land", "fn f() {\n    Command::new(\"sh\");\n}\n"),
        (
            "a",
            "fn f(name: &str) {\n    clap::Command::new(name);\n}\n",
        ),
    ]);
    let found = scan(ws.path());
    assert!(found.findings.is_empty(), "{found:?}");
    write(
        &ws.path().join("crates/clm-land/build.rs"),
        "fn main() {\n    std::process::Command::new(\"git\");\n}\n",
    );
    assert_eq!(scan(ws.path()).findings, ["crates/clm-land/build.rs:2"]);
}

#[test]
fn the_workspace_runs_git_only_through_the_layer() {
    // req:vcs/one-git-layer@672cd7c7
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let found = scan(&workspace);
    assert!(
        found.findings.is_empty(),
        "start processes through clm-git:\n{}",
        found.findings.join("\n")
    );
    for krate in ["clm", "clm-git", "clm-land"] {
        assert!(found.crates.iter().any(|c| c == krate), "{found:?}");
    }
    let ws = fixture(&[("a", "fn f() {}\n"), ("e", "")]);
    std::fs::remove_file(ws.path().join("crates/e/src/lib.rs")).unwrap();
    write(&ws.path().join("crates/e/src/notes.txt"), "");
    assert_eq!(
        scan(ws.path()).findings,
        ["crates/e: no .rs file under src/"]
    );
}
