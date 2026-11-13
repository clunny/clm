//! What each capability costs, from the graph, the ledger's fold and, where there is one,
//! the repository's history (`effort/attribution`, `effort/unsettled-flags`).

use std::collections::{BTreeMap, BTreeSet};

use clm_ledger::{Body, Decision, Fold, Rating, Record};
use clm_model::{BacklogKind, Graph, Layer, Requirement};
use serde::{Deserialize, Serialize};

use crate::dollars;

/// What `report` reads.
pub struct Inputs<'a> {
    pub graph: &'a Graph,
    pub fold: &'a Fold,
    /// What only git knows; `None` in the Worker.
    pub history: Option<&'a History>,
    /// The friction items' text, by slug.
    pub flag_text: &'a BTreeMap<String, String>,
}

/// What only git knows (`effort/history`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct History {
    /// The commits that touched each node's `SPEC.md` or `paths`, by node.
    pub commits: BTreeMap<String, BTreeSet<String>>,
    /// Each life of a `question`, `human` or `friction` item, oldest first.
    pub items: Vec<ItemLife>,
    /// Each node's spec and code tokens.
    pub sizes: BTreeMap<String, u64>,
    /// Now, in seconds since the epoch.
    pub now: i64,
}

/// One life of a backlog item's file: from the commit that added it to the one that deleted
/// it, with its frontmatter as added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemLife {
    pub slug: String,
    pub kind: BacklogKind,
    pub nodes: Vec<String>,
    pub blocks: Vec<String>,
    /// Commit times, in seconds since the epoch.
    pub added: i64,
    pub deleted: Option<i64>,
}

/// Work, as estimated dollars, tokens and time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Spend {
    pub dollars: f64,
    /// Input, output and cache writes.
    pub work_tokens: f64,
    pub cache_read_tokens: f64,
    pub active_seconds: f64,
}

impl Spend {
    fn add(&mut self, other: &Spend, share: f64) {
        self.dollars += other.dollars * share;
        self.work_tokens += other.work_tokens * share;
        self.cache_read_tokens += other.cache_read_tokens * share;
        self.active_seconds += other.active_seconds * share;
    }
}

/// A friction item open on a capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Flag {
    pub slug: String,
    /// The requirement it flags, `<node>/<slug>`.
    pub requirement: String,
    pub title: String,
    pub text: String,
    /// Whether a decision on this capability settled it.
    pub settled: bool,
    /// The date its file was added, `YYYY-MM-DD`; `None` without history.
    pub added: Option<String>,
}

/// What a capability's rules overlap with.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Overlap {
    /// Its rules that serve no other capability.
    pub only: usize,
    /// The other capabilities its rules serve, with how many, in `CAPABILITIES.md`'s order.
    pub shared: Vec<(String, usize)>,
}

/// A review decision on a capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decided {
    pub at: String,
    pub writer: String,
    pub value: Rating,
    pub decision: Decision,
}

/// One capability's page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cost {
    /// `capability/<slug>`.
    pub id: String,
    pub slug: String,
    pub version: String,
    pub statement: String,
    pub why: String,
    /// Its entry's Markdown below its heading (`entry-text`).
    pub text: String,
    /// The rules whose effective serves names it.
    pub rules: Vec<String>,
    /// Those rules' nodes.
    pub nodes: Vec<String>,
    pub spend: Spend,
    pub changes: usize,
    pub approvals: usize,
    pub flags_settled: usize,
    pub burdens: Vec<String>,
    pub commits: Option<usize>,
    pub questions_answered: Option<usize>,
    /// The items that held its rules up, and the days they were open in all.
    pub blocking: Option<(usize, u64)>,
    pub size_tokens: Option<f64>,
    pub flags: Vec<Flag>,
    pub overlap: Overlap,
    pub decisions: Vec<Decided>,
    /// At least one flag is unsettled.
    pub flagged: bool,
}

/// One enforcing rule of an architecture entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Enforcing {
    pub rule: String,
    /// Whether the rule is current; `None` when the caller can't tell.
    pub current: Option<bool>,
}

/// One architecture entry's page (`effort/architecture-report`): a `Cost`'s fields that
/// hold without history, over the entry's enforcing rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constraint {
    /// `architecture/<slug>`.
    pub id: String,
    pub slug: String,
    pub version: String,
    pub statement: String,
    /// Its entry's Markdown below its heading (`entry-text`).
    pub text: String,
    pub enforced_by: Vec<Enforcing>,
    /// Its `#### Untested` text.
    pub untested: Option<String>,
    /// Its enforcing rules.
    pub rules: Vec<String>,
    /// Those rules' nodes.
    pub nodes: Vec<String>,
    pub spend: Spend,
    pub changes: usize,
    pub approvals: usize,
    pub flags_settled: usize,
    pub burdens: Vec<String>,
    pub flags: Vec<Flag>,
    pub overlap: Overlap,
    pub decisions: Vec<Decided>,
    /// At least one flag is unsettled.
    pub flagged: bool,
}

/// Every capability's cost, and the work that went to none.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Report {
    /// By dollars, highest first, ties in `CAPABILITIES.md`'s order.
    pub capabilities: Vec<Cost>,
    pub unattributed: Spend,
    pub architecture_only: Spend,
}

/// The capabilities rule `id` serves that the layer holds (`check` reports any other).
pub fn capabilities_of<'g>(graph: &'g Graph, id: &str) -> Vec<&'g str> {
    let serves = graph.effective_serves(id).iter();
    serves
        .filter(|s| Layer::of(s) == Some(Layer::Capability) && graph.entry(s).is_some())
        .map(String::as_str)
        .collect()
}

/// Each node's share of its cost by capability, and the share on `architecture only`
/// (`None` for a node without rules).
fn shares(graph: &Graph) -> BTreeMap<&str, (BTreeMap<&str, f64>, f64)> {
    let mut out = BTreeMap::new();
    for node in graph.nodes.values() {
        let rules = &node.requirements;
        if rules.is_empty() {
            continue;
        }
        let weight = 1.0 / rules.len() as f64;
        let (mut by, mut architecture) = (BTreeMap::new(), 0.0);
        for rule in rules {
            let caps = capabilities_of(graph, &rule.id);
            if caps.is_empty() {
                architecture += weight;
            }
            for cap in &caps {
                *by.entry(*cap).or_insert(0.0) += weight / caps.len() as f64;
            }
        }
        out.insert(node.id.as_str(), (by, architecture));
    }
    out
}

/// The nodes a branch's work goes to, each with its share; what the shares leave of 1 is
/// unattributed.
fn targets<'g>(graph: &'g Graph, branch: &str) -> Vec<(&'g str, f64)> {
    if let Some(node) = branch.strip_prefix("impl/") {
        return match graph.nodes.get_key_value(node) {
            Some((id, _)) => vec![(id.as_str(), 1.0)],
            None => Vec::new(),
        };
    }
    let Some(change) = branch
        .strip_prefix("spec/")
        .and_then(|c| graph.changes.get(c))
    else {
        return Vec::new();
    };
    let nodes: BTreeSet<&str> = change.deltas.iter().map(|d| d.node.as_str()).collect();
    let share = 1.0 / nodes.len() as f64;
    nodes
        .into_iter()
        .filter_map(|n| graph.nodes.get_key_value(n))
        .map(|(id, _)| (id.as_str(), share))
        .collect()
}

/// An `effort` record's work.
fn spend_of(record: &Record) -> Spend {
    let Body::Effort {
        models,
        active_seconds,
        ..
    } = &record.body
    else {
        return Spend::default();
    };
    let mut spend = Spend {
        active_seconds: *active_seconds as f64,
        ..Spend::default()
    };
    for (model, usage) in models {
        spend.dollars += dollars(model, usage);
        spend.work_tokens += usage.work() as f64;
        spend.cache_read_tokens += usage.cache_read as f64;
    }
    spend
}

/// The branch a record names, for an `effort` or `approved` one.
fn branch_of(record: &Record) -> Option<&str> {
    match &record.body {
        Body::Effort { branch, .. } | Body::Approved { branch, .. } => Some(branch),
        _ => None,
    }
}

/// An entry's text before its first `####` block, or in its `#### <name>` block, as one
/// line.
fn block(entry: &Requirement, name: Option<&str>) -> String {
    let mut lines = entry.text.lines().skip(1);
    if let Some(name) = name {
        let heading = format!("#### {name}");
        lines.by_ref().find(|l| l.trim_end() == heading);
    }
    let text: Vec<&str> = lines
        .take_while(|l| !l.starts_with("#### "))
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    text.join(" ")
}

/// An entry's Markdown after its heading, with the blank lines around it removed.
fn entry_text(entry: &Requirement) -> String {
    let body: Vec<&str> = entry.text.lines().skip(1).collect();
    body.join("\n").trim().to_string()
}

/// The slugs a capability's decisions settled.
fn settled(fold: &Fold, capability: &str) -> BTreeSet<String> {
    let decided = fold.decisions.get(capability).into_iter().flatten();
    decided
        .filter_map(|r| match &r.body {
            Body::Decided { settles, .. } => Some(settles.iter().cloned()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// The friction items open on `entry`, a capability or an architecture entry, by slug, with
/// their requirement.
fn open_flags<'g>(graph: &'g Graph, entry: &str) -> Vec<(&'g str, &'g str, &'g str)> {
    let enforcing = match Layer::of(entry) {
        Some(Layer::Architecture) => graph.entry(entry).map(|e| &e.enforced_by),
        _ => None,
    };
    let items = graph.backlog.values();
    items
        .filter(|i| i.frontmatter.kind == BacklogKind::Friction)
        .filter_map(|i| {
            let r = i.frontmatter.requirement.as_ref()?;
            let open = match enforcing {
                Some(rules) => rules.contains(&r.id),
                None => capabilities_of(graph, &r.id).contains(&entry),
            };
            open.then_some((i.slug.as_str(), r.id.as_str(), i.frontmatter.title.as_str()))
        })
        .collect()
}

/// The friction items' text by slug: each backlog file's body after its frontmatter, trimmed,
/// `read` giving a file's text by its path.
pub fn flag_text(
    graph: &Graph,
    read: &dyn Fn(&std::path::Path) -> Option<String>,
) -> BTreeMap<String, String> {
    graph
        .backlog
        .values()
        .filter(|i| i.frontmatter.kind == BacklogKind::Friction)
        .filter_map(|i| {
            let text = read(&i.path)?;
            let body = clm_model::split_frontmatter(&text).map_or("", |(_, b)| b);
            Some((i.slug.clone(), body.trim().to_string()))
        })
        .collect()
}

/// The friction flags open and unsettled on a capability or an architecture entry, named by
/// its ID, by slug.
pub fn unsettled(graph: &Graph, fold: &Fold, capability: &str) -> Vec<String> {
    let settled = settled(fold, capability);
    open_flags(graph, capability)
        .into_iter()
        .filter(|(slug, _, _)| !settled.contains(*slug))
        .map(|(slug, _, _)| slug.to_string())
        .collect()
}

/// `YYYY-MM-DD` for seconds since the epoch.
fn date(secs: i64) -> String {
    // Howard Hinnant's civil_from_days.
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// A decided entry's decisions, oldest first.
fn decisions_of(fold: &Fold, id: &str) -> Vec<Decided> {
    let decided = fold.decisions.get(id).into_iter().flatten();
    decided
        .filter_map(|r| match &r.body {
            Body::Decided {
                value, decision, ..
            } => Some(Decided {
                at: r.at.clone(),
                writer: r.writer.clone(),
                value: *value,
                decision: *decision,
            }),
            _ => None,
        })
        .collect()
}

/// The friction flags open on `id`, each settled when `settled` names its slug.
fn flags_of(
    graph: &Graph,
    id: &str,
    settled: &BTreeSet<String>,
    flag_text: &BTreeMap<String, String>,
    history: Option<&History>,
) -> Vec<Flag> {
    let added = |slug: &str| {
        let lives = history?.items.iter().rev();
        let mut lives = lives.filter(|l| l.kind == BacklogKind::Friction);
        lives.find(|l| l.slug == slug).map(|l| date(l.added))
    };
    open_flags(graph, id)
        .into_iter()
        .map(|(slug, requirement, title)| Flag {
            slug: slug.into(),
            requirement: requirement.into(),
            title: title.into(),
            text: flag_text.get(slug).cloned().unwrap_or_default(),
            settled: settled.contains(slug),
            added: added(slug),
        })
        .collect()
}

/// The changes, live or archived, whose deltas name one of `nodes`, and their declared
/// burdens.
fn changes_on(graph: &Graph, nodes: &BTreeSet<&str>) -> (usize, Vec<String>) {
    let changes: Vec<_> = graph
        .changes
        .values()
        .filter(|c| c.deltas.iter().any(|d| nodes.contains(d.node.as_str())))
        .collect();
    let burdens: BTreeSet<String> = changes
        .iter()
        .flat_map(|c| c.frontmatter.burdens.iter().cloned())
        .collect();
    (changes.len(), burdens.into_iter().collect())
}

/// The architecture review (`effort/architecture-report`): one `Constraint` per entry of
/// `ARCHITECTURE.md`, by dollars. `current` holds the IDs of the requirements that are
/// current, or is `None` when the caller can't tell.
pub fn architecture(inputs: &Inputs, current: Option<&BTreeSet<String>>) -> Vec<Constraint> {
    let Inputs {
        graph,
        fold,
        flag_text,
        ..
    } = *inputs;
    let entries: &[Requirement] = graph
        .layers
        .get(&Layer::Architecture)
        .map_or(&[], Vec::as_slice);
    // How many entries name each rule: its part is split over them.
    let mut naming: BTreeMap<&str, usize> = BTreeMap::new();
    for rule in entries.iter().flat_map(|e| &e.enforced_by) {
        *naming.entry(rule).or_default() += 1;
    }
    // What each rule of a node is worth of the node's cost.
    let part = |rule: &str| -> Option<(String, f64)> {
        let (node, _) = rule.split_once('/')?;
        let rules = graph.nodes.get(node)?.requirements.len();
        let weight = 1.0 / rules as f64 / naming.get(rule).copied()? as f64;
        Some((node.to_string(), weight))
    };
    let efforts: Vec<(Spend, Vec<(&str, f64)>)> = fold
        .efforts
        .values()
        .map(|r| {
            (
                spend_of(r),
                targets(graph, branch_of(r).unwrap_or_default()),
            )
        })
        .collect();
    let approved: Vec<BTreeSet<&str>> = fold
        .approvals
        .values()
        .flatten()
        .map(|r| {
            let nodes = targets(graph, branch_of(r).unwrap_or_default());
            nodes.into_iter().map(|(n, _)| n).collect()
        })
        .collect();
    let mut constraints: Vec<Constraint> = entries
        .iter()
        .map(|entry| {
            let id = entry.id.as_str();
            let nodes: BTreeSet<&str> = entry
                .enforced_by
                .iter()
                .filter_map(|r| r.split_once('/').map(|(n, _)| n))
                .collect();
            let mut spend = Spend::default();
            for rule in &entry.enforced_by {
                let Some((node, weight)) = part(rule) else {
                    continue;
                };
                for (work, targets) in &efforts {
                    if let Some((_, share)) = targets.iter().find(|(n, _)| *n == node.as_str()) {
                        spend.add(work, share * weight);
                    }
                }
            }
            let (changes, burdens) = changes_on(graph, &nodes);
            let settled = settled(fold, id);
            let flags = flags_of(graph, id, &settled, flag_text, None);
            let (mut only, mut shared) = (0, BTreeMap::new());
            for rule in &entry.enforced_by {
                if naming.get(rule.as_str()) == Some(&1) {
                    only += 1;
                }
                let others = entries.iter().filter(|e| e.id != id);
                for other in others.filter(|e| e.enforced_by.contains(rule)) {
                    *shared.entry(other.id.as_str()).or_default() += 1;
                }
            }
            let shared = entries
                .iter()
                .filter_map(|e| Some((e.id.clone(), *shared.get(e.id.as_str())?)))
                .collect();
            let flagged = flags.iter().any(|f| !f.settled);
            Constraint {
                id: id.into(),
                slug: id.trim_start_matches("architecture/").into(),
                version: entry.version().into(),
                statement: block(entry, None),
                text: entry_text(entry),
                enforced_by: entry
                    .enforced_by
                    .iter()
                    .map(|rule| Enforcing {
                        rule: rule.clone(),
                        current: current.map(|c| c.contains(rule)),
                    })
                    .collect(),
                untested: entry.untested_text.clone(),
                rules: entry.enforced_by.clone(),
                nodes: nodes.iter().map(|n| n.to_string()).collect(),
                spend,
                changes,
                approvals: approved.iter().filter(|a| !a.is_disjoint(&nodes)).count(),
                flags_settled: settled.len(),
                burdens,
                flags,
                overlap: Overlap { only, shared },
                decisions: decisions_of(fold, id),
                flagged,
            }
        })
        .collect();
    // A stable sort keeps the file's order for ties.
    constraints.sort_by(|a, b| b.spend.dollars.total_cmp(&a.spend.dollars));
    constraints
}

/// Every capability's cost.
pub fn report(inputs: &Inputs) -> Report {
    let Inputs {
        graph,
        fold,
        history,
        flag_text,
    } = *inputs;
    let entries: &[Requirement] = graph
        .layers
        .get(&Layer::Capability)
        .map_or(&[], Vec::as_slice);
    let shares = shares(graph);
    let mut out = Report::default();
    let mut spend: BTreeMap<&str, Spend> = BTreeMap::new();
    for record in fold.efforts.values() {
        let work = spend_of(record);
        let mut left = 1.0;
        for (node, share) in targets(graph, branch_of(record).unwrap_or_default()) {
            let Some((by, architecture)) = shares.get(node) else {
                continue;
            };
            left -= share;
            for (cap, part) in by {
                spend.entry(cap).or_default().add(&work, share * part);
            }
            out.architecture_only.add(&work, share * architecture);
        }
        out.unattributed.add(&work, left);
    }
    // The capabilities each branch's nodes serve, for its approvals.
    let served = |branch: &str| -> BTreeSet<&str> {
        let nodes = targets(graph, branch).into_iter();
        nodes
            .filter_map(|(n, _)| shares.get(n))
            .flat_map(|(by, _)| by.keys().copied())
            .collect()
    };
    let mut approvals: BTreeMap<&str, usize> = BTreeMap::new();
    for record in fold.approvals.values().flatten() {
        for cap in served(branch_of(record).unwrap_or_default()) {
            *approvals.entry(cap).or_default() += 1;
        }
    }
    let mut costs: Vec<Cost> = entries
        .iter()
        .map(|entry| {
            let id = entry.id.as_str();
            let rules: Vec<&Requirement> = graph
                .nodes
                .values()
                .flat_map(|n| &n.requirements)
                .filter(|r| capabilities_of(graph, &r.id).contains(&id))
                .collect();
            let nodes: BTreeSet<&str> = rules
                .iter()
                .filter_map(|r| r.id.split_once('/').map(|(n, _)| n))
                .collect();
            let (changes, burdens) = changes_on(graph, &nodes);
            let settled = settled(fold, id);
            let flags = flags_of(graph, id, &settled, flag_text, history);
            let mut overlap = Overlap::default();
            let mut shared: BTreeMap<&str, usize> = BTreeMap::new();
            for rule in &rules {
                let others: Vec<&str> = capabilities_of(graph, &rule.id)
                    .into_iter()
                    .filter(|c| *c != id)
                    .collect();
                if others.is_empty() {
                    overlap.only += 1;
                }
                for other in others {
                    *shared.entry(other).or_default() += 1;
                }
            }
            overlap.shared = entries
                .iter()
                .filter_map(|e| Some((e.id.clone(), *shared.get(e.id.as_str())?)))
                .collect();
            let decisions = decisions_of(fold, id);
            // Whether an item belongs to this capability, and whether it blocks its rules.
            let blocks = |l: &ItemLife| l.blocks.iter().any(|b| rules.iter().any(|r| r.id == *b));
            let ours =
                |l: &ItemLife| blocks(l) || l.nodes.iter().any(|n| nodes.contains(n.as_str()));
            let asked =
                |l: &&ItemLife| matches!(l.kind, BacklogKind::Question | BacklogKind::Human);
            let commits = history.map(|h| {
                let all = nodes.iter().filter_map(|n| h.commits.get(*n)).flatten();
                all.collect::<BTreeSet<_>>().len()
            });
            let questions_answered = history.map(|h| {
                let asked = h.items.iter().filter(asked);
                asked.filter(|l| l.deleted.is_some() && ours(l)).count()
            });
            let blocking = history.map(|h| {
                let held: Vec<&ItemLife> =
                    h.items.iter().filter(asked).filter(|l| blocks(l)).collect();
                let secs: i64 = held
                    .iter()
                    .map(|l| (l.deleted.unwrap_or(h.now) - l.added).max(0))
                    .sum();
                (held.len(), (secs / 86_400) as u64)
            });
            let size_tokens = history.map(|h| {
                let sized = nodes.iter().filter_map(|n| {
                    let part = shares.get(n)?.0.get(id)?;
                    Some(*h.sizes.get(*n)? as f64 * part)
                });
                sized.sum()
            });
            let flagged = flags.iter().any(|f| !f.settled);
            Cost {
                id: id.into(),
                slug: id.trim_start_matches("capability/").into(),
                version: entry.version().into(),
                statement: block(entry, None),
                why: block(entry, Some("Why")),
                text: entry_text(entry),
                rules: rules.iter().map(|r| r.id.clone()).collect(),
                nodes: nodes.iter().map(|n| n.to_string()).collect(),
                spend: spend.get(id).copied().unwrap_or_default(),
                changes,
                approvals: approvals.get(id).copied().unwrap_or_default(),
                flags_settled: settled.len(),
                burdens,
                commits,
                questions_answered,
                blocking,
                size_tokens,
                flags,
                overlap,
                decisions,
                flagged,
            }
        })
        .collect();
    // A stable sort keeps the file's order for ties.
    costs.sort_by(|a, b| b.spend.dollars.total_cmp(&a.spend.dollars));
    out.capabilities = costs;
    out
}

#[cfg(test)]
mod tests {
    use super::date;

    #[test]
    fn dates() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(1_793_318_400), "2026-10-30");
        assert_eq!(date(951_782_400), "2000-02-29");
    }
}
