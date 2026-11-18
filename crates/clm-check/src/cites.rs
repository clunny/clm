//! `tests-cite-their-owner`: a test file a component's `tests` match, or any file a component
//! owns once the root names its `wiring`, cites only its owner's requirements (see specs/check/SPEC.md).

use std::path::PathBuf;

use clm_model::{ExemptRule, Graph, Layer, Violation};

use crate::Marker;
use crate::ownership::{exempt, is_test_of, owner, owners};

pub fn violations(graph: &Graph, markers: &[Marker]) -> Vec<Violation> {
    // Once the root names its `wiring`, every owned file is judged, since the land refuses a
    // build's edit to another component's file (`model/wiring-globs`, `gate/own-files`).
    let wired = graph.wiring().is_some();
    markers
        .iter()
        .filter_map(|m| {
            let file = m.path.to_string_lossy();
            // Until then, only a file some component's `tests` match is judged.
            if !wired && !owners(graph, &file).iter().any(|n| is_test_of(n, &file)) {
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
            if exempt(mine, ExemptRule::TestsCiteTheirOwner, &file) {
                return None;
            }
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
