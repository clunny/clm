use std::fs;
use std::path::Path;

use clm_model::Graph;
use tempfile::TempDir;

const REQ: &str = concat!("req", ":");
const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// `s` as a Rust file whose markers count as a test's: those on its third line on are inside
/// a test, the earlier ones directly above it. Every line of `s` keeps its line number.
fn in_test(s: &str) -> String {
    let (head, body) = match s.strip_prefix('\n') {
        Some(rest) => ("", rest),
        None => s.split_once("\n\n").map_or((s, ""), |(a, b)| (a, b)),
    };
    let nl = if body.ends_with('\n') || body.is_empty() {
        ""
    } else {
        "\n"
    };
    if s.starts_with('\n') {
        format!("#[test]\nfn t() {{{body}{nl}}}\n")
    } else if body.is_empty() {
        format!("{s}#[test]\nfn t() {{}}\n")
    } else {
        format!("{head}\n#[test]\nfn t() {{{body}{nl}}}\n")
    }
}

fn spec(slugs: &[&str]) -> String {
    let mut s = String::from("---\ntitle: A\nstatus: active\nowners: []\n---\n\n## Requirements\n");
    for slug in slugs {
        s.push_str(&format!(
            "\n### Requirement: {slug}\n{slug}.\n\n#### Scenario: s\n- ok\n"
        ));
    }
    s
}

/// Specs with `a/x`, `a/y` and `a/z`, and an `applied` change `r` that removed `a/gone`.
fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &spec(&["x", "y", "z"]));
    write(d, "changes/r/change.md", "---\nstatus: applied\n---\n");
    write(
        d,
        "changes/r/deltas/a.md",
        "---\ncomponent: a\n---\n## REMOVED Requirements\n\n### Requirement: gone <!-- base: 0123456789abcdef -->\n",
    );
    dir
}

fn graph(root: &Path) -> Graph {
    let (graph, violations) = clm_model::load(root);
    assert!(violations.is_empty(), "{violations:?}");
    graph
}

fn lines(root: &Path, node: Option<&str>) -> Vec<String> {
    clm_check::pending_in(root, node)
        .unwrap()
        .iter()
        .map(|p| p.to_string())
        .collect()
}

/// `repo()`, with node `g` gone: change `r` removes it and its requirement `g/x`.
fn removed_node_repo() -> TempDir {
    let dir = repo();
    write(
        dir.path(),
        "changes/r/deltas/g.md",
        "---\ncomponent: g\nremove: true\n---\n## REMOVED Requirements\n\n### Requirement: x <!-- base: 0123456789abcdef -->\n",
    );
    dir
}

/// A node `id` (title `T`) whose frontmatter has `paths`, with no requirements.
fn node_with_paths(root: &Path, id: &str, paths: &str) {
    write(
        root,
        &format!("specs/{}/SPEC.md", id.replace('.', "/")),
        &format!("---\ntitle: T\nstatus: active\nowners: []\npaths: {paths}\n---\n"),
    );
}

#[test]
fn a_removed_node_is_an_unknown_node() {
    // req:check/pending-narrows-on-a-graph@1c3731b9
    let dir = removed_node_repo();
    let d = dir.path();
    write(d, "t.rs", &in_test(&format!("// {REQ}g/x\n")));
    let removed = clm_check::pending_in(d, Some("g")).unwrap_err();
    assert_eq!(removed[0].rule, "check.coverage/pending-report");
    assert_eq!(removed[0].message, "unknown component `g`");
    assert_eq!(removed[0].path, Path::new("specs/g/SPEC.md"));
    let on_graph = clm_check::pending_for(&graph(d), &clm_check::scan(d).req, Some("g"));
    assert_eq!(
        clm_check::format(&on_graph.unwrap_err()[0]),
        clm_check::format(&removed[0])
    );
    let unknown = clm_check::pending_in(d, Some("nope")).unwrap_err();
    assert_eq!(unknown[0].rule, "check.coverage/pending-report");
    assert_eq!(lines(d, Some("clm")), ["g/x obsolete t.rs:1"]);
}

/// Owners agree with git: for each pattern, a node with that one pattern owns a removed
/// node's marker in exactly the files `git ls-files ':(glob)<pattern>'` lists.
#[test]
fn paths_match_as_git_glob_pathspecs_do() {
    // req:check/path-ownership@5c51292c
    let files = [
        "crates/v/t.rs",
        "crates/v/tests/t.rs",
        "crates/vv/t.rs",
        "crates/t.rs",
        "t.rs",
        "a/b.rs",
        "a/x/y/b.rs",
        "ab.rs",
        "a]b.rs",
        "abc.rs",
        "x/a.rs",
        "x/7.rs",
        "a[bc.rs",
        "docs/[draft]/t.rs",
        "a*b.rs",
    ];
    let patterns = [
        "crates/v/**",
        "crates/*",
        "crates/v",
        "crates/v/",
        "crates/v*",
        "**/t.rs",
        "a/**/b.rs",
        "a**.rs",
        "*.rs",
        "a?c.rs",
        "a[]x]b.rs",
        "[!c]*.rs",
        "[^c]*.rs",
        "**",
        "x/[a-c].rs",
        "x/[[:alpha:]].rs",
        "x/[[:digit:]].rs",
        "a[b*",
        "docs/[draft]",
        "a\\*b.rs",
    ];
    for pattern in patterns {
        let dir = removed_node_repo();
        let d = dir.path();
        node_with_paths(d, "v", &format!("[{pattern:?}]"));
        for f in files {
            write(d, f, &in_test(&format!("// {REQ}g/x\n")));
        }
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .args(args)
                .current_dir(d)
                .output()
                .unwrap();
            assert!(out.status.success(), "git {args:?}");
            String::from_utf8(out.stdout).unwrap()
        };
        git(&["init", "-q"]);
        git(&["add", "-A"]);
        let mut expected: Vec<String> = git(&["ls-files", &format!(":(glob){pattern}")])
            .lines()
            .filter(|f| files.contains(f))
            .map(|f| format!("{f}:1"))
            .collect();
        expected.sort();
        let mut owned: Vec<String> = clm_check::pending_in(d, Some("v"))
            .unwrap()
            .iter()
            .flat_map(|p| p.markers.iter().map(|m| m.to_string()))
            .collect();
        owned.sort();
        assert_eq!(owned, expected, "pattern {pattern}");
    }
}

#[test]
fn pending_for_narrows_a_loaded_graph_as_pending_in_does() {
    // req:check/pending-narrows-on-a-graph@1c3731b9
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &spec(&["x"]));
    write(d, "specs/b/SPEC.md", &spec(&["y"]));
    let g = graph(d);
    let markers = clm_check::scan(d).req;
    let from_root = clm_check::pending_in(d, Some("a")).unwrap();
    let from_graph = clm_check::pending_for(&g, &markers, Some("a")).unwrap();
    let ids = |l: &[clm_check::Pending]| l.iter().map(|p| p.to_string()).collect::<Vec<_>>();
    assert_eq!(ids(&from_root), ids(&from_graph));
    assert_eq!(
        clm_check::pending_for(&g, &markers, None).unwrap().len(),
        2,
        "no component leaves the list whole"
    );
    assert_eq!(from_graph.len(), 1);
    assert_eq!(from_graph[0].id, "a/x");
    let first = clm_check::pending_in(d, Some("nope")).unwrap_err();
    let second = clm_check::pending_for(&g, &markers, Some("nope")).unwrap_err();
    assert_eq!(clm_check::format(&first[0]), clm_check::format(&second[0]));
    assert_eq!(second[0].rule, "check.coverage/pending-report");
    assert_eq!(second[0].message, "unknown component `nope`");
    assert_eq!(second[0].path, Path::new("specs/nope/SPEC.md"));
    let dotted = clm_check::pending_for(&g, &markers, Some("a.nope")).unwrap_err();
    assert_eq!(dotted[0].path, Path::new("specs/a/nope/SPEC.md"));
}
