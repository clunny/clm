mod common;

use clm_machine::{Record, State};
use common::{GIB, judged, readings, repo, space};

fn record(peak: u64, builds: &[(&str, u64)]) -> Record {
    Record {
        measured: Some(1),
        peak: Some(peak),
        builds: builds.iter().map(|(n, b)| (n.to_string(), *b)).collect(),
    }
}

#[test]
fn scenario_the_json() {
    // req:machine/machine-report@53f8492b
    let dir = repo();
    let snapshot = clm_machine::read(dir.path(), 3, 4, clm_git::now());
    let json = clm_machine::json(&snapshot.report);
    assert_eq!(json["view"], "machine");
    assert!(json["cpu"]["cores"].as_u64().unwrap() >= 1);
    assert!(json["memory"]["total"].as_u64().unwrap() > 0);
    assert!(json["disk"]["total"].as_u64().unwrap() > 0);
    assert!(json["disk"]["peak"].is_null());
    assert_eq!(json["disk"]["free_places"], 3);
    assert!(json["disk"]["need"].is_null());
    assert!(json["disk"]["measured"].is_string());
    assert_eq!(json["disk"]["builds"], serde_json::json!([]));
}

#[test]
fn scenario_sustained_cpu() {
    // req:machine/machine-report@53f8492b
    let cpu = |load1, load5| {
        let mut r = readings();
        (r.load1, r.load5) = (Some(load1), Some(load5));
        judged(&r, &Record::default(), &[], 3).cpu.state
    };
    assert_eq!(cpu(10.0, 6.0), State::Warn);
    assert_eq!(cpu(10.0, 10.0), State::Critical);
    assert_eq!(cpu(4.0, 10.0), State::Critical);
    assert_eq!(cpu(9.9, 9.9), State::Ok);
}

#[test]
fn scenario_available_not_free() {
    // req:machine/machine-report@53f8492b
    let memory = |available_gib: f64| {
        let mut r = readings();
        r.memory = space(16 * GIB, (available_gib * GIB as f64) as u64);
        judged(&r, &Record::default(), &[], 3).memory.state
    };
    assert_eq!(memory(6.0), State::Ok);
    assert_eq!(memory(2.5), State::Warn);
    assert_eq!(memory(1.0), State::Critical);
}

#[test]
fn scenario_disk_against_the_builds() {
    // req:machine/machine-report@53f8492b
    let rec = record(8 * GIB, &[("a", 5 * GIB), ("b", 8 * GIB)]);
    let judge = |free_gib: u64| {
        let mut r = readings();
        r.disk = space(100 * GIB, free_gib * GIB);
        judged(&r, &rec, &["a", "b"], 3).disk
    };
    let disk = judge(10);
    assert_eq!(disk.need, Some(11 * GIB));
    assert_eq!(disk.free_places, 1);
    assert_eq!(disk.state, State::Warn);
    assert_eq!(judge(2).state, State::Critical);
    assert_eq!(judge(12).state, State::Ok);
    // A build not yet measured owes the whole peak.
    let mut r = readings();
    r.disk = space(100 * GIB, 7 * GIB);
    let disk = judged(&r, &rec, &["a", "new"], 3).disk;
    assert_eq!(disk.need, Some((3 + 8 + 8) * GIB));
    assert_eq!(disk.state, State::Critical);
}

#[test]
fn scenario_disk_before_any_measurement() {
    // req:machine/machine-report@53f8492b
    let judge = |free_gib: u64| {
        let mut r = readings();
        r.disk = space(100 * GIB, free_gib * GIB);
        judged(&r, &Record::default(), &[], 3).disk
    };
    let disk = judge(8);
    assert_eq!(disk.state, State::Warn);
    assert_eq!(disk.need, None);
    assert_eq!(judge(4).state, State::Critical);
    assert_eq!(judge(20).state, State::Ok);
}

#[test]
fn scenario_no_load_average() {
    // req:machine/machine-report@53f8492b
    let mut r = readings();
    (r.load1, r.load5) = (None, None);
    let report = judged(&r, &Record::default(), &[], 3);
    assert_eq!(report.cpu.state, State::Unknown);
    assert_eq!(report.memory.state, State::Ok);
    assert_eq!(report.disk.state, State::Ok);
    let json = clm_machine::json(&report);
    assert!(json["cpu"]["load1"].is_null() && json["cpu"]["load5"].is_null());
    assert_eq!(json["cpu"]["state"], "unknown");
}

#[test]
fn scenario_text() {
    // req:machine/machine-report@53f8492b
    let mut r = readings();
    (r.load1, r.load5) = (Some(12.0), Some(8.0));
    r.memory = space(16 * GIB, 4 * GIB);
    r.disk = space(100 * GIB, 20 * GIB);
    let text = clm_machine::text(&judged(&r, &Record::default(), &[], 3));
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].starts_with("cpu  1.5× of 8 cores, 1.0× over 5 min"));
    assert!(lines[1].starts_with("mem  4.0G available of 16G"));
    assert!(lines[2].starts_with("disk 20G free of 100G"));
    assert!(lines[0].ends_with("  warn"));
    assert!(lines[1].ends_with("  ok"));
    assert!(lines[2].ends_with("  ok"));
}

#[test]
fn scenario_text_names_the_need() {
    // req:machine/machine-report@53f8492b
    let mut r = readings();
    (r.load1, r.load5) = (Some(12.0), Some(8.0));
    r.memory = space(16 * GIB, 4 * GIB);
    r.disk = space(100 * GIB, 20 * GIB);
    let disk_line = |rec: &Record, counting: &[&str]| {
        let text = clm_machine::text(&judged(&r, rec, counting, 3));
        text.lines().nth(2).unwrap().to_string()
    };
    let rec = record(8 * GIB, &[("a", 5 * GIB)]);
    assert_eq!(
        disk_line(&rec, &["a"]),
        "disk 20G free of 100G, 19G needed by 1 builds  ok"
    );
    assert_eq!(
        disk_line(&Record::default(), &["a"]),
        "disk 20G free of 100G  ok"
    );
}

#[test]
fn scenario_no_memory_or_disk_reading() {
    // req:machine/machine-report@53f8492b
    let mut r = readings();
    (r.load1, r.load5) = (Some(12.0), Some(8.0));
    (r.memory, r.disk) = (None, None);
    let rec = record(8 * GIB, &[("a", 5 * GIB)]);
    let report = judged(&r, &rec, &["a"], 3);
    assert_eq!(report.memory.state, State::Unknown);
    assert_eq!(report.disk.state, State::Unknown);
    assert_eq!(report.cpu.state, State::Warn);
    // With no disk reading there is no need and no room, whatever the record holds.
    assert_eq!((report.disk.need, report.disk.room), (None, None));
    let text = clm_machine::text(&report);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[1..], ["mem  -  unknown", "disk -  unknown"]);
    let json = clm_machine::json(&report);
    for key in ["total", "available"] {
        assert!(json["memory"][key].is_null() && json["disk"][key].is_null());
    }
    for key in ["need", "room"] {
        let disk = json["disk"].as_object().unwrap();
        assert!(disk.contains_key(key) && disk[key].is_null(), "{key}");
    }
}

#[test]
fn a_memory_total_of_zero_is_unknown_and_printed() {
    // req:machine/machine-report@53f8492b
    let mut r = readings();
    r.memory = space(0, 0);
    r.disk = space(0, 4096);
    let report = judged(&r, &Record::default(), &[], 3);
    assert_eq!(report.memory.state, State::Unknown);
    let text = clm_machine::text(&report);
    assert_eq!(
        text.lines().nth(1),
        Some("mem  0B available of 0B  unknown")
    );
    assert_eq!(report.disk.state, State::Critical);
    // Nothing free of nothing is critical too.
    r.disk = space(0, 0);
    assert_eq!(
        judged(&r, &Record::default(), &[], 3).disk.state,
        State::Critical
    );
}

#[test]
fn read_reports_whatever_the_files_allow() {
    // req:machine/machine-report@53f8492b
    let dir = repo();
    // A file where the clone's `clm` directory belongs: no record or history can be read
    // under it, and none written.
    let clm_dir = common::common(dir.path()).join("clm");
    std::fs::write(&clm_dir, "not a directory").unwrap();
    let now = clm_git::now();
    let snapshot = clm_machine::read(dir.path(), 3, 4, now);
    assert_eq!(snapshot.report.disk.peak, None);
    assert_eq!(snapshot.report.disk.free_places, 3);
    assert_eq!(snapshot.history.len(), 1);
    assert_eq!(snapshot.history[0].time, now);
    assert_eq!(
        std::fs::read_to_string(&clm_dir).unwrap(),
        "not a directory"
    );
}

#[test]
fn a_ratio_whose_load_average_is_unknown_is_a_dash() {
    // req:machine/machine-report@53f8492b
    let mut r = readings();
    (r.load1, r.load5) = (Some(12.0), None);
    let text = clm_machine::text(&judged(&r, &Record::default(), &[], 3));
    assert_eq!(
        text.lines().next(),
        Some("cpu  1.5× of 8 cores, - over 5 min  unknown")
    );
    (r.load1, r.load5) = (None, Some(8.0));
    let text = clm_machine::text(&judged(&r, &Record::default(), &[], 3));
    assert_eq!(
        text.lines().next(),
        Some("cpu  - of 8 cores, 1.0× over 5 min  unknown")
    );
}

#[test]
fn scenario_the_disks_state_in_the_json() {
    // req:machine/machine-report@53f8492b
    let rec = record(8 * GIB, &[("a", 5 * GIB), ("b", 8 * GIB)]);
    let mut r = readings();
    r.disk = space(100 * GIB, 2 * GIB);
    let json = clm_machine::json(&judged(&r, &rec, &["a", "b"], 3));
    assert_eq!(json["disk"]["state"], "critical");
    assert_eq!(json["disk"]["need"], 11 * GIB);
    assert_eq!(
        json["disk"]["builds"],
        serde_json::json!([{"name": "a", "bytes": 5 * GIB}, {"name": "b", "bytes": 8 * GIB}])
    );
    assert_eq!(json["disk"]["peak"], 8 * GIB);
    assert_eq!(json["disk"]["measured"], "1970-01-01T00:00:01Z");
    assert_eq!(json["disk"]["free_places"], 1);
    assert_eq!(json["disk"]["available"], 2 * GIB);
    assert_eq!(json["disk"]["total"], 100 * GIB);
    assert_eq!(json["disk"]["path"], "/clone/.git");
}

#[test]
fn scenario_an_unmeasured_build_owes_the_peak() {
    // req:machine/machine-report@53f8492b
    let rec = record(8 * GIB, &[("a", 5 * GIB)]);
    let mut r = readings();
    r.disk = space(100 * GIB, 7 * GIB);
    let disk = judged(&r, &rec, &["a", "new"], 3).disk;
    assert_eq!(disk.need, Some(19 * GIB));
    assert_eq!(disk.state, State::Critical);
}

#[test]
fn scenario_outside_a_git_repository() {
    // req:machine/machine-report@53f8492b
    let dir = tempfile::tempdir().unwrap();
    let now = clm_git::now();
    let snapshot = clm_machine::read(dir.path(), 3, 4, now);
    let disk = &snapshot.report.disk;
    assert_eq!(disk.path, dir.path());
    assert!(disk.builds.is_empty());
    assert_eq!(disk.peak, None);
    assert_eq!(disk.measured, None);
    assert_eq!(snapshot.history.len(), 1);
    assert_eq!(snapshot.history[0].time, now);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn scenario_room_against_the_builds() {
    // req:machine/disk-room@6a2b07c4
    let rec = record(8 * GIB, &[("a", 5 * GIB), ("b", 8 * GIB)]);
    let room = |free_gib: u64| {
        let mut r = readings();
        r.disk = space(100 * GIB, free_gib * GIB);
        judged(&r, &rec, &["a", "b"], 3).disk.room
    };
    assert_eq!(room(10), Some(0));
    assert_eq!(room(20), Some(2));
    assert_eq!(room(2), Some(0));
    assert_eq!(room(100), Some(12));
    // The JSON output carries it.
    let mut r = readings();
    r.disk = space(100 * GIB, 20 * GIB);
    let json = clm_machine::json(&judged(&r, &rec, &["a", "b"], 3));
    assert_eq!(json["disk"]["room"], 2);
    // A build not yet measured owes the whole peak: growth 3 + 8 GiB leaves 9 GiB, one build.
    assert_eq!(judged(&r, &rec, &["a", "new"], 3).disk.room, Some(1));
    // A peak of 0 gives no room, as no peak does.
    let mut r = readings();
    r.disk = space(100 * GIB, 50 * GIB);
    assert_eq!(judged(&r, &record(0, &[]), &[], 3).disk.room, None);
    assert_eq!(judged(&r, &Record::default(), &[], 3).disk.room, None);
    // With no disk reading there is no room, the key still present.
    r.disk = None;
    let report = judged(&r, &rec, &["a", "b"], 3);
    assert_eq!(report.disk.room, None);
    let json = clm_machine::json(&report);
    assert!(json["disk"].as_object().unwrap().contains_key("room"));
    assert!(json["disk"]["room"].is_null());
}

#[test]
fn scenario_no_room_before_any_measurement() {
    // req:machine/disk-room@6a2b07c4
    let dir = repo();
    let snapshot = clm_machine::read(dir.path(), 3, 4, clm_git::now());
    let json = clm_machine::json(&snapshot.report);
    assert!(json["disk"].as_object().unwrap().contains_key("room"));
    assert!(json["disk"]["room"].is_null());
}
