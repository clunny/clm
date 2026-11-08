use std::fs;
use std::path::Path;

use clm_check::{State, pending};
use clm_model::{Graph, Priority};
use tempfile::TempDir;

const REQ: &str = concat!("req", ":");
const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
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

fn version(g: &Graph, id: &str) -> String {
    g.requirement(id).unwrap().version().to_string()
}

fn lines(root: &Path, node: Option<&str>) -> Vec<String> {
    clm_check::pending_in(root, node)
        .unwrap()
        .iter()
        .map(|p| p.to_string())
        .collect()
}

#[test]
fn scenario_states() {
    // req:check.coverage/pending-report@89a3068a
    let dir = repo();
    let d = dir.path();
    let g = graph(d);
    let (x, y, z) = (version(&g, "a/x"), version(&g, "a/y"), version(&g, "a/z"));
    write(
        d,
        "t.rs",
        &format!("// {REQ}a/x@11111111\n// {REQ}a/z@{z}\n\n\n\n\n\n\n// {REQ}a/gone\n"),
    );
    assert_eq!(
        lines(d, Some("a")),
        [
            "a/gone obsolete t.rs:9".to_string(),
            format!("a/x {x} behind 11111111"),
            format!("a/y {y} unbuilt"),
        ]
    );
}

#[test]
fn scenario_stale_marker_in_another_components_files() {
    // req:check.coverage/pending-report@89a3068a
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(
        d,
        "specs/SPEC.md",
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    );
    for (n, p) in [("a", "crates/a/**"), ("b", "crates/b/**")] {
        let body = if n == "a" { spec(&["x"]) } else { spec(&["q"]) };
        write(
            d,
            &format!("specs/{n}/SPEC.md"),
            &body.replacen("owners: []", &format!("owners: []\npaths: [\"{p}\"]"), 1),
        );
    }
    let g = graph(d);
    let x = version(&g, "a/x");
    let q = version(&g, "b/q");
    write(d, "crates/a/t.rs", &format!("\n// {REQ}a/x@{x}\n"));
    write(
        d,
        "crates/b/t.rs",
        &format!("\n\n\n\n\n\n// {REQ}a/x@11111111\n// {REQ}b/q@{q}\n"),
    );
    assert_eq!(lines(d, Some("a")), [format!("a/x {x} behind 11111111")]);
    assert_eq!(lines(d, Some("b")), Vec::<String>::new());
}

#[test]
fn behind_lists_each_distinct_cited_version_and_none() {
    // req:check.coverage/pending-report@89a3068a
    let dir = repo();
    let d = dir.path();
    let g = graph(d);
    let x = version(&g, "a/x");
    write(
        d,
        "t.rs",
        &format!("// {REQ}a/x@22222222\n// {REQ}a/x\n// {REQ}a/x@11111111\n// {REQ}a/x@22222222\n"),
    );
    let found = lines(d, Some("a"));
    assert_eq!(found[0], format!("a/x {x} behind 11111111,22222222,none"));
}

#[test]
fn scenario_blocked() {
    // req:check.coverage/pending-report@89a3068a
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [a/y]\n---\n",
    );
    write(
        d,
        "backlog/h.md",
        "---\ntitle: H\nkind: human\nblocks: [a/y, a/x]\n---\n",
    );
    let found = lines(d, Some("a"));
    assert!(
        found[0].starts_with("a/x ") && found[0].ends_with(" unbuilt blocked backlog/h"),
        "{found:?}"
    );
    assert!(
        found[1].ends_with(" unbuilt blocked backlog/h,backlog/q"),
        "{found:?}"
    );
    assert!(found[2].ends_with(" unbuilt"), "{found:?}");
    let items = clm_check::pending_in(d, Some("a")).unwrap();
    assert_eq!(items[1].blocked_by, ["h", "q"]);
}

/// `repo()`, with `a/y` needing `b/z` and `b/w`, and node `b` holding `z` and `w`.
fn needs_repo() -> TempDir {
    let dir = repo();
    let d = dir.path();
    let a = spec(&["x", "y", "z"]).replace("y.\n", "y.\n\n#### Needs\n- b/z\n- b/w\n");
    write(d, "specs/a/SPEC.md", &a);
    write(
        d,
        "specs/b/SPEC.md",
        &spec(&["z", "w"]).replace("title: A", "title: B"),
    );
    dir
}

fn line_for(root: &Path, id: &str) -> String {
    let node = id.split_once('/').unwrap().0;
    lines(root, Some(node))
        .into_iter()
        .find(|l| l.starts_with(&format!("{id} ")))
        .unwrap()
}

#[test]
fn scenario_blocked_by_a_need() {
    // req:check.coverage/pending-report@89a3068a
    let dir = needs_repo();
    let d = dir.path();
    write(
        d,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [a/y]\n---\n",
    );
    let y = line_for(d, "a/y");
    assert!(y.ends_with(" unbuilt blocked backlog/q,b/w,b/z"), "{y}");
    let items = clm_check::pending_in(d, Some("a")).unwrap();
    let p = items.iter().find(|p| p.id == "a/y").unwrap();
    assert_eq!(p.blocked_by, ["q"]);
    assert_eq!(p.waits_on, ["b/w", "b/z"]);
}

#[test]
fn scenario_need_satisfied() {
    // req:check.coverage/pending-report@89a3068a
    let dir = needs_repo();
    let d = dir.path();
    let g = graph(d);
    let (y, z, w) = (version(&g, "a/y"), version(&g, "b/z"), version(&g, "b/w"));
    write(d, "t.rs", &format!("// {REQ}b/z@{z}\n// {REQ}b/w@{w}\n"));
    assert_eq!(line_for(d, "a/y"), format!("a/y {y} unbuilt"));
}

#[test]
fn a_behind_need_still_blocks() {
    // req:check.coverage/pending-report@89a3068a
    let dir = needs_repo();
    let d = dir.path();
    let g = graph(d);
    let w = version(&g, "b/w");
    write(
        d,
        "t.rs",
        &format!("// {REQ}b/z@11111111\n// {REQ}b/w@{w}\n"),
    );
    assert!(line_for(d, "a/y").ends_with(" unbuilt blocked b/z"));
}

#[test]
fn a_behind_requirement_names_a_repeated_need_once() {
    // req:check.coverage/pending-report@89a3068a
    let dir = needs_repo();
    let d = dir.path();
    let a = fs::read_to_string(d.join("specs/a/SPEC.md")).unwrap();
    write(d, "specs/a/SPEC.md", &a.replace("- b/w\n", "- b/z\n"));
    write(
        d,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [a/y]\n---\n",
    );
    write(
        d,
        "backlog/h.md",
        "---\ntitle: H\nkind: human\nblocks: [a/y]\n---\n",
    );
    write(d, "t.rs", &format!("// {REQ}a/y@11111111\n"));
    assert!(
        line_for(d, "a/y").ends_with(" behind 11111111 blocked backlog/h,backlog/q,b/z"),
        "{}",
        line_for(d, "a/y")
    );
}

#[test]
fn scenario_nothing_pending() {
    // req:check.coverage/pending-report@89a3068a
    let dir = repo();
    let d = dir.path();
    let g = graph(d);
    let cite = |id: &str| format!("// {REQ}{id}@{}\n", version(&g, id));
    write(
        d,
        "t.rs",
        &format!("{}{}{}", cite("a/x"), cite("a/y"), cite("a/z")),
    );
    assert_eq!(lines(d, Some("a")), Vec::<String>::new());
}

#[test]
fn every_node_sorted_by_node_then_requirement() {
    // req:check.coverage/pending-report@89a3068a
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/b/SPEC.md",
        &spec(&["q"]).replace("title: A", "title: B"),
    );
    write(
        d,
        "specs/a/b/SPEC.md",
        &spec(&["p"]).replace("title: A", "title: AB"),
    );
    let found: Vec<String> = clm_check::pending_in(d, None)
        .unwrap()
        .iter()
        .map(|p| p.id.clone())
        .collect();
    assert_eq!(found, ["a/x", "a/y", "a/z", "a.b/p", "b/q"]);
    assert!(lines(d, Some("b"))[0].starts_with("b/q "));
}

#[test]
fn priority_is_the_highest_of_the_live_changes_touching_it() {
    // req:check.coverage/pending-report@89a3068a
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "changes/m/change.md",
        "---\nstatus: applied\npriority: P2\n---\n",
    );
    write(
        d,
        "changes/m/deltas/a.md",
        "---\ncomponent: a\n---\n## MODIFIED Requirements\n### Requirement: x <!-- base: 0000000000000000 -->\nx.\n\n#### Scenario: s\n- ok\n",
    );
    write(
        d,
        "changes/archive/n/change.md",
        "---\nstatus: done\npriority: P1\n---\n",
    );
    write(
        d,
        "changes/archive/n/deltas/a.md",
        "---\ncomponent: a\n---\n## ADDED Requirements\n### Requirement: y\ny.\n\n#### Scenario: s\n- ok\n",
    );
    write(
        d,
        "changes/r/change.md",
        "---\nstatus: applied\npriority: P3\n---\n",
    );
    let g = graph(d);
    let gone = clm_check::Marker {
        id: "a/gone".into(),
        version: None,
        path: "t.rs".into(),
        line: 1,
    };
    let items = pending(&g, &[gone]);
    let by_id = |id: &str| items.iter().find(|p| p.id == id).unwrap();
    assert_eq!(by_id("a/x").priority, Priority::P2);
    assert_eq!(by_id("a/y").priority, Priority::P5);
    assert_eq!(by_id("a/gone").priority, Priority::P3);
    assert_eq!(by_id("a/gone").state, State::Obsolete);
    assert_eq!(by_id("a/x").state, State::Unbuilt);
    assert_eq!(by_id("a/x").node, "a");
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
fn scenario_removed_node() {
    // req:check.coverage/pending-report@89a3068a
    let dir = removed_node_repo();
    let d = dir.path();
    node_with_paths(d, "v", r#"["crates/v/**"]"#);
    write(d, "crates/v/tests/t.rs", &format!("\n\n\n// {REQ}g/x\n"));
    assert_eq!(lines(d, Some("v")), ["g/x obsolete crates/v/tests/t.rs:4"]);
    let all = clm_check::pending_in(d, None).unwrap();
    let gx: Vec<_> = all.iter().filter(|p| p.id == "g/x").collect();
    assert_eq!(gx.len(), 1);
    assert_eq!(gx[0].node, "v");
    assert_eq!(gx[0].state, State::Obsolete);
}

#[test]
fn scenario_removed_node_files_of_two_nodes() {
    // req:check.coverage/pending-report@89a3068a
    let dir = removed_node_repo();
    let d = dir.path();
    node_with_paths(d, "v", r#"["crates/v/**"]"#);
    node_with_paths(d, "w", r#"["crates/w/**"]"#);
    write(d, "crates/v/t.rs", &format!("\n\n\n// {REQ}g/x\n"));
    write(d, "crates/w/t.rs", &format!("\n\n\n\n\n\n// {REQ}g/x\n"));
    let gx: Vec<(String, String)> = clm_check::pending_in(d, None)
        .unwrap()
        .iter()
        .filter(|p| p.id == "g/x")
        .map(|p| (p.node.clone(), p.to_string()))
        .collect();
    assert_eq!(
        gx,
        [
            ("v".to_string(), "g/x obsolete crates/v/t.rs:4".to_string()),
            ("w".to_string(), "g/x obsolete crates/w/t.rs:7".to_string()),
        ]
    );
}

#[test]
fn scenario_removed_node_unowned_file() {
    // req:check.coverage/pending-report@89a3068a
    let dir = removed_node_repo();
    let d = dir.path();
    node_with_paths(d, "v", r#"["crates/v/**"]"#);
    write(d, "old/t.rs", &format!("\n\n\n// {REQ}g/x\n"));
    assert_eq!(lines(d, Some("clm")), ["g/x obsolete old/t.rs:4"]);
    assert_eq!(lines(d, Some("v")), Vec::<String>::new());
}

#[test]
fn the_deepest_owner_then_the_first_by_id_takes_a_removed_nodes_marker() {
    // req:check.coverage/pending-report@89a3068a
    let dir = removed_node_repo();
    let d = dir.path();
    node_with_paths(d, "v", r#"["crates/v/**"]"#);
    node_with_paths(d, "v.inner", r#"["crates/v/inner/**"]"#);
    node_with_paths(d, "w", r#"["crates/**"]"#);
    node_with_paths(d, "u", r#"["crates/v/**"]"#);
    write(d, "crates/v/inner/t.rs", &format!("// {REQ}g/x\n"));
    write(d, "crates/v/t.rs", &format!("// {REQ}g/x\n"));
    let owners: Vec<(String, String)> = clm_check::pending_in(d, None)
        .unwrap()
        .iter()
        .filter(|p| p.id == "g/x")
        .map(|p| (p.node.clone(), p.to_string()))
        .collect();
    assert_eq!(
        owners,
        [
            ("u".to_string(), "g/x obsolete crates/v/t.rs:1".to_string()),
            (
                "v.inner".to_string(),
                "g/x obsolete crates/v/inner/t.rs:1".to_string()
            ),
        ]
    );
}

#[test]
fn an_obsolete_marker_whose_node_exists_stays_on_that_node() {
    // req:check.coverage/pending-report@89a3068a
    let dir = repo();
    let d = dir.path();
    node_with_paths(d, "v", r#"["crates/v/**"]"#);
    write(d, "crates/v/t.rs", &format!("// {REQ}a/gone\n"));
    assert_eq!(lines(d, Some("a")), {
        let g = graph(d);
        [
            "a/gone obsolete crates/v/t.rs:1".to_string(),
            format!("a/x {} unbuilt", version(&g, "a/x")),
            format!("a/y {} unbuilt", version(&g, "a/y")),
            format!("a/z {} unbuilt", version(&g, "a/z")),
        ]
    });
    assert_eq!(lines(d, Some("v")), Vec::<String>::new());
}

#[test]
fn a_node_gone_without_a_removal_delta_moves_too() {
    // req:check.coverage/pending-report@89a3068a
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "changes/r/deltas/g.md",
        "---\ncomponent: g\n---\n## REMOVED Requirements\n\n### Requirement: x <!-- base: 0123456789abcdef -->\n",
    );
    node_with_paths(d, "v", r#"["crates/v/**"]"#);
    write(d, "crates/v/t.rs", &format!("// {REQ}g/x\n"));
    assert_eq!(lines(d, Some("v")), ["g/x obsolete crates/v/t.rs:1"]);
}

#[test]
fn a_top_level_node_is_deeper_than_the_root() {
    // req:check.coverage/pending-report@89a3068a
    let dir = removed_node_repo();
    let d = dir.path();
    write(
        d,
        "specs/SPEC.md",
        "---\ntitle: R\nstatus: active\nowners: []\npaths: [\"crates/**\"]\n---\n",
    );
    node_with_paths(d, "v", r#"["crates/v/**"]"#);
    write(d, "crates/v/t.rs", &format!("// {REQ}g/x\n"));
    write(d, "crates/w/t.rs", &format!("// {REQ}g/x\n"));
    assert_eq!(lines(d, Some("v")), ["g/x obsolete crates/v/t.rs:1"]);
    assert_eq!(lines(d, Some("clm")), ["g/x obsolete crates/w/t.rs:1"]);
}

#[test]
fn moved_lines_keep_the_removing_changes_priority() {
    // req:check.coverage/pending-report@89a3068a
    let dir = removed_node_repo();
    let d = dir.path();
    write(
        d,
        "changes/r/change.md",
        "---\nstatus: applied\npriority: P2\n---\n",
    );
    node_with_paths(d, "v", r#"["crates/v/**"]"#);
    node_with_paths(d, "w", r#"["crates/w/**"]"#);
    let g = graph(d);
    let marker = |path: &str| clm_check::Marker {
        id: "g/x".into(),
        version: None,
        path: path.into(),
        line: 1,
    };
    let items = pending(&g, &[marker("crates/v/t.rs"), marker("crates/w/t.rs")]);
    let gx: Vec<_> = items.iter().filter(|p| p.id == "g/x").collect();
    assert_eq!(gx.len(), 2);
    assert!(gx.iter().all(|p| p.priority == Priority::P2), "{gx:?}");
    assert_eq!([gx[0].node.as_str(), gx[1].node.as_str()], ["v", "w"]);
}

#[test]
fn a_removed_node_is_an_unknown_node() {
    // req:check.coverage/pending-report@89a3068a
    let dir = removed_node_repo();
    let d = dir.path();
    write(d, "t.rs", &format!("// {REQ}g/x\n"));
    assert!(clm_check::pending_in(d, Some("g")).is_err());
    let unknown = clm_check::pending_in(d, Some("nope")).unwrap_err();
    assert_eq!(unknown[0].rule, "check.coverage/pending-report");
    assert_eq!(lines(d, Some("clm")), ["g/x obsolete t.rs:1"]);
}

/// Owners agree with git: for each pattern, a node with that one pattern owns a removed
/// node's marker in exactly the files `git ls-files ':(glob)<pattern>'` lists.
#[test]
fn paths_match_as_git_glob_pathspecs_do() {
    // req:check.coverage/pending-report@89a3068a
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
            write(d, f, &format!("// {REQ}g/x\n"));
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
    // req:check/pending-narrows-on-a-graph@1c2fb00a
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
    assert_eq!(first[0].message, second[0].message);
    assert_eq!(first[0].path, second[0].path);
    assert!(first[0].message.contains("nope"));
}
