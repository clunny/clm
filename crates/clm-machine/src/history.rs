//! The last 30 minutes of readings, one sample per line in
//! `<git common dir>/clm/machine-samples` (`machine-history`).

use std::path::Path;

use crate::record::write_atomically;
use crate::{Report, time};

/// Samples older than this are dropped.
const KEEP_SECONDS: u64 = 30 * 60;
/// A run adds no sample when the newest is this young.
const MIN_GAP_SECONDS: u64 = 5;
/// The most samples kept.
const MAX_SAMPLES: usize = 180;

/// A moment's shares: for each reading, how heavy the load is, `None` when unknown. CPU is
/// `load1 / cores` over 2; memory and disk are `1 − available / total`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// Unix seconds.
    pub time: u64,
    pub cpu: Option<f64>,
    pub memory: Option<f64>,
    pub disk: Option<f64>,
}

impl Sample {
    pub fn of(time: u64, report: &Report) -> Sample {
        let used = |space: Option<crate::Space>| {
            space
                .filter(|s| s.total > 0)
                .map(|s| 1.0 - s.available as f64 / s.total as f64)
        };
        Sample {
            time,
            cpu: report
                .cpu
                .load1
                .map(|load| load / report.cpu.cores.max(1) as f64 / 2.0),
            memory: used(report.memory.space),
            disk: used(report.disk.space),
        }
    }

    fn render(&self) -> String {
        let share = |s: Option<f64>| s.map_or("-".to_string(), |s| format!("{s:.4}"));
        format!(
            "{} {} {} {}\n",
            time::format(self.time),
            share(self.cpu),
            share(self.memory),
            share(self.disk)
        )
    }

    fn parse(line: &str) -> Option<Sample> {
        let mut fields = line.split_whitespace();
        let time = time::parse(fields.next()?)?;
        let mut share = || -> Option<Option<f64>> {
            match fields.next()? {
                "-" => Some(None),
                text => text.parse().ok().map(Some),
            }
        };
        let (cpu, memory, disk) = (share()?, share()?, share()?);
        if fields.next().is_some() {
            return None;
        }
        Some(Sample {
            time,
            cpu,
            memory,
            disk,
        })
    }
}

/// The history at `path` after this run: the file's samples within 30 minutes of `now`, plus
/// `sample` unless the newest is under 5 seconds old, at most 180 of them, written back. A
/// file that can't be read counts as empty and one that can't be written is left alone.
pub fn keep(path: &Path, sample: Sample, now: u64) -> Vec<Sample> {
    let mut samples: Vec<Sample> = std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(Sample::parse)
        .collect();
    // A sample from a clock that has since stepped back is not history.
    samples.retain(|s| s.time <= now);
    samples.sort_by_key(|s| s.time);
    if !samples
        .last()
        .is_some_and(|newest| newest.time.abs_diff(now) < MIN_GAP_SECONDS)
    {
        samples.push(sample);
    }
    samples.retain(|s| now.saturating_sub(s.time) <= KEEP_SECONDS);
    let extra = samples.len().saturating_sub(MAX_SAMPLES);
    samples.drain(..extra);
    let text: String = samples.iter().map(Sample::render).collect();
    write_atomically(path, &text);
    samples
}
