//! Imports between nodes that `depends_on` doesn't declare (`undeclared-imports`).

use std::fs;
use std::path::Path;

use tempfile::TempDir;

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn node(paths: &str, depends_on: &str) -> String {
    format!(
        "---\ntitle: N\nstatus: active\nowners: []\ndepends_on: {depends_on}\npaths: {paths}\n---\n## Contract\n"
    )
}

/// The graph of the root, `a` (`crates/a/**`, depending on `a_deps`), `b` (`crates/b/**`)
/// and `b`'s child `b.c` (`crates/c/**`).
fn graph(a_deps: &str) -> clm_model::Graph {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "specs/SPEC.md", &node("[]", "[]"));
    write(root, "specs/a/SPEC.md", &node("[\"crates/a/**\"]", a_deps));
    write(root, "specs/b/SPEC.md", &node("[\"crates/b/**\"]", "[]"));
    write(root, "specs/b/c/SPEC.md", &node("[\"crates/c/**\"]", "[]"));
    let (graph, violations) = clm_model::load(root);
    assert!(violations.is_empty(), "{violations:?}");
    graph
}

fn edges(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

fn lines(graph: &clm_model::Graph, list: &[(&str, &str)]) -> Vec<String> {
    clm_check::undeclared_imports(graph, &edges(list))
        .iter()
        .map(clm_check::format)
        .collect()
}

#[test]
fn undeclared_dependency() {
    // req:check/undeclared-imports@1e2f49ba
    let graph = graph("[]");
    assert_eq!(
        lines(&graph, &[("crates/a/Cargo.toml", "crates/b/Cargo.toml")]),
        [
            "crates/a/Cargo.toml: check/undeclared-imports: imports crates/b/Cargo.toml of component `b`, and `a` doesn't depend on `b`"
        ]
    );
}

#[test]
fn declared() {
    // req:check/undeclared-imports@1e2f49ba
    let graph = graph("[b]");
    assert_eq!(
        lines(&graph, &[("crates/a/Cargo.toml", "crates/b/Cargo.toml")]),
        Vec::<String>::new()
    );
}

#[test]
fn unowned_ends_and_imports_within_a_node() {
    // req:check/undeclared-imports@1e2f49ba
    let graph = graph("[]");
    let list = [
        ("crates/a/src/lib.rs", "tools/x.rs"),
        ("tools/x.rs", "crates/b/src/lib.rs"),
        ("crates/a/src/lib.rs", "crates/a/src/x.rs"),
    ];
    assert_eq!(lines(&graph, &list), Vec::<String>::new());
}

#[test]
fn a_parent_gets_no_exemption() {
    // req:check/undeclared-imports@1e2f49ba
    let graph = graph("[b]");
    // `a` depends on `b`, not on its child `b.c`; `b` doesn't depend on its child either.
    assert_eq!(
        lines(
            &graph,
            &[
                ("crates/a/src/lib.rs", "crates/c/src/lib.rs"),
                ("crates/b/src/lib.rs", "crates/c/src/lib.rs"),
            ]
        ),
        [
            "crates/a/src/lib.rs: check/undeclared-imports: imports crates/c/src/lib.rs of component `b.c`, and `a` doesn't depend on `b.c`",
            "crates/b/src/lib.rs: check/undeclared-imports: imports crates/c/src/lib.rs of component `b.c`, and `b` doesn't depend on `b.c`",
        ]
    );
}

#[test]
fn not_in_run() {
    // req:check/undeclared-imports@1e2f49ba
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "specs/SPEC.md", &node("[]", "[]"));
    write(root, "specs/a/SPEC.md", &node("[\"crates/a/**\"]", "[]"));
    write(root, "specs/b/SPEC.md", &node("[\"crates/b/**\"]", "[]"));
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*\"]\n",
    );
    write(
        root,
        "crates/a/Cargo.toml",
        "[package]\nname = \"a\"\n\n[dependencies]\nb = { path = \"../b\" }\n",
    );
    write(root, "crates/b/Cargo.toml", "[package]\nname = \"b\"\n");
    let status = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
    let status = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
    let found: Vec<_> = clm_check::run(root)
        .into_iter()
        .filter(|v| v.rule == "check/undeclared-imports")
        .collect();
    assert!(found.is_empty(), "{found:?}");
}

/// A repo of `graph("[b]")`'s nodes where `a` depends on `b`, and a `proposed` change
/// `20260101-y` whose delta for `a` sets `depends_on: []` (and, with `contract`, modifies its
/// Contract), written with `fm_base` as its frontmatter base.
fn pending(a_deps: &str, contract: bool, fm_base: Option<&str>) -> (TempDir, clm_model::Graph) {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "specs/SPEC.md", &node("[]", "[]"));
    write(root, "specs/a/SPEC.md", &node("[\"crates/a/**\"]", a_deps));
    write(root, "specs/b/SPEC.md", &node("[\"crates/b/**\"]", "[]"));
    let (graph, violations) = clm_model::load(root);
    assert!(violations.is_empty(), "{violations:?}");
    let hashes = clm_apply::hashes(&graph.nodes["a"]);
    let hash = |e: &str| hashes.iter().find(|(n, _)| n == e).unwrap().1.clone();
    let fm = fm_base.map_or_else(|| hash("frontmatter"), str::to_string);
    let mut base = format!("frontmatter: \"{fm}\"");
    let mut body = "## MODIFIED Frontmatter\n```yaml\ndepends_on: []\n```\n".to_string();
    if contract {
        base.push_str(&format!(", contract: \"{}\"", hash("contract")));
        body.push_str("\n## MODIFIED Contract\nNew.\n");
    }
    write(
        root,
        "changes/20260101-y/change.md",
        "---\nstatus: proposed\n---\n",
    );
    write(
        root,
        "changes/20260101-y/deltas/a.md",
        &format!("---\ncomponent: a\nbase: {{ {base} }}\n---\n{body}"),
    );
    let (graph, _) = clm_model::load(root);
    (dir, graph)
}

const A_TO_B: [(&str, &str); 1] = [("crates/a/Cargo.toml", "crates/b/Cargo.toml")];

/// `app` (`app/main.py`, depending on `feat`) and `feat` (`app/feat.py`, tests
/// `tests/feat/**`, `test_depends_on: [app]`).
fn app_and_feat() -> clm_model::Graph {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "specs/SPEC.md", &node("[]", "[]"));
    write(
        root,
        "specs/app/SPEC.md",
        &node("[\"app/main.py\"]", "[feat]"),
    );
    write(
        root,
        "specs/feat/SPEC.md",
        &format!(
            "{}\n",
            node("[\"app/feat.py\"]", "[]").replace(
                "---\n## Contract",
                "tests: [\"tests/feat/**\"]\ntest_depends_on: [app]\n---\n## Contract"
            )
        ),
    );
    let (graph, violations) = clm_model::load(root);
    assert!(violations.is_empty(), "{violations:?}");
    graph
}

#[test]
fn a_test_imports_the_entry_point() {
    // req:check/undeclared-imports@1e2f49ba
    let graph = app_and_feat();
    assert_eq!(
        lines(&graph, &[("tests/feat/test_x.py", "app/main.py")]),
        Vec::<String>::new()
    );
}

#[test]
fn a_test_among_its_components_paths_imports_the_entry_point() {
    // req:check/undeclared-imports@1e2f49ba
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "specs/SPEC.md", &node("[]", "[]"));
    write(
        root,
        "specs/app/SPEC.md",
        &node("[\"app/main.py\"]", "[feat]"),
    );
    let feat = node("[\"feat/**\"]", "[]").replace(
        "---\n## Contract",
        "tests: [\"feat/test_*.py\"]\ntest_depends_on: [app]\n---\n## Contract",
    );
    write(root, "specs/feat/SPEC.md", &feat);
    let (graph, violations) = clm_model::load(root);
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(
        lines(
            &graph,
            &[
                ("feat/test_x.py", "app/main.py"),
                ("feat/x.py", "app/main.py")
            ]
        ),
        [
            "feat/x.py: check/undeclared-imports: imports app/main.py of component `app`, and `feat` doesn't depend on `app`"
        ]
    );
}

#[test]
fn a_test_inside_another_components_paths_is_judged_as_its_owners() {
    // req:check/undeclared-imports@1e2f49ba
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "specs/SPEC.md", &node("[]", "[]"));
    write(root, "specs/app/SPEC.md", &node("[\"app/main.py\"]", "[]"));
    write(root, "specs/harness/SPEC.md", &node("[\"tests/**\"]", "[]"));
    let feat = node("[\"feat/**\"]", "[]").replace(
        "---\n## Contract",
        "tests: [\"tests/feat/**\"]\ntest_depends_on: [app]\n---\n## Contract",
    );
    write(root, "specs/feat/SPEC.md", &feat);
    let (graph, violations) = clm_model::load(root);
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(
        lines(&graph, &[("tests/feat/t.py", "app/main.py")]),
        Vec::<String>::new()
    );
    assert_eq!(
        lines(&graph, &[("tests/other.py", "app/main.py")]),
        [
            "tests/other.py: check/undeclared-imports: imports app/main.py of component `app`, and `harness` doesn't depend on `app`"
        ]
    );
}

#[test]
fn production_code_still_judged() {
    // req:check/undeclared-imports@1e2f49ba
    let graph = app_and_feat();
    assert_eq!(
        lines(&graph, &[("app/feat.py", "app/main.py")]),
        [
            "app/feat.py: check/undeclared-imports: imports app/main.py of component `app`, and `feat` doesn't depend on `app`"
        ]
    );
}

#[test]
fn not_applied_yet() {
    // req:check/undeclared-imports@1e2f49ba
    let (_dir, graph) = pending("[b]", false, None);
    assert_eq!(
        lines(&graph, &A_TO_B),
        [
            "crates/a/Cargo.toml: check/undeclared-imports: imports crates/b/Cargo.toml of component `b`, and `a` wouldn't depend on `b` after 20260101-y"
        ]
    );
}

#[test]
fn not_applied_yet_already_undeclared() {
    // req:check/undeclared-imports@1e2f49ba
    let (dir, graph) = pending("[]", true, None);
    // The delta applies, so the dedup is what keeps the second line out.
    let apply: Vec<_> = clm_check::run(dir.path())
        .into_iter()
        .filter(|v| v.rule.starts_with("apply/"))
        .collect();
    assert!(apply.is_empty(), "{apply:?}");
    assert_eq!(
        lines(&graph, &A_TO_B),
        [
            "crates/a/Cargo.toml: check/undeclared-imports: imports crates/b/Cargo.toml of component `b`, and `a` doesn't depend on `b`"
        ]
    );
}

#[test]
fn not_applied_yet_stale_delta() {
    // req:check/undeclared-imports@1e2f49ba
    let (dir, graph) = pending("[b]", false, Some("0123456789abcdef"));
    assert_eq!(lines(&graph, &A_TO_B), Vec::<String>::new());
    let stale: Vec<_> = clm_check::run(dir.path())
        .iter()
        .map(|v| v.rule)
        .filter(|r| *r == "apply/stale-base")
        .collect();
    assert_eq!(stale, ["apply/stale-base"]);
}
