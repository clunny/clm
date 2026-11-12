mod common;

use std::fs;

use clm_machine::{Record, Report, Sample, State};
use common::{GIB, common, judged, readings, repo, space};

/// Strips ANSI colour, leaving what a terminal draws.
fn plain(line: &str) -> String {
    let mut out = String::new();
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for e in chars.by_ref() {
                if e == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn sample(time: u64, cpu: Option<f64>) -> Sample {
    Sample {
        time,
        cpu,
        memory: Some(0.5),
        disk: Some(0.5),
    }
}

/// The scenario's latest reading: CPU `warn`, memory `ok`, disk `critical`.
fn latest() -> Report {
    let mut r = readings();
    (r.load1, r.load5) = (Some(8.0), Some(11.0));
    r.memory = space(16 * GIB, 4 * GIB);
    r.disk = space(100 * GIB, 20 * GIB);
    let mut report = judged(&r, &Record::default(), &[], 3);
    report.disk.state = State::Critical;
    report
}

#[test]
fn scenario_rows_from_the_history() {
    // req:machine/machine-history@48d41228
    let history = [
        sample(1, Some(0.2)),
        sample(2, Some(0.3)),
        sample(3, Some(0.5)),
    ];
    let out = clm_machine::rows(&latest(), &history, 40);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3);
    let cells: Vec<String> = lines.iter().map(|l| plain(l)).collect();
    assert_eq!(cells[0], "cpu  ▂▃▄ 1.0× warn");
    assert!(cells[1].starts_with("mem  ") && cells[1].ends_with("▄▄▄ 4.0G avail"));
    assert_eq!(cells[2], "disk ▄▄▄ 20G free critical");
    assert!(lines[0].starts_with("\x1b[33m"), "warn is amber");
    assert!(lines[1].starts_with("\x1b[32m"), "ok is green");
    assert!(lines[2].starts_with("\x1b[31m"), "critical is rose");
    for line in &lines {
        assert!(line.ends_with("\x1b[0m"));
    }
}

#[test]
fn scenario_the_newest_fit() {
    // req:machine/machine-history@48d41228
    let history: Vec<Sample> = (0..180)
        .map(|i| sample(i, Some(i as f64 / 180.0)))
        .collect();
    let out = clm_machine::rows(&latest(), &history, 30);
    for line in out.lines() {
        let cells = plain(line);
        assert!(cells.chars().count() <= 30, "{cells:?}");
    }
    let cpu = plain(out.lines().next().unwrap());
    let chart: String = cpu.chars().filter(|c| "▁▂▃▄▅▆▇█".contains(*c)).collect();
    assert!(chart.chars().count() > 3);
    // The newest sample (share just under 1) is the chart's last block, and the oldest of the
    // history is out of sight.
    assert!(chart.ends_with('█'));
    assert!(chart.starts_with(|c| c != '▁'));
}

#[test]
fn scenario_unknown_cpu() {
    // req:machine/machine-history@48d41228
    let mut r = readings();
    (r.load1, r.load5) = (None, None);
    let report = judged(&r, &Record::default(), &[], 3);
    let history = [sample(1, Some(0.5)), sample(2, None)];
    let out = clm_machine::rows(&report, &history, 40);
    let cpu = out.lines().next().unwrap();
    assert!(cpu.starts_with("\x1b[2m"), "unknown is dimmed");
    assert_eq!(plain(cpu), "cpu  ▄  - unknown");
}

#[test]
fn a_chart_cell_is_the_share_times_eight_rounded_up() {
    // req:machine/machine-history@48d41228
    let shares = [0.0, 0.01, 0.125, 0.126, 0.5, 0.9, 1.0, 1.7];
    let history: Vec<Sample> = shares
        .iter()
        .enumerate()
        .map(|(i, s)| sample(i as u64, Some(*s)))
        .collect();
    let out = clm_machine::rows(&latest(), &history, 80);
    assert!(plain(out.lines().next().unwrap()).starts_with("cpu  ▁▁▁▂▄███ "));
}

/// The history file of a clone, whose samples are `n` seconds apart ending `age` seconds
/// before `now`.
fn write_history(dir: &std::path::Path, now: u64, count: u64, age: u64, spacing: u64) {
    let path = common(dir).join("clm/machine-samples");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let lines: String = (0..count)
        .map(|i| {
            let t = now - age - (count - 1 - i) * spacing;
            format!("{} 0.2000 0.3000 0.4000\n", clm_machine::format_time(t))
        })
        .collect();
    fs::write(path, lines).unwrap();
}

fn samples(dir: &std::path::Path) -> Vec<String> {
    fs::read_to_string(common(dir).join("clm/machine-samples"))
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn scenario_thirty_minutes_kept() {
    // req:machine/machine-history@48d41228
    let dir = repo();
    let now = clm_git::now();
    // 180 samples ten seconds apart would span 30 minutes: the oldest is taken 30 minutes
    // before the run, which comes 10 seconds after the newest.
    write_history(dir.path(), now, 180, 10, 10);
    let before = samples(dir.path());
    assert_eq!(before.len(), 180);
    let oldest_at_start = before[0].clone();
    let snapshot = clm_machine::read(dir.path(), 3, 4, now);
    let after = samples(dir.path());
    assert_eq!(after.len(), 180);
    assert!(!after.contains(&oldest_at_start));
    assert_eq!(snapshot.history.len(), 180);
    assert!(after[179].starts_with(&clm_machine::format_time(now)));
    assert_eq!(snapshot.history.last().unwrap().time, now);
}

#[test]
fn samples_older_than_thirty_minutes_are_dropped() {
    // req:machine/machine-history@48d41228
    let dir = repo();
    let now = clm_git::now();
    write_history(dir.path(), now, 3, 31 * 60, 60);
    let snapshot = clm_machine::read(dir.path(), 3, 4, now);
    assert_eq!(snapshot.history.len(), 1);
    assert_eq!(samples(dir.path()).len(), 1);
}

#[test]
fn scenario_two_runs_at_once() {
    // req:machine/machine-history@48d41228
    let dir = repo();
    let now = clm_git::now();
    write_history(dir.path(), now, 5, 2, 20);
    let before = samples(dir.path());
    let snapshot = clm_machine::read(dir.path(), 3, 4, now);
    assert_eq!(samples(dir.path()), before);
    assert_eq!(snapshot.history.len(), 5);
    // Five seconds after the newest it is a new sample.
    clm_machine::read(dir.path(), 3, 4, now + 2);
    assert_eq!(samples(dir.path()).len(), 5);
    clm_machine::read(dir.path(), 3, 4, now + 3);
    assert_eq!(samples(dir.path()).len(), 6);
}

#[test]
fn scenario_damaged_history() {
    // req:machine/machine-history@48d41228
    let dir = repo();
    let path = common(dir.path()).join("clm/machine-samples");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "not a sample").unwrap();
    let snapshot = clm_machine::read(dir.path(), 3, 4, clm_git::now());
    assert_eq!(snapshot.history.len(), 1);
    let out = clm_machine::rows(&snapshot.report, &snapshot.history, 60);
    for line in out.lines() {
        let blocks = plain(line)
            .chars()
            .filter(|c| "▁▂▃▄▅▆▇█".contains(*c))
            .count();
        assert_eq!(blocks, 1, "{line:?}");
    }
    assert_eq!(samples(dir.path()).len(), 1);
}
