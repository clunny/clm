//! A web write checked against the state: what the Worker's routes checked on request
//! before the replica held the tip and the ledgers (`replica/validate-writes`).

use clm_ledger::Body;

use crate::State;
use crate::compute;

/// A slug's bytes at most, as `clm defect` cuts one.
const SLUG_MAX: usize = 64;

/// A write a person asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Write {
    /// A decision on an entry, `capability/<slug>` or `architecture/<slug>`, naming the
    /// version of it the person saw.
    Decision { id: String, version: String },
    /// A proposed backlog item, with the components it names.
    Proposal { title: String, nodes: Vec<String> },
}

/// What to append for a write that holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Validated {
    /// The slugs of the workaround flags the decision settles.
    Decision { settles: Vec<String> },
    /// The slug the proposal files under.
    Proposal { slug: String },
}

/// Why a write is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// A decision on an entry the tip doesn't hold, or a tip whose specs don't load.
    Unknown,
    /// A decision in a repository that lists its signers.
    Signed,
    /// A decision on an entry reworded since the person saw it.
    Stale,
    /// A proposal whose slug a backlog item, a change's `resolves` or an open proposal has.
    Taken,
    /// A proposal naming a component the graph doesn't hold, or with no slug.
    Invalid,
}

pub(crate) fn validate(state: &State, write: &Write) -> Result<Validated, Refusal> {
    let Some(files) = state.spec_files() else {
        return Err(Refusal::Unknown);
    };
    let (graph, violations) = clm_model::load_files(&files);
    if !violations.is_empty() {
        return Err(Refusal::Unknown);
    }
    match write {
        Write::Decision { id, version } => {
            let entry = graph.entry(id).ok_or(Refusal::Unknown)?;
            if compute::signed(state) {
                return Err(Refusal::Signed);
            }
            if entry.version() != version {
                return Err(Refusal::Stale);
            }
            let fold = compute::fold(state);
            Ok(Validated::Decision {
                settles: clm_effort::unsettled(&graph, &fold, id),
            })
        }
        Write::Proposal { title, nodes } => {
            let slug = slug(title);
            if slug.is_empty() || nodes.iter().any(|n| !graph.nodes.contains_key(n)) {
                return Err(Refusal::Invalid);
            }
            let fold = compute::fold(state);
            let filed = compute::filed(&graph);
            let open = fold
                .proposals
                .values()
                .any(|r| matches!(&r.body, Body::Proposed { slug: s, .. } if *s == slug));
            if filed.contains(slug.as_str()) || open {
                return Err(Refusal::Taken);
            }
            Ok(Validated::Proposal { slug })
        }
    }
}

/// The title in kebab case as `clm defect` slugs one (`plugin.hooks/defect-backlog`):
/// lowercased, split on anything but ASCII letters and digits, the words joined with `-`
/// while the result stays within [`SLUG_MAX`] bytes, a first word longer than that cut to
/// it; empty when the title has no such word.
fn slug(title: &str) -> String {
    let mut slug = String::new();
    let lower = title.to_lowercase();
    for word in lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
    {
        let sep = usize::from(!slug.is_empty());
        if slug.len() + sep + word.len() > SLUG_MAX {
            if slug.is_empty() {
                slug.push_str(&word[..SLUG_MAX]);
            }
            break;
        }
        if sep == 1 {
            slug.push('-');
        }
        slug.push_str(word);
    }
    slug
}
