//! The signals that a requirement is fighting its implementation: `workaround`,
//! `rewritten`, `many-scenarios`, `many-exceptions` and `reconsider`
//! (`requirement-signals`, `reconsider-score`); and `unserved`, the human-layer entries
//! no rule serves (`unserved-entries`).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use clm_model::{BacklogItem, BacklogKind, Graph, Layer, Node, Requirement};

use crate::{Section, Settings, finding, phrase_at};

/// Phrases that make an exception to a rule, matched case-insensitively at word
/// boundaries.
const EXCEPTION_PHRASES: [&str; 8] = [
    "unless",
    "except",
    "but not",
    "other than",
    "only when",
    "in which case",
    "whichever",
    "so also",
];

/// Each requirement of every node, with its node.
pub(crate) fn requirements(graph: &Graph) -> impl Iterator<Item = (&Node, &Requirement)> {
    graph
        .nodes
        .values()
        .flat_map(|n| n.requirements.iter().map(move |r| (n, r)))
}

/// Each entry of the human layers that no rule's effective serves names.
pub(crate) fn unserved(graph: &Graph) -> Section {
    let served: std::collections::BTreeSet<&str> = requirements(graph)
        .flat_map(|(_, r)| graph.effective_serves(&r.id))
        .map(String::as_str)
        .collect();
    let lines = graph
        .layers
        .iter()
        .flat_map(|(layer, entries)| entries.iter().map(move |e| (layer, e)))
        .filter(|(_, e)| !served.contains(e.id.as_str()))
        .map(|(layer, e)| {
            finding(
                Layer::file(*layer),
                "unserved",
                format!("{} served by no rule", e.id),
            )
        })
        .collect();
    Section::of("unserved", "entries", lines)
}

/// The `workaround` items of `items`, with the requirement each names.
fn flags(items: &BTreeMap<String, BacklogItem>) -> impl Iterator<Item = (&BacklogItem, &str)> {
    items.values().filter_map(|i| {
        let r = i.frontmatter.requirement.as_ref()?;
        (i.frontmatter.kind == BacklogKind::Friction).then_some((i, r.id.as_str()))
    })
}

/// Each `workaround` item, with the requirement it names and its title.
pub(crate) fn workaround(graph: &Graph) -> Section {
    let lines = flags(&graph.backlog)
        .map(|(item, id)| {
            let title = &item.frontmatter.title;
            finding(item.path.display(), "workaround", format!("{id} {title}"))
        })
        .collect();
    Section::of("workaround", "flags", lines)
}

/// The IDs of the archived changes whose delta for `node` modifies `slug`.
fn rewrites<'g>(graph: &'g Graph, node: &str, slug: &str) -> Vec<&'g str> {
    graph
        .changes
        .values()
        .filter(|c| c.archived)
        .filter(|c| {
            c.deltas
                .iter()
                .any(|d| d.node == node && d.modified.iter().any(|r| r.slug == slug))
        })
        .map(|c| c.id.as_str())
        .collect()
}

/// What says a requirement is fighting its implementation, and its score
/// (`reconsider-score`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementSignals {
    /// `<node>/<slug>`
    pub id: String,
    /// The paths and titles of the `workaround` items that name it.
    pub workarounds: Vec<(PathBuf, String)>,
    /// The IDs of the archived changes that modified it.
    pub rewritten: Vec<String>,
    pub scenarios: usize,
    /// The exception clauses in its text above its first `####` block.
    pub exceptions: usize,
    /// Zero with no `workaround` item, else the flags, plus the rewrites, plus one when the
    /// scenarios exceed `scenarios` and one when the exceptions exceed `exceptions`.
    pub score: u64,
}

impl RequirementSignals {
    /// `(scenarios, exceptions)`: one for each count over its setting.
    fn over(&self, settings: &Settings) -> (u64, u64) {
        (
            u64::from(self.scenarios as u64 > settings.scenarios),
            u64::from(self.exceptions as u64 > settings.exceptions),
        )
    }

    /// `workarounds <f>, rewritten <r>, scenarios <0|1>, exceptions <0|1>`
    fn breakdown(&self, settings: &Settings) -> String {
        let (s, e) = self.over(settings);
        format!(
            "workarounds {}, rewritten {}, scenarios {s}, exceptions {e}",
            self.workarounds.len(),
            self.rewritten.len()
        )
    }
}

/// Each requirement's signals and score, from the graph and its backlog items, with no
/// git. The thresholds of `settings` decide the scenario and exception points.
pub fn signals(
    graph: &Graph,
    items: &BTreeMap<String, BacklogItem>,
    settings: &Settings,
) -> Vec<RequirementSignals> {
    requirements(graph)
        .map(|(n, r)| {
            let slug = &r.id[n.id.len() + 1..];
            let mut signals = RequirementSignals {
                id: r.id.clone(),
                workarounds: flags(items)
                    .filter(|(_, id)| *id == r.id)
                    .map(|(i, _)| (i.path.clone(), i.frontmatter.title.clone()))
                    .collect(),
                rewritten: rewrites(graph, &n.id, slug)
                    .into_iter()
                    .map(String::from)
                    .collect(),
                scenarios: r.scenarios,
                exceptions: exception_clauses(r),
                score: 0,
            };
            if !signals.workarounds.is_empty() {
                let (s, e) = signals.over(settings);
                signals.score =
                    (signals.workarounds.len() + signals.rewritten.len()) as u64 + s + e;
            }
            signals
        })
        .collect()
}

/// A requirement whose score reaches the threshold, with the text of the item `clm sync`
/// files as `backlog/reconsider-<node>-<slug>.md` (`sync/file-reconsider-item`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reconsider {
    /// `<node>/<slug>`
    pub id: String,
    pub text: String,
}

/// `backlog/reconsider-<node>-<slug>.md` for requirement `<node>/<slug>`.
fn item_path(id: &str) -> String {
    format!("backlog/reconsider-{}.md", id.replacen('/', "-", 1))
}

/// The requirements of `node` whose score reaches `settings.reconsider`, each with the
/// text of its reconsider item.
pub fn reconsider(
    graph: &Graph,
    items: &BTreeMap<String, BacklogItem>,
    settings: &Settings,
    node: &str,
) -> Vec<Reconsider> {
    signals(graph, items, settings)
        .into_iter()
        .filter(|s| {
            s.score >= settings.reconsider && s.id.split_once('/').is_some_and(|(n, _)| n == node)
        })
        .map(|s| Reconsider {
            text: item_text(node, &s, settings),
            id: s.id,
        })
        .collect()
}

/// The reconsider item for `s`, a requirement of `node`.
fn item_text(node: &str, s: &RequirementSignals, settings: &Settings) -> String {
    let title = serde_json::to_string(&format!("Reconsider {}", s.id)).expect("a string");
    let mut text = format!(
        "---\ntitle: {title}\nkind: question\ncomponents: [{node}]\n---\n\n\
`{}` scores {} against the reconsider threshold of {} ({}).\n\nWorkaround flags:\n",
        s.id,
        s.score,
        settings.reconsider,
        s.breakdown(settings),
    );
    for (path, title) in &s.workarounds {
        let _ = writeln!(text, "- {}: {title}", path.display());
    }
    text.push_str("\nArchived changes that modified it:");
    if s.rewritten.is_empty() {
        text.push_str(" none");
    }
    text.push('\n');
    for id in &s.rewritten {
        let _ = writeln!(text, "- {id}");
    }
    let _ = write!(
        text,
        "\nScenarios: {} (setting {}). Exception clauses: {} (setting {}).\n\n\
Three ways out, each a spec change: drop the requirement, change it, or redesign around \
it. Deleting this item without one brings it back on the component's next build.\n",
        s.scenarios, settings.scenarios, s.exceptions, settings.exceptions,
    );
    text
}

/// Each requirement whose score reaches `settings.reconsider`, and whether its item is
/// filed at `root`.
pub(crate) fn reconsider_section(root: &Path, graph: &Graph, settings: &Settings) -> Section {
    let signals = signals(graph, &graph.backlog, settings);
    let lines = signals
        .iter()
        .filter(|s| s.score >= settings.reconsider)
        .filter_map(|s| Some((graph.nodes.get(s.id.split_once('/')?.0)?, s)))
        .map(|(n, s)| {
            let item = item_path(&s.id);
            let filed = if root.join(&item).exists() {
                format!("filed as {item}")
            } else {
                "not filed".into()
            };
            finding(
                n.path.display(),
                "reconsider",
                format!(
                    "{} score {} ({}); {filed}",
                    s.id,
                    s.score,
                    s.breakdown(settings)
                ),
            )
        })
        .collect();
    Section::of("reconsider", "requirements", lines)
}

/// Each requirement that two or more archived changes modified.
pub(crate) fn rewritten(graph: &Graph) -> Section {
    let lines = requirements(graph)
        .filter_map(|(n, r)| {
            let slug = &r.id[n.id.len() + 1..];
            let count = rewrites(graph, &n.id, slug).len();
            (count >= 2).then(|| {
                finding(
                    n.path.display(),
                    "rewritten",
                    format!("{} modified by {count} archived changes", r.id),
                )
            })
        })
        .collect();
    Section::of("rewritten", "requirements", lines)
}

/// Each requirement with more scenarios than `max`.
pub(crate) fn many_scenarios(graph: &Graph, max: u64) -> Section {
    let lines = requirements(graph)
        .filter(|(_, r)| r.scenarios as u64 > max)
        .map(|(n, r)| {
            finding(
                n.path.display(),
                "many-scenarios",
                format!("{} {} scenarios", r.id, r.scenarios),
            )
        })
        .collect();
    Section::of("many-scenarios", "requirements", lines)
}

/// The exception phrases in a requirement's text above its first `####` block, its
/// heading left out.
fn exception_clauses(r: &Requirement) -> usize {
    let text: Vec<&str> = r
        .text
        .lines()
        .skip(1)
        .take_while(|l| !l.starts_with("####"))
        .collect();
    let text = text.join("\n");
    let mut count = 0;
    let mut i = 0;
    while i < text.len() {
        match phrase_at(&text, i, &EXCEPTION_PHRASES) {
            Some(p) => {
                count += 1;
                i += p.len();
            }
            None => i += 1,
        }
    }
    count
}

/// Each requirement whose text holds more exception clauses than `max`.
pub(crate) fn many_exceptions(graph: &Graph, max: u64) -> Section {
    let lines = requirements(graph)
        .filter_map(|(n, r)| {
            let count = exception_clauses(r);
            (count as u64 > max).then(|| {
                finding(
                    n.path.display(),
                    "many-exceptions",
                    format!("{} {count} exception clauses", r.id),
                )
            })
        })
        .collect();
    Section::of("many-exceptions", "requirements", lines)
}
