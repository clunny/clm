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
    // req:machine/machine-report@6fa452c0
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
    // req:machine/machine-report@6fa452c0
    let cpu = |load1, load5| {
        let mut r = readings();
        (r.load1, r.load5) = (Some(load1), Some(load5));
        judged(&r, &Record::default(), &[], 3).cpu.state
    };
    assert_eq!(cpu(20.0, 11.0), State::Warn);
    assert_eq!(cpu(20.0, 16.0), State::Critical);
    assert_eq!(cpu(20.0, 6.0), State::Ok);
}

#[test]
fn scenario_available_not_free() {
    // req:machine/machine-report@6fa452c0
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
    // req:machine/machine-report@6fa452c0
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
    // req:machine/machine-report@6fa452c0
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
    // req:machine/machine-report@6fa452c0
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
    // req:machine/machine-report@6fa452c0
    let mut r = readings();
    (r.load1, r.load5) = (Some(8.0), Some(11.0));
    r.memory = space(16 * GIB, 4 * GIB);
    r.disk = space(100 * GIB, 20 * GIB);
    let text = clm_machine::text(&judged(&r, &Record::default(), &[], 3));
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].starts_with("cpu  1.0× of 8 cores, 1.4× over 5 min"));
    assert!(lines[1].starts_with("mem  4.0G available of 16G"));
    assert!(lines[2].starts_with("disk 20G free of 100G"));
    assert!(lines[0].ends_with("  warn"));
    assert!(lines[1].ends_with("  ok"));
    assert!(lines[2].ends_with("  ok"));
    // With a measured peak the disk line says what the builds need.
    let rec = record(8 * GIB, &[("a", 5 * GIB)]);
    let text = clm_machine::text(&judged(&r, &rec, &["a"], 3));
    assert!(text.contains("disk 20G free of 100G, 19G needed by 1 builds  ok"));
}
