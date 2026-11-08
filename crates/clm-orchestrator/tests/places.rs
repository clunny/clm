//! Build places (`build-places`): the files under `<git common dir>/clm/builds/`, which of
//! them count, and taking, renaming and freeing them.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clm_orchestrator::{
    Error, Place, PlaceKind, Placed, SweepOutcome, Swept, free_place, free_stale_places,
    place_slot, places, rename_place, take_place,
};
use common::*;
use tempfile::TempDir;

/// The clone's places directory, for a clone whose git directory is `.git`.
fn builds(work: &Path) -> PathBuf {
    work.join(".git/clm/builds")
}

/// `YYYY-MM-DDTHH:MM:SSZ` for Unix seconds.
fn rfc3339(secs: u64) -> String {
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Writes place `name` by hand, taken `age` hours ago, with `branches`.
fn write_place(work: &Path, name: &str, kind: &str, age: u64, branches: &[&str]) {
    let mut text = format!("kind {kind}\ntaken {}\n", rfc3339(now() - age * 3600));
    for b in branches {
        text.push_str(&format!("branch {b}\n"));
    }
    write(&builds(work), name, &text);
}

fn counting(work: &Path) -> Vec<String> {
    places(work, 4)
        .unwrap()
        .into_iter()
        .filter(|p| p.counts)
        .map(|p| p.name)
        .collect()
}

fn files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .map(|d| {
            d.map(|e| e.unwrap().file_name().into_string().unwrap())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

fn build(node: &str) -> Vec<String> {
    vec![format!("impl/{node}")]
}

#[test]
fn scenario_lapsed_and_landed_places_dont_count() {
    // req:orchestrator/places@39abce44
    let r = repo();
    push_claim(&r.work, "a", 5);
    push_claim(&r.work, "b", 1);
    merge_claim(&r.work, "b");
    git(&r.work, &["branch", "impl/b", "origin/impl/b"]);
    push_claim(&r.work, "c", 1);
    git(&r.work, &["fetch", "-q", "origin"]);
    write_place(&r.work, "a", "build", 5, &["impl/a"]);
    write_place(&r.work, "b", "build", 0, &["impl/b"]);
    write_place(&r.work, "c", "build", 5, &["impl/c"]);
    assert_eq!(counting(&r.work), ["c"]);
    let all = places(&r.work, 4).unwrap();
    assert_eq!(
        all.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        ["a", "b", "c"]
    );
    let c = &all[2];
    let file = c.file.as_ref().unwrap();
    assert_eq!(file.kind, PlaceKind::Build);
    assert_eq!(file.branches, ["impl/c"]);
    assert!(now() - file.taken >= 5 * 3600 && now() - file.taken < 5 * 3600 + 60);
    assert_eq!(c.path, builds(&r.work).canonicalize().unwrap().join("c"));
}

#[test]
fn a_local_branch_alone_that_isnt_on_main_hasnt_landed() {
    // req:orchestrator/places@39abce44
    let r = repo();
    git(&r.work, &["commit", "-q", "--allow-empty", "-m", "work"]);
    git(&r.work, &["branch", "impl/a"]);
    write_place(&r.work, "a", "build", 0, &["impl/a"]);
    assert_eq!(counting(&r.work), ["a"]);
    git(&r.work, &["branch", "-f", "impl/a", "origin/main"]);
    assert!(counting(&r.work).is_empty(), "its tip is on main: landed");
}

#[test]
fn a_build_lands_only_when_each_tip_is_on_main() {
    // req:orchestrator/places@39abce44
    let r = repo();
    push_claim(&r.work, "a", 1);
    merge_claim(&r.work, "a");
    // The remote tip is on main, the local branch has moved on.
    git(&r.work, &["commit", "-q", "--allow-empty", "-m", "more"]);
    git(&r.work, &["branch", "impl/a"]);
    write_place(&r.work, "a", "build", 0, &["impl/a"]);
    assert_eq!(counting(&r.work), ["a"]);
}

#[test]
fn scenario_taken_before_the_claim() {
    // req:orchestrator/places@39abce44
    let r = repo();
    write_place(&r.work, "a", "build", 0, &["impl/a"]);
    assert_eq!(counting(&r.work), ["a"]);
    // Taken longer ago than the expiry, with no claim: free.
    write_place(&r.work, "a", "build", 5, &["impl/a"]);
    assert!(counting(&r.work).is_empty());
}

#[test]
fn scenario_full() {
    // req:orchestrator/places@39abce44
    let r = repo();
    assert_eq!(
        take_place(&r.work, "a", PlaceKind::Build, &build("a"), 2, 4).unwrap(),
        Placed::New
    );
    assert_eq!(
        take_place(&r.work, "b", PlaceKind::Build, &build("b"), 2, 4).unwrap(),
        Placed::New
    );
    assert_eq!(
        take_place(&r.work, "c", PlaceKind::Build, &build("c"), 2, 4).unwrap(),
        Placed::Full {
            held: 2,
            max_builds: 2
        }
    );
    assert_eq!(files(&builds(&r.work)), ["a", "b"], "no place `c`");
    assert_eq!(
        take_place(&r.work, "a", PlaceKind::Build, &build("a"), 2, 4).unwrap(),
        Placed::Reused
    );
    assert_eq!(files(&builds(&r.work)), ["a", "b"]);
}

#[test]
fn taking_writes_the_file_and_reuse_rewrites_taken() {
    // req:orchestrator/places@39abce44
    let r = repo();
    write_place(&r.work, "a", "build", 3, &["impl/a"]);
    assert_eq!(
        take_place(&r.work, "a", PlaceKind::Build, &build("a"), 1, 4).unwrap(),
        Placed::Reused
    );
    let text = fs::read_to_string(builds(&r.work).join("a")).unwrap();
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("kind build"));
    let taken = lines.next().unwrap().strip_prefix("taken ").unwrap();
    let recent = [rfc3339(now()), rfc3339(now() - 1), rfc3339(now() - 2)];
    assert!(recent.iter().any(|t| t == taken), "{taken}");
    assert_eq!(
        lines.next(),
        Some("slot 1"),
        "a place with no slot gets one"
    );
    assert_eq!(lines.next(), Some("branch impl/a"));
    assert_eq!(lines.next(), None);
}

#[test]
fn taking_deletes_the_places_that_dont_count() {
    // req:orchestrator/places@39abce44
    let r = repo();
    write_place(&r.work, "old", "build", 5, &["impl/old"]);
    write(&builds(&r.work), "z", "not a place");
    assert_eq!(
        take_place(&r.work, "a", PlaceKind::Build, &build("a"), 3, 4).unwrap(),
        Placed::New
    );
    assert_eq!(files(&builds(&r.work)), ["a", "z"], "a damaged file stays");
}

#[test]
fn a_place_name_is_one_file_name() {
    // req:orchestrator/places@39abce44
    let r = repo();
    for bad in ["", "a/b", "..", "."] {
        let res = take_place(&r.work, bad, PlaceKind::Build, &[], 3, 4);
        assert!(matches!(res, Err(Error::Refused(_))), "{bad:?}: {res:?}");
    }
}

/// Run as a subprocess by `scenario_racing_takers`: takes place `CLM_TAKER` in
/// `CLM_TAKER_ROOT` with `max_builds` 1 and prints the outcome.
#[test]
fn place_taker() {
    let (Ok(root), Ok(name)) = (std::env::var("CLM_TAKER_ROOT"), std::env::var("CLM_TAKER")) else {
        return;
    };
    // Both takers start at the same instant, so their reads and writes overlap.
    let start: u128 = std::env::var("CLM_TAKER_START").unwrap().parse().unwrap();
    while std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        < start
    {
        std::hint::spin_loop();
    }
    let placed = take_place(
        Path::new(&root),
        &name,
        PlaceKind::Build,
        &build(&name),
        1,
        4,
    );
    println!("taker:{placed:?}");
}

#[test]
fn scenario_racing_takers() {
    // req:orchestrator/places@39abce44
    let r = repo();
    let exe = std::env::current_exe().unwrap();
    let start = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        + 500;
    let takers: Vec<_> = ["a", "b"]
        .iter()
        .map(|name| {
            Command::new(&exe)
                .args(["place_taker", "--exact", "--nocapture", "--test-threads=1"])
                .env("CLM_TAKER_ROOT", &r.work)
                .env("CLM_TAKER", name)
                .env("CLM_TAKER_START", start.to_string())
                .stdout(std::process::Stdio::piped())
                .spawn()
                .unwrap()
        })
        .collect();
    let outs: Vec<String> = takers
        .into_iter()
        .map(|t| String::from_utf8(t.wait_with_output().unwrap().stdout).unwrap())
        .collect();
    let took = outs.iter().filter(|o| o.contains("taker:Ok(New)")).count();
    let full = outs
        .iter()
        .filter(|o| o.contains("taker:Ok(Full { held: 1, max_builds: 1 })"))
        .count();
    assert_eq!((took, full), (1, 1), "{outs:?}");
    assert_eq!(files(&builds(&r.work)).len(), 1);
}

#[test]
fn scenario_rename_onto_an_existing_place() {
    // req:orchestrator/places@39abce44
    let r = repo();
    write_place(&r.work, "draft-1-q", "draft", 0, &[]);
    write_place(&r.work, "20261027-q", "draft", 0, &["spec/20261027-q"]);
    let before = (
        fs::read(builds(&r.work).join("draft-1-q")).unwrap(),
        fs::read(builds(&r.work).join("20261027-q")).unwrap(),
    );
    match rename_place(&r.work, "draft-1-q", "20261027-q") {
        Err(Error::Refused(why)) => assert!(why.contains("`20261027-q` exists"), "{why}"),
        other => panic!("{other:?}"),
    }
    let after = (
        fs::read(builds(&r.work).join("draft-1-q")).unwrap(),
        fs::read(builds(&r.work).join("20261027-q")).unwrap(),
    );
    assert_eq!(before, after);
}

#[test]
fn rename_moves_the_place() {
    // req:orchestrator/places@39abce44
    let r = repo();
    write_place(&r.work, "draft-1-q", "draft", 0, &[]);
    rename_place(&r.work, "draft-1-q", "20261027-q").unwrap();
    assert_eq!(files(&builds(&r.work)), ["20261027-q"]);
    assert!(matches!(
        rename_place(&r.work, "draft-1-q", "x"),
        Err(Error::Refused(_))
    ));
}

#[test]
fn free_deletes_and_succeeds_when_missing() {
    // req:orchestrator/places@39abce44
    let r = repo();
    free_place(&r.work, "a").unwrap();
    take_place(&r.work, "a", PlaceKind::Build, &build("a"), 3, 4).unwrap();
    free_place(&r.work, "a").unwrap();
    assert!(files(&builds(&r.work)).is_empty());
    free_place(&r.work, "a").unwrap();
}

#[test]
fn scenario_places_in_a_bare_repository() {
    // req:orchestrator/places@39abce44
    let r = repo();
    let tmp = TempDir::new().unwrap();
    let bare = tmp.path().join("bare.git");
    git(
        tmp.path(),
        &[
            "clone",
            "-q",
            "--bare",
            r.origin.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    let wt = tmp.path().join("wt");
    git(
        &bare,
        &["worktree", "add", "-q", wt.to_str().unwrap(), "main"],
    );
    take_place(&wt, "a", PlaceKind::Build, &build("a"), 3, 4).unwrap();
    let listed: Vec<Place> = places(&wt, 4).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "a");
    assert!(listed[0].counts);
    assert!(
        listed[0]
            .path
            .starts_with(bare.canonicalize().unwrap().join("clm/builds")),
        "{:?}",
        listed[0].path
    );
}

#[test]
fn scenario_draft_place() {
    // req:orchestrator/places@39abce44
    let r = repo();
    push_propose(&r.work, "q", 5);
    push_branch(&r.work, "spec/20261027-x", 1);
    git(&r.work, &["fetch", "-q", "origin"]);
    write_place(
        &r.work,
        "20261027-x",
        "draft",
        5,
        &["spec/20261027-x", "propose/q"],
    );
    assert_eq!(counting(&r.work), ["20261027-x"]);
    let p = &places(&r.work, 4).unwrap()[0];
    assert_eq!(p.file.as_ref().unwrap().kind, PlaceKind::Draft);
    write(&r.work, "changes/20261027-x/change.md", &change("proposed"));
    push_main(&r.work, "propose x");
    git(&r.work, &["fetch", "-q", "origin"]);
    assert!(counting(&r.work).is_empty(), "landed");
}

#[test]
fn a_draft_place_lives_on_its_propose_claim_or_autosync_branch() {
    // req:orchestrator/places@39abce44
    let r = repo();
    write_place(&r.work, "d1", "draft", 5, &["spec/d1", "propose/q"]);
    assert!(counting(&r.work).is_empty(), "no claim and taken long ago");
    push_propose(&r.work, "q", 1);
    git(&r.work, &["fetch", "-q", "origin"]);
    assert_eq!(counting(&r.work), ["d1"]);
    write_place(&r.work, "d2", "draft", 5, &["spec/d2"]);
    push_branch(&r.work, "autosync/spec/d2", 1);
    git(&r.work, &["fetch", "-q", "origin"]);
    assert_eq!(counting(&r.work), ["d1", "d2"]);
    // An archived change has landed too.
    write(&r.work, "changes/archive/d2/change.md", &change("applied"));
    push_main(&r.work, "archive d2");
    git(&r.work, &["fetch", "-q", "origin"]);
    assert_eq!(counting(&r.work), ["d1"]);
}

#[test]
fn a_provisional_draft_place_counts_by_its_time() {
    // req:orchestrator/places@39abce44
    let r = repo();
    write_place(&r.work, "draft-1-q", "draft", 0, &[]);
    assert_eq!(counting(&r.work), ["draft-1-q"]);
    write_place(&r.work, "draft-1-q", "draft", 5, &[]);
    assert!(counting(&r.work).is_empty());
}

#[test]
fn scenario_damaged_file() {
    // req:orchestrator/places@39abce44
    let r = repo();
    write(&builds(&r.work), "z", "not a place");
    write(&builds(&r.work), "y", "kind build\n");
    write(
        &builds(&r.work),
        "x",
        "kind boat\ntaken 2026-10-27T00:00:00Z\n",
    );
    let all = places(&r.work, 4).unwrap();
    assert_eq!(all.len(), 3);
    for p in &all {
        assert!(p.counts, "{}", p.name);
        assert!(p.file.is_err(), "{}", p.name);
    }
}

#[test]
fn no_places_directory_is_no_places() {
    // req:orchestrator/places@39abce44
    let r = repo();
    assert!(places(&r.work, 4).unwrap().is_empty());
    assert!(!builds(&r.work).exists(), "reading creates nothing");
}

#[test]
fn places_doesnt_fetch() {
    // req:orchestrator/places@39abce44
    let r = repo();
    write_place(&r.work, "a", "build", 5, &["impl/a"]);
    // Pushed from elsewhere, so the clone's tracking ref doesn't know it yet.
    let other = TempDir::new().unwrap();
    let o = other.path().join("o");
    git(
        other.path(),
        &[
            "clone",
            "-q",
            r.origin.to_str().unwrap(),
            o.to_str().unwrap(),
        ],
    );
    identify(&o);
    push_claim(&o, "a", 1);
    assert!(counting(&r.work).is_empty(), "not fetched yet");
    git(&r.work, &["fetch", "-q", "origin"]);
    assert_eq!(counting(&r.work), ["a"]);
}

fn outcomes(swept: &[Swept]) -> Vec<(&str, &SweepOutcome)> {
    swept
        .iter()
        .map(|s| (s.place.name.as_str(), &s.outcome))
        .collect()
}

#[test]
fn scenario_sweep() {
    // req:orchestrator/free-stale-places@58781ca7
    let r = repo();
    push_claim(&r.work, "a", 5);
    push_claim(&r.work, "b", 1);
    merge_claim(&r.work, "b");
    git(&r.work, &["branch", "impl/b", "origin/impl/b"]);
    push_claim(&r.work, "c", 1);
    git(&r.work, &["fetch", "-q", "origin"]);
    write_place(&r.work, "a", "build", 5, &["impl/a"]);
    write_place(&r.work, "b", "build", 0, &["impl/b"]);
    write_place(&r.work, "c", "build", 5, &["impl/c"]);
    write(&builds(&r.work), "z", "not a place");
    let swept = free_stale_places(&r.work, 4).unwrap();
    assert_eq!(
        outcomes(&swept),
        [
            ("a", &SweepOutcome::Freed),
            ("b", &SweepOutcome::Freed),
            ("c", &SweepOutcome::Kept),
            ("z", &SweepOutcome::Kept),
        ]
    );
    assert_eq!(files(&builds(&r.work)), ["c", "z"]);
    // Each `Place` as `places` would return it.
    let z = &swept[3].place;
    assert!(z.counts);
    // With why it can't be parsed.
    assert_eq!(z.file, Err("unknown key `not`".to_string()));
    assert!(swept[2].place.counts && !swept[0].place.counts);
    assert_eq!(
        places(&r.work, 4).unwrap(),
        [swept[2].place.clone(), z.clone()]
    );
}

#[test]
fn sweeping_no_places_returns_none() {
    // req:orchestrator/free-stale-places@58781ca7
    let r = repo();
    assert!(free_stale_places(&r.work, 4).unwrap().is_empty());
}

#[test]
fn sweeping_doesnt_fetch() {
    // req:orchestrator/free-stale-places@58781ca7
    let r = repo();
    write_place(&r.work, "a", "build", 5, &["impl/a"]);
    // Claimed from elsewhere, so the clone's tracking ref doesn't know it yet.
    let other = TempDir::new().unwrap();
    let o = other.path().join("o");
    git(
        other.path(),
        &[
            "clone",
            "-q",
            r.origin.to_str().unwrap(),
            o.to_str().unwrap(),
        ],
    );
    identify(&o);
    push_claim(&o, "a", 1);
    let swept = free_stale_places(&r.work, 4).unwrap();
    assert_eq!(outcomes(&swept), [("a", &SweepOutcome::Freed)]);
}

/// Run as a subprocess by `scenario_retaken_while_waiting`: holds `builds.lock` in the
/// clone at `CLM_HOLDER_ROOT`, writes `CLM_HOLDER_READY`, waits, rewrites place `a`
/// taken now, and releases the lock by exiting.
#[test]
fn lock_holder() {
    let (Ok(root), Ok(ready)) = (
        std::env::var("CLM_HOLDER_ROOT"),
        std::env::var("CLM_HOLDER_READY"),
    ) else {
        return;
    };
    let work = Path::new(&root);
    let lock = fs::File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(work.join(".git/clm/builds.lock"))
        .unwrap();
    lock.lock().unwrap();
    fs::write(&ready, "").unwrap();
    // Long enough for the sweep to start and block on the lock.
    std::thread::sleep(std::time::Duration::from_millis(1500));
    write_place(work, "a", "build", 0, &["impl/a"]);
    drop(lock);
}

#[test]
fn scenario_retaken_while_waiting() {
    // req:orchestrator/free-stale-places@58781ca7
    let r = repo();
    write_place(&r.work, "a", "build", 5, &["impl/a"]);
    assert!(counting(&r.work).is_empty(), "`a` doesn't count");
    let tmp = TempDir::new().unwrap();
    let ready = tmp.path().join("ready");
    /// Kills the holder when the test fails before it exits, so it never outlives the test.
    struct Holder(std::process::Child);
    impl Drop for Holder {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut holder = Holder(
        Command::new(std::env::current_exe().unwrap())
            .args(["lock_holder", "--exact", "--nocapture", "--test-threads=1"])
            .env("CLM_HOLDER_ROOT", &r.work)
            .env("CLM_HOLDER_READY", &ready)
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !ready.exists() {
        assert!(std::time::Instant::now() < deadline, "holder never locked");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let swept = free_stale_places(&r.work, 4).unwrap();
    assert!(holder.0.wait().unwrap().success());
    assert_eq!(outcomes(&swept), [("a", &SweepOutcome::Kept)]);
    assert!(swept[0].place.counts);
    assert_eq!(files(&builds(&r.work)), ["a"]);
}

#[cfg(unix)]
#[test]
fn a_failed_delete_is_reported_and_the_sweep_goes_on() {
    // req:orchestrator/free-stale-places@58781ca7
    use std::os::unix::fs::PermissionsExt;
    let r = repo();
    write_place(&r.work, "a", "build", 5, &["impl/a"]);
    write_place(&r.work, "b", "build", 5, &["impl/b"]);
    write_place(&r.work, "c", "build", 0, &["impl/c"]);
    let dir = builds(&r.work);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    // Root deletes from a read-only directory: nothing to test there.
    let writable = fs::write(dir.join("probe"), "").is_ok();
    let swept = (!writable).then(|| free_stale_places(&r.work, 4));
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    let Some(swept) = swept else {
        return;
    };
    let swept = swept.unwrap();
    assert_eq!(
        swept
            .iter()
            .map(|s| s.place.name.as_str())
            .collect::<Vec<_>>(),
        ["a", "b", "c"]
    );
    for s in &swept[..2] {
        match &s.outcome {
            SweepOutcome::Failed(why) => assert!(why.contains(&s.place.name), "{why}"),
            other => panic!("{}: {other:?}", s.place.name),
        }
    }
    assert_eq!(swept[2].outcome, SweepOutcome::Kept);
    assert_eq!(files(&dir), ["a", "b", "c"]);
}

/// Writes place `name` by hand, taken now, with `slot` (none for a pre-slot file).
fn write_slotted(work: &Path, name: &str, age: u64, slot: Option<u64>, branches: &[&str]) {
    let mut text = format!("kind build\ntaken {}\n", rfc3339(now() - age * 3600));
    if let Some(n) = slot {
        text.push_str(&format!("slot {n}\n"));
    }
    for b in branches {
        text.push_str(&format!("branch {b}\n"));
    }
    write(&builds(work), name, &text);
}

fn slot_of(work: &Path, name: &str) -> Option<u64> {
    places(work, 4)
        .unwrap()
        .into_iter()
        .find(|p| p.name == name)
        .and_then(|p| p.file.ok())
        .and_then(|f| f.slot)
}

#[test]
fn scenario_the_lowest_free_slot() {
    // req:orchestrator/place-slots@e7802b3b
    let r = repo();
    write_slotted(&r.work, "a", 0, Some(1), &["impl/a"]);
    write_slotted(&r.work, "b", 5, Some(2), &["impl/b"]);
    write_slotted(&r.work, "c", 0, Some(3), &["impl/c"]);
    assert_eq!(
        take_place(&r.work, "d", PlaceKind::Build, &build("d"), 4, 4).unwrap(),
        Placed::New
    );
    let text = fs::read_to_string(builds(&r.work).join("d")).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[1].split(' ').next(), Some("taken"));
    assert_eq!(lines[2], "slot 2", "after `taken`, before `branch`");
    assert_eq!(lines[3], "branch impl/d");
    assert_eq!(slot_of(&r.work, "d"), Some(2));
    assert_eq!(files(&builds(&r.work)), ["a", "c", "d"], "no place `b`");
}

#[test]
fn scenario_a_reused_place_keeps_its_slot() {
    // req:orchestrator/place-slots@e7802b3b
    let r = repo();
    write_slotted(&r.work, "a", 0, Some(3), &["impl/a"]);
    assert_eq!(
        take_place(&r.work, "a", PlaceKind::Build, &build("a"), 4, 4).unwrap(),
        Placed::Reused
    );
    let text = fs::read_to_string(builds(&r.work).join("a")).unwrap();
    assert!(text.lines().any(|l| l == "slot 3"), "{text}");
    assert_eq!(slot_of(&r.work, "a"), Some(3));
}

#[test]
fn scenario_a_place_written_before_slots() {
    // req:orchestrator/place-slots@e7802b3b
    let r = repo();
    write_slotted(&r.work, "a", 0, None, &["impl/a"]);
    write_slotted(&r.work, "b", 0, Some(1), &["impl/b"]);
    assert_eq!(
        slot_of(&r.work, "a"),
        None,
        "it parses as before, with none"
    );
    assert_eq!(
        take_place(&r.work, "a", PlaceKind::Build, &build("a"), 4, 4).unwrap(),
        Placed::Reused
    );
    let text = fs::read_to_string(builds(&r.work).join("a")).unwrap();
    assert!(text.lines().any(|l| l == "slot 2"), "{text}");
}

#[test]
fn drafts_hold_slots_and_a_damaged_file_holds_none() {
    // req:orchestrator/place-slots@e7802b3b
    let r = repo();
    write(&builds(&r.work), "z", "not a place");
    assert_eq!(
        take_place(&r.work, "d", PlaceKind::Draft, &["spec/d".into()], 4, 4).unwrap(),
        Placed::New
    );
    assert_eq!(slot_of(&r.work, "d"), Some(1));
    assert_eq!(slot_of(&r.work, "z"), None);
}

#[test]
fn scenario_the_slot_of_a_branch() {
    // req:orchestrator/place-slots@e7802b3b
    let r = repo();
    write_slotted(&r.work, "x", 0, Some(2), &["impl/x"]);
    assert_eq!(place_slot(&r.work, "impl/x").unwrap(), 2);
    assert_eq!(place_slot(&r.work, "impl/y").unwrap(), 0);
}

#[test]
fn a_slot_is_read_without_judging_the_place() {
    // req:orchestrator/place-slots@e7802b3b
    let r = repo();
    // A place that doesn't count still names its slot, and the first by name wins.
    write_slotted(&r.work, "m", 9, Some(5), &["impl/x"]);
    write_slotted(&r.work, "k", 9, Some(4), &["impl/x"]);
    write_slotted(&r.work, "n", 0, None, &["impl/q"]);
    assert_eq!(place_slot(&r.work, "impl/x").unwrap(), 4);
    assert_eq!(place_slot(&r.work, "impl/q").unwrap(), 0, "no slot line");
    let outside = TempDir::new().unwrap();
    assert!(place_slot(outside.path(), "impl/x").is_err());
}
