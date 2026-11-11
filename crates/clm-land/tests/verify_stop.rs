//! A code land waits for the fix of a component whose slow tests broke (land/verify-stop).

mod common;

use clm_ledger::{Body, VerifyMode};
use common::{Fixture, refused};

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n";
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn node(title: &str, paths: &str) -> String {
    format!(
        "---\ntitle: {title}\nstatus: specd\nowners: []\npaths: [\"{paths}\"]\n---\n\n\
         ## Contract\nc\n\n## Design\nd\n\n## Requirements\n"
    )
}

fn item(components: &str, fails: &str) -> String {
    format!(
        "---\ntitle: Slow tests broke\nkind: bug\ncomponents: [{components}]\n\
         verify_failures: [{fails}]\n---\nThe slow tests of the component fail.\n"
    )
}

/// `main`, on `origin` too, holding components `a` and `b` and `extra` files.
fn repo(extra: &[(&str, &str)]) -> Fixture {
    let f = Fixture::new("");
    let mut files = vec![
        ("specs/SPEC.md", ROOT.to_string()),
        ("specs/a/SPEC.md", node("A", "crates/a/**")),
        ("specs/b/SPEC.md", node("B", "crates/b/**")),
    ];
    files.extend(extra.iter().map(|(p, t)| (*p, t.to_string())));
    let files: Vec<(&str, &str)> = files.iter().map(|(p, t)| (*p, t.as_str())).collect();
    f.commit_files(&files, "components");
    f.git(&f.work(), &["push", "-q", "origin", "main"]);
    f
}

fn verified(f: &Fixture, commit: &str, mode: VerifyMode) {
    f.append(
        &f.work(),
        Body::Verified {
            commit: commit.to_string(),
            passed: false,
            since: None,
            lands: vec![],
            culprit: None,
            components: vec!["b".to_string()],
            mode,
            run: None,
        },
    );
}

#[test]
fn a_held_component_is_refused() {
    // req:land/verify-stop@72bed87d
    let f = repo(&[("backlog/b-broke.md", &item("b", &format!("\"{COMMIT}\"")))]);
    f.branch("impl/b", &[("crates/b/src/lib.rs", "b\n")]);
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("impl/b"), &f.env(&[])));
    assert!(why.contains("land/verify-stop"), "{why}");
    assert!(why.contains("`b`"), "{why}");
    assert!(why.contains("b-broke"), "{why}");
    assert!(why.contains("Resolves: backlog/"), "{why}");
    assert_eq!(f.remote("main"), old);
    assert!(f.lines("check.log").is_empty());
}

#[test]
fn other_components_land() {
    // req:land/verify-stop@72bed87d
    let f = repo(&[("backlog/b-broke.md", &item("b", &format!("\"{COMMIT}\"")))]);
    f.branch("impl/a", &[("crates/a/src/lib.rs", "a\n")]);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
    assert!(out.warnings.iter().all(|w| !w.contains("slow tests")));
}

#[test]
fn only_a_bug_holds_a_component() {
    // req:land/verify-stop@72bed87d
    let text = item("b", &format!("\"{COMMIT}\"")).replace("kind: bug", "kind: deferred");
    let f = repo(&[("backlog/b-broke.md", &text)]);
    f.branch("impl/b", &[("crates/b/src/lib.rs", "b\n")]);
    let out = clm_land::land(&f.work(), Some("impl/b"), &f.env(&[])).unwrap();
    assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
}

#[test]
fn an_item_without_failures_holds_nothing() {
    // req:land/verify-stop@72bed87d
    let f = repo(&[("backlog/b-broke.md", &item("b", ""))]);
    f.branch("impl/b", &[("crates/b/src/lib.rs", "b\n")]);
    let out = clm_land::land(&f.work(), Some("impl/b"), &f.env(&[])).unwrap();
    assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
}

#[test]
fn the_fix_lands() {
    // req:land/verify-stop@72bed87d
    let f = repo(&[("backlog/b-broke.md", &item("b", &format!("\"{COMMIT}\"")))]);
    let work = f.work();
    f.git(&work, &["checkout", "-q", "-b", "impl/b", "main"]);
    std::fs::create_dir_all(work.join("crates/b/src")).unwrap();
    std::fs::write(work.join("crates/b/src/lib.rs"), "fixed\n").unwrap();
    f.git(&work, &["rm", "-q", "backlog/b-broke.md"]);
    f.git(&work, &["add", "-A"]);
    f.git(
        &work,
        &["commit", "-q", "-m", "Fix b\n\nResolves: backlog/b-broke"],
    );
    f.git(&work, &["push", "-q", "origin", "impl/b"]);
    f.git(&work, &["checkout", "-q", "main"]);
    let out = clm_land::land(&work, Some("impl/b"), &f.env(&[])).unwrap();
    assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
}

#[test]
fn deleting_the_item_without_its_trailer_is_not_the_fix() {
    // req:land/verify-stop@72bed87d
    let f = repo(&[("backlog/b-broke.md", &item("b", &format!("\"{COMMIT}\"")))]);
    let work = f.work();
    f.git(&work, &["checkout", "-q", "-b", "impl/b", "main"]);
    std::fs::create_dir_all(work.join("crates/b/src")).unwrap();
    std::fs::write(work.join("crates/b/src/lib.rs"), "b\n").unwrap();
    f.git(&work, &["rm", "-q", "backlog/b-broke.md"]);
    f.git(&work, &["add", "-A"]);
    f.git(&work, &["commit", "-q", "-m", "Quietly drop the item"]);
    f.git(&work, &["push", "-q", "origin", "impl/b"]);
    f.git(&work, &["checkout", "-q", "main"]);
    let why = refused(clm_land::land(&work, Some("impl/b"), &f.env(&[])));
    assert!(why.contains("land/verify-stop"), "{why}");
}

#[test]
fn another_items_trailer_is_not_the_fix() {
    // req:land/verify-stop@72bed87d
    let f = repo(&[("backlog/b-broke.md", &item("b", &format!("\"{COMMIT}\"")))]);
    let work = f.work();
    f.git(&work, &["checkout", "-q", "-b", "impl/b", "main"]);
    f.commit_files(
        &[("crates/b/src/lib.rs", "b\n")],
        "Fix\n\nResolves: backlog/other",
    );
    f.git(&work, &["push", "-q", "origin", "impl/b"]);
    f.git(&work, &["checkout", "-q", "main"]);
    let why = refused(clm_land::land(&work, Some("impl/b"), &f.env(&[])));
    assert!(why.contains("land/verify-stop"), "{why}");
}

#[test]
fn an_untriaged_red_warns() {
    // req:land/verify-stop@72bed87d
    let f = repo(&[]);
    let red = f.rev("origin/main");
    verified(&f, &red, VerifyMode::Local);
    f.branch("impl/b", &[("crates/b/src/lib.rs", "b\n")]);
    let out = clm_land::land(&f.work(), Some("impl/b"), &f.env(&[])).unwrap();
    assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
    let warning = out
        .warnings
        .iter()
        .find(|w| w.starts_with("slow tests failed after "))
        .unwrap_or_else(|| panic!("{:?}", out.warnings));
    assert!(warning.contains(&red), "{warning}");
    assert!(warning.contains("culprit unknown"), "{warning}");
    assert!(warning.contains(" on b;"), "{warning}");
    assert!(warning.contains("/clm:triage"), "{warning}");
    assert_eq!(
        out.warnings.iter().filter(|w| w.contains(&red)).count(),
        1,
        "{:?}",
        out.warnings
    );
}

#[test]
fn a_red_for_other_components_does_not_warn() {
    // req:land/verify-stop@72bed87d
    let f = repo(&[]);
    verified(&f, &f.rev("origin/main"), VerifyMode::Local);
    f.branch("impl/a", &[("crates/a/src/lib.rs", "a\n")]);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(out.warnings.iter().all(|w| !w.contains("slow tests")));
}

#[test]
fn a_red_record_alone_holds_nothing() {
    // req:land/verify-stop@72bed87d
    let f = repo(&[]);
    verified(&f, &f.rev("origin/main"), VerifyMode::Pipeline);
    f.branch("impl/b", &[("crates/b/src/lib.rs", "b\n")]);
    let out = clm_land::land(&f.work(), Some("impl/b"), &f.env(&[])).unwrap();
    assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
}

#[test]
fn a_red_an_item_took_in_does_not_warn_twice() {
    // req:land/verify-stop@72bed87d
    let f = repo(&[]);
    let red = f.rev("origin/main");
    verified(&f, &red, VerifyMode::Local);
    f.commit_files(
        &[("backlog/b-broke.md", &item("a", &format!("\"{red}\"")))],
        "taken in",
    );
    f.git(&f.work(), &["push", "-q", "origin", "main"]);
    f.branch("impl/b", &[("crates/b/src/lib.rs", "b\n")]);
    let out = clm_land::land(&f.work(), Some("impl/b"), &f.env(&[])).unwrap();
    assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
    assert!(out.warnings.iter().all(|w| !w.contains("slow tests")));
}

#[test]
fn two_items_hold_one_component() {
    // req:land/verify-stop@72bed87d
    let fails = format!("\"{COMMIT}\"");
    let f = repo(&[
        ("backlog/b-broke.md", &item("b", &fails)),
        ("backlog/b-slow.md", &item("b", &fails)),
    ]);
    f.branch("impl/b", &[("crates/b/src/lib.rs", "b\n")]);
    let why = refused(clm_land::land(&f.work(), Some("impl/b"), &f.env(&[])));
    assert!(why.contains("land/verify-stop"), "{why}");
    assert!(why.contains("b-broke") && why.contains("b-slow"), "{why}");
    assert!(why.contains("one item holding"), "{why}");
}

#[test]
fn each_fix_of_two_lands() {
    // req:land/verify-stop@72bed87d
    let fails = format!("\"{COMMIT}\"");
    let f = repo(&[
        ("backlog/b-broke.md", &item("b", &fails)),
        ("backlog/b-slow.md", &item("b", &fails)),
    ]);
    let work = f.work();
    f.git(&work, &["checkout", "-q", "-b", "impl/b", "main"]);
    std::fs::create_dir_all(work.join("crates/b/src")).unwrap();
    std::fs::write(work.join("crates/b/src/lib.rs"), "fixed\n").unwrap();
    f.git(&work, &["rm", "-q", "backlog/b-broke.md"]);
    f.git(&work, &["add", "-A"]);
    f.git(
        &work,
        &["commit", "-q", "-m", "Fix\n\nResolves: backlog/b-broke"],
    );
    f.git(&work, &["push", "-q", "origin", "impl/b"]);
    f.git(&work, &["checkout", "-q", "main"]);
    let out = clm_land::land(&work, Some("impl/b"), &f.env(&[])).unwrap();
    assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
    // The other item still holds it.
    f.git(&work, &["pull", "-q", "origin", "main"]);
    f.branch("impl/b2", &[("crates/b/src/lib2.rs", "b\n")]);
    let why = refused(clm_land::land(&work, Some("impl/b2"), &f.env(&[])));
    assert!(why.contains("b-slow") && !why.contains("b-broke"), "{why}");
}

#[test]
fn each_held_component_needs_its_own_fix() {
    // req:land/verify-stop@72bed87d
    let fails = format!("\"{COMMIT}\"");
    let f = repo(&[
        ("backlog/b-broke.md", &item("b", &fails)),
        ("backlog/a-broke.md", &item("a", &fails)),
    ]);
    let work = f.work();
    f.git(&work, &["checkout", "-q", "-b", "impl/ab", "main"]);
    for p in ["crates/a/src", "crates/b/src"] {
        std::fs::create_dir_all(work.join(p)).unwrap();
        std::fs::write(work.join(p).join("lib.rs"), "x\n").unwrap();
    }
    f.git(&work, &["add", "-A"]);
    f.git(
        &work,
        &["commit", "-q", "-m", "Fix\n\nResolves: backlog/b-broke"],
    );
    f.git(&work, &["push", "-q", "origin", "impl/ab"]);
    f.git(&work, &["checkout", "-q", "main"]);
    let why = refused(clm_land::land(&work, Some("impl/ab"), &f.env(&[])));
    assert!(why.contains("`a`") && why.contains("a-broke"), "{why}");
    assert!(!why.contains("`b`"), "{why}");
}
