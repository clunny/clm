//! The verify tree holds nothing git doesn't track once a `clm verify` is done with it
//! (verify/verify-tree-cleared).

mod common;

use clm_ledger::Body;
use common::{Fixture, in_child, write_exec};

/// The start of the marker the fixture's test carries, built so the literal never appears
/// whole in this file.
const MARK: &str = concat!("req", ":a/thing@");

const IGNORE: &str = "clm.local.toml\n.clm/\ntarget/\n";

/// A clone whose `[verify]` runs on every land with `check` as its check's body, and
/// `.gitignore` naming `target/`, with `extra` added to the table. Returns the clone and the
/// land of the settings, which no record names yet.
fn with_check(check: &str, extra: &str) -> (Fixture, String) {
    let f = Fixture::new(Some(
        "mode = \"local\"\ntriggers = []\nevery = { lands = 1 }",
    ));
    let script = f.path("bin/check");
    write_exec(&script, &format!("#!/bin/sh\n{check}\n"));
    let toml = format!(
        "[verify]\ncheck = \"{}\"\nmode = \"local\"\ntriggers = []\nevery = {{ lands = 1 }}\n{extra}",
        script.display()
    );
    let base = f.land(&[("clm.toml", &toml), (".gitignore", IGNORE)], "settings");
    (f, base)
}

fn verified(f: &Fixture) -> Vec<(bool, Option<String>)> {
    f.records()
        .into_iter()
        .map(|b| match b {
            Body::Verified {
                passed, culprit, ..
            } => (passed, culprit),
            _ => unreachable!(),
        })
        .collect()
}

#[test]
fn the_pass_s_builds_are_gone() {
    // req:verify/verify-tree-cleared@f3a40445
    let (f, _) = with_check("mkdir -p target && : > target/check", "");
    let mutate = f.path("bin/mutate");
    write_exec(
        &mutate,
        "#!/bin/sh\nmkdir -p target/job && : > target/job/x && : > junk\n\
         if [ -e target/check ]; then echo 'a/thing killed' > \"$CLM_MUTATE_RESULT\"; fi\n",
    );
    let version = clm_model::load(&f.work())
        .0
        .requirement("a/thing")
        .unwrap()
        .version()
        .to_string();
    let toml = format!(
        "[verify]\ncheck = \"{}\"\nmode = \"local\"\ntriggers = []\nevery = {{ lands = 1 }}\n\
         mutate = \"{}\"\n",
        f.path("bin/check").display(),
        mutate.display()
    );
    let base = f.land(&[("clm.toml", &toml)], "mutate");
    f.verified(&base, true);
    let body = Body::mutated(base, None, vec![], vec![], vec![]);
    clm_ledger::append(&f.work(), body, &f.env()).unwrap();
    let test = format!("#[test]\nfn proves_x() {{\n    // {MARK}{version}\n    let _ = 1;\n}}\n");
    f.land(
        &[
            ("crates/a/src/lib.rs", "1\n"),
            ("crates/a/tests/x.rs", &test),
        ],
        "a new test",
    );
    let out = f.due();
    assert!(out.line.starts_with("verify passed "), "{}", out.line);
    assert_eq!(out.code, 0);
    let records = f.records_of(|b| matches!(b, Body::Mutated { .. }));
    match records.last() {
        Some(Body::Mutated { killed, .. }) => assert_eq!(killed, &["a/thing".to_string()]),
        other => panic!("{other:?}"),
    }
    let tree = f.verify_tree();
    assert!(tree.join("specs").is_dir(), "the tree is still checked out");
    assert!(!tree.join("target").exists());
    assert!(!tree.join("junk").exists());
}

#[test]
fn a_red_run_s_tree() {
    // req:verify/verify-tree-cleared@f3a40445
    let (f, base) = with_check("mkdir -p target && : > target/x\nexit 1", "");
    f.verified(&base, true);
    f.land(&[("a.txt", "1\n")], "one");
    let out = f.due();
    assert_eq!(out.code, 1, "{}", out.line);
    assert_eq!(verified(&f).last().map(|r| r.0), Some(false));
    let tree = f.verify_tree();
    assert!(tree.join("specs").is_dir(), "the tree is still checked out");
    assert!(!tree.join("target").exists());
}

#[test]
fn a_bisect_s_tree() {
    // req:verify/verify-tree-cleared@f3a40445
    let (f, base) = with_check("mkdir -p target && : > target/x\n[ ! -e second.txt ]", "");
    f.verified(&base, true);
    f.land(&[("first.txt", "1\n")], "first");
    let second = f.land(&[("second.txt", "1\n")], "second");
    let out = f.due();
    assert_eq!(out.code, 1, "{}", out.line);
    assert_eq!(verified(&f).last(), Some(&(false, Some(second))));
    assert!(!f.verify_tree().join("target").exists());
}

#[test]
fn a_tree_that_isnt_whole() {
    // req:verify/verify-tree-cleared@f3a40445
    if let Some(stderr) = in_child("a_tree_that_isnt_whole") {
        assert!(
            stderr
                .lines()
                .any(|l| l.starts_with("clm verify: cannot clean .clm/verify-tree: ")),
            "{stderr}"
        );
        return;
    }
    let (f, base) = with_check("mkdir -p target && : > target/x && rm .git", "");
    f.verified(&base, true);
    f.land(
        &[
            ("a.txt", "1\n"),
            (".gitignore", "clm.local.toml\n.clm/\ntarget/\nkeep\n"),
        ],
        "one",
    );
    std::fs::write(f.work().join("keep"), "mine\n").unwrap();
    let out = f.due();
    assert!(out.line.starts_with("verify passed "), "{}", out.line);
    assert_eq!(out.code, 0);
    assert_eq!(verified(&f).last().map(|r| r.0), Some(true));
    assert!(f.work().join("keep").exists());
    assert!(f.verify_tree().join("target/x").exists());
}

#[test]
fn a_call_that_only_judged_a_trigger() {
    // req:verify/verify-tree-cleared@f3a40445
    let f = Fixture::new(Some("mode = \"local\"\ntriggers = [\"queue-drained\"]"));
    let toml = format!(
        "[verify]\ncheck = \"{}\"\nmode = \"local\"\ntriggers = [\"queue-drained\"]\n",
        f.path("bin/check").display()
    );
    let base = f.land(&[("clm.toml", &toml), (".gitignore", IGNORE)], "settings");
    f.verified(&base, true);
    f.land(&[("a.txt", "1\n")], "one");
    // A tree an earlier run left, with a build in it.
    let tree = f.verify_tree();
    f.git(
        &f.work(),
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            tree.to_str().unwrap(),
            "main",
        ],
    );
    std::fs::create_dir_all(tree.join("target")).unwrap();
    std::fs::write(tree.join("target/old"), "old\n").unwrap();
    let out = f.due();
    assert!(out.line.starts_with("verify not-due"), "{}", out.line);
    assert_eq!(out.code, 0);
    assert!(f.lines("check-head.log").is_empty(), "no check ran");
    assert!(!tree.join("target").exists());
    // `--dry-run` too, since judging the trigger checked the tree out.
    std::fs::create_dir_all(tree.join("target")).unwrap();
    std::fs::write(tree.join("target/old"), "old\n").unwrap();
    let out = f.dry(false);
    assert!(out.line.starts_with("verify not-due"), "{}", out.line);
    assert_eq!(out.code, 3);
    assert!(!tree.join("target").exists());
}

#[test]
fn a_call_that_ends_in_an_error() {
    // req:verify/verify-tree-cleared@f3a40445
    let (f, base) = with_check("exit 0", "");
    f.verified(&base, true);
    // A commit whose `[env]` can't be used: `--commit` refuses it once it has checked it out.
    let toml = std::fs::read_to_string(f.work().join("clm.toml")).unwrap();
    let bad = f.land(
        &[("clm.toml", &format!("{toml}[env]\nCLM_X = \"1\"\n"))],
        "bad",
    );
    f.land(&[("clm.toml", &toml)], "good");
    let tree = f.verify_tree();
    f.git(
        &f.work(),
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            tree.to_str().unwrap(),
            "main",
        ],
    );
    std::fs::create_dir_all(tree.join("target")).unwrap();
    std::fs::write(tree.join("target/old"), "old\n").unwrap();
    let out = common::verify(&f.work(), &clm_verify::Request::Commit(bad), &f.env());
    assert!(matches!(out, Err(clm_verify::Error::Refused(_))), "{out:?}");
    assert!(tree.join("specs").is_dir(), "the tree is still checked out");
    assert!(!tree.join("target").exists());
}

#[test]
fn no_other_call_has_the_tree_between_check_and_pass() {
    // req:verify/verify-tree-cleared@f3a40445
    let (f, base) = with_check("mkdir -p target && : > target/check", "");
    f.verified(&base, true);
    f.land(&[("a.txt", "1\n")], "one");
    let lock = f.verify_tree().parent().unwrap().join("verify.lock");
    let stop = std::sync::atomic::AtomicBool::new(false);
    let memory = f.path("no-such-meminfo");
    let call = clm_verify::Call {
        memory: &memory,
        minute: std::time::Duration::from_secs(60),
        grace: std::time::Duration::from_secs(10),
        stop: &stop,
    };
    let request = clm_verify::Request::Due {
        work_done: false,
        dry_run: false,
    };
    let mut free = None;
    // The outcome is announced after the check and before the pass.
    let out = clm_verify::verify_with(&f.work(), &request, &f.env(), &call, |_| {
        let file = std::fs::File::options().write(true).open(&lock).unwrap();
        free = Some(file.try_lock().is_ok());
    })
    .unwrap();
    assert_eq!(out.code, 0, "{}", out.line);
    assert_eq!(
        free,
        Some(false),
        "the verify lock was free after the check"
    );
    // And let go once the call is done.
    let file = std::fs::File::options().write(true).open(&lock).unwrap();
    assert!(file.try_lock().is_ok());
}
