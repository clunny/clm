//! `check/backlog-entry-shape`: a live change's `resolves` entries are bare slugs and its
//! `from` is `backlog/<slug>`. Only the shape is tested, never whether the item exists.

use clm_model::{Graph, Violation};

/// One violation per malformed entry of each change outside the archive.
pub fn violations(graph: &Graph) -> Vec<Violation> {
    let mut out = Vec::new();
    for c in graph.changes.values().filter(|c| !c.archived) {
        let mut report = |message: String| {
            out.push(Violation {
                path: c.path.clone(),
                rule: "check/backlog-entry-shape",
                message,
            })
        };
        for entry in &c.frontmatter.resolves {
            if !is_bare(entry) {
                report(format!(
                    "change `{}` resolves `{entry}`: `resolves` entries are bare slugs; {}",
                    c.id,
                    advice(entry, "")
                ));
            }
        }
        if let Some(from) = &c.frontmatter.from
            && !from.strip_prefix("backlog/").is_some_and(is_bare)
        {
            report(format!(
                "change `{}` has `from: {from}`: `from` is `backlog/<slug>`; {}",
                c.id,
                advice(from, "backlog/")
            ));
        }
    }
    out
}

/// Non-empty, with no `/`, not ending in `.md`.
fn is_bare(slug: &str) -> bool {
    !slug.is_empty() && !slug.contains('/') && !slug.ends_with(".md")
}

/// `write <prefix><slug>` for the entry with one leading `backlog/` and one trailing
/// `.md` removed, when that is a bare slug.
fn advice(entry: &str, prefix: &str) -> String {
    let slug = entry.strip_prefix("backlog/").unwrap_or(entry);
    let slug = slug.strip_suffix(".md").unwrap_or(slug);
    if is_bare(slug) {
        format!("write `{prefix}{slug}`")
    } else {
        "name a top-level backlog item".to_string()
    }
}
