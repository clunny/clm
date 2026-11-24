//! Filing what the mutation pass found unproven (verify/files-unproven).

mod common;

use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use clm_ledger::Body;
use clm_verify::{Call, Outcome};
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
    filing_with(mode, entry, "")
}

/// As `filing`, with `extra` appended to the clone's `clm.toml`, each `{dir}` in it written as
/// the fixture's directory.
fn filing_with(mode: &str, entry: Option<&str>, extra: &str) -> Filing {
    let f = Fixture::new(None);
    let extra = extra.replace("{dir}", &f.path("").display().to_string());
    let version = clm_model::load(&f.work())
        .0
        .requirement("a/thing")
        .unwrap()
        .version()
        .to_string();
    let toml = format!(
        "[verify]\ncheck = \"{}\"\nmode = \"{mode}\"\ntriggers = []\nmutate = \"true\"\n{extra}",
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
    let body = Body::mutated(
        found.clone(),
        None,
        vec![],
        vec![format!("a/thing@{}", entry.unwrap_or(&version))],
        vec![],
    );
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

/// `clm verify --due` with its stop request set to `stop`.
fn due_stopped(p: &Filing, stop: bool) -> Outcome {
    let stop = AtomicBool::new(stop);
    let memory = p.f.path("no-such-meminfo");
    let call = Call {
        memory: &memory,
        minute: Duration::from_secs(60),
        grace: Duration::from_secs(10),
        stop: &stop,
    };
    p.f.due_with(&call)
}

/// No branch `unproven/*` is left, on `origin` or in the clone, and no worktree of one.
fn no_branch_left(p: &Filing) {
    assert_eq!(p.origin(&["branch", "--list", "unproven/*"]), "");
    assert_eq!(
        p.f.git(&p.f.work(), &["branch", "--list", "unproven/*"]),
        ""
    );
    let trees = p.f.git(&p.f.work(), &["worktree", "list", "--porcelain"]);
    assert!(!trees.contains("unproven"), "{trees}");
}

fn said(out: &Outcome, start: &str) -> Option<String> {
    out.notes.iter().find(|n| n.starts_with(start)).cloned()
}

#[test]
fn filed_and_read_as_not_built() {
    // req:verify/files-unproven@ec5a9e04
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
    // req:verify/files-unproven@ec5a9e04
    let p = filing("local", None);
    p.f.due();
    let filed = p.head();
    let out = p.f.due();
    assert_eq!(p.head(), filed);
    assert_eq!(said(&out, "unproven:"), None);
}

#[test]
fn reworded_since() {
    // req:verify/files-unproven@ec5a9e04
    let p = filing("local", Some("11111111"));
    let before = p.head();
    let out = p.f.due();
    assert_eq!(p.head(), before);
    assert_eq!(said(&out, "unproven:"), None);
    assert_eq!(p.on_origin(&p.item()), None);
}

#[test]
fn resolved_by_a_build() {
    // req:verify/files-unproven@ec5a9e04
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
    // req:verify/files-unproven@ec5a9e04
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
    // req:verify/files-unproven@ec5a9e04
    let p = filing("pipeline", None);
    let before = p.head();
    let out = p.f.due();
    assert_eq!(p.head(), before);
    assert_eq!(said(&out, "unproven:"), None);
}

#[test]
fn a_commit_verification_files_nothing() {
    // req:verify/files-unproven@ec5a9e04
    let p = filing("local", None);
    let before = p.head();
    let out = p.f.run(&clm_verify::Request::Commit(p.found.clone()));
    assert_eq!(p.head(), before);
    assert_eq!(said(&out, "unproven:"), None);
}

#[test]
fn a_dry_run_files_nothing() {
    // req:verify/files-unproven@ec5a9e04
    let p = filing("local", None);
    let before = p.head();
    let out = p.f.dry(false);
    // Nothing has landed since the verified commit, so nothing is due.
    assert!(out.line.starts_with("verify not-due"), "{}", out.line);
    assert_eq!(out.code, 3);
    assert_eq!(said(&out, "unproven:"), None);
    assert_eq!(p.head(), before);
    assert_eq!(p.on_origin(&p.item()), None);
    // A verification that ends in a refusal files nothing either.
    let toml = format!(
        "[verify]\ncheck = \"{}\"\nmode = \"local\"\ntriggers = []\nevery = {{ lands = 1 }}\n\
         mutate = \"true\"\n\n[env]\nCLM_X = \"1\"\n",
        p.f.path("bin/check").display()
    );
    let refused =
        p.f.land(&[("clm.toml", &toml)], "an env that can't be used");
    let err = common::verify(
        &p.f.work(),
        &clm_verify::Request::Due {
            work_done: false,
            dry_run: false,
        },
        &p.f.env(),
    )
    .unwrap_err();
    assert!(matches!(err, clm_verify::Error::Refused(_)), "{err}");
    let why = err.to_string();
    assert!(why.contains("env") && why.contains("CLM_X"), "{why}");
    assert_eq!(p.head(), refused);
    assert_eq!(p.on_origin(&p.item()), None);
    let backlog = p.origin(&["ls-tree", "--name-only", "main", "backlog/"]);
    assert_eq!(backlog, "", "{backlog}");
}

#[test]
fn a_verification_that_ends_in_an_error_files_nothing() {
    // req:verify/files-unproven@ec5a9e04
    let p = filing("local", None);
    // A trigger holds, and the verify tree's lock can't be opened (a directory stands in its
    // place), so the verification ends in a failure once its settings are loaded; a filing
    // would need neither the tree nor its lock.
    let toml = format!(
        "[verify]\ncheck = \"{}\"\nmode = \"local\"\ntriggers = []\nevery = {{ lands = 1 }}\n\
         mutate = \"true\"\n",
        p.f.path("bin/check").display()
    );
    let head = p.f.land(&[("clm.toml", &toml)], "a trigger");
    let work = p.f.work();
    fs::create_dir_all(work.join(".clm/verify.lock")).unwrap();
    let err = common::verify(
        &work,
        &clm_verify::Request::Due {
            work_done: false,
            dry_run: false,
        },
        &p.f.env(),
    )
    .unwrap_err();
    assert!(matches!(err, clm_verify::Error::Git(_)), "{err}");
    assert_eq!(p.head(), head);
    assert_eq!(p.on_origin(&p.item()), None);
    // The same entry is filed once the call succeeds.
    fs::remove_dir(work.join(".clm/verify.lock")).unwrap();
    let out = p.f.due();
    assert!(said(&out, "unproven: filed 1 items").is_some(), "{out:?}");
}

#[test]
fn stopped_before_it_files() {
    // req:verify/files-unproven@ec5a9e04
    let p = filing("local", None);
    let before = p.head();
    let out = due_stopped(&p, true);
    assert!(out.line.starts_with("verify not-due"), "{}", out.line);
    assert_eq!(out.code, 0);
    assert_eq!(
        said(&out, "unproven:").as_deref(),
        Some("unproven: not filed: stopped")
    );
    assert_eq!(p.head(), before);
    assert_eq!(p.on_origin(&p.item()), None);
    no_branch_left(&p);
    // With the stop request unset, the next call files the item.
    let out = due_stopped(&p, false);
    assert_eq!(
        said(&out, "unproven:").as_deref(),
        Some("unproven: filed 1 items for 1 requirements")
    );
    assert!(p.on_origin(&p.item()).is_some());
    no_branch_left(&p);
}

#[test]
fn a_stop_with_nothing_current_to_file_says_nothing() {
    // req:verify/files-unproven@ec5a9e04
    let p = filing("local", Some("11111111"));
    let before = p.head();
    let out = due_stopped(&p, true);
    assert_eq!(said(&out, "unproven:"), None);
    assert_eq!(p.head(), before);
    no_branch_left(&p);
}

#[test]
fn a_filing_command_ended_by_a_signal() {
    // req:verify/files-unproven@ec5a9e04
    // The land's check writes its process group and sends `SIGINT` to its own shell, as a
    // Ctrl-C at a terminal reaches every process of `clm verify`'s group.
    let p = filing_with(
        "local",
        None,
        "\n[land]\ncheck = \"ps -o pgid= -p $$ > {dir}check-pgid; kill -INT $$\"\n",
    );
    let before = p.head();
    let out = p.f.due();
    assert_eq!(out.code, 0);
    assert!(out.line.starts_with("verify not-due"), "{}", out.line);
    let note = said(&out, "unproven: not filed:").expect("a note");
    assert!(note.len() > "unproven: not filed:".len(), "{note}");
    assert_ne!(note, "unproven: not filed: stopped");
    // The reason names the check the signal ended.
    assert!(note.contains("kill -INT $$"), "{note}");
    // The check ran in this process's group, not in one of its own, so a signal sent to the
    // group reaches it.
    let own = std::process::Command::new("ps")
        .args(["-o", "pgid=", "-p", &std::process::id().to_string()])
        .output()
        .unwrap();
    let own = String::from_utf8(own.stdout).unwrap();
    let checks = fs::read_to_string(p.f.path("check-pgid")).expect("the check ran");
    assert_eq!(checks.trim(), own.trim());
    assert_eq!(p.head(), before);
    assert_eq!(p.on_origin(&p.item()), None);
    no_branch_left(&p);
    // One more land sets a check that passes, and the next call files the item.
    let toml = format!(
        "[verify]\ncheck = \"{}\"\nmode = \"local\"\ntriggers = []\nmutate = \"true\"\n\n\
         [land]\ncheck = \"exit 0\"\n",
        p.f.path("bin/check").display()
    );
    p.f.land(&[("clm.toml", &toml)], "a check that passes");
    let out = p.f.due();
    assert_eq!(
        said(&out, "unproven:").as_deref(),
        Some("unproven: filed 1 items for 1 requirements"),
        "{out:?}"
    );
    assert!(p.on_origin(&p.item()).is_some());
    no_branch_left(&p);
}

/// A clone whose `main` holds two lands `Q` and then `R`, verified green at `R`, with a
/// `mutated` record for each: `R`'s holding `a/thing` under `left`, and `Q`'s under `left`
/// when `q_left`, else under `killed`. Returns the fixture, `Q` and `R`.
fn left_twice(q_left: bool) -> (Fixture, String, String) {
    let f = Fixture::new(None);
    let toml = format!(
        "[verify]\ncheck = \"{}\"\nmode = \"local\"\ntriggers = []\nmutate = \"true\"\n",
        f.path("bin/check").display()
    );
    let q = f.land(&[("clm.toml", &toml)], "Q");
    let r = f.land(&[("crates/a/src/r.rs", "fn r() {}\n")], "R");
    f.verified(&r, true);
    let thing = vec!["a/thing".to_string()];
    let body = if q_left {
        Body::mutated(q.clone(), None, vec![], vec![], vec![]).with_left(thing.clone())
    } else {
        Body::mutated(q.clone(), None, thing.clone(), vec![], vec![])
    };
    clm_ledger::append(&f.work(), body, &f.env()).unwrap();
    let body = Body::mutated(r.clone(), Some(q.clone()), vec![], vec![], vec![]).with_left(thing);
    clm_ledger::append(&f.work(), body, &f.env()).unwrap();
    (f, q, r)
}

#[test]
fn left_by_two_passes_in_a_row() {
    // req:verify/files-unproven@ec5a9e04
    let (f, q, r) = left_twice(true);
    let origin = |args: &[&str]| f.git(&f.path("origin.git"), args);
    let short = |sha: &str| f.git(&f.work(), &["rev-parse", "--short", sha]);
    let before = origin(&["rev-parse", "main"]);
    let out = f.due();
    assert!(out.line.starts_with("verify not-due"), "{}", out.line);
    assert_eq!(out.code, 0);
    assert_eq!(
        said(&out, "unjudged:").as_deref(),
        Some("unjudged: filed 1 items for 1 requirements"),
        "{out:?}"
    );
    assert_eq!(said(&out, "unproven: filed"), None, "{out:?}");
    assert_ne!(origin(&["rev-parse", "main"]), before);
    let path = format!("backlog/unjudged-a-{}.md", short(&r));
    let item = origin(&["show", &format!("main:{path}")]);
    let slug = path.trim_start_matches("backlog/").trim_end_matches(".md");
    let parsed = clm_model::parse_backlog_item(slug, &item).expect("the item parses");
    assert_eq!(
        parsed.frontmatter.kind,
        clm_model::BacklogKind::Bug,
        "{item}"
    );
    assert_eq!(parsed.frontmatter.nodes, vec!["a".to_string()], "{item}");
    assert_eq!(
        parsed.frontmatter.title,
        "The mutation pass can't judge requirements of a"
    );
    assert!(parsed.frontmatter.unproven.is_empty(), "{item}");
    assert!(item.contains("`a/thing`"), "{item}");
    assert!(
        item.contains(&short(&q)) && item.contains(&short(&r)),
        "{item}"
    );
    assert_eq!(origin(&["branch", "--list", "unproven/*"]), "");
    // A third pass, on a later commit `S`, that leaves it too lands nothing more.
    f.git(&f.work(), &["fetch", "-q", "origin"]);
    f.git(&f.work(), &["reset", "-q", "--hard", "origin/main"]);
    let s = f.land(&[("crates/a/src/s.rs", "fn s() {}\n")], "S");
    f.verified(&s, true);
    let body = Body::mutated(s.clone(), Some(r.clone()), vec![], vec![], vec![])
        .with_left(vec!["a/thing".to_string()]);
    clm_ledger::append(&f.work(), body, &f.env()).unwrap();
    let out = f.due();
    assert_eq!(origin(&["rev-parse", "main"]), s);
    assert_eq!(said(&out, "unjudged:"), None, "{out:?}");
    assert_eq!(said(&out, "unproven:"), None, "{out:?}");
    let backlog = origin(&["ls-tree", "--name-only", "main", "backlog/"]);
    assert_eq!(backlog, path, "{backlog}");
}

/// A clone with three lands, verified green at the last, and a `mutated` record for each
/// `(land, id, left)` in order, holding `id` under `left` when `left`, else under `killed`.
/// Returns whether `clm verify --due` landed anything.
fn files_after(records: &[(usize, &str, bool)]) -> bool {
    let f = Fixture::new(None);
    let toml = format!(
        "[verify]\ncheck = \"{}\"\nmode = \"local\"\ntriggers = []\nmutate = \"true\"\n",
        f.path("bin/check").display()
    );
    let lands = [
        f.land(&[("clm.toml", &toml)], "Q"),
        f.land(&[("crates/a/src/r.rs", "fn r() {}\n")], "R"),
        f.land(&[("crates/a/src/s.rs", "fn s() {}\n")], "S"),
    ];
    f.verified(&lands[2], true);
    for (land, id, left) in records {
        let ids = vec![id.to_string()];
        let body = if *left {
            Body::mutated(lands[*land].clone(), None, vec![], vec![], vec![]).with_left(ids)
        } else {
            Body::mutated(lands[*land].clone(), None, ids, vec![], vec![])
        };
        clm_ledger::append(&f.work(), body, &f.env()).unwrap();
    }
    let out = f.due();
    let head = f.git(&f.path("origin.git"), &["rev-parse", "main"]);
    assert_eq!(said(&out, "unproven:"), None, "{out:?}");
    let filed = head != lands[2];
    assert_eq!(said(&out, "unjudged:").is_some(), filed, "{out:?}");
    filed
}

#[test]
fn only_an_unbroken_run_of_commits_files() {
    // req:verify/files-unproven@ec5a9e04
    // R's pass judged it, so the run is S's alone.
    assert!(!files_after(&[
        (0, "a/thing", true),
        (1, "a/thing", false),
        (2, "a/thing", true)
    ]));
    // Two passes on one commit count as one.
    assert!(!files_after(&[(2, "a/thing", true), (2, "a/thing", true)]));
    // A commit's newest record by `at` is the one read: S's last pass judged it.
    assert!(!files_after(&[
        (1, "a/thing", true),
        (2, "a/thing", true),
        (2, "a/thing", false)
    ]));
    // A requirement `origin/main` no longer holds is not filed.
    assert!(!files_after(&[(1, "a/gone", true), (2, "a/gone", true)]));
    // The same records with the run unbroken file.
    assert!(files_after(&[
        (0, "a/thing", true),
        (1, "a/thing", true),
        (2, "a/thing", true)
    ]));
}

#[test]
fn a_stop_keeps_an_unjudged_item_from_filing() {
    // req:verify/files-unproven@ec5a9e04
    let (f, _, r) = left_twice(true);
    let origin = |args: &[&str]| f.git(&f.path("origin.git"), args);
    let before = origin(&["rev-parse", "main"]);
    let stop = AtomicBool::new(true);
    let memory = f.path("no-such-meminfo");
    let call = Call {
        memory: &memory,
        minute: Duration::from_secs(60),
        grace: Duration::from_secs(10),
        stop: &stop,
    };
    let out = f.due_with(&call);
    assert_eq!(out.code, 0);
    assert_eq!(
        said(&out, "unproven:").as_deref(),
        Some("unproven: not filed: stopped"),
        "{out:?}"
    );
    assert_eq!(said(&out, "unjudged:"), None, "{out:?}");
    assert_eq!(origin(&["rev-parse", "main"]), before);
    assert_eq!(origin(&["branch", "--list", "unproven/*"]), "");
    // Unset, the next call files it.
    stop.store(false, std::sync::atomic::Ordering::SeqCst);
    let out = f.due_with(&call);
    assert!(said(&out, "unjudged: filed 1 items").is_some(), "{out:?}");
    let short = f.git(&f.work(), &["rev-parse", "--short", &r]);
    let path = format!("main:backlog/unjudged-a-{short}.md");
    assert!(origin(&["show", &path]).contains("kind: bug"));
}

#[test]
fn left_by_one_pass() {
    // req:verify/files-unproven@ec5a9e04
    let (f, _, _) = left_twice(false);
    let origin = |args: &[&str]| f.git(&f.path("origin.git"), args);
    let before = origin(&["rev-parse", "main"]);
    let out = f.due();
    assert_eq!(origin(&["rev-parse", "main"]), before);
    assert_eq!(said(&out, "unjudged:"), None, "{out:?}");
    assert_eq!(said(&out, "unproven:"), None, "{out:?}");
    let backlog = origin(&["ls-tree", "--name-only", "main", "backlog/"]);
    assert_eq!(backlog, "", "{backlog}");
}
