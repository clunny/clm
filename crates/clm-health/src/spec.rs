//! The signals of a spec that has grown past what it describes: `uncited-decision`
//! (`uncited-decisions`), `history-words`, `design-heavy` and `outlier-size`
//! (`spec-bloat-signals`).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use clm_check::Size;
use clm_model::outline::{fence_mask, section_spans};
use clm_model::{DecisionStatus, Graph, Node, split_frontmatter};

use crate::{Section, finding, phrase_at, tokens};

/// Phrases that narrate history rather than describe the system (`spec-is-current`),
/// matched case-insensitively at word boundaries.
const HISTORY_PHRASES: [&str; 8] = [
    "as before",
    "today's",
    "amended",
    "predates",
    "formerly",
    "previously",
    "was removed",
    "is gone",
];

/// The text of each inline code span in `text`, outside fenced code blocks.
fn code_spans(text: &str) -> BTreeSet<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mask = fence_mask(&lines);
    lines
        .iter()
        .zip(mask)
        .filter(|(_, fenced)| !fenced)
        .flat_map(|(line, _)| line.split('`').skip(1).step_by(2))
        .map(str::to_string)
        .collect()
}

/// The `*.md` files under `dir`, recursively; none when it doesn't exist. `skip` names a
/// directory left out.
fn markdown(dir: &Path, skip: Option<&Path>) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        // A symlinked directory isn't followed, so a link to an ancestor can't loop.
        if entry.file_type().is_ok_and(|t| t.is_dir()) {
            if skip != Some(path.as_path()) {
                out.extend(markdown(&path, skip));
            }
        } else if path.extension().is_some_and(|x| x == "md") {
            out.push(path);
        }
    }
    out
}

/// Each accepted decision no spec, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, guide page, live
/// change or other decision names in a code span.
pub(crate) fn uncited_decisions(root: &Path, graph: &Graph) -> Section {
    let read = |p: &Path| std::fs::read_to_string(p).unwrap_or_default();
    let mut sources: Vec<PathBuf> = markdown(&root.join("specs"), None)
        .into_iter()
        .filter(|p| p.file_name().is_some_and(|n| n == "SPEC.md"))
        .collect();
    sources.extend(["ARCHITECTURE.md", "CAPABILITIES.md", "CLAUDE.md"].map(|f| root.join(f)));
    sources.extend(markdown(&root.join("docs/guide"), None));
    let changes = root.join("changes");
    sources.extend(markdown(&changes, Some(&changes.join("archive"))));
    let cited: BTreeSet<String> = sources.iter().flat_map(|p| code_spans(&read(p))).collect();
    let by_decision: Vec<(&str, BTreeSet<String>)> = graph
        .decisions
        .values()
        .map(|d| (d.name.as_str(), code_spans(&read(&root.join(&d.path)))))
        .collect();
    let lines = graph
        .decisions
        .values()
        .filter(|d| d.frontmatter.status == DecisionStatus::Accepted)
        .filter(|d| {
            !cited.contains(&d.name)
                && !by_decision
                    .iter()
                    .any(|(other, spans)| *other != d.name && spans.contains(&d.name))
        })
        .map(|d| finding(d.path.display(), "uncited-decision", "nothing names it"))
        .collect();
    Section::of("uncited-decision", "decisions", lines)
}

/// The first date `20\d\d-\d\d-\d\d` or history phrase in `line`, as written.
fn history_word(line: &str) -> Option<&str> {
    (0..line.len()).find_map(|i| {
        if is_date(&line.as_bytes()[i..]) {
            return Some(&line[i..i + 10]);
        }
        phrase_at(line, i, &HISTORY_PHRASES)
    })
}

/// `20\d\d-\d\d-\d\d` at the start of `b`.
fn is_date(b: &[u8]) -> bool {
    b.len() >= 10
        && b.starts_with(b"20")
        && [2, 3, 5, 6, 8, 9].iter().all(|&i| b[i].is_ascii_digit())
        && b[4] == b'-'
        && b[7] == b'-'
}

/// The 0-based line in `node.source` where its body starts, after the frontmatter.
fn body_offset(node: &Node) -> usize {
    split_frontmatter(&node.source).map_or(0, |(_, body)| {
        node.source[..node.source.len() - body.len()]
            .lines()
            .count()
    })
}

/// Each line of a node's Contract or Design, outside fenced code, that narrates history.
pub(crate) fn history_words(graph: &Graph) -> Section {
    let mut lines = Vec::new();
    for node in graph.nodes.values() {
        let offset = body_offset(node);
        let body: Vec<&str> = node.source.lines().skip(offset).collect();
        let mask = fence_mask(&body);
        for s in section_spans(&body)
            .iter()
            .filter(|s| s.name == "Contract" || s.name == "Design")
        {
            for i in (s.heading + 1..s.end).filter(|&i| !mask[i]) {
                if let Some(word) = history_word(body[i]) {
                    let at = format!("{}:{}", node.path.display(), offset + i + 1);
                    lines.push(finding(at, "history-words", word));
                }
            }
        }
    }
    Section::of("history-words", "lines", lines)
}

/// The bytes of a node's Requirements section, as the model stores a section's text.
fn requirements_bytes(node: &Node) -> usize {
    let body: Vec<&str> = node.source.lines().skip(body_offset(node)).collect();
    section_spans(&body)
        .iter()
        .find(|s| s.name == "Requirements")
        .map_or(0, |s| {
            body[s.heading + 1..s.end].iter().map(|l| l.len() + 1).sum()
        })
}

/// Each node whose Design has more tokens than its Contract and Requirements together.
pub(crate) fn design_heavy(graph: &Graph) -> Section {
    let lines = graph
        .nodes
        .values()
        .filter(|n| !n.requirements.is_empty() || !n.contract.trim().is_empty())
        .filter_map(|n| {
            let design = tokens(n.design.len());
            let rest = tokens(n.contract.len() + requirements_bytes(n));
            (design > rest).then(|| {
                finding(
                    n.path.display(),
                    "design-heavy",
                    format!("design {design} tokens, contract and requirements {rest}"),
                )
            })
        })
        .collect();
    Section::of("design-heavy", "components", lines)
}

/// The lower middle of `values` in sorted order.
fn median(mut values: Vec<u64>) -> u64 {
    values.sort_unstable();
    values[(values.len() - 1) / 2]
}

/// Each node whose code or spec tokens per requirement exceed twice the median across the
/// nodes with requirements, one line per count; no median below three such nodes.
pub(crate) fn outlier_size(graph: &Graph, sizes: &[Size]) -> Section {
    const SIGNAL: &str = "outlier-size";
    let ratios: Vec<(&Size, u64, u64)> = sizes
        .iter()
        .filter_map(|s| {
            let n = graph.nodes.get(&s.node)?.requirements.len() as u64;
            (n > 0).then(|| (s, s.code.tokens / n, s.spec.tokens / n))
        })
        .collect();
    if ratios.len() < 3 {
        return Section::skipped(
            SIGNAL,
            "no median: fewer than 3 components with requirements".into(),
        );
    }
    let code = median(ratios.iter().map(|r| r.1).collect());
    let spec = median(ratios.iter().map(|r| r.2).collect());
    let mut nodes = 0;
    let mut lines = Vec::new();
    for (s, c, p) in &ratios {
        let over: Vec<_> = [("code", *c, code), ("spec", *p, spec)]
            .into_iter()
            .filter(|(_, r, m)| *r > 2 * m)
            .collect();
        nodes += usize::from(!over.is_empty());
        for (count, r, m) in over {
            lines.push(finding(
                graph.nodes[&s.node].path.display(),
                SIGNAL,
                format!("{count} {r} tokens per requirement, median {m}"),
            ));
        }
    }
    Section::ran(SIGNAL, nodes, "components", lines)
}
