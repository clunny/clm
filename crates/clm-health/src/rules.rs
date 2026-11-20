//! The signals of the rule for a requirement: `outside-contract`, `test-placement`,
//! `design-exit` and `uncited-test` (`rule-signals`). Each is a heuristic, and advisory.

use std::path::{Path, PathBuf};

use clm_model::outline::fence_mask;
use clm_model::{Graph, Requirement};

use crate::requirements::requirements;
use crate::spec::section_lines;
use crate::{Failure, Section, finding};

/// A requirement's statement: its lines above its first `####` heading, its own heading and
/// fenced lines left out.
fn statement(r: &Requirement) -> Vec<&str> {
    let lines: Vec<&str> = r.text.lines().collect();
    let mask = fence_mask(&lines);
    lines
        .iter()
        .zip(mask)
        .skip(1)
        .take_while(|(l, fenced)| *fenced || !l.starts_with("####"))
        .filter(|(_, fenced)| !fenced)
        .map(|(l, _)| *l)
        .collect()
}

/// Whether `s` is an identifier: a letter or `_`, then letters, digits or `_`.
fn identifier(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The code name a code span's text gives: the text with one trailing parenthesised list
/// taken off, when that is identifiers joined by `::` and the span held the list, or holds
/// `::`, or holds a lowercase letter directly followed by an uppercase one.
fn code_name(span: &str) -> Option<&str> {
    let (name, list) = match span.strip_suffix(')') {
        Some(inner) => {
            let mut depth = 0usize;
            let open = inner.char_indices().rev().find_map(|(i, c)| match c {
                ')' => {
                    depth += 1;
                    None
                }
                '(' if depth == 0 => Some(i),
                '(' => {
                    depth -= 1;
                    None
                }
                _ => None,
            })?;
            (&span[..open], true)
        }
        None => (span, false),
    };
    if !name.split("::").all(identifier) {
        return None;
    }
    let camel = name
        .as_bytes()
        .windows(2)
        .any(|w| w[0].is_ascii_lowercase() && w[1].is_ascii_uppercase());
    (list || name.contains("::") || camel).then_some(name)
}

/// Each distinct code name in a requirement's statement, in the order it first appears.
fn code_names(r: &Requirement) -> Vec<&str> {
    let mut names = Vec::new();
    for line in statement(r) {
        for span in line.split('`').skip(1).step_by(2) {
            if let Some(name) = code_name(span)
                && !names.contains(&name)
            {
                names.push(name);
            }
        }
    }
    names
}

/// Each code name in a requirement's statement that its component's Contract doesn't contain.
pub(crate) fn outside_contract(graph: &Graph) -> Section {
    let lines = requirements(graph)
        .flat_map(|(n, r)| {
            code_names(r)
                .into_iter()
                .filter(|name| !n.contract.contains(name))
                .map(move |name| {
                    finding(
                        n.path.display(),
                        "outside-contract",
                        format!("{} names {name}", r.id),
                    )
                })
        })
        .collect();
    Section::of("outside-contract", "names", lines)
}

/// Each requirement whose statement holds a sentence, as `gate`'s `sentences` splits it, that
/// places a test, with the first placement phrase found (`gate`'s `places_a_test`).
pub(crate) fn test_placement(graph: &Graph) -> Section {
    let lines = requirements(graph)
        .filter_map(|(n, r)| {
            let statement = statement(r).join("\n");
            let phrase = clm_gate::sentences(&statement)
                .into_iter()
                .find_map(clm_gate::places_a_test)?;
            Some(finding(
                n.path.display(),
                "test-placement",
                format!("{} {phrase}", r.id),
            ))
        })
        .collect();
    Section::of("test-placement", "requirements", lines)
}

/// Whether byte `at` of `text` borders a word: past either end, or not a letter, digit or `_`.
fn boundary(text: &str, at: usize) -> bool {
    text.as_bytes()
        .get(at)
        .is_none_or(|b| !(b.is_ascii_alphanumeric() || *b == b'_'))
}

/// The end of the first of `words` that starts at byte `at` of `text`, without regard to
/// case and ending at a word boundary.
fn word_at(text: &str, at: usize, words: &[&str]) -> Option<usize> {
    words.iter().find_map(|w| {
        let end = at + w.len();
        (text
            .as_bytes()
            .get(at..end)?
            .eq_ignore_ascii_case(w.as_bytes())
            && boundary(text, end))
        .then_some(end)
    })
}

/// The end of the whitespace, at least one character of it, that starts at byte `at`.
fn space(text: &str, at: usize) -> Option<usize> {
    let n = text[at..].len() - text[at..].trim_start().len();
    (n > 0).then_some(at + n)
}

/// The end of a run of digits, ending at a word boundary, that starts at byte `at`.
fn number(text: &str, at: usize) -> Option<usize> {
    let n = text[at..].bytes().take_while(u8::is_ascii_digit).count();
    (n > 0 && boundary(text, at + n)).then_some(at + n)
}

/// `exit`, `exits` or `exited`, then optionally `with`, then optionally `status` or `code`,
/// then a number, starting at byte `at` of `text`: the match's end.
fn exit_at(text: &str, at: usize) -> Option<usize> {
    if at > 0 && !boundary(text, at - 1) {
        return None;
    }
    let verb = word_at(text, at, &["exited", "exits", "exit"])?;
    let optional = |from: usize, words: &[&str]| {
        let mut ends = vec![from];
        if let Some(end) = space(text, from).and_then(|s| word_at(text, s, words)) {
            ends.insert(0, end);
        }
        ends
    };
    optional(verb, &["with"]).into_iter().find_map(|with| {
        optional(with, &["status", "code"])
            .into_iter()
            .find_map(|kind| number(text, space(text, kind)?))
    })
}

/// The first exit status `line` states, as written.
fn design_exit_in(line: &str) -> Option<&str> {
    (0..line.len())
        .filter(|&i| line.is_char_boundary(i))
        .find_map(|i| exit_at(line, i).map(|end| &line[i..end]))
}

/// Each line of a component's Design, outside fenced code, that states an exit status. A
/// Contract is never read.
pub(crate) fn design_exit(graph: &Graph) -> Section {
    let mut lines = Vec::new();
    for node in graph.nodes.values() {
        for (line, text) in section_lines(node, &["Design"]) {
            if let Some(exit) = design_exit_in(text) {
                let at = format!("{}:{line}", node.path.display());
                lines.push(finding(at, "design-exit", exit));
            }
        }
    }
    Section::of("design-exit", "lines", lines)
}

/// Each test with no marker in a file of the checkout (tracked, or untracked and not
/// ignored, as `check.coverage`'s scan lists them) that some component owns. `Git` when git
/// can't list the files.
pub(crate) fn uncited_test(root: &Path, graph: &Graph) -> Result<Section, Failure> {
    let args = [
        "ls-files",
        "-z",
        "--cached",
        "--others",
        "--exclude-standard",
    ];
    let out = clm_git::run(root, "git", &args, clm_git::Options::default())
        .map_err(|e| Failure::Git(e.to_string()))?;
    let mut paths: Vec<PathBuf> = out
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| PathBuf::from(String::from_utf8_lossy(s).into_owned()))
        .filter(|p| clm_coverage::owner(graph, p).is_some())
        .collect();
    paths.sort();
    paths.dedup();
    let files: Vec<(PathBuf, String)> = paths
        .into_iter()
        .filter_map(|p| {
            // Unreadable or not UTF-8: not a test source.
            let text = std::fs::read_to_string(root.join(&p)).ok()?;
            Some((p, text))
        })
        .collect();
    let lines = clm_coverage::uncited_tests(&files)
        .into_iter()
        .map(|t| {
            let (file, name) = t.test.split_once("::").unwrap_or(("", &t.test));
            finding(format!("{file}:{}", t.line), "uncited-test", name)
        })
        .collect();
    Ok(Section::of("uncited-test", "tests", lines))
}
