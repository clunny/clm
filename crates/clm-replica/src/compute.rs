//! The views computed from the state: the implementation view's shared part, the
//! capabilities report and the architecture report, beside the snapshot.

use std::collections::{BTreeMap, BTreeSet};

use clm_ledger::{Body, Fold, person};
use clm_model::{Files, Graph};
use clm_orchestrator::{ClaimRef, DEFAULT_EXPIRY_HOURS, Holding, SpecRef, claim_state};
use serde::Serialize;
use serde_json::{Value, json};

use crate::snapshot::{self, Coverage};
use crate::{Event, RefChange, State, Views};

/// The feed's length at most.
const ACTIVITY: usize = crate::EVENTS;

/// The name of a branch under `prefix`, `None` for another prefix or a nested name such as
/// `impl/archive/<id>`, which names no component (`orchestrator/nested-branches-not-claims`).
pub(crate) fn branch<'a>(name: &'a str, prefix: &str) -> Option<&'a str> {
    name.strip_prefix(prefix).filter(|n| !n.contains('/'))
}

/// The fold of every ledger branch the state holds.
pub(crate) fn fold(state: &State) -> Fold {
    let sources: Vec<(&str, &str)> = state
        .ledgers
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();
    clm_ledger::fold_records(&sources)
}

/// The tip's tree holds `docs/allowed_signers`.
pub(crate) fn signed(state: &State) -> bool {
    state.tree.contains_key("docs/allowed_signers")
}

/// Every claim: an `impl/<component>` ref, and a `propose/<item>` one.
fn claims(state: &State, prefix: &str) -> Vec<ClaimRef> {
    state
        .refs
        .iter()
        .filter_map(|(name, r)| {
            Some(ClaimRef {
                node: branch(name, prefix)?.to_string(),
                tip_time: r.time,
                merged: false,
            })
        })
        .collect()
}

/// The time the earliest live claim lapses, at least a second from `now`.
pub(crate) fn next_lapse(state: &State, now: u64) -> Option<u64> {
    let all = claims(state, "impl/")
        .into_iter()
        .chain(claims(state, "propose/"));
    all.filter(|c| claim_state(c, now, DEFAULT_EXPIRY_HOURS) == "live")
        .map(|c| (c.tip_time + DEFAULT_EXPIRY_HOURS * 3600).max(now + 1))
        .min()
}

/// Every view, at `now`.
pub(crate) fn views(state: &State, files: &Files, now: u64) -> Views {
    let (graph, violations) = clm_model::load_files(files);
    let violations: Vec<String> = violations.iter().map(clm_check::format).collect();
    let fold = fold(state);
    let flag_text = clm_effort::flag_text(&graph, &|path| {
        String::from_utf8(files.get(path)?.clone()).ok()
    });
    let inputs = clm_effort::Inputs {
        graph: &graph,
        fold: &fold,
        history: None,
        flag_text: &flag_text,
    };
    let signed = signed(state);
    let current = state.index_complete().then(|| {
        clm_check::report(&graph, &state.markers())
            .into_iter()
            .filter(|c| c.state == clm_check::State::Current)
            .map(|c| c.id)
            .collect::<BTreeSet<String>>()
    });
    let architecture = clm_effort::architecture(&inputs, current.as_ref());
    Views {
        sha: state.tip.clone().unwrap_or_default(),
        at: now,
        snapshot: snapshot::build(state, &graph, &violations, files),
        implementation: implementation(state, &graph, &fold, now),
        capabilities: json!({ "report": clm_effort::report(&inputs), "signed": signed }),
        architecture: json!({
            "architecture": architecture,
            "signed": signed,
            "indexing": current.is_none(),
        }),
    }
}

#[derive(Serialize)]
struct Page {
    number: u64,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    checked: Option<bool>,
}

#[derive(Serialize)]
struct Activity {
    kind: &'static str,
    at: u64,
    actor: String,
    r#ref: String,
}

#[derive(Serialize)]
struct TakenChange {
    id: String,
    state: &'static str,
    items: Vec<String>,
    nodes: Vec<String>,
}

#[derive(Serialize)]
struct TakenClaim {
    item: String,
    nodes: Vec<String>,
}

#[derive(Serialize)]
struct Taken {
    changes: Vec<TakenChange>,
    claims: Vec<TakenClaim>,
}

/// The implementation view's shared part: the proposals carry no `mine`, which is per requester.
fn implementation(state: &State, graph: &Graph, fold: &Fold, now: u64) -> Value {
    let complete = state.index_complete();
    let markers = if complete {
        state.markers()
    } else {
        Vec::new()
    };
    let coverage = if complete {
        Coverage::Complete
    } else {
        Coverage::Indexing {
            done: state.index_done(),
        }
    };
    let refs = claims(state, "impl/");
    json!({
        "sha": state.tip,
        "now": now,
        "expiry_hours": DEFAULT_EXPIRY_HOURS,
        "coverage": coverage,
        "flight": clm_view::flight_view(graph, &markers, &refs, now, DEFAULT_EXPIRY_HOURS),
        "blockers": clm_view::blockers_view(graph, &markers, &refs, now, DEFAULT_EXPIRY_HOURS),
        "pages": pages(state, fold),
        "activity": activity(state, fold),
        "taken": taken(state, graph, now),
        "proposals": proposals(graph, fold),
    })
}

/// The slugs a backlog item has or a change's `resolves` names: spent, since `clm sync`
/// files none of them.
pub(crate) fn filed(graph: &Graph) -> BTreeSet<&str> {
    graph
        .backlog
        .keys()
        .map(String::as_str)
        .chain(
            graph
                .changes
                .values()
                .flat_map(|c| c.frontmatter.resolves.iter().map(String::as_str)),
        )
        .collect()
}

#[derive(Serialize)]
struct Proposal {
    id: String,
    writer: String,
    at: String,
    slug: String,
    title: String,
    body: String,
    nodes: Vec<String>,
}

/// The fold's open proposals by `at` then `id`, leaving out each whose slug a backlog item
/// of the tip has or a change of the tip, archived or not, resolves: the rule `clm sync`
/// files by.
fn proposals(graph: &Graph, fold: &Fold) -> Vec<Proposal> {
    let filed = filed(graph);
    let mut open: Vec<Proposal> = fold
        .proposals
        .values()
        .filter_map(|r| match &r.body {
            Body::Proposed {
                slug,
                title,
                body,
                nodes,
            } if !filed.contains(slug.as_str()) => Some(Proposal {
                id: r.id.clone(),
                writer: r.writer.clone(),
                at: r.at.clone(),
                slug: slug.clone(),
                title: title.clone(),
                body: body.clone(),
                nodes: nodes.clone(),
            }),
            _ => None,
        })
        .collect();
    open.sort_by(|a, b| (&a.at, &a.id).cmp(&(&b.at, &b.id)));
    open
}

/// The events' pushes and claims and the fold's lands and approvals, newest first.
fn activity(state: &State, fold: &Fold) -> Vec<Activity> {
    let events = state.events.iter().map(|e| match e {
        Event::Push { branch, at, actor } => Activity {
            kind: "push",
            at: *at,
            actor: actor.clone(),
            r#ref: branch.clone(),
        },
        Event::Create { branch, at, actor } => Activity {
            kind: "claim",
            at: *at,
            actor: actor.clone(),
            r#ref: branch.strip_prefix("impl/").unwrap_or(branch).to_string(),
        },
    });
    let records = fold.records.iter().filter_map(|r| {
        let (kind, branch) = match &r.body {
            Body::Landed { branch, .. } => ("land", branch),
            Body::Approved { branch, .. } => ("approval", branch),
            _ => return None,
        };
        Some(Activity {
            kind,
            at: unix_time(&r.at)?,
            actor: person(&r.writer).into(),
            r#ref: branch.clone(),
        })
    });
    let mut feed: Vec<Activity> = events.chain(records).collect();
    feed.sort_by_key(|a| std::cmp::Reverse(a.at));
    feed.truncate(ACTIVITY);
    feed
}

/// Each `impl/*` branch with an open pull request, with whether its head was checked.
fn pages(state: &State, fold: &Fold) -> BTreeMap<String, Page> {
    let mut out = BTreeMap::new();
    for pull in state
        .pulls
        .values()
        .filter(|p| p.head.starts_with("impl/") && !p.fork)
    {
        let key = state.refs.get(&pull.head).and_then(|r| match &r.change {
            Some(RefChange::Code { key }) if r.sha == pull.head_sha => Some(key),
            _ => None,
        });
        out.entry(pull.head.clone()).or_insert(Page {
            number: pull.number,
            url: pull.url.clone(),
            checked: key.map(|k| fold.checks.contains_key(k)),
        });
    }
    out
}

/// What holds the backlog's items at `now`, from the refs.
fn taken(state: &State, graph: &Graph, now: u64) -> Taken {
    let spec: Vec<SpecRef> = state
        .refs
        .iter()
        .filter(|(name, _)| {
            branch(name, "spec/").is_some() || branch(name, "autosync/spec/").is_some()
        })
        .map(|(name, r)| {
            let (change, deltas) = match &r.change {
                Some(RefChange::Spec { change, deltas }) => (change.clone(), deltas.clone()),
                _ => (None, Vec::new()),
            };
            SpecRef {
                name: name.clone(),
                change,
                deltas,
            }
        })
        .collect();
    let propose = claims(state, "propose/");
    let taken =
        clm_orchestrator::taken_from_refs(graph, &spec, &propose, now, DEFAULT_EXPIRY_HOURS);
    Taken {
        changes: taken
            .changes
            .into_iter()
            .map(|c| TakenChange {
                id: c.id,
                state: match c.state {
                    Holding::Main => "main",
                    Holding::Proposed => "proposed",
                },
                items: c.items.into_iter().collect(),
                nodes: c.nodes.into_iter().collect(),
            })
            .collect(),
        claims: taken
            .claims
            .into_iter()
            .map(|c| TakenClaim {
                item: c.item,
                nodes: c.nodes.into_iter().collect(),
            })
            .collect(),
    }
}

/// Unix seconds of an RFC 3339 UTC time, `None` for one that doesn't read.
fn unix_time(iso: &str) -> Option<u64> {
    let b = iso.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[10] != b'T' {
        return None;
    }
    let num = |r: std::ops::Range<usize>| iso.get(r)?.parse::<i64>().ok();
    let (y, m, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (hh, mm, ss) = (num(11..13)?, num(14..16)?, num(17..19)?);
    // Days from civil (Howard Hinnant).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + hh * 3600 + mm * 60 + ss).ok()
}
