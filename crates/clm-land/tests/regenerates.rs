//! `land/regenerates-behind-specs`: a branch that adds or applies a change, behind `main` in a
//! file of `specs/` that both sides changed, lands without a `clm rebase` round: its `specs/`
//! is regenerated onto `main`.

mod common;

use clm_ledger::Body;
use common::{Fixture, refused, write_exec};

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

/// `main` (on `origin` too) with node `a` (requirement `p`), and `spec/x` adding change `x`,
/// applied, whose delta adds `a/r`, with `specs/` as applying it gives. The work clone is on
/// `main` after.
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

/// The file `path` at `main` on `origin`.
fn on_main(f: &Fixture, path: &str) -> String {
    let main = f.remote("main");
    f.git(&f.path("origin.git"), &["show", &format!("{main}:{path}")])
}

#[test]
fn scenario_behind_in_a_derived_file_lands() {
    // req:land/regenerates-behind-specs@e2612b11
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    spec_branch(&f);
    // `main` lands a change of its own to the same file, on the same lines.
    f.advance_main("specs/a/SPEC.md", &node("A", &["p", "s"]));
    let head = f.rev("spec/x");
    let out = clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    let main = f.remote("main");
    assert_eq!(out.lines, [format!("landed {main} spec")]);
    let a = on_main(&f, "specs/a/SPEC.md");
    assert!(a.contains("Requirement: r"), "{a}");
    assert!(a.contains("Requirement: s"), "{a}");
    assert!(a.contains("Requirement: p"), "{a}");
    assert!(!a.contains("<<<<<<<"), "{a}");
    // Both gate runs were on the merge commit, which contains `main`, so neither can have
    // said the branch is behind it.
    let gates: Vec<String> = f
        .lines("gate.log")
        .iter()
        .map(|l| l.split(' ').next().unwrap().to_string())
        .collect();
    assert_eq!(gates, [main.clone(), main.clone()]);
    let message = f.git(&f.path("origin.git"), &["log", "-1", "--format=%B", &main]);
    assert!(message.contains("Change: x"), "{message}");
    assert_eq!(f.parents(&f.path("origin.git"), &main)[1], head);
}

#[test]
fn scenario_main_moves_during_the_check() {
    // req:land/regenerates-behind-specs@e2612b11
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    f.write("a-spec-moved", &node("A", &["p", "s"]));
    // A check that, the first time it runs, lands a change to `specs/a/SPEC.md` on `main`.
    let check = f.path("bin/check-moves");
    let dir = f.path("");
    write_exec(
        &check,
        &format!(
            "#!/usr/bin/env bash\nDIR='{}'\necho run >> \"$DIR/runs.log\"\n\
             if [ ! -e \"$DIR/moved\" ]; then\n  touch \"$DIR/moved\"\n  cd \"$DIR/other\"\n  \
             git pull -q --ff-only origin main\n  cp \"$DIR/a-spec-moved\" specs/a/SPEC.md\n  \
             git add -A\n  git commit -q -m 'main moved'\n  git push -q origin HEAD:main\nfi\n",
            dir.display()
        ),
    );
    let toml = format!("[land]\ncheck = \"{}\"\n", check.display());
    f.commit_files(&[("clm.toml", &toml)], "check that moves main");
    f.git(&f.work(), &["push", "-q", "origin", "main"]);
    spec_branch(&f);
    let out = clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    let main = f.remote("main");
    assert_eq!(out.lines, [format!("landed {main} spec")]);
    let a = on_main(&f, "specs/a/SPEC.md");
    assert!(a.contains("Requirement: r"), "{a}");
    assert!(a.contains("Requirement: s"), "{a}");
    // The first attempt checked this code; the second didn't check it again.
    assert_eq!(f.lines("runs.log").len(), 1);
}

#[test]
fn scenario_hand_edit_on_a_behind_branch_refused() {
    // req:land/regenerates-behind-specs@e2612b11
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
    f.advance_main("specs/a/SPEC.md", &node("A", &["p", "s"]));
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
    assert_eq!(f.remote("main"), old);
}

fn design_hash(work: &std::path::Path) -> String {
    let graph = clm_model::load(work).0;
    clm_apply::hashes(&graph.nodes["a"])
        .into_iter()
        .find(|(e, _)| e == "design")
        .unwrap()
        .1
}

#[test]
fn scenario_delta_stale_against_main_refused() {
    // req:land/regenerates-behind-specs@e2612b11
    // `x` rewrites `a`'s Design with its base's hash; `main` has since rewritten it too.
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
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
    let delta = format!(
        "---\ncomponent: a\nbase: {{ design: \"{}\" }}\n---\n## MODIFIED Design\nfrom x\n",
        design_hash(&work)
    );
    f.commit_files(
        &[
            (
                "changes/x/change.md",
                "---\nstatus: proposed\n---\n\n## Intent\nEdit.\n",
            ),
            ("changes/x/deltas/a.md", &delta),
        ],
        "propose x",
    );
    clm_apply::apply(&work, "x").unwrap();
    f.git(&work, &["add", "-A"]);
    f.git(&work, &["commit", "-q", "-m", "apply x"]);
    f.git(&work, &["push", "-q", "origin", "spec/x"]);
    f.git(&work, &["checkout", "-q", "main"]);
    let moved = node("A", &["p"]).replace("## Design\nd", "## Design\nfrom main");
    f.advance_main("specs/a/SPEC.md", &moved);
    let old = f.remote("main");
    let why = refused(clm_land::land(&work, Some("spec/x"), &f.env(&[])));
    assert!(why.contains("land/derived-specs"), "{why}");
    assert!(why.contains("clm rebase"), "{why}");
    assert!(f.lines("check.log").is_empty(), "the check never ran");
    assert_eq!(f.remote("main"), old);
}

#[test]
fn scenario_branch_that_adds_no_change_still_refused() {
    // req:land/regenerates-behind-specs@e2612b11
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
    f.advance_main("specs/a/SPEC.md", &node("A", &["p", "s"]));
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    assert!(why.contains("clm rebase"), "{why}");
    assert_eq!(f.remote("main"), old);
}
