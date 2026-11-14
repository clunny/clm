//! A file's owner, the one place `paths-disjoint`, `unowned-files`, `undeclared-imports` and
//! `tests-cite-their-owner` read it (see specs/check/SPEC.md).

use clm_model::{ExemptRule, Graph, Node};

use crate::matches;

/// The components that own `file`, sorted by ID: those whose `tests` match it or, when no
/// component's `tests` do, those whose `paths` match it (`components-own-their-tests`). More
/// than one is what `paths-disjoint` reports.
pub(crate) fn owners<'g>(graph: &'g Graph, file: &str) -> Vec<&'g Node> {
    let by = |globs: fn(&Node) -> &Vec<String>| -> Vec<&Node> {
        graph
            .nodes
            .values()
            .filter(|n| globs(n).iter().any(|g| matches(g, file)))
            .collect()
    };
    let by_tests = by(|n| &n.frontmatter.tests);
    if by_tests.is_empty() {
        by(|n| &n.frontmatter.paths)
    } else {
        by_tests
    }
}

/// The one component that owns `file`: the first of the components whose `tests` match it, or
/// else the deepest whose `paths` match it, then the first by ID.
pub(crate) fn owner<'g>(graph: &'g Graph, file: &str) -> Option<&'g Node> {
    let owners = owners(graph, file);
    if owners.iter().any(|n| is_test_of(n, file)) {
        return owners.into_iter().next();
    }
    let depth = |n: &Node| {
        if n.id == clm_model::ROOT_ID {
            0
        } else {
            n.id.split('.').count()
        }
    };
    owners
        .into_iter()
        .min_by_key(|n| (std::cmp::Reverse(depth(n)), n.id.clone()))
}

/// Whether one of `node`'s `tests` globs matches `file`.
pub(crate) fn is_test_of(node: &Node, file: &str) -> bool {
    node.frontmatter.tests.iter().any(|g| matches(g, file))
}

/// Whether `node` has an exemption from `rule` whose `files` match `file`
/// (`model/exemptions`).
pub(crate) fn exempt(node: &Node, rule: ExemptRule, file: &str) -> bool {
    node.frontmatter
        .exemptions
        .iter()
        .any(|e| e.rule == rule && e.files.iter().any(|g| matches(g, file)))
}
