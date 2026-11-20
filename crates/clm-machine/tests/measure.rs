mod common;

use std::fs;

use clm_machine::Record;
use common::{MIB, build, common, fill, repo};

fn json(repo: &std::path::Path, now: u64) -> serde_json::Value {
    clm_machine::json(&clm_machine::read(repo, 3, 4, now).report)
}

/// A clone with a counting build whose worktree holds a 3 MiB file and a symbolic link to a
/// 50 MiB file outside it.
fn clone_with_build() -> (tempfile::TempDir, tempfile::TempDir, std::path::PathBuf) {
    let dir = repo();
    let (holder, tree) = build(dir.path(), "a");
    fill(&tree.join("three"), 3 * MIB);
    let outside = holder.path().join("outside");
    fill(&outside, 50 * MIB);
    std::os::unix::fs::symlink(&outside, tree.join("link")).unwrap();
    (dir, holder, tree)
}

#[test]
fn scenario_measured_and_kept() {
    // req:machine/build-measure@25e6eaf6
    let (dir, _holder, tree) = clone_with_build();
    let record_path = common(dir.path()).join("clm/machine");
    let t = clm_git::now();
    let first = json(dir.path(), t);
    let peak = first["disk"]["peak"].as_u64().unwrap();
    assert!((3 * MIB..50 * MIB).contains(&peak), "peak {peak}");
    let bytes = first["disk"]["builds"][0]["bytes"].as_u64().unwrap();
    assert_eq!(first["disk"]["builds"][0]["name"], "a");
    let written = fs::read_to_string(&record_path).unwrap();
    fs::remove_file(tree.join("three")).unwrap();
    let second = json(dir.path(), t + 60);
    assert_eq!(second["disk"]["builds"][0]["bytes"].as_u64(), Some(bytes));
    assert_eq!(second["disk"]["peak"].as_u64(), Some(peak));
    assert_eq!(second["disk"]["measured"], first["disk"]["measured"]);
    assert_eq!(fs::read_to_string(&record_path).unwrap(), written);
}

#[test]
fn scenario_due_again() {
    // req:machine/build-measure@25e6eaf6
    let (dir, _holder, tree) = clone_with_build();
    let t = clm_git::now();
    let first = json(dir.path(), t);
    let peak = first["disk"]["peak"].as_u64().unwrap();
    fs::remove_file(tree.join("three")).unwrap();
    let again = json(dir.path(), t + 11 * 60);
    let bytes = again["disk"]["builds"][0]["bytes"].as_u64().unwrap();
    assert!(bytes < 3 * MIB, "bytes {bytes}");
    assert_eq!(
        again["disk"]["measured"],
        clm_machine::format_time(t + 11 * 60)
    );
    // The peak never falls.
    assert_eq!(again["disk"]["peak"].as_u64(), Some(peak));
    // Not due ten minutes after that: a second measurement would change nothing, and the
    // record keeps the time it has.
    let later = json(dir.path(), t + 11 * 60 + 599);
    assert_eq!(later["disk"]["measured"], again["disk"]["measured"]);
    // Ten minutes old exactly, it is due.
    let due = json(dir.path(), t + 11 * 60 + 600);
    assert_eq!(
        due["disk"]["measured"],
        clm_machine::format_time(t + 11 * 60 + 600)
    );
}

#[test]
fn scenario_hard_links_once() {
    // req:machine/build-measure@25e6eaf6
    let dir = repo();
    let (_holder, tree) = build(dir.path(), "a");
    let empty = json(dir.path(), clm_git::now());
    let base = empty["disk"]["peak"].as_u64().unwrap();
    fill(&tree.join("big"), 4 * MIB);
    fs::hard_link(tree.join("big"), tree.join("again")).unwrap();
    let measured = json(dir.path(), clm_git::now() + 11 * 60);
    let size = measured["disk"]["builds"][0]["bytes"].as_u64().unwrap();
    assert!(size >= 4 * MIB, "size {size}");
    assert!(size < 8 * MIB, "size {size}");
    assert!(size < base + 5 * MIB);
}

#[test]
fn scenario_damaged_record() {
    // req:machine/build-measure@25e6eaf6
    let (dir, _holder, _tree) = clone_with_build();
    let record_path = common(dir.path()).join("clm/machine");
    fs::create_dir_all(record_path.parent().unwrap()).unwrap();
    fs::write(&record_path, "not a record").unwrap();
    let json = json(dir.path(), clm_git::now());
    assert!(json["disk"]["peak"].as_u64().unwrap() >= 3 * MIB);
    let text = fs::read_to_string(&record_path).unwrap();
    let record = Record::parse(&text).expect("the record parses afterwards");
    assert!(record.peak.unwrap() >= 3 * MIB);
    assert_eq!(record.builds.len(), 1);
}

#[test]
fn a_place_without_a_worktree_is_unmeasured() {
    // req:machine/build-measure@25e6eaf6
    let dir = repo();
    clm_orchestrator::take_place(
        dir.path(),
        "ghost",
        clm_orchestrator::PlaceKind::Build,
        &["impl/ghost".to_string()],
        3,
        4,
    )
    .unwrap();
    let json = json(dir.path(), clm_git::now());
    assert_eq!(json["disk"]["builds"][0]["name"], "ghost");
    assert!(json["disk"]["builds"][0]["bytes"].is_null());
    assert!(json["disk"]["peak"].is_null());
    assert_eq!(json["disk"]["free_places"], 2);
}

#[test]
fn a_worktree_on_another_branch_is_not_the_places() {
    // req:machine/build-measure@25e6eaf6
    // `ghost` names `impl/ghost`; the only other worktrees are the main checkout and one on
    // `impl/other`, which holds a 3 MiB file but no place.
    let dir = repo();
    let holder = tempfile::tempdir().unwrap();
    let other = holder.path().join("other");
    clm_git::git(
        dir.path(),
        &[
            "worktree",
            "add",
            "-q",
            other.to_str().unwrap(),
            "-b",
            "impl/other",
        ],
    )
    .unwrap();
    fill(&other.join("three"), 3 * MIB);
    clm_orchestrator::take_place(
        dir.path(),
        "ghost",
        clm_orchestrator::PlaceKind::Build,
        &["impl/ghost".to_string()],
        3,
        4,
    )
    .unwrap();
    let json = json(dir.path(), clm_git::now());
    assert_eq!(json["disk"]["builds"].as_array().unwrap().len(), 1);
    assert_eq!(json["disk"]["builds"][0]["name"], "ghost");
    assert!(json["disk"]["builds"][0]["bytes"].is_null());
    assert!(json["disk"]["peak"].is_null());
}

#[test]
fn a_place_that_does_not_count_is_not_measured() {
    // req:machine/build-measure@25e6eaf6
    // `stale` was taken long ago and has no live claim, so it no longer counts: its 3 MiB
    // worktree is neither listed nor measured, and only `a` is.
    let dir = repo();
    let (_a, _tree_a) = build(dir.path(), "a");
    let (_stale, tree_stale) = build(dir.path(), "stale");
    fill(&tree_stale.join("three"), 3 * MIB);
    let place = common(dir.path()).join("clm/builds/stale");
    let text = fs::read_to_string(&place).unwrap();
    let old: String = text
        .lines()
        .map(|l| {
            if l.starts_with("taken ") {
                "taken 2020-01-01T00:00:00Z\n".to_string()
            } else {
                format!("{l}\n")
            }
        })
        .collect();
    fs::write(&place, old).unwrap();
    let json = json(dir.path(), clm_git::now());
    let builds = json["disk"]["builds"].as_array().unwrap();
    assert_eq!(builds.len(), 1, "{builds:?}");
    assert_eq!(builds[0]["name"], "a");
    assert_eq!(json["disk"]["free_places"], 2);
    assert!(json["disk"]["peak"].as_u64().unwrap() < 3 * MIB);
    let record = fs::read_to_string(common(dir.path()).join("clm/machine")).unwrap();
    assert!(!record.contains("stale"), "{record}");
}

#[test]
fn a_record_round_trips_and_its_peak_only_rises() {
    // req:machine/build-measure@25e6eaf6
    let record = Record {
        measured: Some(1_700_000_000),
        peak: Some(900),
        builds: vec![("a".into(), 900), ("b c".into(), 5)],
    };
    assert_eq!(Record::parse(&record.render()), Some(record.clone()));
    let next = record.measured_at(1_700_000_700, vec![("a".into(), 400)]);
    assert_eq!(next.peak, Some(900));
    assert_eq!(next.builds, vec![("a".to_string(), 400)]);
    let higher = next.measured_at(1_700_001_400, vec![("a".into(), 1000)]);
    assert_eq!(higher.peak, Some(1000));
}

#[test]
fn a_record_dated_in_the_future_is_due() {
    // req:machine/build-measure@25e6eaf6
    let (dir, _holder, tree) = clone_with_build();
    let t = clm_git::now();
    json(dir.path(), t + 3600);
    fs::remove_file(tree.join("three")).unwrap();
    let again = json(dir.path(), t);
    assert_eq!(again["disk"]["measured"], clm_machine::format_time(t));
    let bytes = again["disk"]["builds"][0]["bytes"].as_u64().unwrap();
    assert!(bytes < 3 * MIB, "measured again: bytes {bytes}");
}

#[test]
fn a_record_dated_now_is_reused() {
    // req:machine/build-measure@25e6eaf6
    // Only a record dated after `now` is due: one dated `now` itself is reused.
    let (dir, _holder, tree) = clone_with_build();
    let record_path = common(dir.path()).join("clm/machine");
    let t = clm_git::now();
    let first = json(dir.path(), t);
    let written = fs::read_to_string(&record_path).unwrap();
    fs::remove_file(tree.join("three")).unwrap();
    let again = json(dir.path(), t);
    assert_eq!(again["disk"]["builds"], first["disk"]["builds"]);
    assert!(again["disk"]["builds"][0]["bytes"].as_u64().unwrap() >= 3 * MIB);
    assert_eq!(fs::read_to_string(&record_path).unwrap(), written);
}

#[test]
fn a_record_that_cannot_be_read_counts_as_none() {
    // req:machine/build-measure@25e6eaf6
    let (dir, _holder, _tree) = clone_with_build();
    // A directory where the record belongs can be neither read nor replaced.
    let record_path = common(dir.path()).join("clm/machine");
    fs::create_dir_all(&record_path).unwrap();
    let t = clm_git::now();
    let first = json(dir.path(), t);
    assert!(first["disk"]["peak"].as_u64().unwrap() >= 3 * MIB);
    assert_eq!(first["disk"]["measured"], clm_machine::format_time(t));
    // Nothing was kept, so the next run measures again.
    let again = json(dir.path(), t + 60);
    assert_eq!(again["disk"]["measured"], clm_machine::format_time(t + 60));
    assert!(record_path.is_dir());
}
