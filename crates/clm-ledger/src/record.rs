//! The record kinds (the Contract's `Record kinds`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A record's signed message (`signed-ledger-records`): its line's object without `sig`,
/// keys sorted bytewise at every level, with no whitespace. The keys are sorted here, never
/// left to `serde_json`'s map order, and the fields are exactly as the line holds them: a
/// `code_key` left out stays out, a `null` stays `null`.
pub fn signed_message(line: &Map<String, Value>) -> String {
    let mut out = String::new();
    write_object(line, Some("sig"), &mut out);
    out
}

fn write_object(object: &Map<String, Value>, without: Option<&str>, out: &mut String) {
    let mut keys: Vec<&String> = object
        .keys()
        .filter(|k| Some(k.as_str()) != without)
        .collect();
    keys.sort();
    out.push('{');
    for (n, key) in keys.into_iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        out.push_str(&Value::from(key.as_str()).to_string());
        out.push(':');
        write_value(&object[key], out);
    }
    out.push('}');
}

fn write_value(value: &Value, out: &mut String) {
    match value {
        Value::Object(object) => write_object(object, None, out),
        Value::Array(items) => {
            out.push('[');
            for (n, item) in items.iter().enumerate() {
                if n > 0 {
                    out.push(',');
                }
                write_value(item, out);
            }
            out.push(']');
        }
        // Scalars print alike whatever the map order.
        scalar => out.push_str(&scalar.to_string()),
    }
}

/// One line of `records.jsonl`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// Random 128-bit hex.
    pub id: String,
    /// `<person>/<clone>`.
    pub writer: String,
    /// RFC 3339 UTC, `YYYY-MM-DDTHH:MM:SS.mmmZ` as written here, so string order is time
    /// order.
    pub at: String,
    /// `kind` and the kind's fields.
    #[serde(flatten)]
    pub body: Body,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Body {
    /// A human's approval of a diff (`pinned-approval`).
    Approved {
        fingerprint: String,
        branch: String,
        change: Option<String>,
    },
    /// A tree whose code key passed the check command. A record with a `basis` is a
    /// composed check (`composed-check`).
    Checked {
        code_key: String,
        command: String,
        /// The code keys of a branch's head and of `main` whose checks it was composed
        /// from; left out of the line when the tree was checked whole.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        basis: Option<Basis>,
    },
    Landed {
        commit: String,
        branch: String,
        /// Written `land_kind`: the record's own `kind` is `landed`.
        #[serde(rename = "land_kind")]
        kind: LandKind,
        /// Whether the land needed a human's review (`gate/needs-review`); false for a code
        /// land. Read from `tier` on a record written before review became yes or no:
        /// `HARD` is true, anything else false (`yes-no-review`).
        #[serde(default, alias = "tier", deserialize_with = "review_or_tier")]
        review: bool,
        fingerprint: String,
        /// The landed commit's code key, which the land's `checked` record names. None, and
        /// left out of the line, for a record written before the field existed
        /// (`landed-record-names-its-code-key`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        code_key: Option<String>,
        /// The live change IDs a spec land added or modified.
        changes: Vec<String>,
        /// The check command's wall-clock time in whole seconds, when the land ran
        /// `[land] check`; left out of the line otherwise (`landed-times`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        check_seconds: Option<u64>,
        /// The steps the check marked, in order; left out of the line when there are none.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        steps: Vec<TimedStep>,
        /// How the land checked its merged tree; left out of the line when unknown
        /// (`landed-recheck`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        recheck: Option<Recheck>,
        /// With `overlap` or `selected`, the number of `file` lines the check ran; left out
        /// when absent or when it ran every test.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        recheck_files: Option<u64>,
    },
    /// A browser's Web Push subscription for this repo, which `web.worker` notifies
    /// (`push-subscriptions-in-ledger`).
    Subscribed {
        endpoint: String,
        p256dh: String,
        auth: String,
    },
    /// The end of the subscription whose `subscribed` record's `id` is `subscription`.
    Unsubscribed { subscription: String },
    /// A backlog item proposed from the web app, which `clm sync` files as
    /// `backlog/<slug>.md` (`web-proposals-as-ledger-records`).
    Proposed {
        slug: String,
        title: String,
        body: String,
        nodes: Vec<String>,
    },
    /// The withdrawal of the proposal whose `proposed` record's `id` is `proposal`.
    Withdrawn { proposal: String },
    /// A person's answer from the web app to a backlog item, `done` for a `human-task`
    /// (`web-answers-as-ledger-records`).
    Answered { item: String, answer: String },
    /// A reviewer's comment on a spec branch's diff (`web-review-feedback-as-ledger-records`).
    Commented {
        branch: String,
        fingerprint: String,
        /// A requirement ID `<node>/<slug>`, or null for the change as a whole: always
        /// written, so a line without it doesn't parse.
        #[serde(deserialize_with = "present")]
        target: Option<String>,
        text: String,
    },
    /// A reviewer's request that the branch change before it lands; `land` holds the branch
    /// while its fingerprint is `fingerprint`.
    #[serde(rename = "changes_requested")]
    ChangesRequested {
        branch: String,
        fingerprint: String,
        text: String,
    },
    /// The work one Claude Code session spent on one branch (`effort-accounted`). Its `id`
    /// is `effort_id(session, branch)`, so a later count supersedes an earlier one.
    Effort {
        branch: String,
        session: String,
        /// Token counts by model name.
        models: BTreeMap<String, Usage>,
        active_seconds: u64,
    },
    /// A human's review decision on a capability (`review-capability-cost`).
    Decided {
        /// `capability/<slug>` or `architecture/<slug>`.
        capability: String,
        value: Rating,
        decision: Decision,
        /// The slugs of the friction items it settles.
        settles: Vec<String>,
    },
    /// The result of a project's slow tests run after the land on `commit`
    /// (`verify-after-land`). A report, not a gate: never signed.
    Verified {
        /// A commit of `main`, a full 40-hex ID.
        #[serde(deserialize_with = "commit_id")]
        commit: String,
        passed: bool,
        /// The last commit a `verified` record named before this one.
        #[serde(default, deserialize_with = "optional_commit_id")]
        since: Option<String>,
        /// The first-parent commits of `main` it covers, oldest first.
        #[serde(default, deserialize_with = "commit_ids")]
        lands: Vec<String>,
        /// The land that broke it.
        #[serde(default, deserialize_with = "optional_commit_id")]
        culprit: Option<String>,
        /// The components a red result holds; empty when it passed.
        #[serde(default)]
        components: Vec<String>,
        mode: VerifyMode,
        /// The host pipeline run's URL.
        #[serde(default)]
        run: Option<String>,
    },
}

/// The two code keys a composed `checked` record was built from (`composed-check`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Basis {
    pub head: String,
    pub base: String,
}

/// How a land checked its merged tree (`landed-recheck`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Recheck {
    /// Its code key was already checked.
    Checked,
    /// The overlap selected nothing.
    None,
    /// The overlap ran.
    Overlap,
    /// The branch's own selection ran.
    Selected,
}

/// One step a land's check marked, as a `landed` record holds it (`landed-times`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimedStep {
    pub name: String,
    pub seconds: u64,
    /// The paths the step's `::clm-selected::` lines named; none when the field is absent.
    #[serde(default)]
    pub selected_by: Vec<String>,
}

/// Where a `verified` record's slow tests ran and who recorded the result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerifyMode {
    Local,
    Pipeline,
    Import,
}

/// `id` if it is a full 40-hex commit ID, in lowercase as git writes it, since records are
/// looked up by exact string.
fn full_id<E: serde::de::Error>(id: String) -> Result<String, E> {
    if id.len() == 40 && id.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')) {
        Ok(id)
    } else {
        Err(E::custom(format!("`{id}` is not a full commit ID")))
    }
}

fn commit_id<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    full_id(String::deserialize(d)?)
}

fn optional_commit_id<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(d)?.map(full_id).transpose()
}

fn commit_ids<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    Vec::<String>::deserialize(d)?
        .into_iter()
        .map(full_id)
        .collect()
}

/// One model's token counts in an `effort` record.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

impl Usage {
    /// Work tokens: input, output and cache writes; cache reads are counted apart.
    pub fn work(&self) -> u64 {
        self.input
            .saturating_add(self.output)
            .saturating_add(self.cache_write)
    }
}

/// A capability's value, as a `decided` record's `value` holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rating {
    High,
    Medium,
    Low,
}

impl Rating {
    /// As the line writes it.
    pub fn as_str(self) -> &'static str {
        match self {
            Rating::High => "high",
            Rating::Medium => "medium",
            Rating::Low => "low",
        }
    }
}

/// What a review decided for a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    Keep,
    Cut,
    Redesign,
    Accept,
}

impl Decision {
    /// As the line writes it.
    pub fn as_str(self) -> &'static str {
        match self {
            Decision::Keep => "keep",
            Decision::Cut => "cut",
            Decision::Redesign => "redesign",
            Decision::Accept => "accept",
        }
    }
}

impl Body {
    /// The record kind, as `kind` holds it.
    pub fn kind(&self) -> &'static str {
        match self {
            Body::Approved { .. } => "approved",
            Body::Checked { .. } => "checked",
            Body::Landed { .. } => "landed",
            Body::Subscribed { .. } => "subscribed",
            Body::Unsubscribed { .. } => "unsubscribed",
            Body::Proposed { .. } => "proposed",
            Body::Withdrawn { .. } => "withdrawn",
            Body::Answered { .. } => "answered",
            Body::Commented { .. } => "commented",
            Body::ChangesRequested { .. } => "changes_requested",
            Body::Effort { .. } => "effort",
            Body::Decided { .. } => "decided",
            Body::Verified { .. } => "verified",
        }
    }
}

/// An `Option` field that must be present, `null` or not: with `deserialize_with`, serde no
/// longer takes a missing key as `None`.
fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Option::deserialize(d)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LandKind {
    Spec,
    Code,
}

/// A `landed` record's `review`, or its legacy `tier`: `"HARD"` is true, any other tier or
/// `null` false.
fn review_or_tier<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    match Value::deserialize(d)? {
        Value::Bool(review) => Ok(review),
        Value::String(tier) => Ok(tier == "HARD"),
        Value::Null => Ok(false),
        other => Err(serde::de::Error::custom(format!(
            "`review` must be true or false, not {other}"
        ))),
    }
}
