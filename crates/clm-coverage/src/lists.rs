//! The lists `clm coverage` prints beside its report (`test-lists`): markers no test
//! counts, tests a removal leaves with nothing to cite, and tests a tree has lost; and the
//! tests that cite nothing, for `clm health`. They take files as paths and texts, so they
//! build without the `checkout` feature.

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

/// A test of the base tree that is gone from the head tree (`deleted_tests`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletedTest {
    /// `<file>::<name>`.
    pub test: String,
    /// The test's first line in the base tree.
    pub line: usize,
    /// The IDs its markers cited, sorted.
    pub ids: Vec<String>,
}

/// A test (never a group) with no marker on its lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UncitedTest {
    /// `<file>::<name>`.
    pub test: String,
    /// The test's first line.
    pub line: usize,
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
    name: String,
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
            name: t.name,
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

/// A test's name with every `req:` marker in it left out, ID and version, and the space
/// before each, so a test whose title carries its marker keeps its name when re-cited.
pub fn bare_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut from = 0;
    for m in parse_markers(name) {
        let kept = &name[from..m.range.start];
        out.push_str(kept.strip_suffix(' ').unwrap_or(kept));
        from = m.range.end;
    }
    out.push_str(&name[from..]);
    out
}

/// The tests of `base_files` that are gone from `head_files`, sorted by file and then by
/// line. Tests holding a marker are counted by bare name: where the head holds `m` of a
/// name the base holds `n` of, `m < n`, `n - m` are gone, taken from the files in sorted
/// order, each giving its tests of that name, last first, as many as it holds fewer of at
/// the head. So a test moved to another file is not gone; a renamed, deleted or uncited
/// one is, since a program can't tell a rename from a removal.
pub fn deleted_tests(
    base_files: &[(PathBuf, String)],
    head_files: &[(PathBuf, String)],
) -> Vec<DeletedTest> {
    let mut head_total: BTreeMap<String, usize> = BTreeMap::new();
    let mut head_in: BTreeMap<(&Path, String), usize> = BTreeMap::new();
    for (path, text) in head_files {
        for c in citing_tests(path, text) {
            let name = bare_name(&c.name);
            *head_total.entry(name.clone()).or_default() += 1;
            *head_in.entry((path.as_path(), name)).or_default() += 1;
        }
    }
    // By bare name, then by file in sorted order, each file's tests in line order.
    let mut base: BTreeMap<String, BTreeMap<&Path, Vec<Cited>>> = BTreeMap::new();
    for (path, text) in base_files {
        for c in citing_tests(path, text) {
            base.entry(bare_name(&c.name))
                .or_default()
                .entry(path.as_path())
                .or_default()
                .push(c);
        }
    }
    let mut out: Vec<(&Path, DeletedTest)> = Vec::new();
    for (name, by_file) in base {
        let n: usize = by_file.values().map(Vec::len).sum();
        let mut left = n.saturating_sub(head_total.get(&name).copied().unwrap_or(0));
        for (path, tests) in by_file {
            if left == 0 {
                break;
            }
            let held = head_in.get(&(path, name.clone())).copied().unwrap_or(0);
            let take = tests.len().saturating_sub(held).min(left);
            left -= take;
            for c in tests.into_iter().rev().take(take) {
                let test = DeletedTest {
                    test: c.test,
                    line: c.line,
                    ids: sorted_unique(c.ids),
                };
                out.push((path, test));
            }
        }
    }
    out.sort_by(|a, b| (a.0, a.1.line).cmp(&(b.0, b.1.line)));
    out.into_iter().map(|(_, t)| t).collect()
}

/// The tests of one file, groups left out, that hold a marker.
fn citing_tests(path: &Path, text: &str) -> impl Iterator<Item = Cited> {
    cited_tests(path, text)
        .into_iter()
        .filter(|c| !c.ids.is_empty())
}

/// Each test, groups left out, with no marker on its lines, sorted by file and then by line.
pub fn uncited_tests(files: &[(PathBuf, String)]) -> Vec<UncitedTest> {
    let mut out: Vec<(&Path, UncitedTest)> = Vec::new();
    for (path, text) in files {
        for c in cited_tests(path, text) {
            if c.ids.is_empty() {
                let test = UncitedTest {
                    test: c.test,
                    line: c.line,
                };
                out.push((path, test));
            }
        }
    }
    out.sort_by(|a, b| (a.0, a.1.line).cmp(&(b.0, b.1.line)));
    out.into_iter().map(|(_, t)| t).collect()
}
