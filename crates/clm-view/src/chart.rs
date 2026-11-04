//! Box-drawing charts: lines of styled spans, cut to a width, printed plain or with ANSI
//! colour. Every state is written as a word or a symbol, so a chart reads without colour.

use std::fmt::Write as _;

/// The web app's colour meanings: green for active, converged, current or complete; sky
/// for unbuilt or ready; amber for claimed or behind; rose for blocked, obsolete or
/// deprecated; indigo for headings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Style {
    Plain,
    Green,
    Sky,
    Amber,
    Rose,
    Indigo,
}

impl Style {
    /// The colour of a state word, `Plain` for anything else.
    pub(crate) fn of(word: &str) -> Style {
        match word {
            "active" | "converged" | "current" | "complete" => Style::Green,
            "specd" | "unbuilt" | "ready" => Style::Sky,
            "claimed" | "behind" => Style::Amber,
            "blocked" | "obsolete" | "deprecated" => Style::Rose,
            _ => Style::Plain,
        }
    }

    fn sgr(self) -> &'static str {
        match self {
            Style::Plain => "",
            Style::Green => "\x1b[32m",
            Style::Sky => "\x1b[36m",
            Style::Amber => "\x1b[33m",
            Style::Rose => "\x1b[31m",
            Style::Indigo => "\x1b[34m",
        }
    }

    /// The CSS class the SVG renderer gives a span.
    pub(crate) fn class(self) -> &'static str {
        match self {
            Style::Plain => "",
            Style::Green => "green",
            Style::Sky => "sky",
            Style::Amber => "amber",
            Style::Rose => "rose",
            Style::Indigo => "indigo",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Span {
    pub text: String,
    pub style: Style,
}

pub(crate) type Line = Vec<Span>;
/// One cell of a table row.
pub(crate) type Cell = Vec<Span>;

pub(crate) fn span(text: impl Into<String>, style: Style) -> Span {
    Span {
        text: text.into(),
        style,
    }
}

pub(crate) fn plain(text: impl Into<String>) -> Span {
    span(text, Style::Plain)
}

/// A state word in its colour.
pub(crate) fn word(state: &str) -> Span {
    span(state, Style::of(state))
}

pub(crate) fn heading(text: &str) -> Line {
    vec![span(text, Style::Indigo)]
}

pub(crate) fn width(spans: &[Span]) -> usize {
    spans.iter().map(|s| s.text.chars().count()).sum()
}

/// `line` cut to `width` columns, its last column `…` when anything was cut.
pub(crate) fn fit(line: &[Span], width: usize) -> Line {
    if width == 0 || self::width(line) <= width {
        return line.to_vec();
    }
    let mut out = Line::new();
    let mut left = width - 1;
    for s in line {
        let n = s.text.chars().count();
        if n <= left {
            out.push(s.clone());
            left -= n;
        } else {
            out.push(span(s.text.chars().take(left).collect::<String>(), s.style));
            break;
        }
    }
    out.push(plain("…"));
    out
}

/// `rows` as lines: each column padded to its widest cell, cells two spaces apart, no
/// trailing spaces; a column that is empty on every row is left out.
pub(crate) fn align(rows: Vec<Vec<Cell>>) -> Vec<Line> {
    let mut widths: Vec<usize> = Vec::new();
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            if widths.len() <= i {
                widths.push(0);
            }
            widths[i] = widths[i].max(width(cell));
        }
    }
    rows.into_iter()
        .map(|row| {
            let cells: Vec<(usize, Cell)> = row
                .into_iter()
                .enumerate()
                .filter(|(i, _)| widths[*i] > 0)
                .collect();
            let last = cells.iter().rposition(|(_, c)| width(c) > 0).unwrap_or(0);
            let mut line = Line::new();
            for (n, (i, cell)) in cells.into_iter().enumerate().take(last + 1) {
                let w = width(&cell);
                line.extend(cell);
                if n < last {
                    line.push(plain(" ".repeat(widths[i] - w + 2)));
                }
            }
            line
        })
        .collect()
}

/// `├─ ` or `└─ ` after `│  ` or spaces for each ancestor: `ancestors_last[i]` says whether
/// the ancestor at depth `i` was the last of its siblings.
pub(crate) fn tree_prefix(ancestors_last: &[bool], last: bool) -> String {
    let mut s = String::new();
    for &l in ancestors_last {
        s.push_str(if l { "   " } else { "│  " });
    }
    s.push_str(if last { "└─ " } else { "├─ " });
    s
}

/// A bar of `cells` cells: `done` of `total` filled (`█`, green), the rest `░` (sky). With
/// no requirements, every cell is `·`.
pub(crate) fn ratio_bar(done: usize, total: usize, cells: usize) -> Cell {
    if total == 0 {
        return vec![plain("·".repeat(cells))];
    }
    let filled = (done * cells).div_ceil(total).min(cells);
    let filled = if done == 0 { 0 } else { filled.max(1) };
    let mut bar = Cell::new();
    if filled > 0 {
        bar.push(span("█".repeat(filled), Style::Green));
    }
    if filled < cells {
        bar.push(span("░".repeat(cells - filled), Style::Sky));
    }
    bar
}

/// A bar of `cells` cells shared in proportion among `current` (`█`, green), `behind`
/// (`▓`, amber) and `unbuilt` (`░`, sky) requirements: each non-zero count gets at least one
/// cell and the rest go by largest remainder, ties in that order. With no requirements,
/// every cell is `·`.
pub(crate) fn state_bar(current: usize, behind: usize, unbuilt: usize, cells: usize) -> Cell {
    let counts = [current, behind, unbuilt];
    let total: usize = counts.iter().sum();
    if total == 0 {
        return vec![plain("·".repeat(cells))];
    }
    let mut alloc: Vec<usize> = counts
        .iter()
        .map(|&n| {
            if n == 0 {
                0
            } else {
                (n * cells / total).max(1)
            }
        })
        .collect();
    // How far each count's share is above what it was given, in units of `1 / total` cells.
    let short = |alloc: &[usize], i: usize| (counts[i] * cells) as i64 - (alloc[i] * total) as i64;
    // The minimum of one cell can overfill the bar: take back from the most overserved. A
    // bar narrower than its non-zero counts keeps one cell for each.
    while alloc.iter().sum::<usize>() > cells {
        let Some(i) = (0..3)
            .filter(|&i| alloc[i] > 1)
            .min_by_key(|&i| short(&alloc, i))
        else {
            break;
        };
        alloc[i] -= 1;
    }
    while alloc.iter().sum::<usize>() < cells {
        // `max_by_key` keeps the last of equals; reversing makes ties go in order. Some
        // count is non-zero, since the total is.
        let Some(i) = (0..3)
            .rev()
            .filter(|&i| counts[i] > 0)
            .max_by_key(|&i| short(&alloc, i))
        else {
            break;
        };
        alloc[i] += 1;
    }
    let glyphs = [("█", Style::Green), ("▓", Style::Amber), ("░", Style::Sky)];
    glyphs
        .iter()
        .zip(alloc)
        .filter(|(_, n)| *n > 0)
        .map(|((g, style), n)| span(g.repeat(n), *style))
        .collect()
}

/// One cell per pending requirement state, at most `cap` of them then `…`: `▓` behind
/// (amber), `░` unbuilt (sky), `✕` obsolete (rose).
pub(crate) fn pending_bar<'a>(states: impl Iterator<Item = &'a str>, cap: usize) -> Cell {
    let mut bar = Cell::new();
    for (i, state) in states.enumerate() {
        if i == cap {
            bar.push(plain("…"));
            break;
        }
        bar.push(match state {
            "behind" => span("▓", Style::Amber),
            "obsolete" => span("✕", Style::Rose),
            _ => span("░", Style::Sky),
        });
    }
    bar
}

/// The lines as plain text, each cut to `width` columns.
pub(crate) fn to_text(lines: &[Line], width: usize) -> String {
    let mut out = String::new();
    for line in lines {
        for s in fit(line, width) {
            out.push_str(&s.text);
        }
        out.push('\n');
    }
    out
}

/// The lines with ANSI colour, each cut to `width` columns: the plain text with escapes
/// around the coloured spans and nothing else.
pub(crate) fn to_ansi(lines: &[Line], width: usize) -> String {
    let mut out = String::new();
    for line in lines {
        for s in fit(line, width) {
            if s.style == Style::Plain {
                out.push_str(&s.text);
            } else {
                let _ = write!(out, "{}{}\x1b[0m", s.style.sgr(), s.text);
            }
        }
        out.push('\n');
    }
    out
}

/// A `P1`..`P5` priority as a plain span.
pub(crate) fn priority(p: clm_model::Priority) -> Span {
    plain(p.to_string())
}
