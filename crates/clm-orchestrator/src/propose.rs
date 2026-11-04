//! Backlog claims and proposals (`propose-claims`): `clm claim backlog/<slug>`,
//! `taken_items` and `clm proposals`, the refs read from `origin` and the rules applied by
//! [`taken_from_refs`].

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use clm_git::{git, now};
use clm_model::Graph;

use crate::queue::ClaimRef;
use crate::repo::{
    Claim, PROPOSE_PREFIX, claims, current_branch, extract_main, fetch, load, push_new_claim,
};
use crate::taken::{Holding, SpecRef, Taken, change_id, named, taken_from_refs};
use crate::{DEFAULT_EXPIRY_HOURS, Error, REMOTE, lapsed};

const SPEC_PREFIX: &str = "spec/";
const AUTOSYNC_PREFIX: &str = "autosync/spec/";

/// What `clm proposals` prints: its lines, and the warnings for stderr.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Proposals {
    pub lines: Vec<String>,
    pub warnings: Vec<String>,
}

/// Everything that can hold a backlog item, read from `origin` after fetching.
struct Survey {
    /// The graph at `origin/main`.
    graph: Graph,
    /// The `spec/*` and `autosync/spec/*` refs, named `origin/...`, the checkout's own
    /// change left out.
    spec_refs: Vec<SpecRef>,
    /// `origin/propose/<slug>` claims by slug, lapsed ones included.
    claims: BTreeMap<String, Claim>,
    /// The items the checkout's own `changes/<id>/change.md` names.
    own_items: BTreeSet<String>,
}

fn survey(root: &Path) -> Result<Survey, Error> {
    fetch(root)?;
    let graph = load(extract_main(root)?.path())?;
    let own = current_branch(root)
        .as_deref()
        .and_then(|b| b.strip_prefix(SPEC_PREFIX))
        .map(String::from);
    let own_items = own
        .as_deref()
        .and_then(|id| {
            let changes = root.join("changes");
            std::fs::read_to_string(changes.join(id).join("change.md"))
                .or_else(|_| {
                    std::fs::read_to_string(changes.join("archive").join(id).join("change.md"))
                })
                .ok()
        })
        .and_then(|text| named(&text))
        .unwrap_or_default();
    let refs = git(
        root,
        &[
            "for-each-ref",
            "--format=%(refname)",
            &format!("refs/remotes/{REMOTE}/{SPEC_PREFIX}"),
            &format!("refs/remotes/{REMOTE}/{AUTOSYNC_PREFIX}"),
        ],
    )?;
    let mut spec_refs = Vec::new();
    for full in refs.lines() {
        let Some(short) = full.strip_prefix("refs/remotes/") else {
            continue;
        };
        // The checkout's own change, and work already on main, need no reading.
        let Some(id) = change_id(short) else {
            continue;
        };
        if own.as_deref() == Some(id) || graph.changes.contains_key(id) {
            continue;
        }
        // A tip with no live `change.md` is read through the archive's, and its deltas
        // from there (`archived-spec-work`).
        let (change, dir) = [format!("changes/{id}"), format!("changes/archive/{id}")]
            .into_iter()
            .find_map(|dir| {
                git(root, &["show", &format!("{full}:{dir}/change.md")])
                    .ok()
                    .map(|text| (Some(text), dir))
            })
            .unwrap_or((None, String::new()));
        let deltas = match change {
            Some(_) => git(
                root,
                &["ls-tree", "--name-only", full, &format!("{dir}/deltas/")],
            )?
            .lines()
            .filter_map(|p| Some(Path::new(p).file_name()?.to_str()?.to_string()))
            .collect(),
            None => Vec::new(),
        };
        spec_refs.push(SpecRef {
            name: short.to_string(),
            change,
            deltas,
        });
    }
    let claims = claims(root, PROPOSE_PREFIX)?;
    Ok(Survey {
        graph,
        spec_refs,
        claims,
        own_items,
    })
}

impl Survey {
    /// Whether `origin/propose/<slug>` is a live claim by another session.
    fn claimed(&self, slug: &str, expiry_hours: u64) -> bool {
        self.claims
            .get(slug)
            .is_some_and(|c| !lapsed(c.time, now(), expiry_hours))
            && !self.own_items.contains(slug)
    }

    /// [`taken_from_refs`] on what was read, with the claims of the checkout's own items
    /// left out.
    fn taken(&self, expiry_hours: u64) -> Taken {
        let propose: Vec<ClaimRef> = self
            .claims
            .iter()
            .filter(|(slug, _)| !self.own_items.contains(*slug))
            .map(|(slug, c)| ClaimRef {
                node: slug.clone(),
                tip_time: c.time,
                merged: false,
            })
            .collect();
        taken_from_refs(&self.graph, &self.spec_refs, &propose, now(), expiry_hours)
    }
}

/// Claims `backlog/<slug>` by pushing `propose/<slug>` and returns the branch name, or
/// refuses, naming what holds it.
pub(crate) fn claim_backlog(root: &Path, slug: &str, expiry_hours: u64) -> Result<String, Error> {
    if slug.is_empty() || slug.contains('/') {
        return Err(Error::Refused(format!("`backlog/{slug}` isn't an item")));
    }
    let s = survey(root)?;
    if !s.graph.backlog.contains_key(slug) {
        return Err(Error::Refused(format!(
            "no such item: `backlog/{slug}.md` isn't in `{REMOTE}/main`"
        )));
    }
    let held = if s.claimed(slug, expiry_hours) {
        Some(format!("claimed by {PROPOSE_PREFIX}{slug}"))
    } else {
        let taken = s.taken(expiry_hours);
        let holding = |state: Holding| {
            taken
                .changes
                .iter()
                .find(|c| c.state == state && c.items.contains(slug))
                .map(|c| c.id.clone())
        };
        holding(Holding::Main)
            .map(|id| format!("resolved by {id} on main"))
            .or_else(|| holding(Holding::Proposed).map(|id| format!("proposed in {id}")))
    };
    if let Some(held) = held {
        return Err(Error::Refused(format!("`backlog/{slug}` is {held}")));
    }
    let expect = s.claims.get(slug).map_or(String::new(), |c| c.sha.clone());
    push_new_claim(
        root,
        &format!("backlog/{slug}"),
        &format!("{PROPOSE_PREFIX}{slug}"),
        &expect,
    )
}

/// What holds backlog items, read from `origin` after fetching (`taken-items`), computed
/// by [`taken_from_refs`]: the changes on main whose `resolves` names an item still in
/// `backlog/`, the proposed spec work, and the claims live within `expiry_hours` that none
/// of those changes names. As the Design's *taken* says, a checkout on `spec/<id>` leaves
/// out its own change `<id>` and the claims of the items its working-tree `change.md`
/// names.
pub fn taken_items(root: &Path, expiry_hours: u64) -> Result<Taken, Error> {
    Ok(survey(root)?.taken(expiry_hours))
}

/// `clm proposals [--node <id>]...`: [`taken_items`] at the default expiry, one line each,
/// keeping only the lines whose nodes include one of `nodes` when any are given.
pub fn proposals(root: &Path, nodes: &[String]) -> Result<Proposals, Error> {
    let taken = taken_items(root, DEFAULT_EXPIRY_HOURS)?;
    let list = |set: &BTreeSet<String>| set.iter().cloned().collect::<Vec<_>>().join(",");
    let keep = |n: &BTreeSet<String>| nodes.is_empty() || nodes.iter().any(|w| n.contains(w));
    let changes = taken.changes.iter().filter(|c| keep(&c.nodes)).map(|c| {
        let state = match c.state {
            Holding::Main => "main",
            Holding::Proposed => "proposed",
        };
        let mut line = format!("{} {state} [{}]", c.id, list(&c.nodes));
        for item in &c.items {
            line.push(' ');
            line.push_str(item);
        }
        line
    });
    let claims = taken
        .claims
        .iter()
        .filter(|c| keep(&c.nodes))
        .map(|c| format!("backlog/{} claimed [{}]", c.item, list(&c.nodes)));
    Ok(Proposals {
        lines: changes.chain(claims).collect(),
        warnings: taken.warnings,
    })
}
