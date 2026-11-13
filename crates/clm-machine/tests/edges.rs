mod common;

use std::fs;

use clm_machine::{Record, Sample, State};
use common::{MIB, build, judged, readings, repo};

fn plain(line: &str) -> String {
    let mut out = String::new();
    let mut in_escape = false;
    for c in line.chars() {
        match (in_escape, c) {
            (_, '\x1b') => in_escape = true,
            (true, 'm') => in_escape = false,
            (false, c) => out.push(c),
            _ => {}
        }
    }
    out
}

#[test]
fn a_row_is_cut_to_a_narrow_width() {
    // req:machine/machine-history@67a1a6a6
    let mut r = readings();
    r.load5 = Some(40.0);
    let report = judged(&r, &Record::default(), &[], 3);
    let history = [Sample {
        time: 1,
        cpu: Some(0.5),
        memory: Some(0.5),
        disk: Some(0.5),
        cpu_state: State::Ok,
        memory_state: State::Ok,
        disk_state: State::Ok,
    }];
    let out = clm_machine::rows(&report, &history, 10);
    for line in out.lines() {
        assert_eq!(plain(line).chars().count(), 10, "{line:?}");
    }
}

#[test]
fn the_sample_holds_the_shares() {
    // req:machine/machine-history@67a1a6a6
    let dir = repo();
    let snapshot = clm_machine::read(dir.path(), 3, 4, clm_git::now());
    let report = &snapshot.report;
    let sample = snapshot.history[0];
    let (mem, disk) = (report.memory.space.unwrap(), report.disk.space.unwrap());
    let used = |s: clm_machine::Space| 1.0 - s.available as f64 / s.total as f64;
    assert!((sample.memory.unwrap() - used(mem)).abs() < 1e-3);
    assert!((sample.disk.unwrap() - used(disk)).abs() < 1e-3);
    let cpu = report.cpu.load1.unwrap() / report.cpu.cores as f64 / 2.0;
    assert!((sample.cpu.unwrap() - cpu).abs() < 1e-3);
}

#[test]
fn a_reading_the_system_refuses_is_unknown() {
    // req:machine/machine-report@5f920ce4
    let mut r = readings();
    (r.memory, r.disk) = (None, None);
    let report = judged(&r, &Record::default(), &[], 3);
    assert_eq!(report.memory.state, State::Unknown);
    assert_eq!(report.disk.state, State::Unknown);
    assert_eq!(report.cpu.state, State::Ok);
}

#[test]
fn a_sparse_file_takes_the_blocks_it_has() {
    // req:machine/build-measure@322d7d46
    let dir = repo();
    let (_holder, tree) = build(dir.path(), "a");
    fs::File::create(tree.join("sparse"))
        .unwrap()
        .set_len(50 * MIB)
        .unwrap();
    let snapshot = clm_machine::read(dir.path(), 3, 4, clm_git::now());
    let bytes = snapshot.report.disk.builds[0].bytes.unwrap();
    assert!(bytes < 10 * MIB, "bytes {bytes}");
}
