//! The stages of the regenerated tree (`stale-delta-names-movers`): `origin/main`'s
//! `specs/`, then the tree after each change the regenerate applied, in order. A stale
//! base's text is found at the newest stage where the element hashes to it, and what moved
//! the element since is named from the stages that follow.

use std::cell::RefCell;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use clm_model::{Change, Graph, ROOT_ID};

use crate::Error;
use crate::stale::{History, text};

/// The graphs the regenerate loaded, one per stage, each with the change whose application
/// made it (`None` for `origin/main`'s).
pub(crate) struct Stages {
    stages: Vec<(Option<String>, Graph)>,
    /// `origin/main`'s history of each node's `SPEC.md`, read on first use and kept, so
    /// its commits are parsed once a run.
    histories: RefCell<BTreeMap<String, History>>,
}

/// What moved an element since its base: `main`, when no stage has the base, and the
/// changes whose application moved it, in ID order.
#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct Movers {
    pub main: bool,
    pub changes: BTreeSet<String>,
}

impl Movers {
    /// `main` first, then the changes, joined with ` and `.
    pub(crate) fn join(&self) -> String {
        let mut names: Vec<&str> = Vec::new();
        if self.main {
            names.push("main");
        }
        names.extend(self.changes.iter().map(String::as_str));
        names.join(" and ")
    }

    /// Both movers together, as one commit that merged elements of each names them.
    pub(crate) fn extend(&mut self, other: &Movers) {
        self.main |= other.main;
        self.changes.extend(other.changes.iter().cloned());
    }
}

/// `element`'s hash in `node` of `graph`, as `clm hashes` names it; `None` when the node
/// or the element is absent, which counts as a hash of its own.
fn hash(graph: &Graph, node: &str, element: &str) -> Option<String> {
    let n = graph.nodes.get(node)?;
    clm_apply::hashes(n)
        .into_iter()
        .find(|(e, _)| e == element)
        .map(|(_, h)| h)
}

impl Stages {
    /// No stage yet.
    pub(crate) fn new() -> Self {
        Stages {
            stages: Vec::new(),
            histories: RefCell::default(),
        }
    }

    /// The next stage: the tree after `change` applied, or `origin/main`'s for `None`.
    pub(crate) fn push(&mut self, change: Option<&str>, graph: Graph) {
        self.stages.push((change.map(str::to_string), graph));
    }

    /// The newest stage loaded again, its `specs/` as they were: a merge rewrote a delta.
    pub(crate) fn reload(&mut self, graph: Graph) {
        self.stages.last_mut().expect("one stage at least").1 = graph;
    }

    /// The newest stage: the tree the next change is applied to.
    pub(crate) fn last(&self) -> &Graph {
        &self.stages.last().expect("one stage at least").1
    }

    /// The newest stage where `element` of `node` hashes to `base`.
    fn origin(&self, node: &str, element: &str, base: &str) -> Option<usize> {
        self.stages
            .iter()
            .rposition(|(_, g)| hash(g, node, element).as_deref() == Some(base))
    }

    /// What moved `element` of `node` since `base`: `main` when no stage has the base, and
    /// each change applied after the base's stage (every one when there is none) whose
    /// application changed the element's hash.
    pub(crate) fn movers(&self, node: &str, element: &str, base: &str) -> Movers {
        let origin = self.origin(node, element, base);
        let mut movers = Movers {
            main: origin.is_none(),
            ..Movers::default()
        };
        for i in origin.map_or(1, |o| o + 1)..self.stages.len() {
            let (before, after) = (&self.stages[i - 1].1, &self.stages[i]);
            if hash(before, node, element) != hash(&after.1, node, element)
                && let Some(change) = &after.0
            {
                movers.changes.insert(change.clone());
            }
        }
        movers
    }

    /// Whether a base `change` records for a Contract, Design or requirement it modifies or
    /// removes (`stale_bases` names those `apply` doesn't accept as current) has no text at
    /// any stage or in `onto`'s history: it is a base another change has yet to produce.
    pub(crate) fn has_unknown_base(
        &self,
        root: &Path,
        onto: &str,
        change: &Change,
    ) -> Result<bool, Error> {
        for delta in &change.deltas {
            let node = self.last().nodes.get(&delta.node);
            for (element, base) in clm_apply::stale_bases(node, delta) {
                if self
                    .base_text(root, onto, &delta.node, &element, &base)?
                    .is_none()
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// `base`'s text for `element` of `node`: at its stage, else at the newest commit in
    /// `onto`'s history of the node's `SPEC.md` where it hashes to `base`; `None` when
    /// neither has it.
    pub(crate) fn base_text(
        &self,
        root: &Path,
        onto: &str,
        node: &str,
        element: &str,
        base: &str,
    ) -> Result<Option<String>, Error> {
        if let Some(i) = self.origin(node, element, base) {
            return Ok(self.stages[i]
                .1
                .nodes
                .get(node)
                .and_then(|n| text(n, element)));
        }
        let mut histories = self.histories.borrow_mut();
        let history = match histories.entry(node.to_string()) {
            Entry::Occupied(kept) => kept.into_mut(),
            Entry::Vacant(slot) => {
                let main = &self.stages[0].1;
                let path = main
                    .nodes
                    .get(node)
                    .map_or_else(|| spec_path(node), |n| n.path.clone());
                slot.insert(History::new(root, main, onto, &path)?)
            }
        };
        history.text_at(node, element, base)
    }
}

/// A node's `SPEC.md` path, as `clm-model` places it.
fn spec_path(node: &str) -> PathBuf {
    if node == ROOT_ID {
        PathBuf::from("specs/SPEC.md")
    } else {
        Path::new("specs")
            .join(node.replace('.', "/"))
            .join("SPEC.md")
    }
}
