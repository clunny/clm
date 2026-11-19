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
struct FileTest {
    name: String,
    test: String,
    line: usize,
    ids: Vec<String>,
}

/// Every test of one file, groups left out, cited or not.
fn file_tests(path: &Path, text: &str) -> Vec<FileTest> {
    let lines: Vec<&str> = text.lines().collect();
    tests_in(path, text)
        .into_iter()
        .filter(|t| !t.group)
        .map(|t| FileTest {
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

/// The tests of one file, groups left out, that hold a marker.
fn citing_tests(path: &Path, text: &str) -> impl Iterator<Item = FileTest> {
    file_tests(path, text)
        .into_iter()
        .filter(|c| !c.ids.is_empty())
}

fn sorted_unique(ids: impl IntoIterator<Item = String>) -> Vec<String> {
    ids.into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// The entries sorted by file and then by line, the order every list here returns.
fn by_file_and_line<T>(mut out: Vec<(&Path, usize, T)>) -> Vec<T> {
    out.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
    out.into_iter().map(|(_, _, t)| t).collect()
}

/// Each test holding a marker that cites a requirement or capability a change outside the
/// archive removed, sorted by file and then by line.
pub fn obsolete_tests(graph: &Graph, files: &[(PathBuf, String)]) -> Vec<ObsoleteTest> {
    let removed = removed(graph);
    let mut out = Vec::new();
    for (path, text) in files {
        for c in file_tests(path, text) {
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
            out.push((path.as_path(), c.line, test));
        }
    }
    by_file_and_line(out)
}

/// A test's name with every `req:` marker in it left out, ID and version, and the space
/// before each, so a test whose title carries its marker keeps its name when re-cited.
pub fn bare_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut from = 0;
    for m in parse_markers(name) {
        // A marker can start inside the one before it, when a slug ends at the next
        // marker's colon: that text is already left out.
        let start = m.range.start.max(from);
        let kept = &name[from..start];
        out.push_str(kept.strip_suffix(' ').unwrap_or(kept));
        from = m.range.end.max(from);
    }
    out.push_str(&name[from..]);
    out
}

/// The tests of `base_files` that are gone from `head_files`, sorted by file and then by
/// line. Tests holding a marker are counted by bare name: where the head holds `m` of a
/// name the base holds `n` of, `m < n`, `n - m` are gone, taken from the files in sorted
/// order, each giving its tests of that name, last first, as many as it holds fewer of at
/// the head. A test so taken is *renamed*, and not returned, when its file at the head holds
/// a citing test of a bare name no citing test of the base carries, whose markers cite every
/// ID it cited: taken in the order returned, each is matched with the first such test of its
/// file, in line order, not matched before. So a test moved to another file is not gone, nor
/// one renamed in its file that cites at least what it cited; one renamed and moved at once,
/// renamed citing less, deleted or uncited is.
pub fn deleted_tests(
    base_files: &[(PathBuf, String)],
    head_files: &[(PathBuf, String)],
) -> Vec<DeletedTest> {
    // How many tests of each bare name each file of the head holds.
    let mut head: BTreeMap<String, BTreeMap<&Path, usize>> = BTreeMap::new();
    for (path, text) in head_files {
        for c in citing_tests(path, text) {
            *head
                .entry(bare_name(&c.name))
                .or_default()
                .entry(path.as_path())
                .or_default() += 1;
        }
    }
    // By bare name, then by file in sorted order, each file's tests in line order.
    let mut base: BTreeMap<String, BTreeMap<&Path, Vec<FileTest>>> = BTreeMap::new();
    for (path, text) in base_files {
        for c in citing_tests(path, text) {
            base.entry(bare_name(&c.name))
                .or_default()
                .entry(path.as_path())
                .or_default()
                .push(c);
        }
    }
    let base_names: BTreeSet<String> = base.keys().cloned().collect();
    let none = BTreeMap::new();
    let mut out = Vec::new();
    for (name, by_file) in base {
        let held = head.get(&name).unwrap_or(&none);
        let n: usize = by_file.values().map(Vec::len).sum();
        let mut left = n.saturating_sub(held.values().sum());
        for (path, tests) in by_file {
            if left == 0 {
                break;
            }
            let here = held.get(path).copied().unwrap_or(0);
            let take = tests.len().saturating_sub(here).min(left);
            left -= take;
            for c in tests.into_iter().rev().take(take) {
                let test = DeletedTest {
                    test: c.test,
                    line: c.line,
                    ids: sorted_unique(c.ids),
                };
                out.push((path, c.line, (path, test)));
            }
        }
    }
    let mut gone = by_file_and_line(out);
    // The head's citing tests of a new name, by file in line order, each until matched.
    let mut fresh: BTreeMap<&Path, Vec<Option<BTreeSet<String>>>> = BTreeMap::new();
    for (path, text) in head_files {
        for c in citing_tests(path, text) {
            if !base_names.contains(&bare_name(&c.name)) {
                let ids = Some(c.ids.into_iter().collect());
                fresh.entry(path.as_path()).or_default().push(ids);
            }
        }
    }
    gone.retain(|(path, test)| {
        let Some(new) = fresh.get_mut(path) else {
            return true;
        };
        let cites_all = |ids: &BTreeSet<String>| test.ids.iter().all(|id| ids.contains(id));
        match new.iter_mut().find(|n| n.as_ref().is_some_and(cites_all)) {
            Some(n) => {
                *n = None;
                false
            }
            None => true,
        }
    });
    gone.into_iter().map(|(_, t)| t).collect()
}

/// Each test, groups left out, with no marker on its lines, sorted by file and then by line.
pub fn uncited_tests(files: &[(PathBuf, String)]) -> Vec<UncitedTest> {
    let mut out = Vec::new();
    for (path, text) in files {
        for c in file_tests(path, text) {
            if c.ids.is_empty() {
                let test = UncitedTest {
                    test: c.test,
                    line: c.line,
                };
                out.push((path.as_path(), c.line, test));
            }
        }
    }
    by_file_and_line(out)
}
