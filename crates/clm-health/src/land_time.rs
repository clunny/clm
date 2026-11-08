//! The `land-time` signal: where the last timed lands spent their seconds (`land-time-signal`).

use std::collections::BTreeMap;
use std::path::Path;

use crate::{Failure, Section, finding};

/// The timed lands the signal reads, and the steps and paths it lists.
const LANDS: usize = 50;
const STEPS: usize = 5;
const PATHS: usize = 3;

#[derive(Default)]
struct Step {
    seconds: Vec<u64>,
    paths: BTreeMap<String, usize>,
}

/// The five steps with the most total seconds over the last 50 timed lands, and how many of
/// those lands ran past `target` seconds. `Git` when the ledger can't be read.
pub(crate) fn section(root: &Path, target: u64) -> Result<Section, Failure> {
    let fold = clm_ledger::fold(root).map_err(|e| Failure::Git(e.to_string()))?;
    let lands = fold.timed_lands(LANDS);
    let over = lands.iter().filter(|l| l.check_seconds > target).count();
    let mut steps: BTreeMap<&str, Step> = BTreeMap::new();
    for step in lands.iter().flat_map(|l| &l.steps) {
        let entry = steps.entry(&step.name).or_default();
        entry.seconds.push(step.seconds);
        for path in &step.selected_by {
            *entry.paths.entry(path.clone()).or_default() += 1;
        }
    }
    let mut ranked: Vec<(&str, Step)> = steps.into_iter().collect();
    // `BTreeMap` iterates by name and the sort is stable, so ties stay by name.
    ranked.sort_by_key(|(_, s)| std::cmp::Reverse(s.seconds.iter().sum::<u64>()));
    let lines = ranked
        .into_iter()
        .take(STEPS)
        .map(|(name, mut step)| {
            step.seconds.sort_unstable();
            let total: u64 = step.seconds.iter().sum();
            // The lower middle of an even count, as `outlier-size` takes it.
            let median = step.seconds[(step.seconds.len() - 1) / 2];
            let mut paths: Vec<(String, usize)> = step.paths.into_iter().collect();
            paths.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
            paths.truncate(PATHS);
            let names: Vec<&str> = paths.iter().map(|(p, _)| p.as_str()).collect();
            let selected = if names.is_empty() {
                "no file".to_string()
            } else {
                names.join(", ")
            };
            finding(
                names.first().copied().unwrap_or("clm.toml"),
                "land-time",
                format!(
                    "{name}: {} runs, median {median}s, total {total}s, selected by {selected}",
                    step.seconds.len()
                ),
            )
        })
        .collect();
    Ok(Section {
        signal: "land-time",
        summary: format!("{over} of {} timed lands over {target}s", lands.len()),
        lines,
    })
}
