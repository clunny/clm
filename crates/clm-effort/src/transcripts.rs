//! Reading Claude Code's transcripts (`effort/transcripts`).

use std::collections::{BTreeMap, BTreeSet};
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

/// What a counted directory is to the repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Checkout {
    /// Not a linked worktree: the main worktree, or a `--cwd` outside every linked one.
    Unlinked,
    /// A linked worktree with this branch checked out.
    Branch(String),
    /// A linked worktree with no branch checked out.
    Detached,
}

/// The work in the transcripts under `transcripts` whose entries ran in a `counted`
/// directory, one per session and branch (only `branch`'s when given), in branch and session
/// order, and the warnings. A subagent's entries count, under `<session>/<agent>`, for the
/// branch `pinned` gives that session, else for the branch of the linked worktree they ran
/// in, and for nothing while that worktree is detached.
pub fn sessions(
    transcripts: &Path,
    counted: &[(PathBuf, Checkout)],
    pinned: &BTreeMap<String, String>,
    branch: Option<&str>,
) -> (Vec<SessionWork>, Vec<String>) {
    let Ok(projects) = std::fs::read_dir(transcripts) else {
        return (
            Vec::new(),
            vec![format!("no transcripts at {}", transcripts.display())],
        );
    };
    let prefixes: Vec<String> = counted.iter().map(|(dir, _)| encoded(dir)).collect();
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
    // A subagent's line counts for `branch` when it holds the branch or ran in a worktree
    // that has the branch checked out, so it must name one or the other; with a path JSON
    // would escape, every subagent line is parsed.
    let worktrees: Option<Vec<&str>> = counted
        .iter()
        .filter(
            |(_, held)| matches!((held, branch), (Checkout::Branch(b), Some(only)) if b == only),
        )
        .map(|(dir, _)| dir.to_str().filter(|d| !d.contains(['"', '\\'])))
        .collect();
    // The agents some pin names, whose files are read whole, since their worktree may be
    // gone.
    let pinned_agents: BTreeSet<&str> = pinned
        .keys()
        .filter_map(|key| key.rsplit_once('/').map(|(_, agent)| agent))
        .collect();
    for file in files {
        let Ok(bytes) = std::fs::read(&file) else {
            continue;
        };
        // A subagent's entries carry the launching session's branch, so they count for the
        // branch pinned for them or of the worktree they ran in, which no line need name.
        let agent = file
            .strip_prefix(transcripts)
            .is_ok_and(|rel| rel.components().any(|c| c.as_os_str() == "subagents"))
            .then(|| file.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .flatten();
        let pinned_file = agent
            .as_deref()
            .is_some_and(|agent| pinned_agents.contains(agent));
        let paths: &[&str] = match (&worktrees, &agent) {
            (_, None) => &[],
            (Some(paths), Some(_)) if !pinned_file => paths,
            // The empty string, which every line holds.
            _ => &[""],
        };
        for line in bytes.split(|b| *b == b'\n') {
            let Ok(line) = std::str::from_utf8(line) else {
                bad += 1;
                continue;
            };
            let named = |b: &str| line.contains(b) || paths.iter().any(|p| line.contains(p));
            if line.trim().is_empty() || needle.is_some_and(|b| !named(b)) {
                continue;
            }
            match serde_json::from_str::<Value>(line) {
                Ok(entry) => read_entry(
                    &entry,
                    counted,
                    pinned,
                    agent.as_deref(),
                    branch,
                    &mut groups,
                ),
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

/// Counts `entry` into its group when it ran in a counted directory. An entry of subagent
/// `agent`'s file counts under `<sessionId>/<agent>` for its pinned branch, else for the
/// branch of the linked worktree it ran in, and not at all while that worktree is detached;
/// every other entry counts under its `sessionId` for its `gitBranch`.
fn read_entry(
    entry: &Value,
    counted: &[(PathBuf, Checkout)],
    pinned: &BTreeMap<String, String>,
    agent: Option<&str>,
    only: Option<&str>,
    groups: &mut BTreeMap<(String, String), Group>,
) {
    let field = |name: &str| entry.get(name).and_then(Value::as_str);
    let (Some(session), Some(git_branch), Some(cwd), Some(time)) = (
        field("sessionId"),
        field("gitBranch"),
        field("cwd"),
        field("timestamp").and_then(millis),
    ) else {
        return;
    };
    // The deepest counted directory holding `cwd` is the worktree it ran in.
    let Some((_, checkout)) = counted
        .iter()
        .filter(|(dir, _)| Path::new(cwd).starts_with(dir))
        .max_by_key(|(dir, _)| dir.components().count())
    else {
        return;
    };
    let (session, branch) = match agent {
        Some(agent) => {
            let key = format!("{session}/{agent}");
            match (pinned.get(&key), checkout) {
                (Some(pin), _) => (key, pin.as_str()),
                (None, Checkout::Branch(b)) => (key, b.as_str()),
                (None, Checkout::Detached) => return,
                (None, Checkout::Unlinked) => (session.to_string(), git_branch),
            }
        }
        None => (session.to_string(), git_branch),
    };
    if only.is_some_and(|b| b != branch) {
        return;
    }
    let message = entry.get("message");
    let model = message.and_then(|m| m.get("model")).and_then(Value::as_str);
    if model == Some(SYNTHETIC) {
        return;
    }
    let group = groups.entry((branch.to_string(), session)).or_default();
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
