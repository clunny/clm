mod common;

use clm_ledger::Body;
use common::{Fixture, refused};

/// A `host = "none"` repository with `impl/a` landed through the `origin` `keep_remote` made,
/// and the local `main` brought up to `origin/main`, as a pull does.
fn landed_through_remote() -> (Fixture, clm_land::Output) {
    let f = Fixture::local();
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    f.git(&f.work(), &["pull", "-q", "--ff-only", "origin", "main"]);
    (f, out)
}

/// Points the work clone's `origin` at a new bare repository `origin2.git`, as
/// `git remote set-url` does, leaving the remote-tracking refs of the old one; with `main`,
/// the new one holds an unrelated `main`.
fn repoint_origin(f: &Fixture, main: bool) {
    let bare = f.path("origin2.git");
    let d = bare.parent().unwrap();
    f.git(
        d,
        &["init", "-q", "--bare", "-b", "main", bare.to_str().unwrap()],
    );
    if main {
        let elsewhere = f.path("elsewhere");
        std::fs::create_dir_all(&elsewhere).unwrap();
        f.git(&elsewhere, &["init", "-q", "-b", "main"]);
        std::fs::write(elsewhere.join("z.txt"), "z\n").unwrap();
        f.git(&elsewhere, &["add", "-A"]);
        f.git(
            &elsewhere,
            &[
                "-c",
                "user.name=E",
                "-c",
                "user.email=e@e",
                "commit",
                "-q",
                "-m",
                "z",
            ],
        );
        f.git(&elsewhere, &["push", "-q", bare.to_str().unwrap(), "main"]);
    }
    f.git(
        &f.work(),
        &["remote", "set-url", "origin", bare.to_str().unwrap()],
    );
}

/// `rev` in the repository `repoint_origin` made.
fn new_origin(f: &Fixture, rev: &str) -> String {
    f.git(&f.path("origin2.git"), &["rev-parse", rev])
}

fn ledger_on_new_origin(f: &Fixture) -> bool {
    let refs = f.git(
        &f.path("origin2.git"),
        &[
            "for-each-ref",
            "--format=%(refname)",
            "refs/heads/clm/ledger/",
        ],
    );
    !refs.is_empty()
}

#[test]
fn no_host_lands_through_the_clones_own_remote() {
    // req:land/offline-land@795d1e93
    let f = Fixture::local();
    let old = f.remote("main");
    let head = f.branch("impl/a", &[("b.txt", "b\n")]);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    let main = f.remote("main");
    assert_eq!(f.parents(&f.path("origin.git"), &main), [old, head]);
    assert_eq!(out.lines, [format!("landed {main} code")]);
    assert!(
        f.landed()
            .iter()
            .any(|b| matches!(b, Body::Landed { commit, .. } if *commit == main))
    );
    // The ledger branch is on `origin`, the remote `keep_remote` made.
    let refs = f.git(
        &f.path("origin.git"),
        &[
            "for-each-ref",
            "--format=%(refname)",
            "refs/heads/clm/ledger/",
        ],
    );
    assert!(!refs.is_empty());
    // The land was made on `origin/main`; the local `main` was left alone.
    assert!(
        f.lines("gate.log")
            .iter()
            .all(|l| l.ends_with("--base origin/main")),
        "{:?}",
        f.lines("gate.log")
    );
    assert_ne!(f.rev("main"), main);
    assert_eq!(f.lines("check.log"), [f.tree(&f.path("origin.git"), &main)]);
}

#[test]
fn offline_with_a_remote_refuses() {
    // req:land/offline-land@795d1e93
    let f = Fixture::new("host = \"github\"");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let work = f.work();
    let before = clm_ledger::fold(&work).unwrap().records.len();
    let main = f.rev("main");
    f.git(
        &work,
        &["remote", "set-url", "origin", "/nowhere/at/all.git"],
    );
    let why = refused(clm_land::land(&work, Some("impl/a"), &f.env(&[])));
    assert!(why.contains("offline"), "{why}");
    assert_eq!(f.rev("main"), main);
    assert_eq!(clm_ledger::fold(&work).unwrap().records.len(), before);
    assert!(f.lines("check.log").is_empty());
}

#[test]
fn offline_with_no_host_refuses_too() {
    // req:land/offline-land@795d1e93
    let f = Fixture::local();
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let work = f.work();
    let main = f.rev("main");
    let remote_main = f.remote("main");
    f.git(
        &work,
        &["remote", "set-url", "origin", "/nowhere/at/all.git"],
    );
    let why = refused(clm_land::land(&work, Some("impl/a"), &f.env(&[])));
    assert!(why.contains("offline"), "{why}");
    assert_eq!(f.rev("main"), main);
    assert_eq!(f.remote("main"), remote_main);
    assert!(f.lines("check.log").is_empty());
}

#[test]
fn first_push_after_adding_a_remote() {
    // req:land/offline-land@795d1e93
    let (f, _) = landed_through_remote();
    let landed = f.remote("main");
    repoint_origin(&f, false);
    let out = clm_land::reconnect(&f.work(), &f.env(&[])).unwrap();
    assert_eq!(out.lines, ["reconnected 1 lands"]);
    assert_eq!(new_origin(&f, "main"), landed);
    assert!(ledger_on_new_origin(&f));
    // The old remote's tracking refs were pruned, not kept for the new one's branches.
    assert!(
        f.git(&f.work(), &["for-each-ref", "refs/remotes/origin/impl"])
            .is_empty()
    );
}

#[test]
fn origin_main_behind_local() {
    // req:land/offline-land@795d1e93
    let (f, _) = landed_through_remote();
    let landed = f.remote("main");
    let first = f.parents(&f.path("origin.git"), &landed)[0].clone();
    repoint_origin(&f, false);
    // The new `origin` already has the `main` the land started from.
    f.git(
        &f.work(),
        &["push", "-q", "origin", &format!("{first}:refs/heads/main")],
    );
    let out = clm_land::reconnect(&f.work(), &f.env(&[])).unwrap();
    assert_eq!(out.lines, ["reconnected 1 lands"]);
    assert_eq!(new_origin(&f, "main"), landed);
}

#[test]
fn reconnect_names_rebase() {
    // req:land/reconnect-names-rebase@92d58f31
    let (f, _) = landed_through_remote();
    repoint_origin(&f, true);
    let why = refused(clm_land::reconnect(&f.work(), &f.env(&[])));
    assert!(why.starts_with("origin's `main` has moved"), "{why}");
    assert!(
        why.contains("run `clm rebase` on each branch landed locally, then land it again"),
        "{why}"
    );
    assert!(why.contains("land again impl/a"), "{why}");

    let old = ["sync", "checkpoint", "page"].map(|c| format!("clm {c}"));
    let mut stack = vec![std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src"
    ))];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for name in &old {
                let hit = text.match_indices(name.as_str()).any(|(i, _)| {
                    !text[i + name.len()..]
                        .starts_with(|c: char| c.is_alphanumeric() || c == '_' || c == '-')
                });
                assert!(!hit, "{} names `{name}`", path.display());
            }
        }
    }
}

#[test]
fn origin_moved() {
    // req:land/offline-land@795d1e93
    let (f, _) = landed_through_remote();
    repoint_origin(&f, true);
    let theirs = new_origin(&f, "main");
    let why = refused(clm_land::reconnect(&f.work(), &f.env(&[])));
    assert!(why.contains("land again impl/a"), "{why}");
    assert_eq!(new_origin(&f, "main"), theirs);
    assert!(ledger_on_new_origin(&f));
}

#[test]
fn reconnect_without_origin_refuses() {
    // req:land/offline-land@795d1e93
    let (f, _) = landed_through_remote();
    f.git(&f.work(), &["remote", "remove", "origin"]);
    let why = refused(clm_land::reconnect(&f.work(), &f.env(&[])));
    assert!(why.contains("origin"), "{why}");
}
