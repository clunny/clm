//! `unexercised`: the runs of lines no test runs, from an lcov file
//! (`unexercised-code`, `lcov-coverage-signal`).

use std::collections::BTreeMap;
use std::path::Path;

use clm_model::Graph;

use crate::{Section, finding};

const SIGNAL: &str = "unexercised";

/// Each file's execution count per line, the records' counts for a line added and saturating,
/// by path relative to `root`: an absolute path under `root`, made absolute or with its links
/// resolved, with the root taken off, and any other path as written. Lines other than `SF:`,
/// `DA:` and `end_of_record` are skipped.
fn parse(root: &Path, text: &str) -> BTreeMap<String, BTreeMap<u64, u64>> {
    // Both roots are absolute, so a relative path never starts with either.
    let roots = [std::path::absolute(root).ok(), root.canonicalize().ok()];
    let relative = |p: &str| {
        roots
            .iter()
            .flatten()
            .find_map(|r| Path::new(p).strip_prefix(r).ok())
            .map_or_else(|| p.to_string(), |r| r.to_string_lossy().into_owned())
    };
    let mut files: BTreeMap<String, BTreeMap<u64, u64>> = BTreeMap::new();
    let mut file = None;
    for line in text.lines().map(str::trim) {
        if let Some(path) = line.strip_prefix("SF:") {
            file = Some(relative(path));
        } else if line == "end_of_record" {
            file = None;
        } else if let (Some(f), Some(da)) = (&file, line.strip_prefix("DA:")) {
            let mut parts = da.split(',');
            let (Some(Ok(n)), Some(Ok(count))) = (
                parts.next().map(str::parse::<u64>),
                parts.next().map(str::parse::<u64>),
            ) else {
                continue;
            };
            let sum = files.entry(f.clone()).or_default().entry(n).or_default();
            *sum = sum.saturating_add(count);
        }
    }
    files
}

/// Each run of consecutive lines with count 0 in a file some node owns, from the lcov file
/// `lcov` names relative to `root`; the summary counts lines.
pub(crate) fn unexercised(root: &Path, graph: &Graph, lcov: Option<&str>) -> Section {
    let Some(lcov) = lcov else {
        return Section::skipped(SIGNAL, "no lcov file configured".into());
    };
    let Ok(text) = std::fs::read_to_string(root.join(lcov)) else {
        return Section::skipped(SIGNAL, format!("{lcov} not found"));
    };
    let mut total = 0;
    let mut lines = Vec::new();
    for (path, counts) in parse(root, &text) {
        if clm_coverage::owner(graph, Path::new(&path)).is_none() {
            continue;
        }
        let zeros: Vec<u64> = counts
            .into_iter()
            .filter(|(_, c)| *c == 0)
            .map(|(n, _)| n)
            .collect();
        for run in zeros.chunk_by(|a, b| b - a == 1) {
            let (start, end) = (run[0], run[run.len() - 1]);
            let at = if start == end {
                format!("{path}:{start}")
            } else {
                format!("{path}:{start}-{end}")
            };
            total += run.len();
            lines.push(finding(
                at,
                SIGNAL,
                format!("{} lines no test runs", run.len()),
            ));
        }
    }
    Section::ran(SIGNAL, total, "lines", lines)
}
