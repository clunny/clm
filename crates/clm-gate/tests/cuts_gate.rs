//! What `clm gate` prints of what a spec branch cuts (gate/cuts, gate/needs-review), and
//! what `evaluate_at` returns of it (gate/evaluate-at).

use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use clm_gate::{Cut, Kind, Side, cuts_or_unloaded, evaluate_at};
use tempfile::TempDir;

fn git(d: &Path, args: &[&str]) -> String {
    clm_git::git(d, args).unwrap()
}

fn write(d: &Path, rel: &str, content: &str) {
    let path = d.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn commit(d: &Path, message: &str) {
    git(d, &["add", "-A"]);
    git(
        d,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            message,
        ],
    );
}

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n";

fn component(id: &str, requirements: &str) -> String {
    format!(
        "---\ntitle: {id}\nstatus: active\nowners: []\npaths: [\"src/{id}/**\"]\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n{requirements}"
    )
}

fn requirement(slug: &str, statement: &str, scenarios: &[&str]) -> String {
    let mut out = format!("\n### Requirement: {slug}\n{statement}\n");
    for name in scenarios {
        out.push_str(&format!(
            "\n#### Scenario: {name}\n- GIVEN g\n- WHEN w\n- THEN t\n"
        ));
    }
    out
}

const X: &str = "X SHALL hold.";

/// A test file holding the test `two`, which cites `a/x`; the marker is built from pieces so
/// this file cites nothing.
fn test_file() -> String {
    format!(
        "#[test]\nfn one() {{}}\n\n// {}:a/x@00000000\n#[test]\nfn two() {{}}\n",
        "req"
    )
}

/// A repository whose `main` holds the root, `components` as `(id, requirements)` and
/// `tests/t.rs`, with `pr` checked out from it.
fn repo(components: &[(&str, String)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(d, "specs/SPEC.md", ROOT);
    for (id, reqs) in components {
        write(d, &format!("specs/{id}/SPEC.md"), &component(id, reqs));
    }
    write(d, "tests/t.rs", &test_file());
    commit(d, "base");
    git(d, &["checkout", "-q", "-b", "pr"]);
    dir
}

/// Commits `edits` on `pr` (`None` deletes the file).
fn edit(d: &Path, edits: &[(&str, Option<&str>)]) {
    for (rel, content) in edits {
        match content {
            Some(c) => write(d, rel, c),
            None => fs::remove_file(d.join(rel)).unwrap(),
        }
    }
    commit(d, "edit");
}

/// `clm gate --base main` with `extra` arguments: its status and standard output.
fn gate(d: &Path, extra: &[&str]) -> (u8, String) {
    let mut args: Vec<&OsStr> = ["--base", "main"].iter().map(OsStr::new).collect();
    args.extend(extra.iter().map(OsStr::new));
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = clm_gate::command(args, d, &|_| None, &mut out, &mut err);
    assert!(err.is_empty(), "{}", String::from_utf8_lossy(&err));
    (status, String::from_utf8(out).unwrap())
}

/// The output's `gate/cut` lines.
fn cut_lines(stdout: &str) -> Vec<&str> {
    stdout
        .lines()
        .filter(|l| l.contains(": gate/cut: "))
        .collect()
}

#[test]
fn scenario_a_removed_requirement() {
    // req:gate/cuts@02dbf77f
    // req:gate/needs-review@2373a9ad
    let two = format!(
        "{}{}",
        requirement("x", X, &["first"]),
        requirement("y", X, &["other"])
    );
    let dir = repo(&[("a", two)]);
    let d = dir.path();
    edit(
        d,
        &[(
            "specs/a/SPEC.md",
            Some(&component("a", &requirement("y", X, &["other"]))),
        )],
    );
    let (status, stdout) = gate(d, &[]);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines[..2],
        [
            "spec review",
            "specs/a/SPEC.md: gate/cut: requirement a/x is gone"
        ],
        "{stdout}"
    );
    assert_eq!(status, 1, "{stdout}");
    assert!(lines[2].contains("gate/approval"), "{stdout}");
    assert_eq!(lines.len(), 3, "{stdout}");
    let (status, stdout) = gate(d, &["--approved"]);
    assert_eq!(
        (status, stdout.as_str()),
        (
            0,
            "spec review\nspecs/a/SPEC.md: gate/cut: requirement a/x is gone\n"
        )
    );
}

/// The `gate/cut` lines and the review of a branch from `a`'s `base` requirements to `head`.
fn cuts_of(base: String, head: String) -> (String, Vec<String>) {
    let dir = repo(&[("a", base)]);
    let d = dir.path();
    edit(d, &[("specs/a/SPEC.md", Some(&component("a", &head)))]);
    let (_, stdout) = gate(d, &[]);
    let first = stdout.lines().next().unwrap_or_default().to_string();
    (
        first,
        cut_lines(&stdout).iter().map(|l| l.to_string()).collect(),
    )
}

#[test]
fn scenario_a_removed_scenario_and_a_renamed_one() {
    // req:gate/cuts@02dbf77f
    let base = requirement("x", X, &["first", "second"]);
    let line = "specs/a/SPEC.md: gate/cut: scenario \"second\" of a/x is gone";
    assert_eq!(
        cuts_of(base.clone(), requirement("x", X, &["first"])),
        ("spec review".into(), vec![line.to_string()])
    );
    assert_eq!(
        cuts_of(base, requirement("x", X, &["first", "the second one"])),
        ("spec review".into(), vec![line.to_string()])
    );
}

#[test]
fn scenario_moved_to_another_component() {
    // req:gate/cuts@02dbf77f
    let dir = repo(&[
        ("a", requirement("x", X, &["the chain"])),
        ("b", requirement("y", X, &["other"])),
    ]);
    let d = dir.path();
    edit(
        d,
        &[
            ("specs/a/SPEC.md", Some(&component("a", ""))),
            (
                "specs/b/SPEC.md",
                Some(&component(
                    "b",
                    &requirement("y", X, &["other", "the chain"]),
                )),
            ),
        ],
    );
    assert_eq!(gate(d, &[]), (0, "spec\n".to_string()));
}

#[test]
fn scenario_two_requirements_merged() {
    // req:gate/cuts@02dbf77f
    let base = format!(
        "{}{}",
        requirement("x", X, &["can edit"]),
        requirement("y", X, &["can edit"])
    );
    assert_eq!(
        cuts_of(base, requirement("z", X, &["can edit"])),
        (
            "spec review".into(),
            vec!["specs/a/SPEC.md: gate/cut: requirement a/y is gone".to_string()]
        )
    );
}

#[test]
fn scenario_a_name_another_requirement_already_held() {
    // req:gate/cuts@02dbf77f
    let dir = repo(&[
        ("a", requirement("x", X, &["s", "not switched on"])),
        ("b", requirement("y", X, &["not switched on"])),
    ]);
    let d = dir.path();
    edit(
        d,
        &[(
            "specs/a/SPEC.md",
            Some(&component("a", &requirement("x", X, &["s"]))),
        )],
    );
    let (_, stdout) = gate(d, &[]);
    assert_eq!(
        cut_lines(&stdout),
        ["specs/a/SPEC.md: gate/cut: scenario \"not switched on\" of a/x is gone"]
    );
}

#[test]
fn scenario_a_lost_sentence_a_placing_one_and_a_reworded_one() {
    // req:gate/cuts@02dbf77f
    let two = "The command SHALL print the list. It SHALL exit 1 when the list is empty.";
    let one = "The command SHALL print the list.";
    assert_eq!(
        cuts_of(requirement("x", two, &["s"]), requirement("x", one, &["s"])),
        (
            "spec review".into(),
            vec!["specs/a/SPEC.md: gate/cut: a/x states 1 sentences where it stated 2".to_string()]
        )
    );
    let placed = format!("{one} Every test of a's requirements SHALL sit in a's own files.");
    assert_eq!(
        cuts_of(
            requirement("x", &placed, &["s"]),
            requirement("x", one, &["s"])
        ),
        ("spec".into(), vec![])
    );
    assert_eq!(
        cuts_of(
            requirement("x", one, &["s"]),
            requirement("x", "`clm list` SHALL print one line per item.", &["s"])
        ),
        ("spec".into(), vec![])
    );
}

const CHANGE: &str = "---\nstatus: proposed\n---\n\n## Intent\nx\n";
const LIST: &str = "# merged into a/y\nDELETE tests/t.rs::two\tits requirement is gone\n";

#[test]
fn scenario_a_test_the_change_lists() {
    // req:gate/cuts@02dbf77f
    let dir = repo(&[("a", requirement("x", X, &["s"]))]);
    let d = dir.path();
    edit(
        d,
        &[
            ("changes/c/change.md", Some(CHANGE)),
            ("changes/c/deleted-tests.txt", Some(LIST)),
        ],
    );
    assert_eq!(
        gate(d, &["--approved"]),
        (
            0,
            "spec review\nchanges/c/deleted-tests.txt: gate/cut: test tests/t.rs::two is to be deleted\n"
                .to_string()
        )
    );
}

#[test]
fn scenario_a_line_that_names_no_test() {
    // req:gate/cuts@02dbf77f
    let dir = repo(&[("a", requirement("x", X, &["s"]))]);
    let d = dir.path();
    // `one` holds no marker, so it names no citing test either
    let list =
        format!("{LIST}DELETE tests/t.rs::nope\nremove the old ones\nDELETE tests/t.rs::one\n");
    edit(
        d,
        &[
            ("changes/c/change.md", Some(CHANGE)),
            ("changes/c/deleted-tests.txt", Some(&list)),
        ],
    );
    let out = evaluate_at(d, "main", "HEAD", true).unwrap();
    let found: Vec<String> = out
        .violations
        .iter()
        .map(|v| format!("{}: {}: {}", v.path.display(), v.rule, v.message))
        .collect();
    assert_eq!(found.len(), 3, "{found:?}");
    for (line, named) in
        found
            .iter()
            .zip(["tests/t.rs::nope", "remove the old ones", "tests/t.rs::one"])
    {
        assert!(
            line.starts_with("changes/c/deleted-tests.txt: gate/deleted-tests: ")
                && line.contains(named),
            "{line}"
        );
    }
    let (status, _) = gate(d, &["--approved"]);
    assert_eq!(status, 1);
}

#[test]
fn scenario_a_list_whose_tests_are_gone() {
    // req:gate/cuts@02dbf77f
    let dir = repo(&[("a", requirement("x", X, &["s"]))]);
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(d, "changes/c/change.md", CHANGE);
    write(d, "changes/c/deleted-tests.txt", LIST);
    write(d, "tests/t.rs", "#[test]\nfn one() {}\n");
    commit(d, "the test is gone");
    git(d, &["checkout", "-q", "pr"]);
    git(d, &["merge", "-q", "--ff-only", "main"]);
    let both = format!(
        "{}{}",
        requirement("x", X, &["s"]),
        requirement("y", X, &["t"])
    );
    edit(d, &[("specs/a/SPEC.md", Some(&component("a", &both)))]);
    assert_eq!(gate(d, &[]), (0, "spec\n".to_string()));
}

#[test]
fn scenario_a_list_moved_into_the_archive() {
    // req:gate/cuts@02dbf77f
    let dir = repo(&[("a", requirement("x", X, &["s"]))]);
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(d, "changes/c/change.md", CHANGE);
    write(d, "changes/c/deleted-tests.txt", LIST);
    write(d, "tests/t.rs", "#[test]\nfn one() {}\n");
    commit(d, "the test is gone");
    git(d, &["checkout", "-q", "pr"]);
    git(d, &["merge", "-q", "--ff-only", "main"]);
    fs::create_dir_all(d.join("changes/archive")).unwrap();
    git(d, &["mv", "changes/c", "changes/archive/c"]);
    // a spec edit beside it, so the branch is a spec branch whose lists are read
    let both = format!(
        "{}{}",
        requirement("x", X, &["s"]),
        requirement("y", X, &["t"])
    );
    edit(d, &[("specs/a/SPEC.md", Some(&component("a", &both)))]);
    let (status, stdout) = gate(d, &[]);
    assert!(cut_lines(&stdout).is_empty(), "{stdout}");
    assert!(!stdout.contains("gate/deleted-tests"), "{stdout}");
    assert_eq!(status, 0, "{stdout}");
}

#[test]
fn specs_that_dont_load_need_review() {
    // req:gate/cuts@02dbf77f
    let dir = repo(&[("a", requirement("x", X, &["s"]))]);
    let d = dir.path();
    // a requirement with no scenario doesn't load
    let broken = format!(
        "{}{}",
        requirement("x", X, &["s"]),
        requirement("y", X, &[])
    );
    edit(d, &[("specs/a/SPEC.md", Some(&component("a", &broken)))]);
    let head = git(d, &["rev-parse", "HEAD"]);
    let (status, stdout) = gate(d, &[]);
    assert_eq!(
        stdout.lines().take(2).collect::<Vec<_>>(),
        [
            "spec review",
            format!(
                "specs: gate/cut: the specs at {head} don't load, so what the branch cuts isn't listed"
            )
            .as_str()
        ],
        "{stdout}"
    );
    assert_eq!(status, 1);
}

#[test]
fn evaluate_at_returns_the_cuts() {
    // req:gate/evaluate-at@86fdcdf3
    let dir = repo(&[("a", requirement("x", X, &["first", "second"]))]);
    let d = dir.path();
    edit(
        d,
        &[(
            "specs/a/SPEC.md",
            Some(&component("a", &requirement("x", X, &["first"]))),
        )],
    );
    let s = git(d, &["rev-parse", "HEAD"]);
    git(d, &["checkout", "-q", "main"]);
    let out = evaluate_at(d, "main", &s, true).unwrap();
    assert_eq!((out.kind, out.review), (Kind::Spec, true));
    assert_eq!(
        out.cuts,
        [Cut::Scenario {
            spec: "specs/a/SPEC.md".into(),
            id: "a/x".into(),
            name: "second".into(),
        }]
    );
    assert!(out.violations.is_empty(), "{:?}", out.violations);
}

const ARCH: &str = "# Arch\n\n## Constraints\n\n### Constraint: law\n**Principle.** A law SHALL hold.\n\n#### Scenario: s\n- ok\n\n#### Untested\nA judgement.\n";

#[test]
fn the_graphs_hold_the_human_layers() {
    // req:gate/cuts@02dbf77f
    // A rule that serves an architectural constraint loads only beside `ARCHITECTURE.md`.
    let serving =
        |reqs: &str| component("a", reqs).replace("paths:", "serves: [architecture/law]\npaths:");
    let dir = repo(&[]);
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(d, "ARCHITECTURE.md", ARCH);
    write(
        d,
        "specs/a/SPEC.md",
        &serving(&requirement("x", X, &["first", "second"])),
    );
    commit(d, "a serving rule");
    git(d, &["checkout", "-q", "-B", "pr"]);
    edit(
        d,
        &[(
            "specs/a/SPEC.md",
            Some(&serving(&requirement("x", X, &["first"]))),
        )],
    );
    let (_, stdout) = gate(d, &[]);
    assert_eq!(
        cut_lines(&stdout),
        ["specs/a/SPEC.md: gate/cut: scenario \"second\" of a/x is gone"],
        "{stdout}"
    );
}

#[test]
fn specs_that_dont_load_at_the_merge_base_need_review() {
    // req:gate/cuts@02dbf77f
    let broken = format!(
        "{}{}",
        requirement("x", X, &["s"]),
        requirement("y", X, &[])
    );
    let dir = repo(&[("a", broken)]);
    let d = dir.path();
    let base = git(d, &["rev-parse", "main"]);
    edit(
        d,
        &[(
            "specs/a/SPEC.md",
            Some(&component("a", &requirement("x", X, &["s"]))),
        )],
    );
    let (status, stdout) = gate(d, &[]);
    assert_eq!(
        cut_lines(&stdout),
        [format!(
            "specs: gate/cut: the specs at {base} don't load, so what the branch cuts isn't listed"
        )],
        "{stdout}"
    );
    assert_eq!((stdout.lines().next(), status), (Some("spec review"), 1));
}

#[test]
fn scenario_a_lowercase_shall() {
    // req:gate/cuts@02dbf77f
    let one = "The command SHALL print the list.";
    assert_eq!(
        cuts_of(
            requirement("x", &format!("{one} It shall be quick."), &["s"]),
            requirement("x", one, &["s"])
        ),
        ("spec".into(), vec![])
    );
}

#[test]
fn scenario_a_phrase_inside_a_word() {
    // req:gate/cuts@02dbf77f
    let one = "The command SHALL print the list.";
    assert_eq!(
        cuts_of(
            requirement(
                "x",
                &format!("{one} The file SHALL be what is left intact."),
                &["s"]
            ),
            requirement("x", one, &["s"])
        ),
        (
            "spec review".into(),
            vec!["specs/a/SPEC.md: gate/cut: a/x states 1 sentences where it stated 2".to_string()]
        )
    );
}

#[test]
fn scenario_a_sentence_ending_at_a_line_break() {
    // req:gate/cuts@02dbf77f
    let one = "It SHALL print the list.";
    assert_eq!(
        cuts_of(
            requirement("x", &format!("{one}\nIt SHALL exit 0."), &["s"]),
            requirement("x", one, &["s"])
        ),
        (
            "spec review".into(),
            vec!["specs/a/SPEC.md: gate/cut: a/x states 1 sentences where it stated 2".to_string()]
        )
    );
}

const CAPS: &str = "# Capabilities\n\n## Capabilities\n\n### Capability: c\nA person can c.\n\n#### Why\nW.\n\n#### Scenario: s\n- ok\n";

#[test]
fn scenario_rules_that_serve_the_human_layers() {
    // req:gate/cuts@02dbf77f
    let serving =
        |reqs: &str| component("a", reqs).replace("paths:", "serves: [capability/c]\npaths:");
    let dir = repo(&[]);
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(d, "CAPABILITIES.md", CAPS);
    write(d, "specs/a/SPEC.md", &serving(&requirement("x", X, &["s"])));
    commit(d, "a serving rule");
    git(d, &["checkout", "-q", "-B", "pr"]);
    let both = format!(
        "{}{}",
        requirement("x", X, &["s"]),
        requirement("y", X, &["t"])
    );
    edit(d, &[("specs/a/SPEC.md", Some(&serving(&both)))]);
    assert_eq!(gate(d, &[]), (0, "spec\n".to_string()));
}

/// The graph `clm_model::load_files` gives for the root and `a` holding `requirements`, with
/// its violations.
fn loaded(requirements: &str) -> (clm_model::Graph, Vec<clm_model::Violation>) {
    let mut files = clm_model::Files::new();
    files.insert("specs/SPEC.md".into(), ROOT.as_bytes().to_vec());
    files.insert(
        "specs/a/SPEC.md".into(),
        component("a", requirements).into_bytes(),
    );
    clm_model::load_files(&files)
}

#[test]
fn scenario_specs_that_dont_load() {
    // req:gate/cuts@02dbf77f
    let good = requirement("x", X, &["s"]);
    let broken = format!("{good}{}", requirement("y", X, &[]));
    let dir = repo(&[("a", good.clone())]);
    let d = dir.path();
    edit(d, &[("specs/a/SPEC.md", Some(&component("a", &broken)))]);
    let head = git(d, &["rev-parse", "HEAD"]);
    let unloaded = format!(
        "specs: gate/cut: the specs at {head} don't load, so what the branch cuts isn't listed"
    );
    let (status, stdout) = gate(d, &[]);
    assert_eq!(
        (stdout.lines().next(), cut_lines(&stdout), status),
        (Some("spec review"), vec![unloaded.as_str()], 1),
        "{stdout}"
    );
    let (base_graph, base_violations) = loaded(&good);
    let (head_graph, head_violations) = loaded(&broken);
    assert!(base_violations.is_empty() && !head_violations.is_empty());
    let found = cuts_or_unloaded(
        &Side {
            graph: &base_graph,
            violations: &base_violations,
            revision: "main",
        },
        &Side {
            graph: &head_graph,
            violations: &head_violations,
            revision: &head,
        },
        &[],
    );
    assert_eq!(
        found.iter().map(ToString::to_string).collect::<Vec<_>>(),
        [unloaded]
    );
}

#[test]
fn scenario_unloaded_specs_and_a_listed_test() {
    // req:gate/cuts@02dbf77f
    let good = requirement("x", X, &["s"]);
    let broken = format!("{good}{}", requirement("y", X, &[]));
    let dir = repo(&[("a", good)]);
    let d = dir.path();
    edit(
        d,
        &[
            ("specs/a/SPEC.md", Some(&component("a", &broken))),
            ("changes/c/change.md", Some(CHANGE)),
            ("changes/c/deleted-tests.txt", Some(LIST)),
        ],
    );
    let head = git(d, &["rev-parse", "HEAD"]);
    let (_, stdout) = gate(d, &["--approved"]);
    assert_eq!(
        cut_lines(&stdout),
        [
            format!(
                "specs: gate/cut: the specs at {head} don't load, so what the branch cuts isn't listed"
            )
            .as_str(),
            "changes/c/deleted-tests.txt: gate/cut: test tests/t.rs::two is to be deleted",
        ],
        "{stdout}"
    );
}

#[test]
fn specs_that_load_at_neither_side_name_the_merge_base() {
    // req:gate/cuts@02dbf77f
    let broken = format!(
        "{}{}",
        requirement("x", X, &["s"]),
        requirement("y", X, &[])
    );
    let dir = repo(&[("a", broken.clone())]);
    let d = dir.path();
    let base = git(d, &["rev-parse", "main"]);
    let worse = format!("{broken}{}", requirement("z", X, &[]));
    edit(d, &[("specs/a/SPEC.md", Some(&component("a", &worse)))]);
    let (_, stdout) = gate(d, &[]);
    assert_eq!(
        cut_lines(&stdout),
        [format!(
            "specs: gate/cut: the specs at {base} don't load, so what the branch cuts isn't listed"
        )],
        "{stdout}"
    );
}

#[test]
fn scenario_a_branch_that_touches_no_spec() {
    // req:gate/cuts@02dbf77f
    let dir = repo(&[("a", requirement("x", X, &["s"]))]);
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(d, "changes/c/change.md", CHANGE);
    commit(d, "a live change");
    git(d, &["checkout", "-q", "-B", "pr"]);
    let unknown = CHANGE.replace("status: proposed\n", "status: proposed\nunknown_key: 1\n");
    edit(d, &[("changes/c/change.md", Some(&unknown))]);
    // read as a graph, the head wouldn't load: the cuts aren't read at all
    let (_, stdout) = gate(d, &[]);
    assert!(cut_lines(&stdout).is_empty(), "{stdout}");
}
