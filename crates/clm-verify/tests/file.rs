//! Filing what the mutation pass found unproven (verify/files-unproven).

mod common;

use std::fs;
use std::path::Path;

use clm_ledger::Body;
use clm_verify::Outcome;
use common::{Fixture, write_exec};

/// The start of the marker the fixture's test carries, built so the literal never appears
/// whole in this file.
const MARK: &str = concat!("req", ":a/thing@");

fn test_file(version: &str) -> String {
    format!("#[test]\nfn proves_x() {{\n    // {MARK}{version}\n    let _ = 1;\n}}\n")
}

struct Filing {
    f: Fixture,
    /// The version of `a/thing` now.
    version: String,
    /// The commit the `mutated` record names.
    found: String,
}

/// A clone whose `main` holds a test citing `a/thing`, verified green with a `mutated` record
/// naming `a/thing@<entry>` (the current version when `entry` is `None`), so nothing is due.
fn filing(mode: &str, entry: Option<&str>) -> Filing {
    let f = Fixture::new(None);
    let version = clm_model::load(&f.work())
        .0
        .requirement("a/thing")
        .unwrap()
        .version()
        .to_string();
    let toml = format!(
        "[verify]\ncheck = \"{}\"\nmode = \"{mode}\"\ntriggers = []\nmutate = \"true\"\n",
        f.path("bin/check").display()
    );
    let found = f.land(
        &[
            ("clm.toml", &toml),
            ("crates/a/tests/x.rs", &test_file(&version)),
        ],
        "a test",
    );
    f.verified(&found, true);
    let body = Body::Mutated {
        commit: found.clone(),
        since: None,
        killed: vec![],
        unproven: vec![format!("a/thing@{}", entry.unwrap_or(&version))],
        not_judged: vec![],
    };
    clm_ledger::append(&f.work(), body, &f.env()).unwrap();
    Filing { f, version, found }
}

impl Filing {
    fn item(&self) -> String {
        let short = self
            .f
            .git(&self.f.work(), &["rev-parse", "--short", &self.found]);
        format!("backlog/unproven-a-{short}.md")
    }

    fn origin(&self, args: &[&str]) -> String {
        self.f.git(&self.f.path("origin.git"), args)
    }

    fn head(&self) -> String {
        self.origin(&["rev-parse", "main"])
    }

    fn on_origin(&self, path: &str) -> Option<String> {
        let spec = format!("main:{path}");
        let found = std::process::Command::new("git")
            .args(["show", &spec])
            .current_dir(self.f.path("origin.git"))
            .output()
            .unwrap();
        found
            .status
            .success()
            .then(|| String::from_utf8(found.stdout).unwrap())
    }

    /// The work clone brought to `origin/main`, as a session's checkout is after a land.
    fn sync(&self) {
        let work = self.f.work();
        self.f.git(&work, &["fetch", "-q", "origin"]);
        self.f.git(&work, &["reset", "-q", "--hard", "origin/main"]);
    }
}

fn said(out: &Outcome, start: &str) -> Option<String> {
    out.notes.iter().find(|n| n.starts_with(start)).cloned()
}

#[test]
fn filed_and_read_as_not_built() {
    // req:verify/files-unproven@1425f509
    let p = filing("local", None);
    let before = p.head();
    let out = p.f.due();
    assert!(out.line.starts_with("verify not-due"), "{}", out.line);
    assert_eq!(out.code, 0);
    assert_ne!(p.head(), before);
    assert_eq!(
        said(&out, "unproven:").as_deref(),
        Some("unproven: filed 1 items for 1 requirements")
    );
    let item = p.on_origin(&p.item()).expect("the item is on main");
    assert!(item.contains("kind: bug"), "{item}");
    assert!(item.contains("components: [a]"), "{item}");
    assert!(
        item.contains("title: Tests of a notice no change to its code"),
        "{item}"
    );
    assert!(
        item.contains(&format!("unproven: [a/thing@{}]", p.version)),
        "{item}"
    );
    assert!(item.contains("crates/a/tests/x.rs"), "{item}");
    assert!(item.contains("proves_x"), "{item}");
    let short = p.f.git(&p.f.work(), &["rev-parse", "--short", &p.found]);
    assert!(item.contains(&short), "{item}");
    // No branch is left, on origin or here, and no worktree.
    assert_eq!(p.origin(&["branch", "--list", "unproven/*"]), "");
    assert_eq!(
        p.f.git(&p.f.work(), &["branch", "--list", "unproven/*"]),
        ""
    );
    let trees = p.f.git(&p.f.work(), &["worktree", "list", "--porcelain"]);
    assert!(!trees.contains("unproven"), "{trees}");
    // Read from the new `main`, the requirement is not built although a test cites it.
    p.sync();
    let tree = clm_git::extract_tree(&p.f.work(), "origin/main").unwrap();
    let (graph, _) = clm_model::load(tree.path());
    let path = Path::new("crates/a/tests/x.rs");
    let text = fs::read_to_string(tree.path().join(path)).unwrap();
    let markers = clm_coverage::markers_in(path, &text);
    let line = |graph: &clm_model::Graph| {
        clm_coverage::report(graph, &markers)
            .into_iter()
            .find(|c| c.id == "a/thing")
            .unwrap()
            .to_string()
    };
    assert_eq!(line(&graph), "a/thing unbuilt");
}

#[test]
fn already_filed() {
    // req:verify/files-unproven@1425f509
    let p = filing("local", None);
    p.f.due();
    let filed = p.head();
    let out = p.f.due();
    assert_eq!(p.head(), filed);
    assert_eq!(said(&out, "unproven:"), None);
}

#[test]
fn reworded_since() {
    // req:verify/files-unproven@1425f509
    let p = filing("local", Some("11111111"));
    let before = p.head();
    let out = p.f.due();
    assert_eq!(p.head(), before);
    assert_eq!(said(&out, "unproven:"), None);
    assert_eq!(p.on_origin(&p.item()), None);
}

#[test]
fn resolved_by_a_build() {
    // req:verify/files-unproven@1425f509
    let p = filing("local", None);
    p.f.due();
    p.sync();
    let item = p.item();
    p.f.git(&p.f.work(), &["rm", "-q", &item]);
    p.f.git(&p.f.work(), &["commit", "-q", "-m", "resolved"]);
    p.f.git(&p.f.work(), &["push", "-q", "origin", "main"]);
    let resolved = p.head();
    let out = p.f.due();
    assert_eq!(p.head(), resolved);
    assert_eq!(said(&out, "unproven:"), None);
    assert_eq!(p.on_origin(&item), None);
}

#[test]
fn the_land_fails() {
    // req:verify/files-unproven@1425f509
    let p = filing("local", None);
    let hook = p.f.path("origin.git/hooks/pre-receive");
    write_exec(
        &hook,
        "#!/bin/sh\nwhile read old new ref; do\n  case $ref in refs/heads/unproven/*) echo no >&2; exit 1;; esac\ndone\n",
    );
    let before = p.head();
    let out = p.f.due();
    assert_eq!(out.code, 0);
    assert!(out.line.starts_with("verify not-due"), "{}", out.line);
    let note = said(&out, "unproven: not filed:").expect("a note");
    assert!(note.len() > "unproven: not filed:".len(), "{note}");
    assert_eq!(p.head(), before);
    assert_eq!(
        p.f.git(&p.f.work(), &["branch", "--list", "unproven/*"]),
        ""
    );
    fs::remove_file(&hook).unwrap();
    let out = p.f.due();
    assert!(said(&out, "unproven: filed 1 items").is_some(), "{out:?}");
    assert!(p.on_origin(&p.item()).is_some());
}

#[test]
fn pipeline_mode_files_nothing() {
    // req:verify/files-unproven@1425f509
    let p = filing("pipeline", None);
    let before = p.head();
    let out = p.f.due();
    assert_eq!(p.head(), before);
    assert_eq!(said(&out, "unproven:"), None);
}

#[test]
fn a_commit_verification_files_nothing() {
    // req:verify/files-unproven@1425f509
    let p = filing("local", None);
    let before = p.head();
    let out = p.f.run(&clm_verify::Request::Commit(p.found.clone()));
    assert_eq!(p.head(), before);
    assert_eq!(said(&out, "unproven:"), None);
}

#[test]
fn a_dry_run_files_nothing() {
    // req:verify/files-unproven@1425f509
    let p = filing("local", None);
    let before = p.head();
    p.f.dry(false);
    assert_eq!(p.head(), before);
}
