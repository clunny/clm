//! `clm:remove-by <change-id>` markers must name a live change (see specs/check/SPEC.md).

use std::path::PathBuf;

use clm_model::{ChangeStatus, Graph, Violation};

use crate::Marker;

/// `remove-by-markers` violations: each marker whose change is archived, missing, done
/// or abandoned.
pub fn violations(graph: &Graph, markers: &[Marker]) -> Vec<Violation> {
    markers
        .iter()
        .filter_map(|m| {
            let why = match graph.changes.get(&m.id) {
                Some(c) if c.archived => "is archived",
                None => "doesn't exist",
                Some(c) => match c.frontmatter.status {
                    ChangeStatus::Done => "is done",
                    ChangeStatus::Abandoned => "is abandoned",
                    _ => return None,
                },
            };
            Some(Violation {
                path: PathBuf::from(m.to_string()),
                rule: "check/remove-by-markers",
                message: format!("marker names {}, which {why}", m.id),
            })
        })
        .collect()
}
