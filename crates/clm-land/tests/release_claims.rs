//! A landed change releases the claims on its items (land/landed-change-releases-claims).

mod common;

use common::Fixture;

const CHANGE: &str =
    "---\nstatus: proposed\nfrom: backlog/q\nresolves: [r]\n---\n\n## Intent\n\nx\n";

fn items(f: &Fixture) {
    f.commit_files(
        &[
            ("backlog/q.md", "q\n"),
            ("backlog/r.md", "r\n"),
            ("backlog/s.md", "s\n"),
        ],
        "items",
    );
    f.git(&f.work(), &["push", "-q", "origin", "main"]);
    for slug in ["q", "r", "s"] {
        f.branch(
            &format!("propose/{slug}"),
            &[(&format!("p-{slug}.txt"), "p\n")],
        );
    }
}

fn on_origin(f: &Fixture, slug: &str) -> bool {
    !f.git(
        &f.path("origin.git"),
        &["branch", "--list", &format!("propose/{slug}")],
    )
    .is_empty()
}

fn released(out: &clm_land::Output) -> Vec<&String> {
    out.warnings
        .iter()
        .filter(|w| w.starts_with("released"))
        .collect()
}

#[test]
fn claims_on_the_changes_items_released() {
    // req:land/landed-change-releases-claims@0e64ba8a
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    items(&f);
    f.branch("spec/c", &[("changes/c/change.md", CHANGE)]);
    let out = clm_land::land(&f.work(), Some("spec/c"), &f.env(&[])).unwrap();
    assert_eq!(out.lines, [format!("landed {} spec", f.remote("main"))]);
    assert!(!on_origin(&f, "q"));
    assert!(!on_origin(&f, "r"));
    assert!(on_origin(&f, "s"));
    assert_eq!(
        released(&out),
        ["released propose/q", "released propose/r"],
        "{:?}",
        out.warnings
    );
}

#[test]
fn moved_claim_kept() {
    // req:land/landed-change-releases-claims@0e64ba8a
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    items(&f);
    f.branch("spec/c", &[("changes/c/change.md", CHANGE)]);
    f.write("move-head", "propose/q");
    let out = clm_land::land(&f.work(), Some("spec/c"), &f.env(&[])).unwrap();
    let moved = f.lines("head-moved").remove(0);
    assert_eq!(f.remote("propose/q"), moved);
    assert!(
        out.warnings
            .iter()
            .any(|w| w.contains("propose/q") && !w.starts_with("released")),
        "{:?}",
        out.warnings
    );
    assert!(!on_origin(&f, "r"));
}

#[test]
fn an_edit_of_a_landed_change_releases_nothing() {
    // req:land/landed-change-releases-claims@0e64ba8a
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    items(&f);
    f.commit_files(&[("changes/c/change.md", CHANGE)], "change c");
    f.git(&f.work(), &["push", "-q", "origin", "main"]);
    f.branch(
        "spec/d",
        &[("changes/c/change.md", &format!("{CHANGE}more\n"))],
    );
    let out = clm_land::land(&f.work(), Some("spec/d"), &f.env(&[])).unwrap();
    assert!(on_origin(&f, "q"));
    assert!(on_origin(&f, "r"));
    assert!(released(&out).is_empty());
}

#[test]
fn a_code_land_releases_nothing() {
    // req:land/landed-change-releases-claims@0e64ba8a
    let f = Fixture::new("");
    // Even a gate that calls the land a spec land: the branch isn't `spec/*`.
    f.write("gate-out", "spec\n");
    items(&f);
    f.branch("impl/a", &[("changes/c/change.md", CHANGE)]);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(on_origin(&f, "q"));
    assert!(released(&out).is_empty());
}
