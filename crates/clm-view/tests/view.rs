//! `clm view <name>`'s charts, documents and models, as `render` returns them for a fixture
//! repository's graph and markers and the claims a test gives.

mod common;

use std::fs;
use std::path::Path;

use clm_view::{DEFAULT_WIDTH, Format, Name};
use common::{claim, status_of, view_of};
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

/// A `specd` node depending on nothing, each requirement `(slug, needs)` needing those listed.
fn needs(reqs: &[(&str, &[&str])]) -> String {
    let mut s = node("", &[]);
    for (r, needed) in reqs {
        let list: String = needed.iter().map(|n| format!("- {n}\n")).collect();
        s.push_str(&format!(
            "\n### Requirement: {r}\nIt SHALL.\n\n#### Needs\n{list}\n#### Scenario: s\n- ok\n"
        ));
    }
    s
}

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
    // req:view/view-command@0fb30947
    let dir = two_nodes();
    let text = view(dir.path(), Name::Spec, Format::Text);
    assert!(text.contains(" a ") && text.contains(" b "), "{text}");
    assert!(!text.contains('\u{1b}'), "{text}");
    assert!(text.contains("└─") || text.contains("├─"), "{text}");
}

#[test]
fn scenario_coloured_chart() {
    // req:view/view-command@0fb30947
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
    // req:view/view-command@0fb30947
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
fn scenario_nested_nodes() {
    // req:view/spec-view@380d5f8a
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/check/SPEC.md", &node("", &["c"]));
    write(
        dir.path(),
        "specs/check/drift/SPEC.md",
        &node("model", &["x", "y"]).replace("owners: []", "owners: [\"@a\"]"),
    );
    write(dir.path(), "specs/model/SPEC.md", &node("", &["m"]));
    let (graph, _) = clm_model::load(dir.path());
    let x = graph.requirement("check.drift/x").unwrap().version();
    write(
        dir.path(),
        "tests/t.rs",
        &in_test(&format!(
            "// {REQ}check.drift/x@{x}\n// {REQ}check.drift/y@00000000\n"
        )),
    );
    let v = json(&view(dir.path(), Name::Spec, Format::Json));
    assert_eq!(v["view"], "spec");
    let nodes = v["nodes"].as_array().unwrap();
    let find = |id: &str| nodes.iter().find(|n| n["node"] == id).unwrap();
    assert_eq!(find("check.drift")["parent"], "check");
    assert_eq!(find("check")["parent"], "clm");
    assert_eq!(find("check.drift")["owners"], serde_json::json!(["@a"]));
    assert_eq!(
        find("check.drift")["depends_on"],
        serde_json::json!(["model"])
    );
    for (key, n) in [
        ("current", 1),
        ("behind", 1),
        ("unbuilt", 0),
        ("obsolete", 0),
        ("total", 2),
    ] {
        assert_eq!(find("check.drift")[key], n, "{key}");
    }
    assert_eq!(find("check.drift").get("specd"), None);
    // Scenario: tree chart.
    let text = view(dir.path(), Name::Spec, Format::Text);
    let lines: Vec<&str> = text.lines().collect();
    let check = lines.iter().position(|l| l.contains("check ")).unwrap();
    let drift = lines
        .iter()
        .position(|l| l.contains("check.drift"))
        .unwrap();
    assert!(drift > check, "{text}");
    let indent = |l: &str| l.find(|c: char| c.is_alphanumeric()).unwrap();
    assert!(indent(lines[drift]) > indent(lines[check]), "{text}");
    for fact in ["█▓", "1/2", "@a", "model"] {
        assert!(lines[drift].contains(fact), "{fact} missing from {text}");
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
fn scenario_a_removed_nodes_markers() {
    // req:view/spec-view@380d5f8a
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(
        d,
        "specs/a/SPEC.md",
        &node("", &[]).replace("depends_on", "paths: [\"src/a/**\"]\ndepends_on"),
    );
    write(d, "changes/d/change.md", "---\nstatus: applied\n---\n");
    write(
        d,
        "changes/d/deltas/gone.md",
        "---\ncomponent: gone\nremove: true\n---\n## REMOVED Requirements\n\n\
         ### Requirement: x <!-- base: 0123456789abcdef -->\n",
    );
    write(d, "src/a/t.rs", &in_test(&format!("// {REQ}gone/x\n")));
    let v = json(&view(d, Name::Spec, Format::Json));
    let nodes = v["nodes"].as_array().unwrap();
    let a = nodes.iter().find(|n| n["node"] == "a").unwrap();
    assert_eq!(a["obsolete"], 1, "{v:#}");
    assert!(nodes.iter().all(|n| n["node"] != "gone"), "{v:#}");
    let text = view(d, Name::Spec, Format::Text);
    let line = text
        .lines()
        .find(|l| {
            l.trim_start_matches(|c: char| !c.is_alphanumeric())
                .starts_with("a ")
        })
        .unwrap_or_else(|| panic!("no a in {text}"));
    assert!(line.contains("✕1"), "{text}");
}

#[test]
fn spec_chart_shows_the_queue_state_from_the_claim_refs() {
    // req:view/spec-view@380d5f8a
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
fn scenario_change_progress() {
    // req:view/flight-view@d01ad49e
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/n/SPEC.md", &node("", &["x", "y"]));
    write(
        dir.path(),
        "changes/d/change.md",
        "---\nstatus: applied\n---\n",
    );
    write(
        dir.path(),
        "changes/d/deltas/n.md",
        "---\ncomponent: n\n---\n## ADDED Requirements\n### Requirement: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n\n### Requirement: y\nIt SHALL.\n\n#### Scenario: s\n- ok\n",
    );
    let (graph, _) = clm_model::load(dir.path());
    let x = graph.requirement("n/x").unwrap().version();
    write(
        dir.path(),
        "tests/t.rs",
        &in_test(&format!("// {REQ}n/x@{x}\n// {REQ}n/y@00000000\n")),
    );
    let v = json(&view(dir.path(), Name::Flight, Format::Json));
    assert_eq!(v["changes"][0]["status"], "applied");
    let d = &v["changes"][0]["changes"][0];
    assert_eq!(d["change"], "d");
    assert_eq!(d["current"], 1);
    assert_eq!(d["total"], 2);
    assert_eq!(d["complete"], false);
    assert_eq!(v["totals"]["requirements_pending"], 1);
}

#[test]
fn scenario_requirement_removed_later() {
    // req:view/flight-view@d01ad49e
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/n/SPEC.md", &node("", &["x"]));
    write(
        dir.path(),
        "changes/d/change.md",
        "---\nstatus: applied\n---\n",
    );
    write(
        dir.path(),
        "changes/d/deltas/n.md",
        "---\ncomponent: n\n---\n## ADDED Requirements\n### Requirement: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n\n### Requirement: y\nIt SHALL.\n\n#### Scenario: s\n- ok\n",
    );
    let (graph, _) = clm_model::load(dir.path());
    let x = graph.requirement("n/x").unwrap().version();
    write(
        dir.path(),
        "tests/t.rs",
        &in_test(&format!("// {REQ}n/x@{x}\n")),
    );
    let v = json(&view(dir.path(), Name::Flight, Format::Json));
    let d = &v["changes"][0]["changes"][0];
    assert_eq!(d["change"], "d");
    assert_eq!(d["current"], 1);
    assert_eq!(d["total"], 1);
    assert_eq!(d["obsolete"], serde_json::json!([]));
    assert_eq!(d["complete"], true);
}

#[test]
fn scenario_removed_requirement_still_cited() {
    // req:view/flight-view@d01ad49e
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
    let status = status_of(dir.path(), &[], &[]);
    let row = status
        .lines()
        .find(|l| l.trim_start().starts_with("d "))
        .unwrap_or_else(|| panic!("no row for d in {status}"));
    assert!(!row.contains("complete"), "{row}");
}

#[test]
fn scenario_order() {
    // req:view/flight-view@d01ad49e
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/a/SPEC.md", &node("", &["x"]));
    write(dir.path(), "specs/z/SPEC.md", &node("", &["x"]));
    let delta = |n: &str| {
        format!(
            "---\ncomponent: {n}\n---\n## ADDED Requirements\n### Requirement: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n"
        )
    };
    write(
        dir.path(),
        "changes/a1/change.md",
        "---\nstatus: applied\n---\n",
    );
    write(dir.path(), "changes/a1/deltas/a.md", &delta("a"));
    write(
        dir.path(),
        "changes/z9/change.md",
        "---\nstatus: implementing\npriority: P2\n---\n",
    );
    write(dir.path(), "changes/z9/deltas/z.md", &delta("z"));
    let text = view(dir.path(), Name::Flight, Format::Text);
    let pos = |s: &str| text.find(s).unwrap_or_else(|| panic!("no {s} in {text}"));
    assert!(pos("\n  z ") < pos("\n  a "), "{text}");
    assert!(pos("z9") < pos("a1"), "{text}");
}

#[test]
fn scenario_chain_weight() {
    // req:view/blockers-view@3950c690
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/a/SPEC.md", &node("", &["x"]));
    write(dir.path(), "specs/b/SPEC.md", &needs(&[("y", &["a/x"])]));
    write(dir.path(), "specs/c/SPEC.md", &needs(&[("x", &["b/y"])]));
    let refs = [claim("a", 1)];
    let v = json(&view_of(
        dir.path(),
        Name::Blockers,
        Format::Json,
        DEFAULT_WIDTH,
        &refs,
    ));
    let roots = v["roots"].as_array().unwrap();
    assert_eq!(roots.len(), 1, "{v:#}");
    assert_eq!(roots[0]["root"], "a");
    assert_eq!(roots[0]["weight"], 2);
    assert_eq!(roots[0]["holds"][0]["node"], "b");
    assert_eq!(roots[0]["holds"][1]["node"], "c");
    // `b/y` is blocked, so `c` waits on its blocker `a/x` directly.
    assert_eq!(roots[0]["holds"][1]["via"], serde_json::json!([]));
}

#[test]
fn scenario_human_first() {
    // req:view/blockers-view@3950c690
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/a/SPEC.md", &node("", &["x"]));
    write(dir.path(), "specs/d/SPEC.md", &node("", &["x"]));
    for n in ["e", "f", "g", "h"] {
        write(
            dir.path(),
            &format!("specs/{n}/SPEC.md"),
            &needs(&[("x", &["d/x"])]),
        );
    }
    write(
        dir.path(),
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [a/x]\n---\n",
    );
    let refs = [claim("d", 1)];
    let text = view_of(
        dir.path(),
        Name::Blockers,
        Format::Text,
        DEFAULT_WIDTH,
        &refs,
    );
    let pos = |s: &str| text.find(s).unwrap_or_else(|| panic!("no {s} in {text}"));
    assert!(pos("backlog/q") < pos("\nd "), "{text}");
    assert!(text.contains("holds up 4"), "{text}");
    assert!(text.contains("waits on you"), "{text}");
    let v = json(&view_of(
        dir.path(),
        Name::Blockers,
        Format::Json,
        DEFAULT_WIDTH,
        &refs,
    ));
    assert_eq!(v["roots"][0]["root"], "backlog/q", "{v:#}");
    assert_eq!(v["roots"][0]["weight"], 1, "{v:#}");
    assert_eq!(v["roots"][1]["root"], "d", "{v:#}");
    assert_eq!(v["roots"][1]["weight"], 4, "{v:#}");
}

#[test]
fn scenario_dependency_holds_nothing() {
    // `b` depends on `a`, but its own requirement is unblocked, so it is ready and held by
    // nothing.
    // req:view/blockers-view@3950c690
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/a/SPEC.md", &node("", &["x"]));
    write(dir.path(), "specs/b/SPEC.md", &node("a", &["x"]));
    write(
        dir.path(),
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [a/x]\n---\n",
    );
    let v = json(&view(dir.path(), Name::Blockers, Format::Json));
    let held: Vec<&str> = v["roots"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|r| r["holds"].as_array().unwrap())
        .map(|h| h["node"].as_str().unwrap())
        .collect();
    assert_eq!(held, ["a"], "{v:#}");
    let flight = json(&view(dir.path(), Name::Flight, Format::Json));
    let b = flight["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["node"] == "b")
        .unwrap();
    assert_eq!(b["state"], "ready", "{flight:#}");
}

#[test]
fn scenario_need_reaches_the_needed_requirements_item() {
    // req:view/blockers-view@3950c690
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(
        dir.path(),
        "specs/a/SPEC.md",
        &node("", &[]).replace(
            "## Requirements\n",
            "## Requirements\n\n### Requirement: x\nIt SHALL.\n\n#### Needs\n- b/y\n\n\
             #### Scenario: s\n- ok\n",
        ),
    );
    write(dir.path(), "specs/b/SPEC.md", &node("", &["y", "z"]));
    write(
        dir.path(),
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [b/y]\n---\n",
    );
    let v = json(&view(dir.path(), Name::Blockers, Format::Json));
    let roots = v["roots"].as_array().unwrap();
    assert_eq!(roots.len(), 1, "{v:#}");
    assert_eq!(roots[0]["root"], "backlog/q");
    assert_eq!(roots[0]["weight"], 1);
    assert_eq!(roots[0]["holds"][0]["node"], "a");
}

#[test]
fn scenario_nothing_blocked() {
    // req:view/blockers-root-lines@8d9bf009
    let dir = two_nodes();
    // `b` depends on `a`: with `a` converged, `b` is ready.
    let (graph, _) = clm_model::load(dir.path());
    let x = graph.requirement("a/x").unwrap().version();
    write(
        dir.path(),
        "tests/t.rs",
        &in_test(&format!("// {REQ}a/x@{x}\n")),
    );
    let text = view(dir.path(), Name::Blockers, Format::Text);
    let lines: Vec<&str> = text.lines().collect();
    let at = lines.iter().position(|l| *l == "blockers").expect(&text);
    assert_eq!(&lines[at + 1..], ["nothing is blocked"], "{text}");
}
