//! The text edits a rename makes: references and terms (see specs/rename/SPEC.md,
//! `references` and `term-swap`).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

/// What a rename changes, as the edits below read it.
pub(crate) struct Edits {
    /// Every renamed ID, component or requirement, to its new one.
    ids: BTreeMap<String, String>,
    /// Each renamed requirement's slug to its new one, when no other requirement, entry or
    /// decision has the slug, so a bare slug in a code span names that requirement alone.
    slugs: BTreeMap<String, String>,
    terms: Vec<(String, String)>,
    /// How many times each term was swapped.
    hits: RefCell<Vec<usize>>,
    /// The old IDs and slugs the references rewritten so far replaced, which `rewritten`
    /// collects.
    seen: RefCell<BTreeSet<String>>,
}

impl Edits {
    pub(crate) fn new(
        ids: BTreeMap<String, String>,
        slugs: BTreeMap<String, String>,
        terms: Vec<(String, String)>,
    ) -> Self {
        let hits = RefCell::new(vec![0; terms.len()]);
        Edits {
            ids,
            slugs,
            terms,
            hits,
            seen: RefCell::default(),
        }
    }

    /// The terms that no text swapped.
    pub(crate) fn unswapped(&self) -> Vec<&str> {
        let hits = self.hits.borrow();
        let unused = |(_, n): &(&(String, String), &usize)| **n == 0;
        let pairs = self.terms.iter().zip(hits.iter());
        pairs.filter(unused).map(|((t, _), _)| t.as_str()).collect()
    }

    /// The renamed ID `id`, when it is one.
    pub(crate) fn id(&self, id: &str) -> Option<&str> {
        self.ids.get(id).map(String::as_str)
    }

    /// The old IDs, old slugs and old terms, for the leftovers a rename reports.
    pub(crate) fn olds(&self) -> impl Iterator<Item = (&str, Mention)> {
        let ids = self.ids.keys().map(|id| (id.as_str(), Mention::Id));
        let slugs = self.slugs.keys().map(|s| (s.as_str(), Mention::Slug));
        let terms = self
            .terms
            .iter()
            .map(|(old, _)| (old.as_str(), Mention::Term));
        ids.chain(slugs).chain(terms)
    }

    /// The old IDs and slugs that `references` rewrites in `text`, spec text as `body`
    /// reads it.
    pub(crate) fn rewritten(&self, text: &str) -> BTreeSet<String> {
        let counted = self.hits.borrow().clone();
        self.seen.borrow_mut().clear();
        self.body(text, &BTreeMap::new());
        *self.hits.borrow_mut() = counted;
        std::mem::take(&mut *self.seen.borrow_mut())
    }

    /// A frontmatter or backlog value line, `key: value`, with each ID in the value of one of
    /// `keys` renamed; any other line as it was. A recorded version (`id@8hex`) keeps its
    /// version.
    pub(crate) fn values(&self, text: &str, keys: &[&str]) -> String {
        text.split_inclusive('\n')
            .map(|line| match line.split_once(':') {
                Some((key, value)) if keys.contains(&key) => {
                    format!("{key}:{}", self.tokens(value))
                }
                _ => line.to_string(),
            })
            .collect()
    }

    fn tokens(&self, value: &str) -> String {
        let id_char = |c: char| c.is_alphanumeric() || "_./-".contains(c);
        let mut out = String::new();
        let mut rest = value;
        while !rest.is_empty() {
            let end = rest.find(|c| !id_char(c)).unwrap_or(rest.len());
            if end > 0 {
                let token = &rest[..end];
                out.push_str(self.id(token).unwrap_or(token));
                rest = &rest[end..];
            } else {
                let c = rest.chars().next().unwrap_or(' ');
                out.push(c);
                rest = &rest[c.len_utf8()..];
            }
        }
        out
    }

    /// Spec text with references and terms rewritten (`references`, `term-swap`). Headings,
    /// except those in `headings`, which are replaced whole, are left alone, as are the
    /// lines of fenced code for references; the ID lines of `Needs`, `Serves` and
    /// `Enforced by` blocks are references, never swapped.
    pub(crate) fn body(&self, text: &str, headings: &BTreeMap<String, String>) -> String {
        let mut out = String::new();
        let (mut fenced, mut ids) = (false, false);
        for line in text.split_inclusive('\n') {
            let content = line.trim_end_matches(['\n', '\r']);
            let eol = &line[content.len()..];
            let marker = content.trim_start();
            if marker.starts_with("```") || marker.starts_with("~~~") {
                fenced = !fenced;
                out.push_str(line);
            } else if content.starts_with('#') {
                ids = matches!(
                    content.trim_end(),
                    "#### Needs" | "#### Serves" | "#### Enforced by"
                );
                out.push_str(headings.get(content.trim_end()).map_or(content, |h| h));
                out.push_str(eol);
            } else if ids && !fenced {
                out.push_str(&self.id_line(content));
                out.push_str(eol);
            } else {
                let spans = if fenced {
                    content.to_string()
                } else {
                    self.spans(content)
                };
                out.push_str(&self.swap(&spans));
                out.push_str(eol);
            }
        }
        out
    }

    /// A line of an ID block, `- <id>`, with its ID renamed.
    fn id_line(&self, line: &str) -> String {
        match line.trim_start().strip_prefix("- ") {
            Some(id) => {
                let indent = &line[..line.len() - line.trim_start().len()];
                let id = id.trim();
                let new = self.id(id);
                if new.is_some() {
                    self.seen.borrow_mut().insert(id.to_string());
                }
                format!("{indent}- {}", new.unwrap_or(id))
            }
            None => line.to_string(),
        }
    }

    /// `line` with each code span whose whole text is an old requirement ID, or an old slug
    /// that is unique, renamed. A component's bare ID is a word, so it stays.
    fn spans(&self, line: &str) -> String {
        map_spans(line, |span| self.reference(span))
    }

    /// The new text of a code span, noting the old one when it is a reference.
    fn reference(&self, span: &str) -> String {
        let id = self.ids.get(span).filter(|_| span.contains('/'));
        let new = id.or_else(|| self.slugs.get(span));
        if new.is_some() {
            self.seen.borrow_mut().insert(span.to_string());
        }
        new.map_or(span, |n| n).to_string()
    }

    /// `line` with each term swapped where its neighbours don't continue an ID or a path.
    fn swap(&self, line: &str) -> String {
        let mut line = line.to_string();
        for (n, (old, new)) in self.terms.iter().enumerate() {
            let mut out = String::new();
            let mut at = 0;
            let mut swapped = 0;
            for (i, _) in line.match_indices(old.as_str()) {
                if i >= at && stands_alone(&line, i, old.len(), "-_/.:") {
                    out.push_str(&line[at..i]);
                    out.push_str(new);
                    at = i + old.len();
                    swapped += 1;
                }
            }
            out.push_str(&line[at..]);
            line = out;
            self.hits.borrow_mut()[n] += swapped;
        }
        line
    }
}

/// Whether `text[at..at + len]` is a whole word: its neighbours don't continue an ID or a
/// path (`term-swap`).
fn stands_alone(text: &str, at: usize, len: usize, joiners: &str) -> bool {
    let before = text[..at].chars().next_back();
    if before.is_some_and(|c| c.is_alphanumeric() || joiners.contains(c)) {
        return false;
    }
    let mut after = text[at + len..].chars();
    match after.next() {
        None => true,
        Some('.' | ':') => after.next().is_none_or(char::is_whitespace),
        Some(c) => !(c.is_alphanumeric() || "-_/".contains(c)),
    }
}

/// What a mention names: an old ID or term as a whole word, or an old slug written as a code
/// span's whole text.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mention {
    Id,
    Slug,
    Term,
}

/// `line` with the text of each code span run through `edit`; the backticks stay.
fn map_spans(line: &str, mut edit: impl FnMut(&str) -> String) -> String {
    let run = |at: usize| line[at..].bytes().take_while(|b| *b == b'`').count();
    let mut out = String::new();
    let mut at = 0;
    while at < line.len() {
        if !line[at..].starts_with('`') {
            let c = line[at..].chars().next().unwrap_or(' ');
            out.push(c);
            at += c.len_utf8();
            continue;
        }
        let ticks = run(at);
        let mut close = None;
        let mut j = at + ticks;
        while j < line.len() {
            match run(j) {
                0 => j += line[j..].chars().next().map_or(1, char::len_utf8),
                n if n == ticks => {
                    close = Some(j);
                    break;
                }
                n => j += n,
            }
        }
        out.push_str(&line[at..at + ticks]);
        at += ticks;
        if let Some(close) = close {
            out.push_str(&edit(&line[at..close]));
            out.push_str(&line[close..close + ticks]);
            at = close + ticks;
        }
    }
    out
}

/// Whether `line` mentions `old` as `kind` counts it. An ID may follow a `:`, as in a test
/// marker `req:<id>@<version>`.
pub(crate) fn mentions(line: &str, old: &str, kind: Mention) -> bool {
    if kind == Mention::Slug {
        let mut found = false;
        map_spans(line, |span| {
            found |= span == old;
            span.to_string()
        });
        return found;
    }
    let joiners = if kind == Mention::Id { "-_/." } else { "-_/.:" };
    line.match_indices(old)
        .any(|(i, _)| stands_alone(line, i, old.len(), joiners))
}
