//! SVG renderers. The spec view is a layered graph, dependents above their dependencies and
//! children inside their parent's box; the other views are their charts set in monospace.
//! Every document embeds its styles, and fonts fall back to the system's.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::chart::{Line, Span, Style, plain, span, width, word};
use crate::spec::{self, SpecNode, SpecView, States, children};

const FONT: &str = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace";
/// Width of one character at the font size, in px.
const CH: f64 = 7.4;
const LINE: f64 = 18.0;
const PAD: f64 = 10.0;
const GAP: f64 = 16.0;
const HEADER: f64 = 24.0;
/// The label and the bar line under it.
const TOP: f64 = HEADER + LINE;

const STYLE: &str = "\
text{font-family:FONT;font-size:12px;fill:#1f2937}\
.green{fill:#0e9f65}.sky{fill:#0a8bd0}.amber{fill:#b7791f}.rose{fill:#dc3558}.indigo{fill:#4f46e5}\
rect.node{fill:#ffffff;stroke:#0a8bd0;stroke-width:1.2}\
rect.node.active{stroke:#0e9f65}rect.node.deprecated{stroke:#dc3558}\
rect.node.oversized{stroke:#b7791f}\
rect.node.claimed{stroke:#b7791f}\
rect.node.blocked{stroke:#dc3558}\
rect.node.ready,rect.node.claimed,rect.node.blocked{stroke-width:2}\
.none{fill:#e5e7eb}\
rect.bg{fill:#fbfbfd}\
line.edge{stroke:#6b7280;stroke-width:1;marker-end:url(#arrow)}\
";

fn style() -> String {
    STYLE.replace("FONT", FONT)
}

pub(crate) fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

fn document(width: f64, height: f64, body: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width:.0}\" height=\"{height:.0}\" \
         viewBox=\"0 0 {width:.0} {height:.0}\">\n<style>{}</style>\n<defs><marker id=\"arrow\" \
         viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" \
         orient=\"auto-start-reverse\"><path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"#6b7280\"/>\
         </marker></defs>\n<rect class=\"bg\" width=\"100%\" height=\"100%\"/>\n{body}</svg>\n",
        style()
    )
}

/// A chart's lines as rows of text.
pub(crate) fn text(lines: &[Line]) -> String {
    let cols = lines
        .iter()
        .map(|l| crate::chart::width(l))
        .max()
        .unwrap_or(0);
    let width = cols as f64 * CH + 2.0 * PAD;
    let height = lines.len() as f64 * LINE + 2.0 * PAD;
    let mut body = String::new();
    for (i, line) in lines.iter().enumerate() {
        let y = PAD + (i as f64 + 0.8) * LINE;
        let _ = writeln!(
            body,
            "<text x=\"{PAD}\" y=\"{y:.1}\" xml:space=\"preserve\">{}</text>",
            spans(line)
        );
    }
    document(width, height, &body)
}

/// A laid-out node: its size, and its children's offsets within it.
struct Box_ {
    w: f64,
    h: f64,
    children: Vec<(f64, f64, String)>,
}

/// A node's label: its ID, status, its queue state when it isn't converged, and `⚠` when
/// it is oversized.
fn label(n: &SpecNode, state: Option<&String>) -> Vec<Span> {
    let mut s = vec![plain(format!("{}  ", n.node)), word(&n.status)];
    if let Some(state) = state {
        s.push(plain("  "));
        s.push(word(state));
    }
    if n.oversized == Some(true) {
        s.push(plain("  "));
        s.push(span("⚠", Style::Amber));
    }
    s
}

/// What follows a node's bar on the line under its label: `current/total`,
/// `✕<obsolete>` and owners.
fn after_bar(n: &SpecNode) -> Vec<Span> {
    let mut s = spec::count(n);
    if !n.owners.is_empty() {
        s.push(plain(format!("  {}", n.owners.join(" "))));
    }
    s
}

/// The width in columns of the line under a node's label.
fn bar_line_width(n: &SpecNode) -> usize {
    width(&spec::bar(n)) + width(&after_bar(n))
}

/// Spans as SVG text content, the coloured ones in `tspan`s.
fn spans(line: &[Span]) -> String {
    let mut out = String::new();
    for s in line {
        if s.style == Style::Plain {
            out.push_str(&escape(&s.text));
        } else {
            let _ = write!(
                out,
                "<tspan class=\"{}\">{}</tspan>",
                s.style.class(),
                escape(&s.text)
            );
        }
    }
    out
}

/// A node's bar line at `(x, y)`, its baseline: the bar's cells as coloured rectangles,
/// then the rest as text.
fn draw_bar(body: &mut String, n: &SpecNode, x: f64, y: f64) {
    let mut x = x;
    // One rectangle per state's run of cells, as wide as the cells.
    for s in spec::bar(n) {
        let w = s.text.chars().count() as f64 * CH;
        let class = match s.style {
            Style::Plain => "none",
            other => other.class(),
        };
        let _ = writeln!(
            body,
            "<rect class=\"seg {class}\" x=\"{x:.1}\" y=\"{:.1}\" width=\"{w:.1}\" height=\"9\"/>",
            y - 9.0
        );
        x += w;
    }
    let _ = writeln!(
        body,
        "<text x=\"{x:.1}\" y=\"{y:.1}\" xml:space=\"preserve\">{}</text>",
        spans(&after_bar(n))
    );
}

/// The spec view as nested, layered boxes, each pending node outlined in the colour of its
/// queue state in `states`, below `head`, the checkout's line, when there is one.
pub(crate) fn spec(view: &SpecView, states: &States, head: Option<&Line>) -> String {
    let kids = children(view);
    let by_id: BTreeMap<&str, &SpecNode> =
        view.nodes.iter().map(|n| (n.node.as_str(), n)).collect();
    let mut boxes: BTreeMap<String, Box_> = BTreeMap::new();
    // Children are laid out before their parents: walk the view's order backwards.
    for n in view.nodes.iter().rev() {
        let siblings: Vec<&SpecNode> = kids
            .get(&Some(n.node.as_str()))
            .cloned()
            .unwrap_or_default();
        let own_w =
            width(&label(n, states.get(&n.node))).max(bar_line_width(n)) as f64 * CH + 2.0 * PAD;
        if siblings.is_empty() {
            boxes.insert(
                n.node.clone(),
                Box_ {
                    w: own_w,
                    h: TOP + PAD,
                    children: Vec::new(),
                },
            );
            continue;
        }
        // Layers among the siblings: a sibling sits one layer above every sibling it
        // depends on, so dependents are drawn above their dependencies.
        let ids: Vec<&str> = siblings.iter().map(|s| s.node.as_str()).collect();
        let mut layer: BTreeMap<&str, usize> = BTreeMap::new();
        fn depth<'a>(
            id: &'a str,
            ids: &[&'a str],
            by_id: &BTreeMap<&str, &'a SpecNode>,
            layer: &mut BTreeMap<&'a str, usize>,
            guard: usize,
        ) -> usize {
            if let Some(&d) = layer.get(id) {
                return d;
            }
            let d = if guard > ids.len() {
                0
            } else {
                by_id[id]
                    .depends_on
                    .iter()
                    .filter(|dep| ids.contains(&dep.as_str()))
                    .map(|dep| {
                        let dep = ids.iter().find(|i| **i == dep.as_str()).unwrap();
                        depth(dep, ids, by_id, layer, guard + 1) + 1
                    })
                    .max()
                    .unwrap_or(0)
            };
            layer.insert(id, d);
            d
        }
        for id in &ids {
            depth(id, &ids, &by_id, &mut layer, 0);
        }
        let top = layer.values().copied().max().unwrap_or(0);
        let mut children = Vec::new();
        let mut y = TOP + PAD;
        let mut w = own_w;
        for l in (0..=top).rev() {
            let mut x = PAD;
            let mut row_h: f64 = 0.0;
            for id in ids.iter().filter(|i| layer[*i] == l) {
                let b = &boxes[*id];
                children.push((x, y, id.to_string()));
                x += b.w + GAP;
                row_h = row_h.max(b.h);
            }
            w = w.max(x - GAP + PAD);
            y += row_h + GAP;
        }
        boxes.insert(
            n.node.clone(),
            Box_ {
                w,
                h: y - GAP + PAD,
                children,
            },
        );
    }

    // Absolute positions, from the roots.
    let mut at: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    let mut stack: Vec<(String, f64, f64)> = Vec::new();
    let (mut x, mut total_w, mut total_h) = (PAD, 0.0_f64, 0.0_f64);
    let mut body = String::new();
    // The checkout's line takes the first line of text; the boxes start below it.
    let top = match head {
        Some(line) => {
            let _ = writeln!(
                body,
                "<text x=\"{PAD}\" y=\"{:.1}\" xml:space=\"preserve\">{}</text>",
                PAD + 0.8 * LINE,
                spans(line)
            );
            total_w = width(line) as f64 * CH + 2.0 * PAD;
            total_h = LINE + 2.0 * PAD;
            PAD + LINE
        }
        None => PAD,
    };
    for r in kids.get(&None).cloned().unwrap_or_default() {
        stack.push((r.node.clone(), x, top));
        x += boxes[&r.node].w + GAP;
        total_w = total_w.max(x - GAP + PAD);
        total_h = total_h.max(boxes[&r.node].h + top + PAD);
    }
    while let Some((id, x, y)) = stack.pop() {
        at.insert(id.clone(), (x, y));
        for (dx, dy, child) in &boxes[&id].children {
            stack.push((child.clone(), x + dx, y + dy));
        }
    }

    for n in &view.nodes {
        let (x, y) = at[&n.node];
        let b = &boxes[&n.node];
        let state = states.get(&n.node);
        // A pending node is outlined in its queue state's colour, a converged oversized one
        // in amber (`oversized-is-a-state`), any other in its status's.
        let outline = match state {
            Some(state) => state.as_str(),
            None if n.oversized == Some(true) => "oversized",
            None => &n.status,
        };
        let _ = writeln!(
            body,
            "<rect class=\"node {}\" rx=\"6\" x=\"{x:.1}\" y=\"{y:.1}\" width=\"{:.1}\" height=\"{:.1}\"/>",
            escape(outline),
            b.w,
            b.h
        );
        let _ = writeln!(
            body,
            "<text x=\"{:.1}\" y=\"{:.1}\" xml:space=\"preserve\">{}</text>",
            x + PAD,
            y + HEADER - 7.0,
            spans(&label(n, state))
        );
        draw_bar(&mut body, n, x + PAD, y + TOP - 7.0);
    }
    for n in &view.nodes {
        let (x, y) = at[&n.node];
        let b = &boxes[&n.node];
        for dep in &n.depends_on {
            let Some(&(dx, dy)) = at.get(dep.as_str()) else {
                continue;
            };
            let db = &boxes[dep.as_str()];
            let _ = writeln!(
                body,
                "<line class=\"edge\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\"/>",
                x + b.w / 2.0,
                y + b.h,
                dx + db.w / 2.0,
                dy
            );
        }
    }
    document(total_w.max(2.0 * PAD), total_h.max(2.0 * PAD), &body)
}
