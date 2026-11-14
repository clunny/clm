//! The lists `clm coverage` prints beside its report (`test-lists`): markers no test
//! counts, tests a removal leaves with nothing to cite, and tests a tree has lost. They
//! take files as paths and texts, so they build without the `checkout` feature.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use clm_model::Graph;

use crate::coverage::{Marker, parse_markers, removed};
use crate::spans::tests_in;

/// A requirement or capability that markers outside a test cite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uncounted {
    pub id: String,
    /// Where the markers are, sorted by file and line.
    pub markers: Vec<Marker>,
}

/// A test (never a group) holding a marker that cites something a change removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObsoleteTest {
    /// `<file>::<name>`.
    pub test: String,
    /// The test's first line.
    pub line: usize,
    /// The removed IDs its markers cite, sorted.
    pub ids: Vec<String>,
    /// Whether every marker on the test's lines cites a removed ID.
    pub every: bool,
}

/// A test of the base tree that the head tree holds no test of that name for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletedTest {
    /// `<file>::<name>`.
    pub test: String,
    /// The test's first line in the base tree.
    pub line: usize,
    /// The IDs its markers cited, sorted.
    pub ids: Vec<String>,
}

/// Each requirement or capability that exists, or that a change outside the archive
/// removed, and that a marker in `outside` cites, sorted by ID.
pub fn uncounted(graph: &Graph, outside: &[Marker]) -> Vec<Uncounted> {
    let removed = removed(graph);
    let mut by_id: BTreeMap<&str, Vec<Marker>> = BTreeMap::new();
    for m in outside {
        if graph.requirement(&m.id).is_some() || removed.contains_key(&m.id) {
            by_id.entry(m.id.as_str()).or_default().push(m.clone());
        }
    }
    by_id
        .into_iter()
        .map(|(id, mut markers)| {
            markers.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
            Uncounted {
                id: id.to_string(),
                markers,
            }
        })
        .collect()
}

/// A test of one file, groups left out, with the IDs of the markers on its lines.
struct Cited {
    test: String,
    line: usize,
    ids: Vec<String>,
}

fn cited_tests(path: &Path, text: &str) -> Vec<Cited> {
    let lines: Vec<&str> = text.lines().collect();
    tests_in(path, text)
        .into_iter()
        .filter(|t| !t.group)
        .map(|t| Cited {
            test: format!("{}::{}", path.display(), t.name),
            line: t.first,
            ids: lines[t.first - 1..t.last.min(lines.len())]
                .iter()
                .flat_map(|l| parse_markers(l))
                .map(|s| s.id)
                .collect(),
        })
        .collect()
}

fn sorted_unique(ids: impl IntoIterator<Item = String>) -> Vec<String> {
    ids.into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Each test holding a marker that cites a requirement or capability a change outside the
/// archive removed, sorted by file and then by line.
pub fn obsolete_tests(graph: &Graph, files: &[(PathBuf, String)]) -> Vec<ObsoleteTest> {
    let removed = removed(graph);
    let mut out: Vec<(&Path, ObsoleteTest)> = Vec::new();
    for (path, text) in files {
        for c in cited_tests(path, text) {
            let gone: Vec<String> = c
                .ids
                .iter()
                .filter(|id| removed.contains_key(*id))
                .cloned()
                .collect();
            if gone.is_empty() {
                continue;
            }
            let test = ObsoleteTest {
                test: c.test,
                line: c.line,
                every: gone.len() == c.ids.len(),
                ids: sorted_unique(gone),
            };
            out.push((path, test));
        }
    }
    out.sort_by(|a, b| (a.0, a.1.line).cmp(&(b.0, b.1.line)));
    out.into_iter().map(|(_, t)| t).collect()
}

/// Each test holding a marker in `base_files` that `head_files` holds no test of that name
/// in a file of that path for, sorted by file and then by line. A renamed test is listed,
/// since a program can't tell a rename from a removal.
pub fn deleted_tests(
    base_files: &[(PathBuf, String)],
    head_files: &[(PathBuf, String)],
) -> Vec<DeletedTest> {
    let kept: BTreeSet<String> = head_files
        .iter()
        .flat_map(|(path, text)| cited_tests(path, text))
        .map(|c| c.test)
        .collect();
    let mut out: Vec<(&Path, DeletedTest)> = Vec::new();
    for (path, text) in base_files {
        for c in cited_tests(path, text) {
            if c.ids.is_empty() || kept.contains(&c.test) {
                continue;
            }
            let test = DeletedTest {
                test: c.test,
                line: c.line,
                ids: sorted_unique(c.ids),
            };
            out.push((path, test));
        }
    }
    out.sort_by(|a, b| (a.0, a.1.line).cmp(&(b.0, b.1.line)));
    out.into_iter().map(|(_, t)| t).collect()
}
