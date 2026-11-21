//! The lists `clm coverage` prints beside its report (`test-lists`): markers no test
//! counts, tests a removal leaves with nothing to cite, and tests a tree has lost; and the
//! tests that cite nothing, for `clm health`. They take files as paths and texts, so they
//! build without the `checkout` feature.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use clm_model::{Graph, Layer};

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
/// removed, and that a marker in `outside` cites, sorted by ID; never an architecture entry,
/// which no test cites (`check/dangling-markers`).
pub fn uncounted(graph: &Graph, outside: &[Marker]) -> Vec<Uncounted> {
    let removed = removed(graph);
    let mut by_id: BTreeMap<&str, Vec<Marker>> = BTreeMap::new();
    for m in outside {
        if Layer::of(&m.id) == Some(Layer::Architecture) {
            continue;
        }
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
/// line. Tests holding a marker are counted by bare name. A file's *gained* tests of a name
/// are the unmarked tests of it the file lost, at most as many as it holds cited at the head:
/// those may be unmarked tests that gained a marker, and stand for no cited base test. Where
/// the head holds `m` cited tests of a name, less the gained ones, and the base `n`, `m < n`,
/// `n - m` are gone, taken from the files in sorted order, each giving its tests of that name,
/// last first, as many as its cited ones at the head, less its gained ones, fall short. A test
/// so taken *fits* a citing test of its file at the head whose bare name no test of the base
/// carries, cited or not, and whose markers cite every ID it cited. Taken in the order
/// returned, each is *renamed*, and not returned, when it and every earlier renamed one can
/// each be paired with a different test it fits, so as many are renamed as can be and the
/// earlier wins a tie. A base test with no marker that its file's head doesn't account for by
/// name (its gained tests, and its unmarked ones less those that may be cited tests *stripped*
/// of their marker) could have become any new test of its file, so it takes one first: a file's renames number no
/// more than its new tests, cited or not, less those gone unmarked tests; and its renames of
/// one name no more than the tests of that name, cited or not, it holds fewer of at the head,
/// since a name a file still carries as often was not renamed. So a test moved to another file
/// is not gone, unless that file's gained tests take its place, nor one renamed in its file
/// that cites at least what it cited; one renamed and moved at once, renamed citing less,
/// replaced by a base test that gains a marker, deleted or uncited is.
pub fn deleted_tests(
    base_files: &[(PathBuf, String)],
    head_files: &[(PathBuf, String)],
) -> Vec<DeletedTest> {
    // The citing tests by bare name, then by file in sorted order, each file's tests in line
    // order; every bare name a base test carries, cited or not, which no rename takes; and the
    // count of each file's tests of each bare name, cited and not.
    let mut base: BTreeMap<String, BTreeMap<&Path, Vec<FileTest>>> = BTreeMap::new();
    let mut base_names: BTreeSet<String> = BTreeSet::new();
    let mut base_count: BTreeMap<(&Path, String), Count> = BTreeMap::new();
    for (path, text) in base_files {
        let path = path.as_path();
        for c in file_tests(path, text) {
            let name = bare_name(&c.name);
            base_names.insert(name.clone());
            base_count.entry((path, name.clone())).or_default().add(&c);
            if !c.ids.is_empty() {
                base.entry(name)
                    .or_default()
                    .entry(path)
                    .or_default()
                    .push(c);
            }
        }
    }
    // The same counts at the head; the head's citing tests of a new name, by file in line
    // order, each with its IDs; and each file's new tests, cited or not.
    let mut head_count: BTreeMap<(&Path, String), Count> = BTreeMap::new();
    let mut fresh: BTreeMap<&Path, Vec<BTreeSet<String>>> = BTreeMap::new();
    let mut room: BTreeMap<&Path, usize> = BTreeMap::new();
    for (path, text) in head_files {
        let path = path.as_path();
        for c in file_tests(path, text) {
            let name = bare_name(&c.name);
            head_count.entry((path, name.clone())).or_default().add(&c);
            if base_names.contains(&name) {
                continue;
            }
            *room.entry(path).or_default() += 1;
            if !c.ids.is_empty() {
                fresh
                    .entry(path)
                    .or_default()
                    .push(c.ids.into_iter().collect());
            }
        }
    }
    let none = Count::default();
    // How many cited tests of each bare name each head file holds, less its gained ones.
    let mut held: BTreeMap<&str, BTreeMap<&Path, usize>> = BTreeMap::new();
    for ((path, name), h) in &head_count {
        let b = base_count.get(&(*path, name.clone())).unwrap_or(&none);
        let gained = b.gained(h);
        held.entry(name.as_str())
            .or_default()
            .insert(*path, h.marked - gained);
    }
    // Each file's room for renames is its new tests less its gone unmarked ones, which take
    // new tests first; and its room for renames of one name what it lost of that name.
    let mut lost: BTreeMap<(&Path, String), usize> = BTreeMap::new();
    for ((path, name), b) in &base_count {
        let h = head_count.get(&(*path, name.clone())).unwrap_or(&none);
        if let Some(r) = room.get_mut(path) {
            *r = r.saturating_sub(b.unmarked.saturating_sub(b.accounted(h)));
        }
        lost.insert((*path, name.clone()), b.total().saturating_sub(h.total()));
    }
    let no_files = BTreeMap::new();
    let mut out = Vec::new();
    for (name, by_file) in base {
        let held = held.get(name.as_str()).unwrap_or(&no_files);
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
                out.push((path, c.line, (path, name.clone(), test)));
            }
        }
    }
    let gone = by_file_and_line(out);
    // For each taken test in order, the indexes of the fresh tests of its file it fits.
    let fits: Vec<Vec<usize>> = gone
        .iter()
        .map(|(path, _, test)| {
            fresh.get(path).map_or_else(Vec::new, |new| {
                (0..new.len())
                    .filter(|&i| test.ids.iter().all(|id| new[i].contains(id)))
                    .collect()
            })
        })
        .collect();
    // Which taken test each fresh test stands for, by file; a taken test is renamed when an
    // augmenting path pairs it without unpairing an earlier one (`pair`).
    let mut holders: BTreeMap<&Path, Vec<Option<usize>>> = fresh
        .iter()
        .map(|(path, new)| (*path, vec![None; new.len()]))
        .collect();
    let mut out = Vec::new();
    for (k, (path, name, test)) in gone.into_iter().enumerate() {
        let left = room.get_mut(path).filter(|r| **r > 0);
        let of_name = lost.get_mut(&(path, name)).filter(|r| **r > 0);
        let renamed = left.zip(of_name).is_some_and(|(r, s)| {
            holders.get_mut(path).is_some_and(|holders| {
                let mut seen = vec![false; holders.len()];
                let paired = pair(k, &fits, holders, &mut seen);
                *r -= usize::from(paired);
                *s -= usize::from(paired);
                paired
            })
        });
        if !renamed {
            out.push(test);
        }
    }
    out
}

/// How many tests of one bare name one file holds, with a marker and without.
#[derive(Default)]
struct Count {
    marked: usize,
    unmarked: usize,
}

impl Count {
    fn add(&mut self, c: &FileTest) {
        if c.ids.is_empty() {
            self.unmarked += 1;
        } else {
            self.marked += 1;
        }
    }

    fn total(&self) -> usize {
        self.marked + self.unmarked
    }

    /// The *gained* tests of the head's count `h` against this base count: the unmarked
    /// tests lost, at most the cited ones the head holds, which may have gained a marker.
    fn gained(&self, h: &Count) -> usize {
        self.unmarked.saturating_sub(h.unmarked).min(h.marked)
    }

    /// How many of this base count's unmarked tests the head's count `h` accounts for: its
    /// gained tests, and its unmarked tests less its *stripped* ones, the cited tests lost, at
    /// most the unmarked ones the head holds, which may have lost their marker.
    fn accounted(&self, h: &Count) -> usize {
        let stripped = self.marked.saturating_sub(h.marked).min(h.unmarked);
        self.gained(h) + (h.unmarked - stripped)
    }
}

/// Pair taken test `k` with a fresh test it fits, moving an earlier pairing to another test
/// it fits when that frees one (Kuhn's augmenting path). Every taken test `holders` names is
/// of the same file as `k`, so `fits` indexes the same fresh tests for each.
fn pair(k: usize, fits: &[Vec<usize>], holders: &mut [Option<usize>], seen: &mut [bool]) -> bool {
    for &i in &fits[k] {
        if seen[i] {
            continue;
        }
        seen[i] = true;
        if holders[i].is_none_or(|j| pair(j, fits, holders, seen)) {
            holders[i] = Some(k);
            return true;
        }
    }
    false
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
