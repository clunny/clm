//! What holds backlog items (`propose-claims`), without git or a checkout
//! (`taken-from-refs`): [`taken_from_refs`] is what `taken_items` computes its result with,
//! and what the `web` Worker calls on refs it read itself.

use std::collections::{BTreeMap, BTreeSet};

use clm_model::{Graph, split_frontmatter};
use serde::Deserialize;

use crate::lapsed;
use crate::queue::ClaimRef;

/// A `spec/<id>` or `autosync/spec/<id>` ref: its name as warnings should name it (any
/// prefix, such as the remote's, followed by `spec/<id>` or `autosync/spec/<id>`), the text
/// of its tip's `changes/<id>/change.md` (else `changes/archive/<id>/change.md`), or `None`
/// when the tip has neither, and the file names under that file's `deltas/`, such as `a.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecRef {
    pub name: String,
    pub change: Option<String>,
    pub deltas: Vec<String>,
}

impl SpecRef {
    /// The change ID the ref names, as [`change_id`] reads it from its name.
    fn id(&self) -> Option<&str> {
        change_id(&self.name)
    }
}

/// The change ID of a ref named `spec/<id>` or `autosync/spec/<id>`, either one alone or
/// after one leading segment such as the remote's (`origin/spec/<id>`), where `<id>` holds
/// no `/`; `None` for any other name, so a nested branch names no change.
pub(crate) fn change_id(name: &str) -> Option<&str> {
    let after_segment = name.split_once('/').map(|(_, rest)| rest);
    [Some(name), after_segment]
        .into_iter()
        .flatten()
        .filter_map(|n| {
            n.strip_prefix("autosync/spec/")
                .or_else(|| n.strip_prefix("spec/"))
        })
        .find(|id| !id.is_empty() && !id.contains('/'))
}

/// Where a change that holds backlog items is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Holding {
    /// Outside the archive in `origin/main`'s tree.
    Main,
    /// On `origin/spec/<id>` or `origin/autosync/spec/<id>`, and not on main.
    Proposed,
}

/// A change that holds backlog items: its ID, where it is, the items it names (for a
/// change on main, those still in `backlog/`) and its nodes as `clm proposals` gives them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoldingChange {
    pub id: String,
    pub state: Holding,
    pub items: BTreeSet<String>,
    pub nodes: BTreeSet<String>,
}

/// A live `propose/<slug>` claim that no [`HoldingChange`] names, with the item's nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BareClaim {
    pub item: String,
    pub nodes: BTreeSet<String>,
}

/// What holds backlog items: the changes sorted by ID, then the bare claims sorted by
/// item, and the warnings about changes whose frontmatter doesn't parse.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Taken {
    pub changes: Vec<HoldingChange>,
    pub claims: Vec<BareClaim>,
    pub warnings: Vec<String>,
}

/// The `from` and `resolves` of a `change.md`; every other key is ignored, so a branch
/// that adds a frontmatter field still counts.
#[derive(Deserialize, Default)]
struct Named {
    #[serde(default)]
    from: Option<String>,
    #[serde(default)]
    resolves: Vec<String>,
}

/// The items a `change.md` names in `from` or `resolves`, or `None` when its frontmatter
/// doesn't parse.
pub(crate) fn named(text: &str) -> Option<BTreeSet<String>> {
    let text = text.replace("\r\n", "\n");
    let (fm, _) = split_frontmatter(&text)?;
    let named: Named = serde_yaml::from_str(fm).ok()?;
    let mut items: BTreeSet<String> = named.resolves.into_iter().collect();
    if let Some(slug) = named
        .from
        .as_deref()
        .and_then(|f| f.strip_prefix("backlog/"))
    {
        items.insert(slug.to_string());
    }
    Some(items)
}

/// What holds backlog items (`taken-from-refs`), for `main`'s graph `main`, the spec refs
/// `spec_refs`, and the `propose/<slug>` claims `propose` (`node` holding the slug; a
/// backlog claim is never merged, so `merged` isn't read: only the expiry ends one), at
/// `now` with claims live for `expiry_hours`: the changes outside the archive on main whose
/// `resolves` names an item still in `backlog/`; the proposed spec work, a change on a spec
/// ref with a `change.md` that isn't under `changes/` or `changes/archive/` on main, both
/// refs of one change joined; and the live claims none of those changes names. Items and
/// their nodes are looked up in `main`'s backlog.
pub fn taken_from_refs(
    main: &Graph,
    spec_refs: &[SpecRef],
    propose: &[ClaimRef],
    now: u64,
    expiry_hours: u64,
) -> Taken {
    let item_nodes = |slug: &str| -> BTreeSet<String> {
        main.backlog
            .get(slug)
            .map(|i| i.frontmatter.nodes.iter().cloned().collect())
            .unwrap_or_default()
    };
    // A change is on main or proposed, never both, so IDs don't collide.
    let mut changes: BTreeMap<String, HoldingChange> = BTreeMap::new();
    let mut warnings = Vec::new();
    for c in main.changes.values().filter(|c| !c.archived) {
        let items: BTreeSet<String> = c
            .frontmatter
            .resolves
            .iter()
            .filter(|r| main.backlog.contains_key(*r))
            .cloned()
            .collect();
        if items.is_empty() {
            continue;
        }
        changes.insert(
            c.id.clone(),
            HoldingChange {
                id: c.id.clone(),
                state: Holding::Main,
                items,
                nodes: c.deltas.iter().map(|d| d.node.clone()).collect(),
            },
        );
    }
    for r in spec_refs {
        let (Some(id), Some(text)) = (r.id(), &r.change) else {
            continue;
        };
        if main.changes.contains_key(id) {
            continue;
        }
        let entry = changes
            .entry(id.to_string())
            .or_insert_with(|| HoldingChange {
                id: id.to_string(),
                state: Holding::Proposed,
                items: BTreeSet::new(),
                nodes: BTreeSet::new(),
            });
        match named(text) {
            Some(items) => entry.items.extend(items),
            None => warnings.push(format!(
                "{}: changes/{id}/change.md has no readable frontmatter, so the items it \
                 names are unknown",
                r.name
            )),
        }
        entry.nodes.extend(
            r.deltas
                .iter()
                .filter_map(|d| d.strip_suffix(".md"))
                .map(String::from),
        );
    }
    let mut named_items: BTreeSet<String> = BTreeSet::new();
    for c in changes.values_mut() {
        match c.state {
            Holding::Main => {
                named_items.extend(main.changes[&c.id].frontmatter.resolves.iter().cloned())
            }
            Holding::Proposed => {
                named_items.extend(c.items.iter().cloned());
                for item in &c.items {
                    c.nodes.extend(item_nodes(item));
                }
            }
        }
    }
    let mut claims: Vec<BareClaim> = propose
        .iter()
        .filter(|p| !lapsed(p.tip_time, now, expiry_hours) && !named_items.contains(&p.node))
        .map(|p| BareClaim {
            item: p.node.clone(),
            nodes: item_nodes(&p.node),
        })
        .collect();
    claims.sort_by(|a, b| a.item.cmp(&b.item));
    claims.dedup_by(|a, b| a.item == b.item);
    Taken {
        changes: changes.into_values().collect(),
        claims,
        warnings,
    }
}
