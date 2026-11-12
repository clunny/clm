//! The three output forms: text, JSON and the chart rows (`machine-report`,
//! `machine-history`).

use serde_json::{Value, json};

use crate::{Report, Sample, State};

const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// A size as powers of 1024, `850M`, `12G` or `1.3T`, one decimal under 10.
pub fn size(bytes: u64) -> String {
    let mut value = bytes as f64;
    let mut unit = 'B';
    for next in ['K', 'M', 'G', 'T'] {
        if value < 1000.0 {
            break;
        }
        value /= 1024.0;
        unit = next;
    }
    if unit != 'B' && value < 9.95 {
        format!("{value:.1}{unit}")
    } else {
        format!("{value:.0}{unit}")
    }
}

fn ratio(load: Option<f64>, cores: u64) -> Option<String> {
    load.map(|l| format!("{:.1}×", l / cores.max(1) as f64))
}

/// The report as the three lines `clm machine` prints by default.
pub fn text(report: &Report) -> String {
    let cpu = &report.cpu;
    let mem = &report.memory;
    let disk = &report.disk;
    let dash = || "-".to_string();
    let mut out = format!(
        "cpu  {} of {} cores, {} over 5 min  {}\n",
        ratio(cpu.load1, cpu.cores).unwrap_or_else(dash),
        cpu.cores,
        ratio(cpu.load5, cpu.cores).unwrap_or_else(dash),
        cpu.state.word()
    );
    match mem.space {
        Some(s) => out.push_str(&format!(
            "mem  {} available of {}  {}\n",
            size(s.available),
            size(s.total),
            mem.state.word()
        )),
        None => out.push_str(&format!("mem  -  {}\n", mem.state.word())),
    }
    match disk.space {
        Some(s) => {
            out.push_str(&format!(
                "disk {} free of {}",
                size(s.available),
                size(s.total)
            ));
            if let Some(need) = disk.need {
                out.push_str(&format!(
                    ", {} needed by {} builds",
                    size(need),
                    disk.builds.len()
                ));
            }
            out.push_str(&format!("  {}\n", disk.state.word()));
        }
        None => out.push_str(&format!("disk -  {}\n", disk.state.word())),
    }
    out
}

/// The report as `clm machine --format json` prints it.
pub fn json(report: &Report) -> Value {
    let cpu = &report.cpu;
    let mem = &report.memory;
    let disk = &report.disk;
    json!({
        "view": "machine",
        "cpu": {
            "cores": cpu.cores,
            "load1": cpu.load1,
            "load5": cpu.load5,
            "state": cpu.state.word(),
        },
        "memory": {
            "total": mem.space.map(|s| s.total),
            "available": mem.space.map(|s| s.available),
            "state": mem.state.word(),
        },
        "disk": {
            "path": disk.path.display().to_string(),
            "total": disk.space.map(|s| s.total),
            "available": disk.space.map(|s| s.available),
            "peak": disk.peak,
            "measured": disk.measured.map(crate::time::format),
            "builds": disk.builds.iter()
                .map(|b| json!({"name": b.name, "bytes": b.bytes}))
                .collect::<Vec<_>>(),
            "free_places": disk.free_places,
            "need": disk.need,
            "state": disk.state.word(),
        },
    })
}

fn block(share: Option<f64>) -> char {
    match share {
        None => ' ',
        Some(s) => BLOCKS[((s * 8.0).ceil().clamp(1.0, 8.0) as usize) - 1],
    }
}

fn paint(state: State, line: &str) -> String {
    let sgr = match state {
        State::Ok => "\x1b[32m",
        State::Warn => "\x1b[33m",
        State::Critical => "\x1b[31m",
        State::Unknown => "\x1b[2m",
    };
    format!("{sgr}{line}\x1b[0m")
}

/// The `cpu`, `mem` and `disk` chart rows, no line wider than `width` cells, coloured by state.
pub fn rows(report: &Report, history: &[Sample], width: usize) -> String {
    let cpu = &report.cpu;
    let figures = [
        (
            "cpu",
            cpu.state,
            ratio(cpu.load1, cpu.cores).unwrap_or_else(|| "-".into()),
            history.iter().map(|s| s.cpu).collect::<Vec<_>>(),
        ),
        (
            "mem",
            report.memory.state,
            report
                .memory
                .space
                .map_or("-".into(), |s| format!("{} avail", size(s.available))),
            history.iter().map(|s| s.memory).collect(),
        ),
        (
            "disk",
            report.disk.state,
            report
                .disk
                .space
                .map_or("-".into(), |s| format!("{} free", size(s.available))),
            history.iter().map(|s| s.disk).collect(),
        ),
    ];
    let mut out = String::new();
    for (label, state, figure, shares) in figures {
        let tail = match state {
            State::Ok => String::new(),
            other => format!(" {}", other.word()),
        };
        // Label padded to four cells, a space, the chart, a space, the figure, the state.
        let room = width.saturating_sub(4 + 1 + 1 + figure.chars().count() + tail.chars().count());
        let shown = &shares[shares.len().saturating_sub(room)..];
        let chart: String = shown.iter().map(|s| block(*s)).collect();
        let line = format!("{label:<4} {chart} {figure}{tail}");
        out.push_str(&paint(state, &line));
        out.push('\n');
    }
    out
}
