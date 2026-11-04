//! No crate that lists `clm-git` runs `git` or `gh` itself (vcs/one-git-layer).

use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
struct Scan {
    /// `<file>:<line>` of each direct `git` or `gh` run in a crate that lists `clm-git`.
    violations: Vec<String>,
    /// Crates that don't list `clm-git` yet, so aren't scanned.
    uncovered: Vec<String>,
}

/// Scans the `src/` tree of each workspace member whose `Cargo.toml` lists `clm-git`.
fn scan(workspace: &Path) -> Scan {
    let mut found = Scan::default();
    for krate in members(workspace) {
        let manifest: toml::Table = read(&krate.join("Cargo.toml")).parse().unwrap();
        let lists = lists_layer(&manifest);
        if !lists {
            let name = manifest["package"]["name"].as_str().unwrap().to_string();
            found.uncovered.push(name);
            continue;
        }
        for file in rust_files(&krate.join("src")) {
            for (n, line) in read(&file).lines().enumerate() {
                if CALLS.iter().any(|call| line.contains(call)) {
                    let file = file.strip_prefix(workspace).unwrap().display();
                    found.violations.push(format!("{file}:{}", n + 1));
                }
            }
        }
    }
    found
}

/// Whether any dependency table, a target-specific one included, lists `clm-git`, by its
/// own name or renamed through `package`.
fn lists_layer(manifest: &toml::Table) -> bool {
    let mut all = dependency_tables(manifest);
    if let Some(targets) = manifest.get("target").and_then(|t| t.as_table()) {
        for target in targets.values().filter_map(|t| t.as_table()) {
            all.extend(dependency_tables(target));
        }
    }
    all.iter().flat_map(|deps| deps.iter()).any(|(name, dep)| {
        name == "clm-git" || dep.get("package").and_then(|p| p.as_str()) == Some("clm-git")
    })
}

fn dependency_tables(table: &toml::Table) -> Vec<&toml::Table> {
    ["dependencies", "dev-dependencies", "build-dependencies"]
        .into_iter()
        .filter_map(|k| table.get(k)?.as_table())
        .collect()
}

/// What the scan test prints: each crate it didn't scan.
fn report(found: &Scan) -> String {
    found
        .uncovered
        .iter()
        .map(|name| format!("not yet covered: {name}\n"))
        .collect()
}

const CALLS: [&str; 2] = ["Command::new(\"git\")", "Command::new(\"gh\")"];

/// The member directories named by the workspace's `members`, a trailing `/*` expanded.
fn members(workspace: &Path) -> Vec<PathBuf> {
    let manifest: toml::Table = read(&workspace.join("Cargo.toml")).parse().unwrap();
    let mut out = Vec::new();
    for member in manifest["workspace"]["members"].as_array().unwrap() {
        let member = member.as_str().unwrap();
        match member.strip_suffix("/*") {
            Some(parent) => out.extend(
                sorted(&workspace.join(parent))
                    .into_iter()
                    .filter(|d| d.join("Cargo.toml").is_file()),
            ),
            None => out.push(workspace.join(member)),
        }
    }
    out
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

fn fixture(crates: &[(&str, bool, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write(
        &dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/*\"]\n",
    );
    for (name, lists, source) in crates {
        let deps = if *lists {
            "clm-git.workspace = true\n"
        } else {
            ""
        };
        let krate = dir.path().join("crates").join(name);
        write(
            &krate.join("Cargo.toml"),
            &format!("[package]\nname = \"{name}\"\n\n[dependencies]\n{deps}"),
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
const DECLARES_MOD: &str = "mod m;\n";

#[test]
fn a_wrapper_comes_back() {
    // req:vcs/one-git-layer@149b178a
    let ws = fixture(&[("a", true, RUNS_GIT)]);
    write(
        &ws.path().join("crates/a/src/m/n.rs"),
        "\n\nlet _ = Command::new(\"gh\");\n",
    );
    let found = scan(ws.path());
    assert_eq!(
        found.violations,
        ["crates/a/src/lib.rs:2", "crates/a/src/m/n.rs:3"]
    );
}

#[test]
fn a_crate_that_has_not_migrated() {
    // req:vcs/one-git-layer@149b178a
    let ws = fixture(&[("a", false, RUNS_GIT), ("b", true, DECLARES_MOD)]);
    let found = scan(ws.path());
    assert!(found.violations.is_empty(), "{found:?}");
    assert_eq!(found.uncovered, ["a"]);
    assert_eq!(report(&found), "not yet covered: a\n");
}

#[test]
fn clean_dependants() {
    // req:vcs/one-git-layer@149b178a
    let ws = fixture(&[("a", true, "fn f() {}\n"), ("b", true, DECLARES_MOD)]);
    let found = scan(ws.path());
    assert!(found.violations.is_empty(), "{found:?}");
    assert!(found.uncovered.is_empty(), "{found:?}");
}

#[test]
fn the_workspace_runs_git_only_through_the_layer() {
    // req:vcs/one-git-layer@149b178a
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let found = scan(&workspace);
    print!("{}", report(&found));
    assert!(
        found.violations.is_empty(),
        "run git and gh through clm-git:\n{}",
        found.violations.join("\n")
    );
    // `clm-git` never lists itself, so a scan that misses it read no crates at all.
    assert!(found.uncovered.iter().any(|c| c == "clm-git"), "{found:?}");
}

#[test]
fn a_renamed_or_target_specific_dependency_is_scanned() {
    // req:vcs/one-git-layer@149b178a
    let ws = fixture(&[("a", false, RUNS_GIT), ("b", false, RUNS_GIT)]);
    write(
        &ws.path().join("crates/a/Cargo.toml"),
        "[package]\nname = \"a\"\n\n[dependencies]\ngit = { package = \"clm-git\", path = \"x\" }\n",
    );
    write(
        &ws.path().join("crates/b/Cargo.toml"),
        "[package]\nname = \"b\"\n\n[target.'cfg(unix)'.dependencies]\nclm-git.workspace = true\n",
    );
    let found = scan(ws.path());
    assert_eq!(
        found.violations,
        ["crates/a/src/lib.rs:2", "crates/b/src/lib.rs:2"]
    );
}
