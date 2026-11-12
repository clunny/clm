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
    // req:machine/build-measure@322d7d46
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
    // req:machine/build-measure@322d7d46
    let (dir, _holder, tree) = clone_with_build();
    let t = clm_git::now();
    let first = json(dir.path(), t);
    let peak = first["disk"]["peak"].as_u64().unwrap();
    fs::remove_file(tree.join("three")).unwrap();
    let again = json(dir.path(), t + 11 * 60);
    let bytes = again["disk"]["builds"][0]["bytes"].as_u64().unwrap();
    assert!(bytes < 3 * MIB, "bytes {bytes}");
    assert_ne!(again["disk"]["measured"], first["disk"]["measured"]);
    // The peak never falls.
    assert_eq!(again["disk"]["peak"].as_u64(), Some(peak));
    // Not due ten minutes after that: a second measurement would change nothing, and the
    // record keeps the time it has.
    let later = json(dir.path(), t + 11 * 60 + 599);
    assert_eq!(later["disk"]["measured"], again["disk"]["measured"]);
}

#[test]
fn scenario_hard_links_once() {
    // req:machine/build-measure@322d7d46
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
    // req:machine/build-measure@322d7d46
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
    // req:machine/build-measure@322d7d46
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
fn a_record_round_trips_and_its_peak_only_rises() {
    // req:machine/build-measure@322d7d46
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
