//! `clm machine`: the machine's CPU load, available memory and free disk, judged against what
//! the clone's builds use, and the last 30 minutes of them kept for the pane's charts.

mod history;
mod record;
mod render;
mod system;
mod time;
mod walk;

use std::path::{Path, PathBuf};

pub use history::Sample;
use record::MEASURE_EVERY_SECONDS;
pub use record::Record;
pub use render::{json, rows, text};
use system::read_system;
pub use time::format as format_time;

/// How a reading stands against what the builds need.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Ok,
    Warn,
    Critical,
    Unknown,
}

impl State {
    pub fn word(self) -> &'static str {
        match self {
            State::Ok => "ok",
            State::Warn => "warn",
            State::Critical => "critical",
            State::Unknown => "unknown",
        }
    }

    /// The state a word names, `None` for any other word.
    pub(crate) fn from_word(word: &str) -> Option<State> {
        [State::Ok, State::Warn, State::Critical, State::Unknown]
            .into_iter()
            .find(|s| s.word() == word)
    }
}

/// A filesystem's or memory's total and what is still available, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Space {
    pub total: u64,
    pub available: u64,
}

/// What the system gave; `None` where it refused.
#[derive(Debug, Clone, PartialEq)]
pub struct Readings {
    pub cores: u64,
    pub load1: Option<f64>,
    pub load5: Option<f64>,
    pub memory: Option<Space>,
    pub disk_path: PathBuf,
    pub disk: Option<Space>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cpu {
    pub cores: u64,
    pub load1: Option<f64>,
    pub load5: Option<f64>,
    pub state: State,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Memory {
    pub space: Option<Space>,
    pub state: State,
}

/// A counting build and the size of its worktree when last measured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Build {
    pub name: String,
    pub bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Disk {
    pub path: PathBuf,
    pub space: Option<Space>,
    pub peak: Option<u64>,
    pub measured: Option<u64>,
    pub builds: Vec<Build>,
    pub free_places: u64,
    pub need: Option<u64>,
    pub state: State,
}

/// The three readings with their states.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub cpu: Cpu,
    pub memory: Memory,
    pub disk: Disk,
}

/// A report and the history the rows chart.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub report: Report,
    pub history: Vec<Sample>,
}

/// The states of `readings`, given the record and the names of the builds that count: a pure
/// function (`machine-report`).
pub fn judge(readings: &Readings, record: &Record, counting: &[String], max_builds: u64) -> Report {
    let cores = readings.cores.max(1) as f64;
    let cpu_state = match (readings.load1, readings.load5) {
        (Some(load1), Some(load5)) => {
            if load5 / cores >= 1.25 {
                State::Critical
            } else if load1 / cores >= 1.25 {
                State::Warn
            } else {
                State::Ok
            }
        }
        _ => State::Unknown,
    };
    let memory_state = match readings.memory {
        Some(m) if m.total > 0 => {
            let share = m.available as f64 / m.total as f64;
            if share < 0.10 {
                State::Critical
            } else if share < 0.20 {
                State::Warn
            } else {
                State::Ok
            }
        }
        _ => State::Unknown,
    };
    let builds: Vec<Build> = counting
        .iter()
        .map(|name| Build {
            name: name.clone(),
            bytes: record
                .builds
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, bytes)| *bytes),
        })
        .collect();
    let free_places = max_builds.saturating_sub(counting.len() as u64);
    let (need, disk_state) = judge_disk(readings.disk, record.peak, &builds, free_places);
    Report {
        cpu: Cpu {
            cores: readings.cores,
            load1: readings.load1,
            load5: readings.load5,
            state: cpu_state,
        },
        memory: Memory {
            space: readings.memory,
            state: memory_state,
        },
        disk: Disk {
            path: readings.disk_path.clone(),
            space: readings.disk,
            peak: record.peak,
            measured: record.measured,
            builds,
            free_places,
            need,
            state: disk_state,
        },
    }
}

fn judge_disk(
    space: Option<Space>,
    peak: Option<u64>,
    builds: &[Build],
    free_places: u64,
) -> (Option<u64>, State) {
    let Some(space) = space else {
        return (None, State::Unknown);
    };
    let Some(peak) = peak else {
        let share = space.available as f64 / space.total.max(1) as f64;
        let state = if share < 0.05 {
            State::Critical
        } else if share < 0.10 {
            State::Warn
        } else {
            State::Ok
        };
        return (None, state);
    };
    let growth: u64 = builds
        .iter()
        .map(|b| peak.saturating_sub(b.bytes.unwrap_or(0)))
        .sum();
    let need = growth + peak * free_places;
    let state = if space.available < growth {
        State::Critical
    } else if space.available < need {
        State::Warn
    } else {
        State::Ok
    };
    (Some(need), state)
}

/// Takes the readings, measures the builds when the record is due, keeps the sample and returns
/// them (`machine-report`, `build-measure`, `machine-history`). `now` is in Unix seconds. It
/// keeps nothing outside a git repository, and what it can't write it leaves unwritten.
pub fn read(root: &Path, max_builds: u64, expiry_hours: u64, now: u64) -> Snapshot {
    let common = clm_git::common_dir(root).ok();
    let disk_path = common.clone().unwrap_or_else(|| root.to_path_buf());
    let readings = read_system(&disk_path);
    let mut record = Record::default();
    let mut counting = Vec::new();
    let mut history_file = None;
    if let Some(common) = &common {
        let dir = common.join("clm");
        let places = clm_orchestrator::places(root, expiry_hours).unwrap_or_default();
        let trees = clm_git::worktrees(root).unwrap_or_default();
        let record_path = dir.join("machine");
        record = Record::load(&record_path);
        let due = record
            .measured
            .is_none_or(|at| at > now || now - at >= MEASURE_EVERY_SECONDS);
        let mut sizes = Vec::new();
        for place in places.iter().filter(|p| p.counts) {
            counting.push(place.name.clone());
            if due {
                let tree = place.file.as_ref().ok().and_then(|file| {
                    trees
                        .iter()
                        .find(|t| t.branch.as_ref().is_some_and(|b| file.branches.contains(b)))
                });
                if let Some(tree) = tree {
                    sizes.push((place.name.clone(), walk::size(&tree.path)));
                }
            }
        }
        if due {
            record = record.measured_at(now, sizes);
            record.store(&record_path);
        }
        history_file = Some(dir.join("machine-samples"));
    }
    let report = judge(&readings, &record, &counting, max_builds);
    let sample = Sample::of(now, &report);
    let history = match history_file {
        Some(path) => history::keep(&path, sample, now),
        None => vec![sample],
    };
    Snapshot { report, history }
}
