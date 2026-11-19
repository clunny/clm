//! A test in one component's file whose every marker cites another component's removed
//! requirement, which no approved list names (`obsolete-tests-elsewhere`).

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

const RULE: &str = "check/obsolete-tests-elsewhere";

/// The scenarios' change, which removed `b/gone`.
const CHANGE: &str = "20260101-x";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// A marker citing `id`, written so the marker literal never appears whole in this file.
fn marker(id: &str) -> String {
    format!("{}:{id}", concat!("req", ""))
}

/// Seven lines of code, then a test `name` whose first line is line 8, citing `ids`.
fn test_file(name: &str, ids: &[&str]) -> String {
    let cites: String = ids
        .iter()
        .map(|id| format!("    // {}\n", marker(id)))
        .collect();
    format!(
        "{}#[test]\nfn {name}() {{\n{cites}}}\n",
        "use std::fs;\n".repeat(7)
    )
}

/// A component's frontmatter with `paths` and `extra` lines, and the requirements `slugs`.
fn component(paths: &str, extra: &str, slugs: &[&str]) -> String {
    let requirements: String = slugs
        .iter()
        .map(|s| {
            format!("### Requirement: {s}\nIt SHALL.\n\n#### Scenario: s\n- GIVEN g\n- WHEN w\n- THEN t\n\n")
        })
        .collect();
    format!(
        "---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\n{extra}---\n## Contract\n\n## Requirements\n\n{requirements}"
    )
}

/// The root's frontmatter: `wiring` when given.
fn root_spec(wiring: Option<&str>) -> String {
    let extra = wiring.map(|w| format!("wiring: {w}\n")).unwrap_or_default();
    component("[]", &extra, &["r"])
}

/// An `applied` change `CHANGE` whose deltas remove `<component>/<slug>` for each of
/// `removes` and add `<component>/<slug>` for each of `adds`.
fn change(root: &Path, removes: &[(&str, &str)], adds: &[(&str, &str)]) {
    write(
        root,
        &format!("changes/{CHANGE}/change.md"),
        "---\nstatus: applied\n---\n",
    );
    let mut deltas: std::collections::BTreeMap<&str, String> = Default::default();
    for (c, slug) in removes {
        deltas.entry(c).or_default().push_str(&format!(
            "## REMOVED Requirements\n\n### Requirement: {slug} <!-- base: 0123456789abcdef -->\n\n"
        ));
    }
    for (c, slug) in adds {
        deltas.entry(c).or_default().push_str(&format!(
            "## ADDED Requirements\n\n### Requirement: {slug}\nIt SHALL.\n\n#### Scenario: s\n- GIVEN g\n- WHEN w\n- THEN t\n\n"
        ));
    }
    for (c, body) in deltas {
        write(
            root,
            &format!("changes/{CHANGE}/deltas/{c}.md"),
            &format!("---\ncomponent: {c}\n---\n{body}"),
        );
    }
}

/// The first scenario's repository: a root with `wiring`, components `b` and `c`, the
/// change removing `b/gone`, and `files`.
fn repo(wiring: Option<&str>, files: &[(&str, String)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "specs/SPEC.md", &root_spec(wiring));
    write(
        root,
        "specs/b/SPEC.md",
        &component("[\"crates/b/**\"]", "", &["y"]),
    );
    write(
        root,
        "specs/c/SPEC.md",
        &component("[\"crates/c/**\"]", "", &["y"]),
    );
    change(root, &[("b", "gone")], &[]);
    for (path, text) in files {
        write(root, path, text);
    }
    git(root);
    dir
}

fn git(root: &Path) {
    for args in [&["init", "-q"][..], &["add", "."]] {
        let status = Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .unwrap();
        assert!(status.success());
    }
}

fn found(root: &Path) -> Vec<String> {
    clm_check::run(root)
        .iter()
        .filter(|v| v.rule == RULE)
        .map(clm_check::format)
        .collect()
}

fn unlisted() -> TempDir {
    repo(
        Some("[]"),
        &[("crates/c/t.rs", test_file("old_rule", &["b/gone"]))],
    )
}

const UNLISTED_LINE: &str = "crates/c/t.rs:8: check/obsolete-tests-elsewhere: test old_rule cites b/gone, which 20260101-x removed, from `c`'s file; list it in that change's deleted-tests.txt, and prove what stays in the files of the component that states it";

#[test]
fn a_test_nobody_listed() {
    // req:check/obsolete-tests-elsewhere@c840a483
    let dir = unlisted();
    assert_eq!(found(dir.path()), [UNLISTED_LINE]);
}

#[test]
fn the_command_exits_1_with_the_line() {
    // req:check/obsolete-tests-elsewhere@c840a483
    let dir = unlisted();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let inputs = clm_check::CheckInputs {
        detectors: &|_| Ok(Vec::new()),
        more: &|_| Vec::new(),
    };
    let status = clm_check::check_command(
        ["--no-detectors"],
        dir.path(),
        &|_: &str| None,
        &mut out,
        &mut err,
        &inputs,
    );
    assert_eq!(status, 1);
    let out = String::from_utf8(out).unwrap();
    assert!(out.lines().any(|l| l == UNLISTED_LINE), "{out}");
}

#[test]
fn a_listed_test() {
    // req:check/obsolete-tests-elsewhere@c840a483
    for list in [
        format!("changes/{CHANGE}/deleted-tests.txt"),
        "changes/archive/20250101-old/deleted-tests.txt".to_string(),
    ] {
        let dir = unlisted();
        write(dir.path(), &list, "# why\nDELETE crates/c/t.rs::old_rule\n");
        assert_eq!(found(dir.path()), Vec::<String>::new(), "{list}");
    }
}

#[test]
fn a_list_naming_another_test_or_file_lists_nothing() {
    // req:check/obsolete-tests-elsewhere@c840a483
    for line in [
        "DELETE crates/c/t.rs::other",
        "DELETE crates/c/u.rs::old_rule",
    ] {
        let dir = unlisted();
        write(
            dir.path(),
            &format!("changes/{CHANGE}/deleted-tests.txt"),
            &format!("{line}\n"),
        );
        assert_eq!(found(dir.path()), [UNLISTED_LINE], "{line}");
    }
}

#[test]
fn a_list_names_a_test_by_its_bare_name() {
    // req:check/obsolete-tests-elsewhere@c840a483
    let dir = unlisted();
    write(
        dir.path(),
        &format!("changes/{CHANGE}/deleted-tests.txt"),
        &format!(
            "DELETE crates/c/t.rs::old_rule {}@0123abcd\n",
            marker("b/gone")
        ),
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

/// No component `g`; the change removed `g/x`, and `adds` when given; `crates/c/t.rs` holds
/// a test citing `g/x` alone.
fn removed_component(adds: &[(&str, &str)], c_slugs: &[&str]) -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "specs/SPEC.md", &root_spec(Some("[]")));
    write(
        root,
        "specs/c/SPEC.md",
        &component("[\"crates/c/**\"]", "", c_slugs),
    );
    change(root, &[("g", "x")], adds);
    write(root, "crates/c/t.rs", &test_file("old_rule", &["g/x"]));
    git(root);
    dir
}

#[test]
fn a_removed_components_test() {
    // req:check/obsolete-tests-elsewhere@c840a483
    let dir = removed_component(&[], &["y"]);
    assert_eq!(
        found(dir.path()),
        [
            "crates/c/t.rs:8: check/obsolete-tests-elsewhere: test old_rule cites g/x, which 20260101-x removed, from `c`'s file; list it in that change's deleted-tests.txt, and prove what stays in the files of the component that states it"
        ]
    );
}

#[test]
fn moved_into_the_files_owner() {
    // req:check/obsolete-tests-elsewhere@c840a483
    let dir = removed_component(&[("c", "x")], &["y", "x"]);
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn its_own_components_file_and_a_test_citing_a_live_requirement_too() {
    // req:check/obsolete-tests-elsewhere@c840a483
    let dir = repo(
        Some("[]"),
        &[
            ("crates/b/t.rs", test_file("old_rule", &["b/gone"])),
            ("crates/c/t.rs", test_file("old_rule", &["b/gone", "c/y"])),
        ],
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn a_test_citing_a_capability_too() {
    // req:check/obsolete-tests-elsewhere@c840a483
    let dir = repo(
        Some("[]"),
        &[(
            "crates/c/t.rs",
            test_file("old_rule", &["b/gone", "capability/c"]),
        )],
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn a_test_citing_the_owners_own_removed_requirement_too() {
    // req:check/obsolete-tests-elsewhere@c840a483
    let dir = repo(
        Some("[]"),
        &[(
            "crates/c/t.rs",
            test_file("old_rule", &["b/gone", "c/gone"]),
        )],
    );
    change(dir.path(), &[("b", "gone"), ("c", "gone")], &[]);
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn a_marker_outside_a_test() {
    // req:check/obsolete-tests-elsewhere@c840a483
    let dir = repo(
        Some("[]"),
        &[(
            "crates/c/t.rs",
            format!("// {}\nfn helper() {{}}\n", marker("b/gone")),
        )],
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn a_wiring_file_and_an_unowned_file() {
    // req:check/obsolete-tests-elsewhere@c840a483
    let dir = repo(
        Some("[\"crates/c/shared.rs\"]"),
        &[
            ("crates/c/shared.rs", test_file("old_rule", &["b/gone"])),
            ("old/t.rs", test_file("old_rule", &["b/gone"])),
        ],
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn not_switched_on() {
    // req:check/obsolete-tests-elsewhere@c840a483
    let dir = repo(
        None,
        &[("crates/c/t.rs", test_file("old_rule", &["b/gone"]))],
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn no_exemption_excuses_it() {
    // req:check/obsolete-tests-elsewhere@c840a483
    let dir = unlisted();
    write(
        dir.path(),
        "specs/c/SPEC.md",
        &component(
            "[\"crates/c/**\"]",
            "exemptions: [{rule: tests-cite-their-owner, files: [\"crates/c/t.rs\"], until: backlog/x}]\n",
            &["y"],
        ),
    );
    assert!(found(dir.path()).contains(&UNLISTED_LINE.to_string()));
}
