//! `land/derived-specs`: a branch that adds a change lands only when its `specs/` is what
//! regenerating from the base and the change's deltas gives.

mod common;

use clm_ledger::Body;
use common::{Fixture, refused};

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n";

fn requirement(slug: &str) -> String {
    format!("\n### Requirement: {slug}\nIt SHALL {slug}.\n\n#### Scenario: s\n- ok\n")
}

fn node(title: &str, reqs: &[&str]) -> String {
    let mut s = format!(
        "---\ntitle: {title}\nstatus: specd\nowners: []\n---\n\n## Contract\nc\n\n\
         ## Design\nd\n\n## Requirements\n"
    );
    for r in reqs {
        s.push_str(&requirement(r));
    }
    s
}

/// `main` (on `origin` too) with nodes `a` (requirement `p`), and `spec/x` adding change
/// `x`, applied, whose delta adds `a/r`, with `specs/` as applying it gives. The work clone
/// is on `main` after.
fn spec_branch(f: &Fixture) {
    let work = f.work();
    f.commit_files(
        &[
            ("specs/SPEC.md", ROOT),
            ("specs/a/SPEC.md", &node("A", &["p"])),
        ],
        "nodes",
    );
    f.git(&work, &["push", "-q", "origin", "main"]);
    f.git(&work, &["checkout", "-q", "-b", "spec/x"]);
    f.commit_files(
        &[
            (
                "changes/x/change.md",
                "---\nstatus: proposed\n---\n\n## Intent\nAdd r.\n",
            ),
            (
                "changes/x/deltas/a.md",
                &format!(
                    "---\ncomponent: a\n---\n## ADDED Requirements\n{}",
                    requirement("r")
                ),
            ),
        ],
        "propose x",
    );
    clm_apply::apply(&work, "x").unwrap();
    f.git(&work, &["add", "-A"]);
    f.git(&work, &["commit", "-q", "-m", "apply x"]);
    f.git(&work, &["push", "-q", "origin", "spec/x"]);
    f.git(&work, &["checkout", "-q", "main"]);
}

#[test]
fn scenario_regenerated_branch_lands() {
    // req:land/derived-specs@eb99146d
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    spec_branch(&f);
    f.advance_main("specs/b/SPEC.md", &node("B", &["s"]));
    let out = clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    let main = f.remote("main");
    assert_eq!(out.lines, [format!("landed {main} spec")]);
    let origin = f.path("origin.git");
    let a = f.git(&origin, &["show", &format!("{main}:specs/a/SPEC.md")]);
    assert!(a.contains("Requirement: r"), "{a}");
    let b = f.git(&origin, &["show", &format!("{main}:specs/b/SPEC.md")]);
    assert!(b.contains("Requirement: s"), "{b}");
}

fn design_hash(work: &std::path::Path) -> String {
    let graph = clm_model::load(work).0;
    clm_apply::hashes(&graph.nodes["a"])
        .into_iter()
        .find(|(e, _)| e == "design")
        .unwrap()
        .1
}

fn design_delta(base: &str, text: &str) -> String {
    format!("---\ncomponent: a\nbase: {{ design: \"{base}\" }}\n---\n## MODIFIED Design\n{text}\n")
}

#[test]
fn scenario_change_built_on_another_lands_whatever_its_id() {
    // req:land/derived-specs@eb99146d
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    let work = f.work();
    let a = node("A", &["p"]).replace("## Design\nd", "## Design\none\ntwo\nthree");
    f.commit_files(&[("specs/SPEC.md", ROOT), ("specs/a/SPEC.md", &a)], "nodes");
    f.git(&work, &["push", "-q", "origin", "main"]);
    f.git(&work, &["checkout", "-q", "-b", "spec/x"]);
    let apply = |id: &str, text: &str| {
        let delta = design_delta(&design_hash(&work), text);
        f.commit_files(
            &[
                (
                    &format!("changes/{id}/change.md"),
                    "---\nstatus: proposed\n---\n\n## Intent\nEdit.\n",
                ),
                (&format!("changes/{id}/deltas/a.md"), &delta),
            ],
            id,
        );
        clm_apply::apply(&work, id).unwrap();
        f.git(&work, &["add", "-A"]);
        f.git(&work, &["commit", "-q", "-m", &format!("apply {id}")]);
    };
    apply("x", "X1\ntwo\nthree");
    apply("w", "X1\nW2\nthree");
    f.git(&work, &["push", "-q", "origin", "spec/x"]);
    f.git(&work, &["checkout", "-q", "main"]);
    let out = clm_land::land(&work, Some("spec/x"), &f.env(&[])).unwrap();
    let main = f.remote("main");
    assert_eq!(out.lines, [format!("landed {main} spec")]);
    let a = f.git(
        &f.path("origin.git"),
        &["show", &format!("{main}:specs/a/SPEC.md")],
    );
    assert!(a.contains("X1\nW2\nthree"), "{a}");
}

#[test]
fn scenario_hand_edit_under_specs_refused() {
    // req:land/derived-specs@eb99146d
    let f = Fixture::new("");
    f.write("gate-out", "spec review\n");
    f.write("needs-approval", "");
    spec_branch(&f);
    let work = f.work();
    f.git(&work, &["checkout", "-q", "spec/x"]);
    let spec = std::fs::read_to_string(work.join("specs/a/SPEC.md")).unwrap();
    f.commit_files(
        &[(
            "specs/a/SPEC.md",
            &spec.replace("It SHALL p.", "It SHALL p, by hand."),
        )],
        "hand edit",
    );
    f.git(&work, &["push", "-q", "origin", "spec/x"]);
    f.git(&work, &["checkout", "-q", "main"]);
    let fp = f.fingerprint("spec/x");
    f.append(
        &work,
        Body::Approved {
            fingerprint: fp,
            branch: "spec/x".into(),
            change: Some("x".into()),
        },
    );
    let old = f.remote("main");
    let why = refused(clm_land::land(&work, Some("spec/x"), &f.env(&[])));
    assert!(why.contains("land/derived-specs"), "{why}");
    assert!(why.contains("specs/a/SPEC.md"), "{why}");
    assert!(why.contains("clm rebase"), "{why}");
    assert!(f.lines("check.log").is_empty(), "the check never ran");
    assert!(f.lines("clm-check.log").is_empty(), "clm check never ran");
    assert_eq!(f.remote("main"), old);
}

#[test]
fn a_change_that_does_not_apply_is_refused() {
    // req:land/derived-specs@eb99146d
    // `x` adds `a/p`, which `a` already holds: regenerating it fails.
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    spec_branch(&f);
    let work = f.work();
    f.git(&work, &["checkout", "-q", "spec/x"]);
    f.commit_files(
        &[(
            "changes/x/deltas/a.md",
            &format!(
                "---\ncomponent: a\n---\n## ADDED Requirements\n{}",
                requirement("p")
            ),
        )],
        "delta clashes",
    );
    f.git(&work, &["push", "-q", "origin", "spec/x"]);
    f.git(&work, &["checkout", "-q", "main"]);
    let old = f.remote("main");
    let why = refused(clm_land::land(&work, Some("spec/x"), &f.env(&[])));
    assert!(why.contains("land/derived-specs"), "{why}");
    assert!(why.contains("clm rebase"), "{why}");
    // The violations are named, as `clm check` prints them.
    assert!(why.contains("changes/x/deltas/a.md: apply/"), "{why}");
    assert!(f.lines("check.log").is_empty(), "the check never ran");
    assert_eq!(f.remote("main"), old);
}

#[test]
fn scenario_branch_that_adds_no_change() {
    // req:land/derived-specs@eb99146d
    let f = Fixture::new("");
    f.commit_files(
        &[
            ("specs/SPEC.md", ROOT),
            ("specs/a/SPEC.md", &node("A", &["p"])),
        ],
        "nodes",
    );
    f.git(&f.work(), &["push", "-q", "origin", "main"]);
    let active = node("A", &["p"]).replace("status: specd", "status: active");
    f.branch("impl/a", &[("specs/a/SPEC.md", &active)]);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    let main = f.remote("main");
    assert_eq!(out.lines, [format!("landed {main} code")]);
    let a = f.git(
        &f.path("origin.git"),
        &["show", &format!("{main}:specs/a/SPEC.md")],
    );
    assert!(a.contains("status: active"), "{a}");
}

#[test]
fn a_proposed_change_regenerates_nothing() {
    // req:land/derived-specs@eb99146d
    // Only an applied change is applied; a proposed one's `specs/` stay the base's.
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    f.commit_files(
        &[
            ("specs/SPEC.md", ROOT),
            ("specs/a/SPEC.md", &node("A", &["p"])),
        ],
        "nodes",
    );
    f.git(&f.work(), &["push", "-q", "origin", "main"]);
    f.branch(
        "spec/x",
        &[
            (
                "changes/x/change.md",
                "---\nstatus: proposed\n---\n\n## Intent\nAdd r.\n",
            ),
            (
                "changes/x/deltas/a.md",
                &format!(
                    "---\ncomponent: a\n---\n## ADDED Requirements\n{}",
                    requirement("r")
                ),
            ),
        ],
    );
    let out = clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    assert_eq!(out.lines, [format!("landed {} spec", f.remote("main"))]);
}

/// Commits `files` on `spec/x` and pushes it; the work clone is back on `main` after.
fn on_spec_branch(f: &Fixture, edit: impl FnOnce(&std::path::Path)) {
    let work = f.work();
    f.git(&work, &["checkout", "-q", "spec/x"]);
    edit(&work);
    f.git(&work, &["add", "-A"]);
    f.git(&work, &["commit", "-q", "-m", "edit"]);
    f.git(&work, &["push", "-q", "origin", "spec/x"]);
    f.git(&work, &["checkout", "-q", "main"]);
}

/// Lands `spec/x`, expecting `land/derived-specs` naming `path`, and `main` unchanged.
fn refused_naming(f: &Fixture, path: &str) {
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])));
    assert!(why.contains("land/derived-specs"), "{why}");
    assert!(why.contains(path), "{why}");
    assert!(f.lines("check.log").is_empty(), "the check never ran");
    assert_eq!(f.remote("main"), old);
}

#[test]
fn a_new_archived_change_done_is_regenerated() {
    // req:land/derived-specs@eb99146d
    // A branch that adds change `x` straight to the archive, `done`, regenerates it.
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    spec_branch(&f);
    on_spec_branch(&f, |work| {
        let live = work.join("changes/x");
        let archived = work.join("changes/archive/x");
        std::fs::create_dir_all(archived.parent().unwrap()).unwrap();
        std::fs::rename(&live, &archived).unwrap();
        let md = archived.join("change.md");
        let text = std::fs::read_to_string(&md).unwrap();
        std::fs::write(&md, text.replace("status: applied", "status: done")).unwrap();
    });
    f.advance_main("specs/b/SPEC.md", &node("B", &["s"]));
    let out = clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    assert_eq!(out.lines, [format!("landed {} spec", f.remote("main"))]);
}

#[test]
fn a_file_added_by_hand_under_specs_refused() {
    // req:land/derived-specs@eb99146d
    // A path on one side only is a difference.
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    spec_branch(&f);
    on_spec_branch(&f, |work| {
        std::fs::write(work.join("specs/a/notes.md"), "by hand\n").unwrap();
    });
    refused_naming(&f, "specs/a/notes.md");
}

#[cfg(unix)]
#[test]
fn a_file_with_a_name_that_is_not_utf8_refused() {
    // req:land/derived-specs@eb99146d
    use std::os::unix::ffi::OsStrExt;
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    spec_branch(&f);
    on_spec_branch(&f, |work| {
        let name = std::ffi::OsStr::from_bytes(b"caf\xe9.md");
        std::fs::write(work.join("specs/a").join(name), "by hand\n").unwrap();
    });
    refused_naming(&f, "specs/a/caf");
}

#[cfg(unix)]
#[test]
fn a_symlink_in_place_of_a_regenerated_file_refused() {
    // req:land/derived-specs@eb99146d
    // The link points at a file holding the regenerated bytes; git stores the link itself.
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    spec_branch(&f);
    on_spec_branch(&f, |work| {
        let spec = work.join("specs/a/SPEC.md");
        std::fs::copy(&spec, work.join("specs/copy.md")).unwrap();
        std::fs::remove_file(&spec).unwrap();
        std::os::unix::fs::symlink("../copy.md", &spec).unwrap();
    });
    refused_naming(&f, "specs/a/SPEC.md");
}

#[test]
fn scenario_no_copy_of_the_regenerate() {
    // req:land/derived-specs@eb99146d
    // The regenerate is `sync.regenerate`'s alone, taken from `clm-regenerate`, never through
    // `clm-sync` (`clm/health-isolated`).
    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut stack = vec![crate_dir.join("src")];
    let mut read = 0;
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            read += 1;
            for copy in ["clm_apply::reapply", "clm_model::load"] {
                assert!(!text.contains(copy), "{} holds {copy}", path.display());
            }
        }
    }
    assert!(read > 0, "read no source");
    let manifest = std::fs::read_to_string(crate_dir.join("Cargo.toml")).unwrap();
    let manifest: toml::Table = manifest.parse().unwrap();
    let deps = manifest["dependencies"].as_table().unwrap();
    assert!(deps.contains_key("clm-regenerate"), "{deps:?}");
    assert!(!deps.contains_key("clm-sync"), "{deps:?}");
}

/// `main` holds change `y` with `base_status` and a delta adding `a/x`, with `specs/` not yet
/// holding `a/x`. `spec/z` carries `branch` files (and none of `y`'s deltas unless named).
fn y_on_main(f: &Fixture, base_status: &str) {
    f.commit_files(
        &[
            ("specs/SPEC.md", ROOT),
            ("specs/a/SPEC.md", &node("A", &["p"])),
            (
                "changes/y/change.md",
                &format!("---\nstatus: {base_status}\n---\n\n## Intent\nAdd x.\n"),
            ),
            (
                "changes/y/deltas/a.md",
                &format!(
                    "---\ncomponent: a\n---\n## ADDED Requirements\n{}",
                    requirement("x")
                ),
            ),
        ],
        "main with y",
    );
    f.git(&f.work(), &["push", "-q", "origin", "main"]);
}

fn land_z(f: &Fixture) -> Result<clm_land::Output, clm_land::Error> {
    clm_land::land(&f.work(), Some("spec/z"), &f.env(&[]))
}

fn applied_y() -> (&'static str, &'static str) {
    (
        "changes/y/change.md",
        "---\nstatus: applied\n---\n\n## Intent\nAdd x.\n",
    )
}

#[test]
fn scenario_applied_changes_deltas_lost() {
    // req:land/derived-specs@eb99146d
    // `spec/z` adds `z` (no delta) and sets `y` applied without `specs/` gaining `a/x`.
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    y_on_main(&f, "proposed");
    f.branch(
        "spec/z",
        &[
            (
                "changes/z/change.md",
                "---\nstatus: proposed\n---\n\n## Intent\nNothing.\n",
            ),
            applied_y(),
        ],
    );
    let old = f.remote("main");
    let why = refused(land_z(&f));
    assert!(why.contains("land/derived-specs"), "{why}");
    assert!(why.contains("specs/a/SPEC.md"), "{why}");
    assert!(why.contains("clm rebase"), "{why}");
    assert_eq!(f.remote("main"), old);
}

#[test]
fn a_head_that_only_applies_a_change_is_checked() {
    // req:land/derived-specs@eb99146d
    // Nothing is added; an `approved` change is applied, `specs/` lacking `a/x`.
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    y_on_main(&f, "approved");
    f.branch("spec/z", &[applied_y()]);
    let why = refused(land_z(&f));
    assert!(why.contains("land/derived-specs"), "{why}");
    assert!(why.contains("specs/a/SPEC.md"), "{why}");
}

#[test]
fn a_regenerated_applied_change_lands() {
    // req:land/derived-specs@eb99146d
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    y_on_main(&f, "proposed");
    let with_x = node("A", &["p", "x"]);
    f.branch("spec/z", &[applied_y(), ("specs/a/SPEC.md", &with_x)]);
    let out = land_z(&f).unwrap();
    assert_eq!(out.lines, [format!("landed {} spec", f.remote("main"))]);
}

#[test]
fn a_change_archived_done_on_the_branch_is_checked() {
    // req:land/derived-specs@eb99146d
    // `y` is gone from `changes/` and `done` in the archive with its deltas, `specs/` lacking
    // `a/x`.
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    y_on_main(&f, "proposed");
    let delta = format!(
        "---\ncomponent: a\n---\n## ADDED Requirements\n{}",
        requirement("x")
    );
    f.branch(
        "spec/z",
        &[
            (
                "changes/archive/y/change.md",
                "---\nstatus: done\n---\n\n## Intent\nAdd x.\n",
            ),
            ("changes/archive/y/deltas/a.md", &delta),
        ],
    );
    let work = f.work();
    f.git(&work, &["checkout", "-q", "spec/z"]);
    f.git(&work, &["rm", "-q", "-r", "changes/y"]);
    f.git(&work, &["commit", "-q", "-m", "archive y"]);
    f.git(&work, &["push", "-q", "origin", "spec/z"]);
    f.git(&work, &["checkout", "-q", "main"]);
    let why = refused(land_z(&f));
    assert!(why.contains("land/derived-specs"), "{why}");
    assert!(why.contains("specs/a/SPEC.md"), "{why}");
}

#[test]
fn a_change_already_applied_on_main_is_left_alone() {
    // req:land/derived-specs@eb99146d
    // `y` is `applied` on `main` (its deltas are in `specs/`); a branch touching its text
    // applies nothing, so no regenerate runs and a hand edit of `specs/` is the branch's own.
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    y_on_main(&f, "applied");
    let edited = node("A", &["p"]).replace("It SHALL p.", "It SHALL p, edited.");
    f.branch("spec/z", &[applied_y(), ("specs/a/SPEC.md", &edited)]);
    let out = land_z(&f).unwrap();
    assert_eq!(out.lines, [format!("landed {} spec", f.remote("main"))]);
}
