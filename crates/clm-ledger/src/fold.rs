//! The fold over records already read (`ledger/fold`, `ledger/ledger-builds-for-wasm`).

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::{Body, LandKind, Record};

/// The state the ledgers say. Each index lists its records in `at` order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fold {
    /// Every record once, by `at` (then `id`).
    pub records: Vec<Record>,
    /// `approved` by fingerprint.
    pub approvals: BTreeMap<String, Vec<Record>>,
    /// `checked` by code key.
    pub checks: BTreeMap<String, Vec<Record>>,
    /// `landed` by commit.
    pub lands: BTreeMap<String, Vec<Record>>,
    /// The live push subscriptions by `id`: each `subscribed` record no `unsubscribed`
    /// record ends.
    pub subscriptions: BTreeMap<String, Record>,
    /// The open web proposals by `id`: each `proposed` record no `withdrawn` record's
    /// `proposal` names, whoever wrote it.
    pub proposals: BTreeMap<String, Record>,
    /// `commented` by branch.
    pub comments: BTreeMap<String, Vec<Record>>,
    /// `changes_requested` by branch.
    pub change_requests: BTreeMap<String, Vec<Record>>,
    /// `effort` by `id`: for each, the record with the latest `at`, whoever wrote it.
    pub efforts: BTreeMap<String, Record>,
    /// `decided` by capability.
    pub decisions: BTreeMap<String, Vec<Record>>,
    /// `verified` by commit, the newest by `at` first.
    pub verifications: BTreeMap<String, Vec<Record>>,
    /// Each record's line as read, by `id`: its JSON object, `sig` included, so a signature
    /// is checked against the fields the line holds, a field a newer `clm` added included.
    pub lines: BTreeMap<String, Map<String, Value>>,
    /// Each line skipped, naming its ledger and line number.
    pub warnings: Vec<String>,
}

impl Fold {
    /// Whether a `changes_requested` record is cleared at its fingerprint: an `approved` record
    /// for that fingerprint by the requester's person, at a later `at` as a string, signed or
    /// not (`ledger/request-cleared`). Any other kind of record is never cleared. Whether the
    /// request names the fingerprint a land or a review is at is the caller's test.
    pub fn cleared(&self, request: &Record) -> bool {
        let Body::ChangesRequested { fingerprint, .. } = &request.body else {
            return false;
        };
        let requester = person(&request.writer);
        self.approvals.get(fingerprint).is_some_and(|approvals| {
            approvals
                .iter()
                .any(|a| person(&a.writer) == requester && a.at > request.at)
        })
    }

    /// The red results nothing has since cleared (`ledger/verified-record`): given a
    /// first-parent chain of commits, oldest first, the newest record of each commit after the
    /// last one whose newest record passed (every commit when none passed) whose newest record
    /// failed, in chain order.
    pub fn unresolved_reds(&self, chain: &[String]) -> Vec<Record> {
        let newest = |commit: &String| self.verifications.get(commit).and_then(|v| v.first());
        let passed = |record: &Record| matches!(record.body, Body::Verified { passed: true, .. });
        let from = chain
            .iter()
            .rposition(|c| newest(c).is_some_and(passed))
            .map_or(0, |n| n + 1);
        chain[from..]
            .iter()
            .filter_map(newest)
            .filter(|r| !passed(r))
            .cloned()
            .collect()
    }
}

/// Lowercased, each run of anything but letters and digits replaced by `-`, with none at
/// either end: the slug `writer` makes of git's `user.name` and `verify` of an author name.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

/// The person part of a `<person>/<clone>` writer: the text before its first `/`, or the
/// whole writer when it has none.
pub fn person(writer: &str) -> &str {
    writer.split('/').next().unwrap_or_default()
}

/// The fold over `(ledger, records.jsonl text)` pairs, a record read twice counting once by
/// `id`, except an `effort` one, whose repeated `id` is a later count: the latest `at` wins.
/// A line that doesn't parse is skipped with a warning; a blank one, or a `reviewed` one (a
/// kind gone with yes/no review, `yes-no-review`), silently.
pub fn fold_records(sources: &[(&str, &str)]) -> Fold {
    let mut fold = Fold::default();
    // Where each `id`'s record is in `fold.records`.
    let mut position = BTreeMap::new();
    for (ledger, text) in sources {
        for (n, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parsed = serde_json::from_str::<Value>(line);
            if parsed.as_ref().is_ok_and(|v| v["kind"] == "reviewed") {
                continue;
            }
            let parsed = parsed.and_then(|value| {
                let record = Record::deserialize(&value)?;
                match value {
                    Value::Object(object) => Ok((record, object)),
                    _ => Err(serde::de::Error::custom("not a JSON object")),
                }
            });
            match parsed {
                Ok((record, object)) => match fold.lines.entry(record.id.clone()) {
                    Entry::Vacant(slot) => {
                        slot.insert(object);
                        position.insert(record.id.clone(), fold.records.len());
                        fold.records.push(record);
                    }
                    Entry::Occupied(mut slot) => {
                        let held = &mut fold.records[position[&record.id]];
                        if later_effort(&record, &object, held, slot.get()) {
                            slot.insert(object);
                            *held = record;
                        }
                    }
                },
                Err(e) => fold.warnings.push(format!(
                    "{ledger} line {}: skipped, not a record: {e}",
                    n + 1
                )),
            }
        }
    }
    fold.records
        .sort_by(|a, b| (&a.at, &a.id).cmp(&(&b.at, &b.id)));
    // An end may be read before what it ends, so the ends are gathered first.
    let mut ended = BTreeSet::new();
    let mut withdrawn = BTreeSet::new();
    for record in &fold.records {
        match &record.body {
            Body::Unsubscribed { subscription } => ended.insert(subscription.clone()),
            Body::Withdrawn { proposal } => withdrawn.insert(proposal.clone()),
            _ => false,
        };
    }
    for record in &fold.records {
        let (index, key) = match &record.body {
            Body::Approved { fingerprint, .. } => (&mut fold.approvals, fingerprint),
            Body::Checked { code_key, .. } => (&mut fold.checks, code_key),
            Body::Landed { commit, .. } => (&mut fold.lands, commit),
            Body::Commented { branch, .. } => (&mut fold.comments, branch),
            Body::ChangesRequested { branch, .. } => (&mut fold.change_requests, branch),
            Body::Decided { capability, .. } => (&mut fold.decisions, capability),
            // Ascending here; the newest comes first once all are in.
            Body::Verified { commit, .. } => (&mut fold.verifications, commit),
            Body::Effort { .. } => {
                fold.efforts.insert(record.id.clone(), record.clone());
                continue;
            }
            Body::Subscribed { .. } => {
                if !ended.contains(&record.id) {
                    fold.subscriptions.insert(record.id.clone(), record.clone());
                }
                continue;
            }
            Body::Proposed { .. } => {
                if !withdrawn.contains(&record.id) {
                    fold.proposals.insert(record.id.clone(), record.clone());
                }
                continue;
            }
            Body::Unsubscribed { .. } | Body::Withdrawn { .. } => continue,
        };
        index.entry(key.clone()).or_default().push(record.clone());
    }
    for records in fold.verifications.values_mut() {
        records.reverse();
    }
    fold
}

/// Whether `record`, read as `line`, is an `effort` count that supersedes `held`, another
/// one read as `held_line`: by `at`, then writer, then the line's text, so every read order
/// folds alike. A record of another kind never replaces or is replaced.
fn later_effort(
    record: &Record,
    line: &Map<String, Value>,
    held: &Record,
    held_line: &Map<String, Value>,
) -> bool {
    let efforts =
        matches!(record.body, Body::Effort { .. }) && matches!(held.body, Body::Effort { .. });
    let text = |line: &Map<String, Value>| serde_json::to_string(line).unwrap_or_default();
    efforts
        && (&record.at, &record.writer)
            .cmp(&(&held.at, &held.writer))
            .then_with(|| text(line).cmp(&text(held_line)))
            .is_gt()
}

/// `clm ledger show`'s lines: one per record in `at` order, its kind first, then what it
/// names, its branch where it has one, its writer and its time, and last a comment's or a
/// change request's text, on one line.
pub fn show(fold: &Fold) -> Vec<String> {
    fold.records
        .iter()
        .map(|r| {
            let (kind, writer, at) = (r.body.kind(), &r.writer, &r.at);
            let what = match &r.body {
                Body::Approved {
                    fingerprint,
                    branch,
                    ..
                } => format!("{fingerprint} {branch}"),
                Body::Checked { code_key, .. } => code_key.clone(),
                Body::Landed {
                    commit,
                    branch,
                    kind,
                    review,
                    ..
                } => {
                    let kind = match kind {
                        LandKind::Spec => "spec",
                        LandKind::Code => "code",
                    };
                    let review = if *review { "review" } else { "-" };
                    format!("{commit} {branch} {kind} {review}")
                }
                // Never the keys, nor the endpoint's path, which addresses the browser.
                Body::Subscribed { endpoint, .. } => format!("{} {}", r.id, origin(endpoint)),
                Body::Unsubscribed { subscription } => subscription.clone(),
                Body::Proposed { slug, .. } => format!("{} {slug}", r.id),
                Body::Withdrawn { proposal } => proposal.clone(),
                Body::Commented {
                    branch,
                    fingerprint,
                    target,
                    text,
                } => {
                    let target = target.as_deref().unwrap_or("-");
                    let text = one_line(text);
                    return format!("{kind} {branch} {fingerprint} {target} {writer} {at} {text}");
                }
                Body::ChangesRequested {
                    branch,
                    fingerprint,
                    text,
                } => {
                    let text = one_line(text);
                    return format!("{kind} {branch} {fingerprint} {writer} {at} {text}");
                }
                Body::Effort {
                    branch,
                    session,
                    models,
                    active_seconds,
                } => {
                    let work = models
                        .values()
                        .fold(0u64, |n, u| n.saturating_add(u.work()));
                    format!("{branch} {session} {work} {active_seconds}s")
                }
                Body::Decided {
                    capability,
                    value,
                    decision,
                    ..
                } => format!("{capability} {} {}", value.as_str(), decision.as_str()),
                Body::Verified { commit, passed, .. } => {
                    let result = if *passed { "passed" } else { "failed" };
                    format!("{commit} {result}")
                }
            };
            format!("{kind} {what} {writer} {at}")
        })
        .collect()
}

/// `text` trimmed, with each run of whitespace that holds a line break replaced by one space,
/// unquoted.
fn one_line(text: &str) -> String {
    let mut out = String::new();
    let mut run = String::new();
    for c in text.trim().chars() {
        if c.is_whitespace() {
            run.push(c);
            continue;
        }
        if run.contains(['\n', '\r', '\x0b', '\x0c', '\u{85}', '\u{2028}', '\u{2029}']) {
            out.push(' ');
        } else {
            out.push_str(&run);
        }
        run.clear();
        out.push(c);
    }
    out
}

/// A URL's origin, `<scheme>://<host>[:<port>]`, without any user information, or `-` for
/// a value that doesn't start with a scheme.
fn origin(url: &str) -> String {
    let scheme = url.split_once("://").map(|(scheme, _)| scheme).filter(|s| {
        s.starts_with(|c: char| c.is_ascii_alphabetic())
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
    });
    let Some(scheme) = scheme else {
        return "-".into();
    };
    let rest = &url[scheme.len() + 3..];
    let authority = &rest[..rest.find(['/', '?', '#']).unwrap_or(rest.len())];
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    format!("{scheme}://{host}")
}

#[cfg(test)]
mod tests {
    #[test]
    fn origin_of_an_endpoint() {
        // req:ledger/subscription-records@e4cb2df2
        for (url, want) in [
            ("https://push.example/s/2", "https://push.example"),
            ("https://push.example", "https://push.example"),
            (
                "https://push.example:8443/s?x=1",
                "https://push.example:8443",
            ),
            ("https://push.example?x=1", "https://push.example"),
            ("not a url", "-"),
            (
                "https://user:token@push.example/s/1",
                "https://push.example",
            ),
            ("foo?x=https://a/b", "-"),
        ] {
            assert_eq!(super::origin(url), want, "{url}");
        }
    }
}
