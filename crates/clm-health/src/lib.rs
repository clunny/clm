//! `clm health`: one advisory report of the repository's health (see
//! specs/check/health/SPEC.md). Nothing gates on it.

mod churn;
mod flag;
mod land_time;
mod lcov;
mod requirements;
mod spec;
mod state;

use std::fmt;
use std::path::Path;

pub use churn::{Churn, churn};
pub use clm_git::Failure;
pub use flag::{crowded, workaround};
pub use requirements::{Reconsider, RequirementSignals, reconsider, signals};

/// What `clm health` flags, filled by the `clm` crate from `config`'s `health` and
/// `component_size`, so this crate doesn't depend on `clm-config`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// The score at which a requirement is reported under `reconsider`.
    pub reconsider: u64,
    /// The scenario count above which a requirement is reported.
    pub scenarios: u64,
    /// The exception-clause count above which a requirement is reported.
    pub exceptions: u64,
    /// The days after which an open `question` or `human-task` item is reported.
    pub stale_days: u64,
    /// The lcov file to read, relative to the repository root.
    pub lcov: Option<String>,
    /// The seconds a land's check aims to stay within, which the `land-time` summary names.
    pub land_check_target_seconds: u64,
    /// The size budgets of a node whose `size_budget` doesn't set them.
    pub component_size: clm_check::Budgets,
}

/// One signal: its summary (`<n> <noun>`, or why it couldn't run) and its findings, each
/// `<path>: health/<signal>: <message>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Section {
    pub signal: &'static str,
    pub summary: String,
    pub lines: Vec<String>,
}

impl Section {
    /// A section that ran: `count` things of `noun`, with its finding lines.
    fn ran(signal: &'static str, count: usize, noun: &str, lines: Vec<String>) -> Section {
        Section {
            signal,
            summary: format!("{count} {noun}"),
            lines,
        }
    }

    /// A section with one finding per line, counted as `noun`.
    fn of(signal: &'static str, noun: &str, lines: Vec<String>) -> Section {
        Section::ran(signal, lines.len(), noun, lines)
    }

    /// A section that couldn't run, saying why.
    fn skipped(signal: &'static str, why: String) -> Section {
        Section {
            signal,
            summary: why,
            lines: Vec::new(),
        }
    }
}

/// `<path>: health/<signal>: <message>`
fn finding(path: impl fmt::Display, signal: &str, message: impl fmt::Display) -> String {
    format!("{path}: health/{signal}: {message}")
}

/// What `clm health` prints: the sections in the order of the Contract's table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub(crate) sections: Vec<Section>,
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for s in &self.sections {
            writeln!(f, "health/{}: {}", s.signal, s.summary)?;
            for line in &s.lines {
                writeln!(f, "{line}")?;
            }
        }
        Ok(())
    }
}

/// Whether `HEAD` names a commit: `git rev-parse --verify --quiet HEAD` exits 0, or 1
/// when it doesn't; any other failure is git failing.
fn has_history(root: &Path) -> Result<bool, Failure> {
    match clm_git::git(root, &["rev-parse", "--verify", "--quiet", "HEAD"]) {
        Ok(_) => Ok(true),
        Err(clm_git::Error::Failed { status, .. }) if status.code() == Some(1) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// A count of tokens: four bytes, rounded up (`check/node-size`).
fn tokens(bytes: usize) -> u64 {
    (bytes as u64).div_ceil(4)
}

/// The phrase of `phrases` that starts at byte `i` of `text`, case-insensitively and at
/// word boundaries, as `text` writes it.
fn phrase_at<'t>(text: &'t str, i: usize, phrases: &[&str]) -> Option<&'t str> {
    let bytes = text.as_bytes();
    let word = |at: usize| bytes.get(at).is_some_and(|b| b.is_ascii_alphanumeric());
    if !text.is_char_boundary(i) || (i > 0 && word(i - 1)) {
        return None;
    }
    phrases
        .iter()
        .find(|p| {
            bytes[i..]
                .get(..p.len())
                .is_some_and(|b| b.eq_ignore_ascii_case(p.as_bytes()))
                && !word(i + p.len())
        })
        .map(|p| &text[i..i + p.len()])
}

/// The report for the checkout at `root`, with claims lapsing after `expiry` hours.
/// `Refused` when `specs/SPEC.md` is missing; `Git` when git fails.
pub fn report(root: &Path, settings: &Settings, expiry: u64) -> Result<Report, Failure> {
    if !root.join("specs/SPEC.md").is_file() {
        return Err(Failure::Refused(format!(
            "{}: specs/SPEC.md not found",
            root.display()
        )));
    }
    let (graph, _) = clm_model::load(root);
    let markers = clm_check::scan(root).req;
    let sizes = clm_check::size(root, &graph, &markers, settings.component_size)
        .map_err(|e| Failure::Git(e.to_string()))?;
    let now = clm_git::now();
    Ok(Report {
        sections: vec![
            state::check(root),
            state::pending(&graph, &markers),
            state::oversized(&graph, &sizes),
            state::lapsed_claims(root, expiry, now)?,
            state::stalled_changes(&graph, &markers),
            state::unanswered(root, &graph, settings.stale_days, now)?,
            spec::uncited_decisions(root, &graph),
            requirements::unserved(&graph),
            spec::history_words(&graph),
            spec::design_heavy(&graph),
            spec::outlier_size(&graph, &sizes),
            lcov::unexercised(root, &graph, settings.lcov.as_deref()),
            requirements::workaround(&graph),
            requirements::rewritten(&graph),
            churn::section(root, &graph)?,
            land_time::section(root, settings.land_check_target_seconds)?,
            requirements::many_scenarios(&graph, settings.scenarios),
            requirements::many_exceptions(&graph, settings.exceptions),
            requirements::reconsider_section(root, &graph, settings),
        ],
    })
}
