//! Reading Claude Code's transcripts (`effort/transcripts`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use clm_ledger::Usage;
use serde_json::Value;

/// A gap between two entries longer than this isn't active time.
const IDLE_SECONDS: i64 = 600;

/// The model Claude Code names for a message it made up itself, which cost nothing.
const SYNTHETIC: &str = "<synthetic>";

/// The work one session spent on one branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionWork {
    pub session: String,
    pub branch: String,
    /// Token counts by model.
    pub models: BTreeMap<String, Usage>,
    pub active_seconds: u64,
}

/// One session's entries on one branch, as read so far.
#[derive(Default)]
struct Group {
    /// The work of entries without a `message.id`, by model.
    models: BTreeMap<String, Usage>,
    /// Each response's model and counts by `message.id`: one response is written as one line
    /// per content block, and each count is the largest any of its lines holds.
    messages: BTreeMap<String, (String, Usage)>,
    /// Milliseconds since the epoch.
    times: Vec<i64>,
}

/// The work in the transcripts under `transcripts` whose entries ran in a `counted`
/// directory, one per session and branch (only `branch`'s when given), in branch and session
/// order, and the warnings.
pub fn sessions(
    transcripts: &Path,
    counted: &[PathBuf],
    branch: Option<&str>,
) -> (Vec<SessionWork>, Vec<String>) {
    let Ok(projects) = std::fs::read_dir(transcripts) else {
        return (
            Vec::new(),
            vec![format!("no transcripts at {}", transcripts.display())],
        );
    };
    let prefixes: Vec<String> = counted.iter().map(|dir| encoded(dir)).collect();
    let mut files = Vec::new();
    for project in projects.flatten() {
        let name = project.file_name().to_string_lossy().into_owned();
        if prefixes.iter().any(|p| name.starts_with(p.as_str())) {
            jsonl_files(&project.path(), &mut files);
        }
    }
    files.sort();
    let mut groups: BTreeMap<(String, String), Group> = BTreeMap::new();
    let mut bad = 0;
    // With a branch named plainly, a line that doesn't hold it can't be its entry, and isn't
    // parsed.
    let needle = branch.filter(|b| !b.contains(['"', '\\']));
    for file in files {
        let Ok(bytes) = std::fs::read(&file) else {
            continue;
        };
        for line in bytes.split(|b| *b == b'\n') {
            let Ok(line) = std::str::from_utf8(line) else {
                bad += 1;
                continue;
            };
            if line.trim().is_empty() || needle.is_some_and(|b| !line.contains(b)) {
                continue;
            }
            match serde_json::from_str::<Value>(line) {
                Ok(entry) => read_entry(&entry, counted, branch, &mut groups),
                Err(_) => bad += 1,
            }
        }
    }
    let mut warnings = Vec::new();
    if bad > 0 {
        warnings.push(format!("skipped {bad} transcript lines that aren't JSON"));
    }
    let work = groups
        .into_iter()
        .map(|((branch, session), mut group)| {
            group.times.sort_unstable();
            let active: i64 = group
                .times
                .windows(2)
                .map(|pair| pair[1] - pair[0])
                .filter(|gap| *gap <= IDLE_SECONDS * 1000)
                .sum();
            let mut models = group.models;
            for (model, usage) in group.messages.into_values() {
                add(models.entry(model).or_default(), &usage);
            }
            SessionWork {
                session,
                branch,
                models,
                active_seconds: (active / 1000) as u64,
            }
        })
        .collect();
    (work, warnings)
}

/// Counts `entry` into its group when it ran in a counted directory.
fn read_entry(
    entry: &Value,
    counted: &[PathBuf],
    only: Option<&str>,
    groups: &mut BTreeMap<(String, String), Group>,
) {
    let field = |name: &str| entry.get(name).and_then(Value::as_str);
    let (Some(session), Some(branch), Some(cwd), Some(time)) = (
        field("sessionId"),
        field("gitBranch"),
        field("cwd"),
        field("timestamp").and_then(millis),
    ) else {
        return;
    };
    if only.is_some_and(|b| b != branch)
        || !counted.iter().any(|dir| Path::new(cwd).starts_with(dir))
    {
        return;
    }
    let message = entry.get("message");
    let model = message.and_then(|m| m.get("model")).and_then(Value::as_str);
    if model == Some(SYNTHETIC) {
        return;
    }
    let group = groups
        .entry((branch.to_string(), session.to_string()))
        .or_default();
    group.times.push(time);
    let (Some(model), Some(usage)) = (model, message.and_then(|m| m.get("usage"))) else {
        return;
    };
    let count = |name: &str| usage.get(name).and_then(Value::as_u64).unwrap_or(0);
    let counts = Usage {
        input: count("input_tokens"),
        output: count("output_tokens"),
        cache_read: count("cache_read_input_tokens"),
        cache_write: count("cache_creation_input_tokens"),
    };
    match message.and_then(|m| m.get("id")).and_then(Value::as_str) {
        Some(id) => {
            let (_, held) = group
                .messages
                .entry(id.to_string())
                .or_insert_with(|| (model.to_string(), Usage::default()));
            held.input = held.input.max(counts.input);
            held.output = held.output.max(counts.output);
            held.cache_read = held.cache_read.max(counts.cache_read);
            held.cache_write = held.cache_write.max(counts.cache_write);
        }
        None => add(group.models.entry(model.to_string()).or_default(), &counts),
    }
}

/// `counts` added to `total`.
fn add(total: &mut Usage, counts: &Usage) {
    total.input = total.input.saturating_add(counts.input);
    total.output = total.output.saturating_add(counts.output);
    total.cache_read = total.cache_read.saturating_add(counts.cache_read);
    total.cache_write = total.cache_write.saturating_add(counts.cache_write);
}

/// Claude Code's project directory name for `dir`: every character but an ASCII letter or
/// digit replaced by `-`.
fn encoded(dir: &Path) -> String {
    dir.to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Every `.jsonl` file under `dir`, recursively.
fn jsonl_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            jsonl_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "jsonl") {
            out.push(path);
        }
    }
}

/// An RFC 3339 UTC time, `YYYY-MM-DDTHH:MM:SS[.fff]Z`, as milliseconds since the epoch.
fn millis(text: &str) -> Option<i64> {
    let text = text.strip_suffix('Z')?;
    let (date, time) = text.split_once('T')?;
    let mut date = date.splitn(3, '-').map(|n| n.parse::<i64>().ok());
    let (y, m, d) = (date.next()??, date.next()??, date.next()??);
    let (clock, fraction) = time.split_once('.').unwrap_or((time, "0"));
    let mut clock = clock.splitn(3, ':').map(|n| n.parse::<i64>().ok());
    let (h, min, s) = (clock.next()??, clock.next()??, clock.next()??);
    if !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let ms: i64 = format!("{fraction:0<3}")[..3].parse().ok()?;
    Some((days(y, m, d) * 86_400 + h * 3600 + min * 60 + s) * 1000 + ms)
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Howard Hinnant's `days_from_civil`).
fn days(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    #[test]
    fn millis_of_a_timestamp() {
        assert_eq!(super::millis("1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(super::millis("1970-01-02T00:00:01.5Z"), Some(86_401_500));
        assert_eq!(
            super::millis("2024-03-01T00:00:00Z"),
            Some(19_783 * 86_400_000)
        );
        assert_eq!(super::millis("yesterday"), None);
        // A fraction that isn't digits is no time, not a panic.
        assert_eq!(super::millis("2026-10-30T00:00:00.\u{e9}\u{e9}Z"), None);
    }
}
