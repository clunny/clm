//! `clm view <name>`'s charts, documents and models, as `render` returns them for a fixture
//! repository's graph and markers and the claims a test gives.

mod common;

use std::fs;
use std::path::Path;

use clm_view::{DEFAULT_WIDTH, Format, Name};
use common::{claim, view_of};
use tempfile::TempDir;

const REQ: &str = concat!("req", ":");

/// What `clm view <name> --format <format>` prints for the fixture at `root`, at the default
/// width and with no claims.
fn view(root: &Path, name: Name, format: Format) -> String {
    view_of(root, name, format, DEFAULT_WIDTH, &[])
}

fn json(text: &str) -> serde_json::Value {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("{e}: {text}"))
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";

/// A `specd` node depending on `deps` with requirements `reqs`.
fn node(deps: &str, reqs: &[&str]) -> String {
    let mut s = format!(
        "---\ntitle: T\nstatus: specd\nowners: []\ndepends_on: [{deps}]\n---\n\n## Requirements\n"
    );
    for r in reqs {
        s.push_str(&format!(
            "\n### Requirement: {r}\nIt SHALL.\n\n#### Scenario: s\n- ok\n"
        ));
    }
    s
}

/// Nodes `a` and `b`, `b` depending on `a`.
fn two_nodes() -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/a/SPEC.md", &node("", &["x"]));
    write(dir.path(), "specs/b/SPEC.md", &node("a", &["y"]));
    dir
}

/// `s` as a Rust file whose markers count as a test's: those on its third line on are inside
/// a test, the earlier ones directly above it. Every line of `s` keeps its line number.
fn in_test(s: &str) -> String {
    let (head, body) = match s.strip_prefix('\n') {
        Some(rest) => ("", rest),
        None => s.split_once("\n\n").map_or((s, ""), |(a, b)| (a, b)),
    };
    let nl = if body.ends_with('\n') || body.is_empty() {
        ""
    } else {
        "\n"
    };
    if s.starts_with('\n') {
        format!("#[test]\nfn t() {{{body}{nl}}}\n")
    } else if body.is_empty() {
        format!("{s}#[test]\nfn t() {{}}\n")
    } else {
        format!("{head}\n#[test]\nfn t() {{{body}{nl}}}\n")
    }
}

#[test]
fn scenario_plain_chart() {
    // req:view/view-command@c4a4df22
    let dir = two_nodes();
    let text = view(dir.path(), Name::Spec, Format::Text);
    assert!(text.contains(" a ") && text.contains(" b "), "{text}");
    assert!(!text.contains('\u{1b}'), "{text}");
    assert!(text.contains("└─") || text.contains("├─"), "{text}");
}

#[test]
fn scenario_coloured_chart() {
    // req:view/view-command@c4a4df22
    let dir = two_nodes();
    let plain = view(dir.path(), Name::Spec, Format::Text);
    let ansi = view(dir.path(), Name::Spec, Format::Ansi);
    assert!(ansi.contains("\u{1b}["), "{ansi}");
    let mut stripped = String::new();
    let mut chars = ansi.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            stripped.push(c);
        }
    }
    assert_eq!(stripped, plain);
}

#[test]
fn scenario_svg() {
    // req:view/view-command@c4a4df22
    let dir = two_nodes();
    let text = view(dir.path(), Name::Spec, Format::Svg);
    let doc = roxmltree::Document::parse(&text).unwrap_or_else(|e| panic!("{e}: {text}"));
    assert_eq!(doc.root_element().tag_name().name(), "svg");
    // The labels: the text of the `<text>` elements, not the embedded stylesheet.
    let texts: Vec<String> = doc
        .descendants()
        .filter(|n| n.has_tag_name("text"))
        .map(|n| n.descendants().filter_map(|t| t.text()).collect::<String>())
        .collect();
    assert!(texts.iter().any(|t| t.starts_with("a ")), "{texts:?}");
    assert!(texts.iter().any(|t| t.starts_with("b ")), "{texts:?}");
    for name in [Name::Flight, Name::Blockers] {
        let text = view(dir.path(), name, Format::Svg);
        let doc = roxmltree::Document::parse(&text).unwrap_or_else(|e| panic!("{e}: {text}"));
        assert_eq!(doc.root_element().tag_name().name(), "svg");
    }
}

#[test]
fn scenario_node_detail_from_the_command() {
    // req:view/component-detail@d0431adf
    let dir = two_nodes();
    write(
        dir.path(),
        "specs/a/SPEC.md",
        &node("", &["x"]).replace(
            "\n## Requirements",
            "\n## Contract\nAlpha does X.\n\n## Requirements",
        ),
    );
    write(
        dir.path(),
        "changes/20260101-c/change.md",
        "---\nstatus: applied\n---\n",
    );
    write(
        dir.path(),
        "changes/20260101-c/deltas/a.md",
        "---\ncomponent: a\n---\n## ADDED Requirements\n### Requirement: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n",
    );
    write(
        dir.path(),
        "backlog/q.md",
        "---\ntitle: Ask\nkind: question\ncomponents: [a]\n---\n",
    );
    let v = json(&view(dir.path(), Name::Spec, Format::Json));
    let nodes = v["nodes"].as_array().unwrap();
    let find = |id: &str| nodes.iter().find(|n| n["node"] == id).unwrap();
    let a = find("a");
    assert_eq!(a["contract"], "Alpha does X.", "{a:#}");
    assert_eq!(
        a["changes"],
        serde_json::json!([{ "change": "20260101-c", "status": "applied",
                             "current": 0, "total": 1, "complete": false }])
    );
    assert_eq!(
        a["backlog"],
        serde_json::json!([{ "slug": "q", "kind": "question", "title": "Ask" }])
    );
    let b = find("b");
    assert_eq!(b["changes"], serde_json::json!([]));
    assert_eq!(b["backlog"], serde_json::json!([]));
    assert_eq!(b["contract"], "");
}

#[test]
fn spec_chart_shows_the_queue_state_from_the_claim_refs() {
    // req:view/spec-view@249ab038
    let dir = two_nodes();
    let refs = [claim("a", 1)];
    let text = view_of(dir.path(), Name::Spec, Format::Text, DEFAULT_WIDTH, &refs);
    // The line whose ID, after the tree prefix, is `id`.
    let line = |id: &str| {
        text.lines()
            .find(|l| {
                l.trim_start_matches(|c: char| !c.is_alphanumeric())
                    .starts_with(&format!("{id} "))
            })
            .unwrap_or_else(|| panic!("no {id} in {text}"))
    };
    assert!(line("a").contains("claimed"), "{text}");
    // `b` depends on `a`, but needs nothing of it, so it is ready (`needs-alone-order`).
    assert!(line("b").contains("ready"), "{text}");
    // The JSON is the model alone, with no state.
    let v = json(&view_of(
        dir.path(),
        Name::Spec,
        Format::Json,
        DEFAULT_WIDTH,
        &refs,
    ));
    for n in v["nodes"].as_array().unwrap() {
        assert_eq!(n.get("state"), None, "{n:#}");
    }
    assert!(!v.to_string().contains("claimed"), "{v:#}");
}

#[test]
fn scenario_removed_requirement_still_cited() {
    // req:view/flight-view@86ea7638
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/n/SPEC.md", &node("", &["y"]));
    write(
        dir.path(),
        "changes/d/change.md",
        "---\nstatus: applied\n---\n",
    );
    write(
        dir.path(),
        "changes/d/deltas/n.md",
        "---\ncomponent: n\n---\n## REMOVED Requirements\n### Requirement: x\n",
    );
    write(
        dir.path(),
        "tests/t.rs",
        &in_test(&format!("//\n//\n// {REQ}n/x\n")),
    );
    let v = json(&view(dir.path(), Name::Flight, Format::Json));
    let d = &v["changes"][0]["changes"][0];
    assert_eq!(d["change"], "d", "{v:#}");
    assert_eq!(d["obsolete"], serde_json::json!(["tests/t.rs:3"]), "{v:#}");
    assert_eq!(d["complete"], false);
}
