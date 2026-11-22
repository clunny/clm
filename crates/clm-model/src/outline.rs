//! Line-indexed outline of a markdown body: `##` sections and `### Requirement:` blocks
//! (or the human layers' entry blocks), ignoring headings inside fenced code. Shared by
//! parsing and by in-place editing.

use std::ops::Range;

/// A `## <name>` section. Its body is lines `heading + 1..end`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionSpan {
    pub name: String,
    pub heading: usize,
    pub end: usize,
}

/// A `### Requirement: <slug> [<!-- base: <hash> -->]` block covering `lines`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementSpan {
    pub slug: String,
    pub base: Option<String>,
    pub lines: Range<usize>,
}

/// `true` for each line that opens or closes a fenced code block or lies inside one, read
/// as CommonMark reads fences (`code-fences`): a run of three or more backticks or tildes
/// after leading white space opens one, unless the run is of backticks and another
/// backtick follows it; the first later line that, trimmed, is only a run of the same
/// character at least as long closes it; an unclosed fence runs to the end.
pub fn fence_mask(lines: &[&str]) -> Vec<bool> {
    let mut open: Option<(char, usize)> = None;
    lines
        .iter()
        .map(|line| match open {
            Some(fence) => {
                if closes(line, fence) {
                    open = None;
                }
                true
            }
            None => {
                open = fence_opener(line);
                open.is_some()
            }
        })
        .collect()
}

/// Whether `line` closes `fence`: trimmed, it is only a run of the fence's character at
/// least as long as the fence's opening run.
fn closes(line: &str, (c, len): (char, usize)) -> bool {
    let t = line.trim();
    t.len() >= len && t.chars().all(|x| x == c)
}

/// The fence `line` opens, as (character, run length).
fn fence_opener(line: &str) -> Option<(char, usize)> {
    let t = line.trim_start();
    let c = t.chars().next().filter(|&c| c == '`' || c == '~')?;
    let len = t.len() - t.trim_start_matches(c).len();
    (len >= 3 && !(c == '`' && t[len..].contains('`'))).then_some((c, len))
}

pub fn section_spans(lines: &[&str]) -> Vec<SectionSpan> {
    let mask = fence_mask(lines);
    let headings: Vec<(usize, &str)> = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| !mask[*i])
        .filter_map(|(i, l)| l.strip_prefix("## ").map(|n| (i, n.trim())))
        .collect();
    headings
        .iter()
        .enumerate()
        .map(|(k, (i, name))| SectionSpan {
            name: (*name).into(),
            heading: *i,
            end: headings.get(k + 1).map_or(lines.len(), |(j, _)| *j),
        })
        .collect()
}

/// Requirement blocks within `within`. Each runs to the next requirement heading or `within.end`.
pub fn requirement_spans(lines: &[&str], within: Range<usize>) -> Vec<RequirementSpan> {
    entry_spans(lines, within, REQUIREMENT)
}

const REQUIREMENT: &str = "### Requirement:";

/// Blocks within `within` whose headings start with `prefix`, such as `### Requirement:`
/// or `### Capability:`. Each runs to the next such heading or `within.end`.
pub(crate) fn entry_spans(
    lines: &[&str],
    within: Range<usize>,
    prefix: &str,
) -> Vec<RequirementSpan> {
    let mask = fence_mask(lines);
    let starts: Vec<(usize, String, Option<String>)> = within
        .clone()
        .filter(|i| !mask[*i])
        .filter_map(|i| entry_heading(lines[i], prefix).map(|(s, b)| (i, s, b)))
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(k, (i, slug, base))| RequirementSpan {
            slug: slug.clone(),
            base: base.clone(),
            lines: *i..starts.get(k + 1).map_or(within.end, |(j, _, _)| *j),
        })
        .collect()
}

/// Parses `### Requirement: <slug> [<!-- base: <hash> -->]`.
pub fn requirement_heading(line: &str) -> Option<(String, Option<String>)> {
    entry_heading(line, REQUIREMENT)
}

/// Parses `<prefix> <slug> [<!-- base: <hash> -->]`.
pub(crate) fn entry_heading(line: &str, prefix: &str) -> Option<(String, Option<String>)> {
    let (slug, base) = with_base(line.strip_prefix(prefix)?);
    Some((slug.into(), base))
}

/// Parses a `## REMOVED Assets` entry, `` - `<path>` [<!-- base: <hash> -->] ``, whose
/// path stays inside `assets/`: relative, `/`-separated, with no empty, `.` or `..` part.
pub(crate) fn asset_entry(line: &str) -> Option<(String, Option<String>)> {
    let (item, base) = with_base(line.strip_prefix("- ")?);
    let path = item.strip_prefix('`')?.strip_suffix('`')?;
    let inside = path.split('/').all(|p| !matches!(p, "" | "." | ".."));
    inside.then(|| (path.into(), base))
}

/// Splits `<text> [<!-- base: <hash> -->]` into the trimmed text and the hash.
fn with_base(text: &str) -> (&str, Option<String>) {
    match text.split_once("<!--") {
        Some((before, comment)) => {
            let base = comment
                .trim()
                .strip_suffix("-->")
                .and_then(|c| c.trim().strip_prefix("base:"))
                .map(|h| h.trim().to_string());
            (before.trim(), base)
        }
        None => (text.trim(), None),
    }
}

/// Counts `#### Scenario:` headings (outside fences) in `range`.
pub(crate) fn scenario_count(lines: &[&str], range: Range<usize>) -> usize {
    let mask = fence_mask(lines);
    range
        .filter(|i| !mask[*i] && lines[*i].starts_with("#### Scenario:"))
        .count()
}

/// `heading` lines, such as `#### Needs` or `#### Serves` (outside fences), each with its
/// block's body: the lines after it up to the next `##`, `###` or `####` heading.
pub fn blocks_named(lines: &[&str], heading: &str) -> Vec<(usize, Range<usize>)> {
    let mask = fence_mask(lines);
    let any_heading = |i: &usize| {
        let l = lines[*i];
        !mask[*i] && l.starts_with("##") && !l.starts_with("#####")
    };
    (0..lines.len())
        .filter(|i| !mask[*i] && lines[*i].trim_end() == heading)
        .map(|i| {
            let end = (i + 1..lines.len())
                .find(any_heading)
                .unwrap_or(lines.len());
            (i, i + 1..end)
        })
        .collect()
}

/// Lines joined with a trailing newline each, as stored in element text.
pub fn join(lines: &[&str]) -> String {
    lines.iter().map(|l| format!("{l}\n")).collect()
}

/// The content of the first fence in `lines` whose info string is `yaml`, read as
/// `fence_mask` reads fences; `None` when there is none or a fence before it never closes.
pub(crate) fn yaml_fence(lines: &[&str]) -> Option<String> {
    let mut i = 0;
    while i < lines.len() {
        let Some(fence) = fence_opener(lines[i]) else {
            i += 1;
            continue;
        };
        let end = i + 1 + lines[i + 1..].iter().position(|l| closes(l, fence))?;
        if lines[i].trim_start()[fence.1..].trim() == "yaml" {
            return Some(join(&lines[i + 1..end]));
        }
        i = end + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::yaml_fence;

    // req:model/code-fences@59b932df
    #[test]
    fn yaml_fence_reads_fences_as_fence_mask_does() {
        let lines = [
            "````", "```yaml", "shown: 1", "```", "````", "~~~yaml", "a: 1", "```", "~~~~", "after",
        ];
        assert_eq!(yaml_fence(&lines).as_deref(), Some("a: 1\n```\n"));
    }
}
