use std::fs;
use std::path::Path;

use clm_rename::Failure;
use tempfile::TempDir;

fn git(d: &Path, args: &[&str]) {
    clm_git::git(d, args).unwrap();
}

/// Every violation of the tree at `d`.
fn violations(d: &Path) -> Vec<clm_model::Violation> {
    clm_model::load(d).1
}

/// `clm rename` in a git checkout with everything staged, as the command runs.
fn rename(d: &Path, id: &str) -> Result<Vec<String>, Failure> {
    git(d, &["init", "-q"]);
    git(d, &["add", "-A"]);
    clm_rename::rename(d, id)
}

const MARKER: &str = concat!("req", ":sprocket/x");
const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n";

pub fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

pub fn read(root: &Path, rel: &str) -> String {
    fs::read_to_string(root.join(rel)).unwrap()
}

/// A node spec: `extra` frontmatter lines, a Design and requirements `(slug, body)`.
pub fn node(extra: &str, design: &str, reqs: &[(&str, &str)]) -> String {
    let blocks: String = reqs
        .iter()
        .map(|(s, body)| format!("### Requirement: {s}\n{body}\n\n#### Scenario: holds\n- ok\n\n"))
        .collect();
    format!(
        "---\ntitle: N\nstatus: specd\nowners: [\"@a\"]\n{extra}---\n\n## Contract\nA contract.\n\n## Design\n{design}\n\n## Requirements\n\n{blocks}"
    )
}

pub fn change(renames: &str, extra: &str) -> String {
    format!("---\nstatus: proposed\n{extra}renames:\n{renames}---\n\n## Intent\nRename.\n")
}

pub fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    dir
}

#[test]
// req:rename/component-rename@4e038ba9
fn a_component_and_its_child() {
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &node("", "design", &[("x", "The x SHALL hold.")]),
    );
    write(
        d,
        "specs/a/c/SPEC.md",
        &node("", "design", &[("y", "The y SHALL hold.")]),
    );
    write(
        d,
        "specs/d/SPEC.md",
        &node("depends_on: [a, a.c]\n", "d", &[("z", "Z SHALL.")]),
    );
    write(
        d,
        "changes/r/change.md",
        &change("  - component: a\n    to: b\n", ""),
    );
    let printed = rename(d, "r").unwrap();
    assert_eq!(printed, ["renamed a -> b"]);
    assert!(read(d, "specs/b/SPEC.md").contains("### Requirement: x\n"));
    assert!(read(d, "specs/b/c/SPEC.md").contains("### Requirement: y\n"));
    assert!(!d.join("specs/a").exists());
    assert!(read(d, "specs/d/SPEC.md").contains("depends_on: [b, b.c]"));
    let deltas = |node: &str| read(d, &format!("changes/archive/r/deltas/{node}.md"));
    assert!(deltas("a").contains("remove: true"));
    assert!(deltas("a.c").contains("remove: true"));
    assert!(deltas("b").contains("create: true"));
    assert!(deltas("b.c").contains("create: true"));
    assert!(violations(d).is_empty(), "{:?}", violations(d));
    assert!(clm_model::load(d).0.requirement("b.c/y").is_some());
}

#[test]
// req:rename/component-rename@4e038ba9
fn a_component_keeps_its_assets() {
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &node("", "design", &[("x", "X SHALL.")]),
    );
    write(d, "specs/a/assets/m/page.html", "<p>hi</p>");
    write(
        d,
        "changes/r/change.md",
        &change("  - component: a\n    to: b\n", ""),
    );
    rename(d, "r").unwrap();
    assert_eq!(read(d, "specs/b/assets/m/page.html"), "<p>hi</p>");
    assert!(!d.join("specs/a").exists());
}

#[test]
// req:rename/requirement-rename@82f3aff4
fn a_node_rule() {
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &node(
            "",
            "design",
            &[("stale-base", "It SHALL hold."), ("other", "Other SHALL.")],
        ),
    );
    write(
        d,
        "changes/r/change.md",
        &change("  - requirement: a/stale-base\n    to: base-moved\n", ""),
    );
    assert_eq!(
        rename(d, "r").unwrap(),
        ["renamed a/stale-base -> a/base-moved"]
    );
    let spec = read(d, "specs/a/SPEC.md");
    assert!(spec.contains("### Requirement: base-moved\nIt SHALL hold."));
    assert!(!spec.contains("stale-base"));
    assert!(spec.contains("### Requirement: other\n"));
}

const ARCHITECTURE: &str = "# Architecture\n\nIntro about sprocket-scoped-context.\n\n## Constraints\n\n### Constraint: sprocket-scoped-context\nAn agent SHALL get the sprocket.\n\n#### Scenario: s\n- ok\n\n#### Untested\nA judgement.\n";

#[test]
// req:rename/requirement-rename@82f3aff4
fn an_architecture_entry() {
    let dir = repo();
    let d = dir.path();
    write(d, "ARCHITECTURE.md", ARCHITECTURE);
    write(
        d,
        "specs/a/SPEC.md",
        &node(
            "serves: [architecture/sprocket-scoped-context]\n",
            "design",
            &[("x", "X SHALL.")],
        ),
    );
    write(
        d,
        "changes/r/change.md",
        &change(
            "  - requirement: architecture/sprocket-scoped-context\n    to: cogs-scoped-context\n",
            "",
        ),
    );
    rename(d, "r").unwrap();
    let architecture = read(d, "ARCHITECTURE.md");
    assert!(architecture.contains("### Constraint: cogs-scoped-context\n"));
    assert!(architecture.contains("Intro about sprocket-scoped-context."));
    assert!(read(d, "specs/a/SPEC.md").contains("serves: [architecture/cogs-scoped-context]"));
    assert!(violations(d).is_empty(), "{:?}", violations(d));
}

fn references_repo() -> TempDir {
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &node("", "design", &[("x", "X SHALL hold.")]),
    );
    write(
        d,
        "specs/b/SPEC.md",
        &node(
            "",
            "Design cites `a/x` and `apply` here.",
            &[("z", "Z SHALL cite (`a/x`, `x`).\n\n#### Needs\n- a/x")],
        ),
    );
    write(
        d,
        "ARCHITECTURE.md",
        "# A\n\n## Constraints\n\n### Constraint: arch\nA SHALL.\n\n#### Scenario: s\n- ok\n\n#### Enforced by\n- a/x\n",
    );
    write(
        d,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\ncomponents: [a]\nblocks: [a/x]\n---\n\nbody blocks: a/x\n",
    );
    write(
        d,
        "changes/r/change.md",
        &change("  - requirement: a/x\n    to: y\n", ""),
    );
    dir
}

#[test]
// req:rename/references@659c44f9
fn references_across_the_repository() {
    let dir = references_repo();
    let d = dir.path();
    rename(d, "r").unwrap();
    let b = read(d, "specs/b/SPEC.md");
    assert!(b.contains("#### Needs\n- a/y"), "{b}");
    assert!(b.contains("(`a/y`, `y`)"), "{b}");
    assert!(read(d, "ARCHITECTURE.md").contains("#### Enforced by\n- a/y"));
    let q = read(d, "backlog/q.md");
    assert!(q.contains("blocks: [a/y]"), "{q}");
    assert!(q.contains("body blocks: a/x"), "{q}");
}

#[test]
// req:rename/references@659c44f9
fn an_ambiguous_slug_stays() {
    let dir = references_repo();
    let d = dir.path();
    write(
        d,
        "specs/e/SPEC.md",
        &node("", "Design cites `x` alone.", &[("x", "E SHALL.")]),
    );
    rename(d, "r").unwrap();
    assert!(read(d, "specs/e/SPEC.md").contains("Design cites `x` alone."));
    assert!(read(d, "specs/b/SPEC.md").contains("(`a/y`, `x`)"));
}

#[test]
// req:rename/references@659c44f9
fn a_decision_with_the_slug_makes_it_ambiguous() {
    let dir = references_repo();
    let d = dir.path();
    write(
        d,
        "docs/decisions/x.md",
        "---\ntitle: T\ndate: 2026-01-01\nstatus: accepted\n---\n\nBody.\n",
    );
    rename(d, "r").unwrap();
    assert!(read(d, "specs/b/SPEC.md").contains("(`a/y`, `x`)"));
}

#[test]
// req:rename/component-rename@4e038ba9
fn a_component_and_its_own_rule_are_not_renamed_together() {
    let dir = repo();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &node("", "d", &[("x", "X SHALL.")]));
    let renames = "  - component: a\n    to: b\n  - requirement: a/x\n    to: y\n";
    write(d, "changes/r/change.md", &change(renames, ""));
    let why = rename(d, "r").unwrap_err().to_string();
    assert!(
        why.contains("a/x") && why.contains("separate changes"),
        "{why}"
    );
    assert!(d.join("specs/a/SPEC.md").exists());
    assert!(!d.join("specs/b").exists());
}

#[test]
// req:rename/component-rename@4e038ba9
fn deltas_already_in_the_change_are_replaced() {
    let dir = repo();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &node("", "d", &[("x", "X SHALL.")]));
    write(d, "changes/r/deltas/old.md", "not a delta\n");
    write(
        d,
        "changes/r/change.md",
        &change("  - component: a\n    to: b\n", ""),
    );
    rename(d, "r").unwrap();
    assert!(!d.join("changes/archive/r/deltas/old.md").exists());
    assert!(d.join("changes/archive/r/deltas/b.md").exists());
}

#[test]
// req:rename/requirement-rename@82f3aff4
fn a_capability_entry() {
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "CAPABILITIES.md",
        "# Capabilities\n\n## Capabilities\n\n### Capability: land-safely\nA person can land.\n\n#### Why\nBecause.\n\n#### Scenario: s\n- GIVEN a\n- WHEN b\n- THEN c\n",
    );
    write(
        d,
        "specs/a/SPEC.md",
        &node(
            "serves: [capability/land-safely]\n",
            "d",
            &[("x", "X SHALL.")],
        ),
    );
    let renames = "  - requirement: capability/land-safely\n    to: land-checked\n";
    write(d, "changes/r/change.md", &change(renames, ""));
    rename(d, "r").unwrap();
    assert!(read(d, "CAPABILITIES.md").contains("### Capability: land-checked\n"));
    assert!(read(d, "specs/a/SPEC.md").contains("serves: [capability/land-checked]"));
    assert!(violations(d).is_empty(), "{:?}", violations(d));
}

#[test]
// req:rename/references@659c44f9
fn a_components_bare_name_is_a_word() {
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/apply/SPEC.md",
        &node("", "design", &[("x", "X SHALL.")]),
    );
    write(
        d,
        "specs/e/SPEC.md",
        &node(
            "depends_on: [apply]\n",
            "`apply` writes `specs/`",
            &[("w", "W SHALL.")],
        ),
    );
    write(
        d,
        "changes/r/change.md",
        &change("  - component: apply\n    to: fold\n", ""),
    );
    rename(d, "r").unwrap();
    let e = read(d, "specs/e/SPEC.md");
    assert!(e.contains("`apply` writes `specs/`"));
    assert!(e.contains("depends_on: [fold]"));
}

#[test]
// req:rename/references@659c44f9
fn archived_changes_and_decisions_keep_their_names() {
    let dir = references_repo();
    let d = dir.path();
    write(
        d,
        "docs/decisions/note.md",
        "---\ntitle: T\ndate: 2026-01-01\nstatus: accepted\n---\n\nSee `a/x`.\n",
    );
    write(
        d,
        "changes/archive/old/change.md",
        "---\nstatus: done\n---\n\n## Intent\nSee `a/x`.\n",
    );
    rename(d, "r").unwrap();
    assert!(read(d, "docs/decisions/note.md").contains("`a/x`"));
    assert!(read(d, "changes/archive/old/change.md").contains("`a/x`"));
}

fn terms_repo(term: &str, to: &str) -> TempDir {
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &node(
            "",
            "the sprocket of a component",
            &[("x", "Print the sprocket."), ("w", "W SHALL.")],
        ),
    );
    write(
        d,
        "changes/r/change.md",
        &change(&format!("  - term: {term}\n    to: {to}\n"), ""),
    );
    dir
}

#[test]
// req:rename/term-swap@b6300332
fn a_word_in_prose() {
    let dir = terms_repo("sprocket", "cogs");
    let d = dir.path();
    rename(d, "r").unwrap();
    let spec = read(d, "specs/a/SPEC.md");
    assert!(spec.contains("the cogs of a component"));
    assert!(spec.contains("Print the cogs."));
    let delta = read(d, "changes/archive/r/deltas/a.md");
    assert!(delta.contains("## MODIFIED Design"), "{delta}");
    assert!(
        delta.contains("## MODIFIED Requirements\n### Requirement: x <!-- base: "),
        "{delta}"
    );
    assert!(!delta.contains("### Requirement: w"), "{delta}");
}

#[test]
// req:rename/term-swap@b6300332
fn part_of_an_id_or_a_path_is_not_a_term() {
    let dir = terms_repo("sprocket", "cogs");
    let d = dir.path();
    write(
        d,
        "specs/e/SPEC.md",
        &node(
            "",
            &format!(
                "the sprocket, sprocket-scoped-context, crates/sprocket/, Sprocket, pre-sprocket, a.sprocket, b:sprocket and {MARKER}"
            ),
            &[("q", "Q SHALL.")],
        ),
    );
    rename(d, "r").unwrap();
    assert!(read(d, "specs/e/SPEC.md").contains(&format!(
        "the cogs, sprocket-scoped-context, crates/sprocket/, Sprocket, pre-sprocket, a.sprocket, b:sprocket and {MARKER}"
    )));
}

#[test]
// req:rename/term-swap@b6300332
fn a_term_before_punctuation_and_a_following_dot_path() {
    let dir = terms_repo("sprocket", "cogs");
    let d = dir.path();
    write(
        d,
        "specs/e/SPEC.md",
        &node(
            "",
            "sprocket: yes. Also sprocket.x and sprocket:x",
            &[("q", "Q SHALL.")],
        ),
    );
    rename(d, "r").unwrap();
    assert!(read(d, "specs/e/SPEC.md").contains("cogs: yes. Also sprocket.x and sprocket:x"));
}

#[test]
// req:rename/references@659c44f9
fn several_renames_apply_together() {
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &node("", "the sprocket", &[("x", "X SHALL.")]),
    );
    write(d, "specs/e/SPEC.md", &node("", "e", &[("q", "Q SHALL.")]));
    write(
        d,
        "specs/f/SPEC.md",
        &node(
            "depends_on: [a, e]\n",
            "f",
            &[(
                "w",
                "W SHALL, see `e/q` and `a/x`.\n\n#### Needs\n- e/q\n- a/x",
            )],
        ),
    );
    let renames = "  - requirement: e/q\n    to: r\n  - component: a\n    to: b\n  - term: sprocket\n    to: cogs\n";
    write(d, "changes/r/change.md", &change(renames, ""));
    assert_eq!(
        rename(d, "r").unwrap(),
        [
            "renamed e/q -> e/r",
            "renamed a -> b",
            "renamed sprocket -> cogs"
        ]
    );
    let f = read(d, "specs/f/SPEC.md");
    assert!(f.contains("depends_on: [b, e]"), "{f}");
    assert!(f.contains("see `e/r` and `b/x`."), "{f}");
    assert!(f.contains("#### Needs\n- e/r\n- b/x"), "{f}");
    assert!(read(d, "specs/b/SPEC.md").contains("the cogs"));
    assert!(violations(d).is_empty(), "{:?}", violations(d));
}

#[test]
// req:rename/term-swap@b6300332
fn a_heading_is_left_alone() {
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &node("", "the sprocket", &[("sprocket", "It SHALL.")]),
    );
    write(
        d,
        "changes/r/change.md",
        &change("  - term: sprocket\n    to: cogs\n", ""),
    );
    rename(d, "r").unwrap();
    let spec = read(d, "specs/a/SPEC.md");
    assert!(spec.contains("### Requirement: sprocket\n"));
    assert!(spec.contains("the cogs"));
}

#[test]
// req:rename/term-swap@b6300332
fn needs_lines_are_not_swapped() {
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &node("", "d", &[("sprocket", "It SHALL.")]),
    );
    write(
        d,
        "specs/b/SPEC.md",
        &node(
            "",
            "the sprocket",
            &[("w", "W SHALL.\n\n#### Needs\n- a/sprocket")],
        ),
    );
    write(
        d,
        "changes/r/change.md",
        &change("  - term: sprocket\n    to: cogs\n", ""),
    );
    rename(d, "r").unwrap();
    assert!(read(d, "specs/b/SPEC.md").contains("- a/sprocket"));
}

#[test]
// req:rename/refusals@8232ad18
fn a_term_that_is_nowhere_refuses_and_writes_nothing() {
    let dir = terms_repo("grommet stub", "spare part");
    let d = dir.path();
    let before = read(d, "specs/a/SPEC.md");
    let why = rename(d, "r").unwrap_err().to_string();
    assert!(why.contains("grommet stub"), "{why}");
    assert_eq!(read(d, "specs/a/SPEC.md"), before);
    assert!(read(d, "changes/r/change.md").contains("renames:"));
    assert!(!d.join("changes/r/deltas").exists());
}

#[test]
// req:rename/finish@90976f4b
fn done_and_archived() {
    let dir = references_repo();
    let d = dir.path();
    write(
        d,
        "backlog/gone.md",
        "---\ntitle: G\nkind: deferred\n---\n\nbody\n",
    );
    write(d, "plugin/skills/s/SKILL.md", "line one\nsee a/x here\n");
    write(
        d,
        "docs/decisions/note.md",
        "---\ntitle: T\ndate: 2026-01-01\nstatus: accepted\n---\n\nhistory of a/x\n",
    );
    write(
        d,
        "crates/x/tests/t.rs",
        &format!("// {}\n", concat!("req", ":a/x@11111111")),
    );
    let source = change("  - requirement: a/x\n    to: y\n", "resolves: [gone]\n");
    write(d, "changes/r/change.md", &source);
    let printed = rename(d, "r").unwrap();
    assert_eq!(
        printed,
        [
            "renamed a/x -> a/y",
            "still names a/x backlog/q.md:8",
            "still names a/x plugin/skills/s/SKILL.md:2"
        ]
    );
    assert_eq!(
        read(d, "crates/x/tests/t.rs"),
        format!("// {}\n", concat!("req", ":a/y@11111111"))
    );
    let archived = read(d, "changes/archive/r/change.md");
    assert_eq!(archived, source.replace("status: proposed", "status: done"));
    assert!(!d.join("changes/r").exists());
    assert!(!d.join("backlog/gone.md").exists());
}

/// Component `a` owning `owns`, whose requirement `x` cites the architecture entry
/// `sprocket-scoped-context` in a code span, a change renaming that entry to
/// `cogs-scoped-context`, and `ARCHITECTURE.md`.
fn slug_repo(owns: &str) -> TempDir {
    let dir = repo();
    let d = dir.path();
    write(d, "ARCHITECTURE.md", ARCHITECTURE);
    write(
        d,
        "specs/a/SPEC.md",
        &node(
            &format!("paths: [\"{owns}\"]\n"),
            "design",
            &[("x", "X SHALL hold (`sprocket-scoped-context`).")],
        ),
    );
    write(
        d,
        "changes/r/change.md",
        &change(
            "  - requirement: architecture/sprocket-scoped-context\n    to: cogs-scoped-context\n",
            "",
        ),
    );
    dir
}

#[test]
// req:rename/finish@90976f4b
fn a_bare_slug_is_left_as_a_code_span_only() {
    let dir = slug_repo("plugin/**");
    let d = dir.path();
    write(
        d,
        "plugin/skills/s/SKILL.md",
        "see (`sprocket-scoped-context`)\nthe sprocket-scoped-context rule\n",
    );
    assert_eq!(
        rename(d, "r").unwrap(),
        [
            "renamed architecture/sprocket-scoped-context -> architecture/cogs-scoped-context",
            "still names sprocket-scoped-context plugin/skills/s/SKILL.md:1"
        ]
    );
}

#[test]
// req:rename/finish@90976f4b
fn an_ambiguous_slug_is_not_reported() {
    let dir = slug_repo("plugin/**");
    let d = dir.path();
    write(
        d,
        "docs/decisions/sprocket-scoped-context.md",
        "---\ntitle: T\ndate: 2026-01-01\nstatus: accepted\n---\n\nbody\n",
    );
    write(
        d,
        "plugin/skills/s/SKILL.md",
        "see `sprocket-scoped-context`\n",
    );
    assert_eq!(
        rename(d, "r").unwrap(),
        ["renamed architecture/sprocket-scoped-context -> architecture/cogs-scoped-context"]
    );
}

#[test]
// req:rename/marker-rewrite@8ae3d4f6
fn code_that_still_names_a_rewritten_entry_leaves_the_requirement_behind() {
    let dir = slug_repo("plugin/**");
    let d = dir.path();
    let old = version_of(d, "a/x");
    write(d, "crates/x/tests/t.rs", &cite("a/x", &old));
    write(d, "plugin/skills/s/SKILL.md", "`sprocket-scoped-context`\n");
    let printed = rename(d, "r").unwrap();
    assert!(
        printed.contains(&"still names sprocket-scoped-context plugin/skills/s/SKILL.md:1".into()),
        "{printed:?}"
    );
    assert!(read(d, "specs/a/SPEC.md").contains("(`cogs-scoped-context`)"));
    assert_eq!(read(d, "crates/x/tests/t.rs"), cite("a/x", &old));
    assert_eq!(pending_ids(d), ["a/x"]);
}

#[test]
// req:rename/marker-rewrite@8ae3d4f6
fn a_held_back_marker_still_takes_the_new_id() {
    let dir = slug_repo("plugin/**");
    let d = dir.path();
    let renames = "  - requirement: architecture/sprocket-scoped-context\n    to: cogs-scoped-context\n  - requirement: a/x\n    to: y\n";
    write(d, "changes/r/change.md", &change(renames, ""));
    let old = version_of(d, "a/x");
    write(d, "crates/x/tests/t.rs", &cite("a/x", &old));
    write(d, "plugin/skills/s/SKILL.md", "`sprocket-scoped-context`\n");
    rename(d, "r").unwrap();
    assert_eq!(read(d, "crates/x/tests/t.rs"), cite("a/y", &old));
    assert_eq!(pending_ids(d), ["a/y"]);
}

#[test]
// req:rename/marker-rewrite@8ae3d4f6
fn code_that_still_names_a_rewritten_id_leaves_the_requirement_behind() {
    let dir = slug_repo("plugin/**");
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &read(d, "specs/a/SPEC.md").replace(
            "(`sprocket-scoped-context`)",
            "(`architecture/sprocket-scoped-context`)",
        ),
    );
    let old = version_of(d, "a/x");
    write(d, "crates/x/tests/t.rs", &cite("a/x", &old));
    write(
        d,
        "plugin/skills/s/SKILL.md",
        "see architecture/sprocket-scoped-context\n",
    );
    rename(d, "r").unwrap();
    assert_eq!(read(d, "crates/x/tests/t.rs"), cite("a/x", &old));
    assert_eq!(pending_ids(d), ["a/x"]);
}

#[test]
// req:rename/marker-rewrite@8ae3d4f6
fn a_file_holding_a_marker_of_the_requirement_holds_it_back() {
    let dir = slug_repo("crates/y/**");
    let d = dir.path();
    let old = version_of(d, "a/x");
    write(
        d,
        "scripts/t.sh",
        &format!("# {}a/x@{old}\nuse `sprocket-scoped-context`\n", "req:"),
    );
    rename(d, "r").unwrap();
    assert_eq!(pending_ids(d), ["a/x"]);
}

#[test]
// req:rename/marker-rewrite@8ae3d4f6
fn another_components_code_naming_it_does_not_hold_the_marker_back() {
    let dir = slug_repo("crates/**");
    let d = dir.path();
    write(
        d,
        "specs/b/SPEC.md",
        &node("paths: [\"plugin/**\"]\n", "design", &[("z", "Z SHALL.")]),
    );
    let old = version_of(d, "a/x");
    write(d, "crates/x/tests/t.rs", &cite("a/x", &old));
    write(d, "plugin/skills/s/SKILL.md", "`sprocket-scoped-context`\n");
    rename(d, "r").unwrap();
    let new = version_of(d, "a/x");
    assert_ne!(new, old);
    assert_eq!(read(d, "crates/x/tests/t.rs"), cite("a/x", &new));
    assert_eq!(pending_ids(d), Vec::<String>::new());
}

#[test]
// req:rename/marker-rewrite@8ae3d4f6
fn a_term_left_in_the_code_does_not_hold_the_marker_back() {
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &node(
            "paths: [\"plugin/**\"]\n",
            "design",
            &[("x", "X SHALL print the sprocket.")],
        ),
    );
    write(
        d,
        "changes/r/change.md",
        &change("  - term: sprocket\n    to: cogs\n", ""),
    );
    let old = version_of(d, "a/x");
    write(d, "crates/x/tests/t.rs", &cite("a/x", &old));
    write(d, "plugin/skills/s/SKILL.md", "the sprocket\n");
    let printed = rename(d, "r").unwrap();
    assert!(
        printed.contains(&"still names sprocket plugin/skills/s/SKILL.md:1".into()),
        "{printed:?}"
    );
    let new = version_of(d, "a/x");
    assert_eq!(read(d, "crates/x/tests/t.rs"), cite("a/x", &new));
    assert_eq!(pending_ids(d), Vec::<String>::new());
}

#[test]
// req:rename/marker-rewrite@8ae3d4f6
fn a_term_spelt_like_a_rewritten_slug_does_not_hold_the_marker_back() {
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &node(
            "paths: [\"plugin/**\"]\n",
            "design",
            &[("x", "X SHALL be slow (`slow`).")],
        ),
    );
    write(
        d,
        "specs/b/SPEC.md",
        &node("", "design", &[("slow", "S SHALL.")]),
    );
    let renames = "  - requirement: b/slow\n    to: quick\n  - term: slow\n    to: fast\n";
    write(d, "changes/r/change.md", &change(renames, ""));
    let old = version_of(d, "a/x");
    write(d, "crates/x/tests/t.rs", &cite("a/x", &old));
    write(d, "plugin/skills/s/SKILL.md", "a slow path\n");
    rename(d, "r").unwrap();
    let new = version_of(d, "a/x");
    assert_ne!(new, old);
    assert_eq!(read(d, "crates/x/tests/t.rs"), cite("a/x", &new));
}

#[test]
// req:rename/finish@90976f4b
fn the_report_is_in_record_order_and_sorted() {
    let dir = terms_repo("sprocket", "cogs");
    let d = dir.path();
    write(
        d,
        "changes/r/change.md",
        &change(
            "  - requirement: a/w\n    to: v\n  - term: sprocket\n    to: cogs\n",
            "",
        ),
    );
    write(d, "z.txt", "a/w\n");
    write(d, "a.txt", "x\nthe sprocket\na/w\n");
    assert_eq!(
        rename(d, "r").unwrap(),
        [
            "renamed a/w -> a/v",
            "renamed sprocket -> cogs",
            "still names sprocket a.txt:2",
            "still names a/w a.txt:3",
            "still names a/w z.txt:1",
        ]
    );
}

#[test]
// req:rename/refusals@8232ad18
fn a_change_that_is_not_proposed_is_refused() {
    let dir = repo();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &node("", "d", &[("x", "X SHALL.")]));
    write(
        d,
        "changes/r/change.md",
        &change("  - term: d\n    to: e\n", "").replace("proposed", "approved"),
    );
    let why = rename(d, "r").unwrap_err().to_string();
    assert!(why.contains("only a proposed change"), "{why}");
    assert!(d.join("changes/r/change.md").exists());
}

#[test]
// req:rename/refusals@8232ad18
fn a_target_that_exists_refuses_and_writes_nothing() {
    let dir = repo();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &node("", "d", &[("x", "X SHALL.")]));
    write(d, "specs/b/SPEC.md", &node("", "d", &[("y", "Y SHALL.")]));
    let source = change("  - component: a\n    to: b\n", "");
    write(d, "changes/x/change.md", &source);
    let why = rename(d, "x").unwrap_err().to_string();
    assert!(why.contains("component `b` already exists"), "{why}");
    assert_eq!(read(d, "changes/x/change.md"), source);
    assert!(d.join("specs/a/SPEC.md").exists());
    assert!(!d.join("changes/x/deltas").exists());
    assert!(!d.join("changes/archive/x").exists());
}

#[test]
// req:rename/refusals@8232ad18
fn a_move_the_model_rejects_names_its_rule_and_entry() {
    let dir = repo();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &node("", "d", &[("x", "X SHALL.")]));
    write(d, "specs/b/SPEC.md", &node("", "d", &[("y", "Y SHALL.")]));
    let source = change("  - component: a\n    to: b.a\n", "");
    write(d, "changes/x/change.md", &source);
    let why = rename(d, "x").unwrap_err().to_string();
    assert!(why.contains("model/rename-records"), "{why}");
    assert!(why.contains("entry 1"), "{why}");
    assert_eq!(read(d, "changes/x/change.md"), source);
    assert!(d.join("specs/a/SPEC.md").exists());
    assert!(!d.join("changes/x/deltas").exists());
    assert!(!d.join("changes/archive/x").exists());
}

#[test]
// req:rename/refusals@8232ad18
fn a_loaded_change_with_a_violation_is_refused() {
    let dir = repo();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &node("", "d", &[("x", "X SHALL.")]));
    let source = change("  - term: d\n    to: e\n", "gate: none\n");
    write(d, "changes/x/change.md", &source);
    let why = rename(d, "x").unwrap_err().to_string();
    assert!(why.starts_with("model/parse-changes: "), "{why}");
    assert!(why.contains("`gate` is gone"), "{why}");
    assert_eq!(read(d, "changes/x/change.md"), source);
}

#[test]
// req:rename/refusals@8232ad18
fn the_archived_record_loads_and_keeps_its_renames() {
    let dir = repo();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &node("", "d", &[("x", "X SHALL.")]));
    write(
        d,
        "changes/r/change.md",
        &change("  - requirement: a/x\n    to: y\n", ""),
    );
    rename(d, "r").unwrap();
    assert!(violations(d).is_empty(), "{:?}", violations(d));
    let archived = read(d, "changes/archive/r/change.md");
    assert!(archived.contains("renames:"), "{archived}");
    assert!(archived.contains("status: done"), "{archived}");
}

#[test]
// req:rename/refusals@8232ad18
fn a_change_without_renames_and_a_missing_name_refuse() {
    let dir = repo();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &node("", "d", &[("x", "X SHALL.")]));
    write(
        d,
        "changes/n/change.md",
        "---\nstatus: proposed\n---\n\n## Intent\nNone.\n",
    );
    let why = rename(d, "n").unwrap_err().to_string();
    assert!(why.contains("no renames"), "{why}");
    write(
        d,
        "changes/m/change.md",
        &change("  - requirement: a/nope\n    to: y\n", ""),
    );
    let why = clm_rename::rename(d, "m").unwrap_err().to_string();
    assert!(why.contains("a/nope"), "{why}");
    let why = clm_rename::rename(d, "absent").unwrap_err().to_string();
    assert!(why.contains("absent"), "{why}");
}

fn contested() -> (TempDir, String) {
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &node("", "d", &[("old", "Old SHALL hold.")]),
    );
    let source = change("  - requirement: a/old\n    to: new\n", "");
    write(d, "changes/x/change.md", &source);
    (dir, source)
}

fn contested_by(delta: &str) -> (TempDir, String) {
    let (dir, source) = contested();
    let d = dir.path();
    let base = {
        let graph = clm_model::load(d).0;
        graph.requirement("a/old").unwrap().hash.clone()
    };
    write(
        d,
        "changes/y/change.md",
        "---\nstatus: proposed\n---\n\n## Intent\nY.\n",
    );
    write(d, "changes/y/deltas/a.md", &delta.replace("BASE", &base));
    (dir, source)
}

#[test]
// req:rename/refusals@8232ad18
fn another_change_that_modifies_the_requirement_refuses() {
    let (dir, source) = contested_by(
        "---\ncomponent: a\n---\n## MODIFIED Requirements\n### Requirement: old <!-- base: BASE -->\nOld SHALL hold more.\n\n#### Scenario: holds\n- ok\n",
    );
    let d = dir.path();
    let why = rename(d, "x").unwrap_err().to_string();
    assert!(why.contains("change `y`") && why.contains("a/old"), "{why}");
    assert_eq!(read(d, "changes/x/change.md"), source);
    assert!(read(d, "specs/a/SPEC.md").contains("Requirement: old"));
    assert!(!d.join("changes/x/deltas").exists());
}

#[test]
// req:rename/refusals@8232ad18
fn another_change_that_removes_the_requirement_refuses() {
    let (dir, _) = contested_by(
        "---\ncomponent: a\n---\n## REMOVED Requirements\n### Requirement: old <!-- base: BASE -->\n",
    );
    let why = rename(dir.path(), "x").unwrap_err().to_string();
    assert!(why.contains("change `y`") && why.contains("a/old"), "{why}");
}

#[test]
// req:rename/refusals@8232ad18
fn another_change_with_a_delta_for_the_component_refuses() {
    let (dir, source) = contested();
    let d = dir.path();
    let source = source.replace("requirement: a/old\n    to: new", "component: a\n    to: b");
    write(d, "changes/x/change.md", &source);
    write(
        d,
        "changes/y/change.md",
        "---\nstatus: proposed\n---\n\n## Intent\nY.\n",
    );
    write(
        d,
        "changes/y/deltas/a.md",
        "---\ncomponent: a\n---\n## ADDED Requirements\n### Requirement: more\nMore SHALL hold.\n\n#### Scenario: holds\n- ok\n",
    );
    let why = rename(d, "x").unwrap_err().to_string();
    assert!(why.contains("change `y`") && why.contains("`a`"), "{why}");
    assert!(d.join("specs/a/SPEC.md").exists());
    assert_eq!(read(d, "changes/x/change.md"), source);
}

#[test]
// req:rename/refusals@8232ad18
fn another_change_adding_a_requirement_to_the_node_does_not_refuse() {
    let (dir, _) = contested_by(
        "---\ncomponent: a\n---\n## ADDED Requirements\n### Requirement: more\nMore SHALL hold.\n\n#### Scenario: holds\n- ok\n",
    );
    assert!(rename(dir.path(), "x").is_ok());
}

/// A repository with requirement `a/x` and a rename change `r` built from `renames`.
fn markers_repo(design: &str, renames: &str) -> TempDir {
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &node("", design, &[("x", "X SHALL print the sprocket.")]),
    );
    write(d, "changes/r/change.md", &change(renames, ""));
    dir
}

fn version_of(d: &Path, id: &str) -> String {
    clm_model::load(d)
        .0
        .requirement(id)
        .unwrap()
        .version()
        .to_string()
}

fn pending_ids(d: &Path) -> Vec<String> {
    let graph = clm_model::load(d).0;
    let scan = clm_coverage::scan(d);
    clm_coverage::pending(&graph, &scan.req)
        .into_iter()
        .filter(|p| p.node == "a")
        .map(|p| p.id)
        .collect()
}

fn cite(id: &str, version: &str) -> String {
    format!("// {}{id}@{version}\n", "req:")
}

#[test]
// req:rename/marker-rewrite@8ae3d4f6
fn current_and_behind_markers_follow_a_rename() {
    let dir = markers_repo("d", "  - requirement: a/x\n    to: y\n");
    let d = dir.path();
    let old = version_of(d, "a/x");
    let code = |id: &str, v1: &str, v2: &str| {
        format!(
            "fn keep() {{}}\n{}fn t() {{}}\n{}fn u() {{}} // other text stays\n",
            cite(id, v1),
            cite(id, v2)
        )
    };
    write(d, "crates/x/tests/t.rs", &code("a/x", &old, "00000000"));
    git(d, &["init", "-q"]);
    git(d, &["add", "-A"]);
    assert_eq!(pending_ids(d), ["a/x"]);
    clm_rename::rename(d, "r").unwrap();
    let new = version_of(d, "a/y");
    assert_eq!(
        read(d, "crates/x/tests/t.rs"),
        code("a/y", &new, "00000000")
    );
    assert_eq!(pending_ids(d), ["a/y"]);
}

#[test]
// req:rename/marker-rewrite@8ae3d4f6
fn a_behind_marker_stays_behind() {
    let dir = markers_repo("d", "  - requirement: a/x\n    to: y\n");
    let d = dir.path();
    write(d, "crates/x/tests/t.rs", &cite("a/x", "00000000"));
    git(d, &["init", "-q"]);
    git(d, &["add", "-A"]);
    clm_rename::rename(d, "r").unwrap();
    assert_eq!(read(d, "crates/x/tests/t.rs"), cite("a/y", "00000000"));
    assert_eq!(pending_ids(d), ["a/y"]);
}

#[test]
// req:rename/marker-rewrite@8ae3d4f6
fn a_reworded_requirement_keeps_its_marker_current() {
    let dir = markers_repo(
        "the sprocket of a component",
        "  - term: sprocket\n    to: cogs\n",
    );
    let d = dir.path();
    let old = version_of(d, "a/x");
    write(d, "crates/x/tests/t.rs", &cite("a/x", &old));
    git(d, &["init", "-q"]);
    git(d, &["add", "-A"]);
    clm_rename::rename(d, "r").unwrap();
    let new = version_of(d, "a/x");
    assert_ne!(new, old);
    assert_eq!(read(d, "crates/x/tests/t.rs"), cite("a/x", &new));
    assert_eq!(pending_ids(d), Vec::<String>::new());
}

#[test]
// req:rename/marker-rewrite@8ae3d4f6
fn other_requirements_markers_and_bare_markers_are_handled() {
    let dir = markers_repo("d", "  - requirement: a/x\n    to: y\n");
    let d = dir.path();
    let body = format!("// {}b/z@12345678 and {}a/x\n", "req:", "req:");
    write(d, "crates/x/tests/t.rs", &body);
    git(d, &["init", "-q"]);
    git(d, &["add", "-A"]);
    clm_rename::rename(d, "r").unwrap();
    assert_eq!(
        read(d, "crates/x/tests/t.rs"),
        format!("// {}b/z@12345678 and {}a/y\n", "req:", "req:")
    );
}

#[test]
// req:rename/refusals@8232ad18
fn an_archived_change_and_the_changes_own_old_deltas_do_not_refuse() {
    let (dir, _) = contested();
    let d = dir.path();
    let base = clm_model::load(d)
        .0
        .requirement("a/old")
        .unwrap()
        .hash
        .clone();
    let delta = format!(
        "---\ncomponent: a\n---\n## MODIFIED Requirements\n### Requirement: old <!-- base: {base} -->\nOld SHALL hold more.\n\n#### Scenario: holds\n- ok\n"
    );
    write(
        d,
        "changes/archive/y/change.md",
        "---\nstatus: done\n---\n\n## Intent\nY.\n",
    );
    write(d, "changes/archive/y/deltas/a.md", &delta);
    write(d, "changes/x/deltas/a.md", &delta);
    rename(d, "x").unwrap();
    assert!(read(d, "specs/a/SPEC.md").contains("Requirement: new"));
}

#[test]
// req:rename/refusals@8232ad18
fn an_applied_change_does_not_refuse() {
    let (dir, _) = contested_by(
        "---\ncomponent: a\n---\n## REMOVED Requirements\n### Requirement: old <!-- base: BASE -->\n",
    );
    let d = dir.path();
    write(
        d,
        "changes/y/change.md",
        "---\nstatus: applied\n---\n\n## Intent\nY.\n",
    );
    assert!(rename(d, "x").is_ok());
}

#[test]
// req:rename/refusals@8232ad18
fn a_missing_component_and_an_existing_requirement_refuse() {
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &node("", "d", &[("x", "X SHALL."), ("y", "Y SHALL.")]),
    );
    write(
        d,
        "changes/m/change.md",
        &change("  - component: zz\n    to: b\n", ""),
    );
    let why = clm_rename::rename(d, "m").unwrap_err().to_string();
    assert!(why.contains("no component `zz`"), "{why}");
    write(
        d,
        "changes/n/change.md",
        &change("  - requirement: a/x\n    to: y\n", ""),
    );
    let why = clm_rename::rename(d, "n").unwrap_err().to_string();
    assert!(why.contains("requirement `a/y` already exists"), "{why}");
}

fn flag_repo(renames: &str) -> (TempDir, String) {
    let dir = markers_repo("d", renames);
    let d = dir.path();
    let old = version_of(d, "a/x");
    write(d, "backlog/f.md", &flag(&old));
    git(d, &["init", "-q"]);
    git(d, &["add", "-A"]);
    (dir, old)
}

fn flag(version: &str) -> String {
    format!(
        "---\ntitle: \"hard\"\nkind: workaround\ncomponents: [a]\nrequirement: a/x@{version}\n---\n\nBody a/x here.\n"
    )
}

#[test]
// req:rename/flag-rerecord@d115e4a4
fn a_flag_on_a_reworded_requirement_stays_current() {
    let (dir, old) = flag_repo("  - term: sprocket\n    to: cogs\n");
    let d = dir.path();
    clm_rename::rename(d, "r").unwrap();
    let new = version_of(d, "a/x");
    assert_ne!(new, old);
    assert_eq!(read(d, "backlog/f.md"), flag(&new));
    assert!(violations(d).iter().all(|v| !v.path.starts_with("backlog")));
}

#[test]
// req:rename/flag-rerecord@d115e4a4
fn a_flag_on_a_renamed_requirement_follows_it() {
    let (dir, old) = flag_repo("  - requirement: a/x\n    to: y\n");
    let d = dir.path();
    clm_rename::rename(d, "r").unwrap();
    let new = version_of(d, "a/y");
    assert_ne!(new, old);
    let want = flag(&new).replace("requirement: a/x@", "requirement: a/y@");
    assert_eq!(read(d, "backlog/f.md"), want);
    assert!(violations(d).iter().all(|v| !v.path.starts_with("backlog")));
}

#[test]
// req:rename/flag-rerecord@d115e4a4
fn a_flag_at_an_older_version_keeps_it() {
    let (dir, _) = flag_repo("  - requirement: a/x\n    to: y\n");
    let d = dir.path();
    write(d, "backlog/f.md", &flag("00000000"));
    git(d, &["add", "-A"]);
    clm_rename::rename(d, "r").unwrap();
    let want = flag("00000000").replace("requirement: a/x@", "requirement: a/y@");
    assert_eq!(read(d, "backlog/f.md"), want);
}

fn component_repo(items: &[(&str, &str)]) -> TempDir {
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &node("", "design", &[("p", "P SHALL.")]),
    );
    write(
        d,
        "specs/a/b/SPEC.md",
        &node("", "design", &[("x", "X SHALL.")]),
    );
    write(
        d,
        "specs/d/SPEC.md",
        &node("depends_on: [a.b]\n", "design", &[("z", "Z SHALL.")]),
    );
    for (name, fm) in items {
        write(
            d,
            &format!("backlog/{name}.md"),
            &format!("---\ntitle: T\nkind: idea\n{fm}\n---\n\nbody\n"),
        );
    }
    write(
        d,
        "changes/r/change.md",
        &change("  - component: a.b\n    to: a.c\n", ""),
    );
    dir
}

#[test]
// req:rename/component-key@82737665
fn deltas_name_their_component() {
    let dir = component_repo(&[]);
    let d = dir.path();
    rename(d, "r").unwrap();
    let deltas = fs::read_dir(d.join("changes/archive/r/deltas")).unwrap();
    let mut seen = 0;
    for entry in deltas {
        let text = fs::read_to_string(entry.unwrap().path()).unwrap();
        let head = text.split("\n---\n").next().unwrap();
        assert!(head.contains("\ncomponent: "), "{head}");
        assert!(!head.contains("\nnode:"), "{head}");
        seen += 1;
    }
    assert!(seen >= 3);
}

#[test]
// req:rename/component-key@82737665
fn items_by_the_components_key() {
    let dir = component_repo(&[("x", "components: [a.b]")]);
    let d = dir.path();
    rename(d, "r").unwrap();
    assert!(read(d, "backlog/x.md").contains("components: [a.c]"));
}
