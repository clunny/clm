//! `check/priority-caps`: at most 1 `P1`, 2 `P2`, 3 `P3` and 4 `P4` among the changes
//! outside the archive and the backlog items (`priority-caps`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use clm_model::{ChangeStatus, Graph, Priority, Violation};

/// One violation per priority with more holders than its cap, naming every holder sorted.
pub fn violations(graph: &Graph) -> Vec<Violation> {
    let mut holders: BTreeMap<Priority, BTreeSet<String>> = BTreeMap::new();
    let mut resolved: BTreeSet<&str> = BTreeSet::new();
    for c in graph
        .changes
        .values()
        .filter(|c| !c.archived && c.frontmatter.status != ChangeStatus::Abandoned)
    {
        holders
            .entry(c.frontmatter.priority)
            .or_default()
            .insert(format!("changes/{}", c.id));
        resolved.extend(c.frontmatter.resolves.iter().map(String::as_str));
    }
    for item in graph.backlog.values() {
        if !resolved.contains(item.slug.as_str()) {
            holders
                .entry(item.frontmatter.priority)
                .or_default()
                .insert(format!("backlog/{}", item.slug));
        }
    }
    holders
        .into_iter()
        .filter_map(|(priority, names)| {
            let cap = priority.cap().filter(|cap| names.len() > *cap)?;
            let list: Vec<&str> = names.iter().map(String::as_str).collect();
            // The first holder's file: `backlog/<slug>.md` or `changes/<id>/change.md`.
            let path = match list[0].strip_prefix("backlog/") {
                Some(slug) => format!("backlog/{slug}.md"),
                None => format!("{}/change.md", list[0]),
            };
            Some(Violation {
                path: PathBuf::from(path),
                rule: "check/priority-caps",
                message: format!(
                    "{} holders of {priority}, cap {cap}: {}; demote some",
                    names.len(),
                    list.join(", ")
                ),
            })
        })
        .collect()
}
