//! `tests-cite-their-owner`: a test file a component's `tests` match cites only that
//! component's requirements (see specs/check/SPEC.md).

use std::path::PathBuf;

use clm_model::{Graph, Layer, Violation};

use crate::Marker;
use crate::ownership::{is_test_of, owner, owners};

pub fn violations(graph: &Graph, markers: &[Marker]) -> Vec<Violation> {
    markers
        .iter()
        .filter_map(|m| {
            let file = m.path.to_string_lossy();
            // Only a file some component's `tests` match is judged.
            if !owners(graph, &file).iter().any(|n| is_test_of(n, &file)) {
                return None;
            }
            // A marker naming no requirement is `dangling-markers`'s, an architecture entry
            // too; a capability is the capability component's requirement.
            graph.requirement(&m.id)?;
            let cited = match Layer::of(&m.id) {
                Some(Layer::Architecture) => return None,
                Some(Layer::Capability) => graph.capability_tests_node(),
                None => m.id.split_once('/')?.0,
            };
            let mine = owner(graph, &file)?;
            (mine.id != cited).then(|| Violation {
                path: PathBuf::from(m.to_string()),
                rule: "check/tests-cite-their-owner",
                message: format!(
                    "cites {} of `{cited}`, but the file is `{}`'s test; move the test to `{cited}`'s tests",
                    m.id, mine.id
                ),
            })
        })
        .collect()
}
