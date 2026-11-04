mod common;

use clm_ledger::Body;
use common::{Fixture, refused};

/// A `host = "none"` repository with `impl/a` landed locally.
fn landed_locally() -> (Fixture, clm_land::Output) {
    let f = Fixture::local();
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    (f, out)
}

/// A bare `origin.git` added as the work clone's `origin`; with `main`, holding an
/// unrelated `main`.
fn add_origin(f: &Fixture, main: bool) {
    let bare = f.path("origin.git");
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
        &["remote", "add", "origin", bare.to_str().unwrap()],
    );
}

fn ledger_on_origin(f: &Fixture) -> bool {
    let refs = f.git(
        &f.path("origin.git"),
        &[
            "for-each-ref",
            "--format=%(refname)",
            "refs/heads/clm/ledger/",
        ],
    );
    !refs.is_empty()
}

#[test]
fn no_remote_lands_locally() {
    // req:land/offline-land@98b44467
    let f = Fixture::local();
    let old = f.rev("main");
    let head = f.branch("impl/a", &[("b.txt", "b\n")]);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    let main = f.rev("main");
    assert_eq!(f.parents(&f.work(), &main), [old, head]);
    assert_eq!(out.lines, [format!("landed {main} code (local)")]);
    assert!(
        f.landed()
            .iter()
            .any(|b| matches!(b, Body::Landed { commit, .. } if *commit == main))
    );
    // The checkout on `main` moved with it, clean.
    assert_eq!(f.git(&f.work(), &["status", "--porcelain"]), "");
    // The same gate and checks ran, against the local `main`.
    assert_eq!(f.lines("check.log"), [f.tree(&f.work(), &main)]);
    assert_eq!(f.lines("clm-check.log").len(), 1);
    assert!(
        f.lines("gate.log")
            .iter()
            .all(|l| l.ends_with("--base main")),
        "{:?}",
        f.lines("gate.log")
    );
    // The branch is its only copy, kept for `clm reconnect`'s `land again`.
    assert_eq!(f.rev("impl/a"), f.parents(&f.work(), &main)[1]);
}

#[test]
fn offline_with_a_remote_refuses() {
    // req:land/offline-land@98b44467
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
fn first_push_after_adding_a_remote() {
    // req:land/offline-land@98b44467
    let (f, _) = landed_locally();
    add_origin(&f, false);
    let out = clm_land::reconnect(&f.work(), &f.env(&[])).unwrap();
    assert_eq!(out.lines, ["reconnected 1 lands"]);
    assert_eq!(f.remote("main"), f.rev("main"));
    assert!(ledger_on_origin(&f));
}

#[test]
fn origin_main_behind_local() {
    // req:land/offline-land@98b44467
    let (f, _) = landed_locally();
    add_origin(&f, false);
    // `origin` already has the `main` the local land started from.
    let first = f.parents(&f.work(), "main")[0].clone();
    f.git(
        &f.work(),
        &["push", "-q", "origin", &format!("{first}:refs/heads/main")],
    );
    let out = clm_land::reconnect(&f.work(), &f.env(&[])).unwrap();
    assert_eq!(out.lines, ["reconnected 1 lands"]);
    assert_eq!(f.remote("main"), f.rev("main"));
}

#[test]
fn reconnect_names_rebase() {
    // req:land/reconnect-names-rebase@92d58f31
    let (f, _) = landed_locally();
    add_origin(&f, true);
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
    // req:land/offline-land@98b44467
    let (f, _) = landed_locally();
    add_origin(&f, true);
    let theirs = f.remote("main");
    let why = refused(clm_land::reconnect(&f.work(), &f.env(&[])));
    assert!(why.contains("land again impl/a"), "{why}");
    assert_eq!(f.remote("main"), theirs);
    assert!(ledger_on_origin(&f));
}

#[test]
fn reconnect_without_origin_refuses() {
    // req:land/offline-land@98b44467
    let (f, _) = landed_locally();
    let why = refused(clm_land::reconnect(&f.work(), &f.env(&[])));
    assert!(why.contains("origin"), "{why}");
}
