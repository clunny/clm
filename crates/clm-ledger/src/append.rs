//! `append` (`ledger/append`).

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Body, Error, Record, Signed, branch};

/// What `append` wrote, and whether it reached `origin`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Appended {
    pub record: Record,
    /// False with `host = "none"`, or when `origin` couldn't be reached: the record is
    /// committed locally, and `clm reconnect` pushes it later.
    pub pushed: bool,
}

/// What `append` takes: a record's body, given a fresh `id`, the writer and the time, or a
/// record `sign` returned, written with its `id`, `writer`, `at` and `sig` unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    Body(Body),
    Signed(Signed),
}

impl From<Body> for Entry {
    fn from(body: Body) -> Self {
        Entry::Body(body)
    }
}

impl From<Signed> for Entry {
    fn from(signed: Signed) -> Self {
        Entry::Signed(signed)
    }
}

/// Commits the entry as the last line of its writer's `records.jsonl`, and pushes the branch:
/// `append_all` with one entry. `env` is the environment `config` reads.
pub fn append(
    root: &Path,
    entry: impl Into<Entry>,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Appended, Error> {
    let mut appended = append_all(root, vec![entry.into()], env)?;
    Ok(appended.pop().expect("one entry, one record"))
}

/// Commits the entries, in order, as the last lines of the writer's `records.jsonl` in one
/// commit, and pushes the branch once (`ledger/append-all`). No entries, no commit.
pub fn append_all(
    root: &Path,
    entries: Vec<Entry>,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Vec<Appended>, Error> {
    if entries.is_empty() {
        return Ok(Vec::new());
    }
    let mut records = Vec::new();
    let mut lines = Vec::new();
    for entry in entries {
        let (record, mut line) = match entry {
            Entry::Body(body) => {
                let record = fresh(root, body, env)?;
                let line = serde_json::to_vec(&record).expect("a record serializes");
                (record, line)
            }
            Entry::Signed(signed) => {
                // Only this clone writes its branch: another writer's record goes on theirs.
                let writer = crate::writer(root, env)?;
                if signed.record.writer != writer {
                    return Err(Error::Writer(format!(
                        "the record is signed as writer {}, but this clone writes as {writer}",
                        signed.record.writer
                    )));
                }
                let line = serde_json::to_vec(&signed.line()).expect("a line serializes");
                (signed.record, line)
            }
        };
        line.push(b'\n');
        lines.extend(line);
        records.push(record);
    }
    let name = branch::ref_of(&records[0].writer);
    let message = match records.as_slice() {
        [one] => format!("ledger: {}", one.body.kind()),
        many => format!("ledger: {} records", many.len()),
    };
    branch::rewrite(root, &name, &message, |current| {
        let mut content = current.to_vec();
        if !content.is_empty() && !content.ends_with(b"\n") {
            content.push(b'\n');
        }
        content.extend_from_slice(&lines);
        Some(content)
    })?;
    let pushed = branch::push(root, &name, env)?;
    Ok(records
        .into_iter()
        .map(|record| Appended { record, pushed })
        .collect())
}

/// An `effort` record's `id`: the first 32 hex digits of SHA-256 over `<session>\n<branch>`,
/// so a later count of the same session and branch supersedes the earlier one.
pub fn effort_id(session: &str, branch: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(format!("{session}\n{branch}"));
    digest[..16].iter().map(|b| format!("{b:02x}")).collect()
}

/// `body` with a fresh `id` (`effort_id` for an `effort` body), this clone's writer and the
/// time now.
pub(crate) fn fresh(
    root: &Path,
    body: Body,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Record, Error> {
    let id = match &body {
        Body::Effort {
            session, branch, ..
        } => effort_id(session, branch),
        _ => crate::random_hex(16)?,
    };
    Ok(Record {
        id,
        writer: crate::writer(root, env)?,
        at: now(),
        body,
    })
}

/// The time now as `YYYY-MM-DDTHH:MM:SS.mmmZ`.
pub(crate) fn now() -> String {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock is after 1970");
    let secs = since.as_secs();
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (y, m, d) = civil(days as i64);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60,
        since.subsec_millis()
    )
}

/// The proleptic Gregorian date of `days` since 1970-01-01 (Howard Hinnant's
/// `civil_from_days`).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    #[test]
    fn civil_dates() {
        assert_eq!(super::civil(0), (1970, 1, 1));
        assert_eq!(super::civil(19_723), (2024, 1, 1));
        assert_eq!(super::civil(19_782), (2024, 2, 29));
    }
}
