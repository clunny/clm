//! What a spec branch cuts (`cuts`, `removal-needs-a-yes`), read from the spec graphs at its
//! merge base and its head and from the lists of tests its changes declare, and the
//! exemptions it adds or widens (`needs-review-of`). Pure functions, so the gate and the
//! `web` Worker give one answer: they run no process and build for `wasm32-unknown-unknown`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use clm_model::{Exemption, Graph, Node, Requirement, Violation};

/// The phrases that mark a sentence as one that places a test, which a requirement may lose
/// with no separate yes (`cuts-are-what-a-program-lists`). A constant, changed by a code PR.
const PLACEMENT_PHRASES: [&str; 3] = ["SHALL sit", "sat in", "is left in"];

/// What a spec branch cuts, each printing as its `gate/cut` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cut {
    /// A requirement the base holds and the head doesn't, not every scenario of which moved.
    Requirement { spec: PathBuf, id: String },
    /// A scenario whose name its requirement no longer holds, and that didn't move.
    Scenario {
        spec: PathBuf,
        id: String,
        name: String,
    },
    /// A requirement that states fewer sentences at the head (`now`) than at the base (`was`).
    Shortened {
        spec: PathBuf,
        id: String,
        now: usize,
        was: usize,
    },
    /// A test a change's `deleted-tests.txt` gains a line to delete.
    Test {
        list: String,
        file: String,
        name: String,
    },
    /// The specs at `revision` don't load, so the cuts can't be read; `evaluate_at` adds it.
    Unloaded { revision: String },
}

impl fmt::Display for Cut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cut::Requirement { spec, id } => {
                write!(f, "{}: gate/cut: requirement {id} is gone", spec.display())
            }
            Cut::Scenario { spec, id, name } => write!(
                f,
                "{}: gate/cut: scenario \"{name}\" of {id} is gone",
                spec.display()
            ),
            Cut::Shortened { spec, id, now, was } => write!(
                f,
                "{}: gate/cut: {id} states {now} sentences where it stated {was}",
                spec.display()
            ),
            Cut::Test { list, file, name } => {
                write!(f, "{list}: gate/cut: test {file}::{name} is to be deleted")
            }
            Cut::Unloaded { revision } => write!(
                f,
                "specs: gate/cut: the specs at {revision} don't load, so what the branch cuts isn't listed"
            ),
        }
    }
}

/// A change's `deleted-tests.txt`: its path at the head, its text there, and its text at the
/// merge base (for an archived list with none of its own, the live list's), `None` where
/// there is none.
#[derive(Debug, Clone, Copy)]
pub struct TestList<'a> {
    pub path: &'a str,
    pub head: &'a str,
    pub base: Option<&'a str>,
}

/// A line `DELETE <file>::<name>` of a list, the name ending at a tab or the line's end.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ListLine {
    pub file: String,
    pub name: String,
}

/// One line of a list, read.
enum Line<'a> {
    /// Blank, or starting `#`.
    Skipped,
    Delete(ListLine),
    /// Neither skipped nor a list line.
    Other(&'a str),
}

fn read_line(line: &str) -> Line<'_> {
    let line = line.strip_suffix('\r').unwrap_or(line);
    if line.trim().is_empty() || line.starts_with('#') {
        return Line::Skipped;
    }
    let parsed = line.strip_prefix("DELETE ").and_then(|rest| {
        let (file, name) = rest.split_once("::")?;
        let name = name.split('\t').next().unwrap_or_default();
        (!file.is_empty() && !name.is_empty()).then(|| ListLine {
            file: file.to_string(),
            name: name.to_string(),
        })
    });
    parsed.map_or(Line::Other(line), Line::Delete)
}

/// Each list line of a `deleted-tests.txt`, in order.
pub fn list_lines(text: &str) -> Vec<ListLine> {
    text.lines()
        .filter_map(|l| match read_line(l) {
            Line::Delete(d) => Some(d),
            _ => None,
        })
        .collect()
}

/// What the head's text of a list adds to the base's, in the file's order: each new list
/// line as `Ok`, and each new line that is neither skipped nor a list line as `Err`.
pub(crate) fn added_lines<'a>(list: &TestList<'a>) -> Vec<Result<ListLine, &'a str>> {
    let base = list.base.unwrap_or_default();
    let had: BTreeSet<ListLine> = list_lines(base).into_iter().collect();
    let had_other: BTreeSet<&str> = base
        .lines()
        .filter_map(|l| match read_line(l) {
            Line::Other(o) => Some(o),
            _ => None,
        })
        .collect();
    let mut added = Vec::new();
    for line in list.head.lines() {
        match read_line(line) {
            Line::Delete(d) if !had.contains(&d) && !added.contains(&Ok(d.clone())) => {
                added.push(Ok(d))
            }
            Line::Other(o) if !had_other.contains(o) && !added.contains(&Err(o)) => {
                added.push(Err(o))
            }
            _ => {}
        }
    }
    added
}

/// Whether `text`, the file at `file`, holds a test named `name` (compared by
/// `check.coverage`'s `bare_name`) with a marker on its lines.
pub(crate) fn names_a_test(file: &Path, text: &str, name: &str) -> bool {
    let name = clm_coverage::bare_name(name);
    let markers = clm_coverage::markers_in(file, text);
    clm_coverage::tests_in(file, text).iter().any(|t| {
        !t.group
            && clm_coverage::bare_name(&t.name) == name
            && markers.iter().any(|m| (t.first..=t.last).contains(&m.line))
    })
}

/// Whether `word` occurs in `text` with no letter, digit or `_` on either side.
fn holds_word(text: &str, word: &str) -> Option<usize> {
    let inner = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(word).map(|(at, _)| at).find(|&at| {
        let before = text[..at].chars().next_back();
        let after = text[at + word.len()..].chars().next();
        !before.is_some_and(inner) && !after.is_some_and(inner)
    })
}

/// The first placement phrase `sentence` holds, compared without regard to case.
pub fn places_a_test(sentence: &str) -> Option<&'static str> {
    let lower = sentence.to_lowercase();
    PLACEMENT_PHRASES
        .iter()
        .filter_map(|p| holds_word(&lower, &p.to_lowercase()).map(|at| (at, *p)))
        .min()
        .map(|(_, p)| p)
}

/// The sentences of `statement` that state something: each holds the word `SHALL` and no
/// placement phrase. A sentence ends at a `.`, `?` or `!` that whitespace or the end follows.
pub fn stating_sentences(statement: &str) -> Vec<&str> {
    let mut sentences = Vec::new();
    let mut start = 0;
    for (at, c) in statement.char_indices() {
        let end = at + c.len_utf8();
        if matches!(c, '.' | '?' | '!')
            && statement[end..]
                .chars()
                .next()
                .is_none_or(char::is_whitespace)
        {
            sentences.push(statement[start..end].trim());
            start = end;
        }
    }
    sentences.push(statement[start..].trim());
    sentences
        .into_iter()
        .filter(|s| holds_word(s, "SHALL").is_some() && places_a_test(s).is_none())
        .collect()
}

/// A requirement's statement, its text above its first `####` heading, and the names of its
/// scenarios in order, headings inside a fence left out.
fn outline(r: &Requirement) -> (String, Vec<String>) {
    let lines: Vec<&str> = r.text.lines().collect();
    let mask = clm_model::outline::fence_mask(&lines);
    let heading = |i: usize| !mask[i] && lines[i].starts_with("####");
    let first = (1..lines.len())
        .find(|&i| heading(i))
        .unwrap_or(lines.len());
    let statement = lines[1.min(first)..first].join("\n").trim().to_string();
    let scenarios = (first..lines.len())
        .filter(|&i| heading(i))
        .filter_map(|i| lines[i].strip_prefix("#### Scenario: "))
        .map(|name| name.trim_end().to_string())
        .collect();
    (statement, scenarios)
}

/// Every component requirement of `graph`, with the spec it sits in, by ID.
fn requirements(graph: &Graph) -> BTreeMap<&str, (&Path, &Requirement)> {
    graph
        .nodes
        .values()
        .flat_map(|n| {
            n.requirements
                .iter()
                .map(move |r| (r.id.as_str(), (n.path.as_path(), r)))
        })
        .collect()
}

/// What a branch cuts, from the spec graphs at its merge base and its head and the lists of
/// tests its changes declare: gone requirements, then gone scenarios, then shortened
/// requirements, each by requirement ID, then the tests the lists add, by list path and then
/// in the list's order (`cuts`).
pub fn cuts(base: &Graph, head: &Graph, lists: &[TestList]) -> Vec<Cut> {
    let (was, now) = (requirements(base), requirements(head));
    let outlines =
        |reqs: &BTreeMap<&str, (&Path, &Requirement)>| -> BTreeMap<String, (String, Vec<String>)> {
            reqs.iter()
                .map(|(id, (_, r))| (id.to_string(), outline(r)))
                .collect()
        };
    let (was_outline, now_outline) = (outlines(&was), outlines(&now));
    let held = |outline: &BTreeMap<String, (String, Vec<String>)>, id: &str, name: &str| {
        outline
            .get(id)
            .is_some_and(|(_, names)| names.iter().any(|n| n == name))
    };

    // Each scenario of the head that a gone scenario may move to: under a requirement that
    // held no scenario of its name at the base.
    let mut receivers: BTreeMap<&str, usize> = BTreeMap::new();
    for (id, (_, names)) in &now_outline {
        for name in names {
            if !held(&was_outline, id, name) {
                *receivers.entry(name.as_str()).or_default() += 1;
            }
        }
    }

    // Every gone scenario, by requirement ID and place, takes one receiver of its name.
    let mut unmoved: BTreeSet<(&str, usize)> = BTreeSet::new();
    for (id, (_, names)) in &was_outline {
        for (place, name) in names.iter().enumerate() {
            // Still held where it was: a requirement gone from the head holds none.
            if held(&now_outline, id, name) {
                continue;
            }
            match receivers.get_mut(name.as_str()).filter(|n| **n > 0) {
                Some(n) => *n -= 1,
                None => {
                    unmoved.insert((id.as_str(), place));
                }
            }
        }
    }

    let mut gone = Vec::new();
    let mut scenarios = Vec::new();
    let mut shortened = Vec::new();
    for (id, (spec, _)) in &was {
        let (statement, names) = &was_outline[*id];
        let Some((now_spec, _)) = now.get(id) else {
            // The model refuses a requirement with no scenario, so one always has some.
            if unmoved.iter().any(|(u, _)| u == id) {
                gone.push(Cut::Requirement {
                    spec: spec.to_path_buf(),
                    id: id.to_string(),
                });
            }
            continue;
        };
        for (place, name) in names.iter().enumerate() {
            if unmoved.contains(&(*id, place)) {
                scenarios.push(Cut::Scenario {
                    spec: now_spec.to_path_buf(),
                    id: id.to_string(),
                    name: name.clone(),
                });
            }
        }
        let was_n = stating_sentences(statement).len();
        let now_n = stating_sentences(&now_outline[*id].0).len();
        if now_n < was_n {
            shortened.push(Cut::Shortened {
                spec: now_spec.to_path_buf(),
                id: id.to_string(),
                now: now_n,
                was: was_n,
            });
        }
    }

    let mut lists: Vec<&TestList> = lists.iter().collect();
    lists.sort_by_key(|l| l.path);
    let tests = lists.into_iter().flat_map(|list| {
        added_lines(list).into_iter().flatten().map(|l| Cut::Test {
            list: list.path.to_string(),
            file: l.file,
            name: l.name,
        })
    });

    gone.into_iter()
        .chain(scenarios)
        .chain(shortened)
        .chain(tests)
        .collect()
}

/// One side of a branch, its merge base or its head: the spec graph loaded there, the
/// violations its loading gave, and the revision as `clm gate` prints it.
#[derive(Debug, Clone, Copy)]
pub struct Side<'a> {
    pub graph: &'a Graph,
    pub violations: &'a [Violation],
    pub revision: &'a str,
}

/// What `clm gate` lists of a branch's cuts (`cuts`): when a side's graph loaded with any
/// violation, the one cut that says the specs there don't load, naming the merge base when
/// both fail, then the tests the lists add, which need no graph; otherwise `cuts`' result.
pub fn cuts_or_unloaded(base: &Side, head: &Side, lists: &[TestList]) -> Vec<Cut> {
    let Some(unloaded) = [base, head].into_iter().find(|s| !s.violations.is_empty()) else {
        return cuts(base.graph, head.graph, lists);
    };
    let none = Graph::default();
    std::iter::once(Cut::Unloaded {
        revision: unloaded.revision.to_string(),
    })
    .chain(cuts(&none, &none, lists))
    .collect()
}

/// Each exemption a component of `head` holds whose `files` aren't all among the `files` of
/// an exemption with the same `rule` and `until` that the component of that ID holds in
/// `base`: added or widened, as `needs-review` counts them (`temporary-exemptions`).
pub fn widened_exemptions<'a>(base: &Graph, head: &'a Graph) -> Vec<(&'a Node, &'a Exemption)> {
    let mut out = Vec::new();
    for node in head.nodes.values() {
        let before = base
            .nodes
            .get(&node.id)
            .map_or(&[][..], |n| n.frontmatter.exemptions.as_slice());
        for e in &node.frontmatter.exemptions {
            let covered = before.iter().any(|b| {
                b.rule == e.rule
                    && b.until == e.until
                    && e.files.iter().all(|f| b.files.contains(f))
            });
            if !covered {
                out.push((node, e));
            }
        }
    }
    out
}
