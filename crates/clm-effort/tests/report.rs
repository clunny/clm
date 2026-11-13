//! What each capability costs (`effort/attribution`, `effort/unsettled-flags`,
//! `effort/capabilities-list`, `effort/capability-page`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use clm_effort::{
    Constraint, Enforcing, Flag, Inputs, Overlap, Report, Spend, architecture, architecture_text,
    constraint_text, list_text, page_text, report, unsettled,
};
use clm_ledger::{Fold, fold_records};
use clm_model::{Files, Graph, load_files};

const ARCHITECTURE: &str =
    "## Constraints\n\n### Constraint: z\nGit SHALL hold it.\n\n#### Scenario: s\n- ok\n";

const CAPABILITIES: &str = "## Capabilities\n\n### Capability: p\nA person can p.\n\n#### Why\np why\n\n#### Scenario: s\n- ok\n\n### Capability: q\nA person can q.\n\n#### Why\nq why\n\n#### Scenario: s\n- ok\n";

fn spec(serves: &str, requirements: &[&str]) -> String {
    format!(
        "---\ntitle: T\nstatus: active\nowners: []\nserves: [{serves}]\n---\n\n## Contract\nc\n\n## Requirements\n\n{}",
        requirements.join("\n")
    )
}

fn req(slug: &str, serves: &[&str]) -> String {
    let block = if serves.is_empty() {
        String::new()
    } else {
        let lines: Vec<String> = serves.iter().map(|s| format!("- {s}\n")).collect();
        format!("\n#### Serves\n{}", lines.concat())
    };
    format!("### Requirement: {slug}\nIt SHALL.\n\n#### Scenario: s\n- ok\n{block}")
}

/// Node `a` (`x` serves `p`, `y` serves `p` and `q`), node `b` (`w` serves `q`), node `r`
/// (`z` serves only `architecture/z`), change `c` with deltas on `a` and `b`, the archived
/// change `d` with a delta on `a`, and the
/// files `extra` adds.
fn graph(extra: &[(&str, &str)]) -> Graph {
    let mut files = Files::new();
    let mut put = |path: &str, text: &str| {
        files.insert(PathBuf::from(path), text.to_string().into());
    };
    put(
        "specs/SPEC.md",
        "---\ntitle: T\nstatus: active\nowners: []\n---\n\n## Contract\nc\n",
    );
    put("ARCHITECTURE.md", ARCHITECTURE);
    put("CAPABILITIES.md", CAPABILITIES);
    put(
        "specs/a/SPEC.md",
        &spec(
            "capability/p",
            &[&req("x", &[]), &req("y", &["capability/p", "capability/q"])],
        ),
    );
    put("specs/b/SPEC.md", &spec("capability/q", &[&req("w", &[])]));
    put(
        "specs/r/SPEC.md",
        &spec("architecture/z", &[&req("z", &[])]),
    );
    put(
        "changes/c/change.md",
        "---\nstatus: applied\nburdens: [one approval]\n---\n\nIntent.\n",
    );
    put(
        "changes/archive/d/change.md",
        "---\nstatus: done\nburdens: [one approval]\n---\n\nIntent.\n",
    );
    for path in ["c/deltas/a", "c/deltas/b", "archive/d/deltas/a"] {
        let node = &path[path.len() - 1..];
        put(
            &format!("changes/{path}.md"),
            &format!(
                "---\ncomponent: {node}\n---\n## ADDED Requirements\n\n### Requirement: new\nIt SHALL.\n\n#### Scenario: s\n- ok\n"
            ),
        );
    }
    for (path, text) in extra {
        put(path, text);
    }
    let (graph, _) = load_files(&files);
    assert_eq!(graph.changes["c"].deltas.len(), 2);
    assert!(graph.changes["d"].archived);
    graph
}

/// An `effort` line on `branch` worth `$4 × dollars` on Opus 5.5 (a million input tokens
/// each), with two hours active.
fn effort(id: &str, branch: &str, dollars: u64) -> String {
    serde_json::json!({
        "id": id, "kind": "effort", "writer": "ann/1", "at": "2026-10-30T00:00:00.000Z",
        "branch": branch, "session": id,
        "models": {"claude-opus-5-5": {"input": dollars * 1_000_000, "output": 0, "cache_read": 0, "cache_write": 0}},
        "active_seconds": 7200,
    })
    .to_string()
}

fn fold(lines: &[String]) -> Fold {
    let text = lines.join("\n");
    fold_records(&[("origin/clm/ledger/ann/1", &text)])
}

fn run(graph: &Graph, fold: &Fold) -> Report {
    report(&Inputs {
        graph,
        fold,
        history: None,
        flag_text: &BTreeMap::new(),
    })
}

fn dollars(report: &Report, slug: &str) -> f64 {
    let cost = report.capabilities.iter().find(|c| c.slug == slug).unwrap();
    cost.spend.dollars
}

#[test]
fn impl_branch_cost_goes_to_its_capabilities() {
    // req:effort/attribution@fb0ae2bb
    let report = run(&graph(&[]), &fold(&[effort("e1", "impl/a", 1)]));
    assert_eq!(dollars(&report, "p"), 3.0);
    assert_eq!(dollars(&report, "q"), 1.0);
    assert_eq!(report.unattributed, Spend::default());
}

#[test]
fn spec_branch_splits_over_delta_nodes() {
    // req:effort/attribution@fb0ae2bb
    // $2 to `a` (p 1.5, q 0.5) and $2 to `b` (q 2).
    let report = run(&graph(&[]), &fold(&[effort("e1", "spec/c", 1)]));
    assert_eq!(dollars(&report, "p"), 1.5);
    assert_eq!(dollars(&report, "q"), 2.5);
    let q = &report.capabilities[0];
    assert_eq!(
        (q.changes, q.burdens.clone()),
        (2, vec!["one approval".into()])
    );
}

#[test]
fn main_and_unknown_are_unattributed() {
    // req:effort/attribution@fb0ae2bb
    let lines = [
        effort("e1", "main", 1),
        effort("e2", "propose/x", 1),
        effort("e3", "impl/gone", 1),
        effort("e4", "spec/gone", 1),
    ];
    let report = run(&graph(&[]), &fold(&lines));
    assert_eq!(report.unattributed.dollars, 16.0);
    assert_eq!(report.unattributed.active_seconds, 4.0 * 7200.0);
    assert_eq!(dollars(&report, "p") + dollars(&report, "q"), 0.0);
}

#[test]
fn architecture_only_rules() {
    // req:effort/attribution@fb0ae2bb
    let report = run(&graph(&[]), &fold(&[effort("e1", "impl/r", 1)]));
    assert_eq!(report.architecture_only.dollars, 4.0);
    assert_eq!(report.unattributed.dollars, 0.0);
}

#[test]
fn ranked_by_cost() {
    // req:effort/attribution@fb0ae2bb
    let slugs =
        |r: &Report| -> Vec<String> { r.capabilities.iter().map(|c| c.slug.clone()).collect() };
    let none = run(&graph(&[]), &fold(&[]));
    assert_eq!(slugs(&none), ["p", "q"]);
    let spec = run(&graph(&[]), &fold(&[effort("e1", "spec/c", 1)]));
    assert_eq!(slugs(&spec), ["q", "p"]);
}

#[test]
fn approvals_count_whole() {
    // req:effort/attribution@fb0ae2bb
    let approved = serde_json::json!({
        "id": "a1", "kind": "approved", "writer": "ann/1", "at": "2026-10-30T00:00:00.000Z",
        "fingerprint": "f", "branch": "spec/c", "change": "c",
    })
    .to_string();
    let report = run(&graph(&[]), &fold(&[approved]));
    for cost in &report.capabilities {
        assert_eq!(cost.approvals, 1, "{}", cost.slug);
    }
}

#[test]
fn overlap_names_other_capabilities() {
    // req:effort/attribution@fb0ae2bb
    let report = run(&graph(&[]), &fold(&[]));
    let p = &report.capabilities[0];
    assert_eq!(p.rules, ["a/x", "a/y"]);
    assert_eq!(p.nodes, ["a"]);
    assert_eq!(
        p.overlap,
        Overlap {
            only: 1,
            shared: vec![("capability/q".into(), 1)]
        }
    );
}

/// A friction item on `a/y`, and a `decided` `accept` for `p` settling it.
fn flagged() -> (Graph, Fold) {
    let probe = graph(&[]);
    let version = probe.requirement("a/y").unwrap().version().to_string();
    let item = format!(
        "---\ntitle: Y is hard\nkind: friction\ncomponents: [a]\nrequirement: a/y@{version}\n---\n\nIt needed a fake.\n"
    );
    let graph = graph(&[("backlog/fr.md", &item)]);
    assert!(graph.backlog.contains_key("fr"));
    let decided = serde_json::json!({
        "id": "d1", "kind": "decided", "writer": "ann/1", "at": "2026-10-30T09:00:00.000Z",
        "capability": "capability/p", "value": "high", "decision": "accept", "settles": ["fr"],
    })
    .to_string();
    (graph, fold(&[decided, effort("e1", "impl/a", 1)]))
}

#[test]
fn flag_on_an_architecture_entry() {
    // req:effort/unsettled-flags@30490d0a
    let architecture = "## Constraints\n\n### Constraint: e\nGit SHALL hold it.\n\n#### Scenario: s\n- ok\n\n#### Enforced by\n- a/y\n";
    let (graph, fold) = flagged();
    let item = "---\ntitle: Y is hard\nkind: friction\ncomponents: [a]\nrequirement: a/y@00000000\n---\n\nIt needed a fake.\n";
    let version = graph.requirement("a/y").unwrap().version().to_string();
    let item = item.replace("00000000", &version);
    let graph = self::graph(&[("ARCHITECTURE.md", architecture), ("backlog/fr.md", &item)]);
    assert_eq!(unsettled(&graph, &fold, "architecture/e"), ["fr"]);
    // A decision for the entry settles it there, and `p` stays flagged only by its own.
    let decided = serde_json::json!({
        "id": "d2", "kind": "decided", "writer": "ann/1", "at": "2026-10-30T09:00:00.000Z",
        "capability": "architecture/e", "value": "low", "decision": "accept", "settles": ["fr"],
    })
    .to_string();
    let fold = self::fold(&[decided]);
    assert!(unsettled(&graph, &fold, "architecture/e").is_empty());
    let report = run(&graph, &fold);
    let p = report.capabilities.iter().find(|c| c.slug == "p").unwrap();
    assert!(p.flagged);
}

#[test]
fn flag_on_two_capabilities() {
    // req:effort/unsettled-flags@30490d0a
    let (graph, fold) = flagged();
    let report = run(&graph, &fold);
    let (p, q) = (&report.capabilities[0], &report.capabilities[1]);
    assert_eq!((p.slug.as_str(), p.flagged), ("p", false));
    assert_eq!((q.slug.as_str(), q.flagged), ("q", true));
    assert!(p.flags[0].settled && !q.flags[0].settled);
    assert_eq!((p.flags_settled, q.flags_settled), (1, 0));
    assert!(unsettled(&graph, &fold, "capability/p").is_empty());
    assert_eq!(unsettled(&graph, &fold, "capability/q"), ["fr"]);
}

#[test]
fn list_text_lines() {
    // req:effort/capabilities-list@daa12c28
    let (graph, fold) = flagged();
    let report = run(&graph, &fold);
    assert_eq!(
        list_text(&report),
        [
            "capabilities 2 · 1 ⚑ · costs are estimates: a component's cost is split by its rules' share",
            "  p  ≈$3.00  750k tok  - commits  0 reviews  0 workarounds  accept high 2026-10-30",
            "⚑ q  ≈$1.00  250k tok  - commits  0 reviews  1 workarounds  never decided",
            "unattributed       ≈$0.00  0 tok",
            "architecture only  ≈$0.00  0 tok",
        ]
    );
}

#[test]
fn page_text_sections() {
    // req:effort/capability-page@dd387c2f
    let (graph, fold) = flagged();
    let text = BTreeMap::from([("fr".to_string(), "It needed a fake.".to_string())]);
    let report = report(&Inputs {
        graph: &graph,
        fold: &fold,
        history: None,
        flag_text: &text,
    });
    assert_eq!(
        page_text(&report.capabilities[1]),
        [
            "q ⚑",
            "A person can q.",
            "2 rules · components a, b",
            "",
            "why we need it",
            "  q why",
            "what it costs (an estimate: a component's cost is split by its rules' share)",
            "  ≈$1.00 · 250k tokens · 0.5h active · 0 cache reads",
            "  - commits · 2 changes",
            "  your attention: 0 approvals · - questions answered · 0 flags settled",
            "  blocking: - items held its rules up for - days",
            "  size: 2 rules · - tokens",
            "  declared burdens: one approval",
            "workarounds (1 open, 1 unsettled)",
            "  ⚑ fr a/y Y is hard",
            "    It needed a fake.",
            "can we live without it?",
            "  1 of its 2 rules serve only it; the rest also serve p (1)",
            "rules",
            "  a/y",
            "  b/w",
            "decisions",
            "  never decided",
            "decide: clm capabilities decide q --value high|medium|low --decision keep|cut|redesign|accept",
        ]
    );
    let p = page_text(&report.capabilities[0]);
    assert!(p.contains(&"    fr a/y Y is hard".to_string()), "{p:?}");
    assert!(
        p.contains(&"  2026-10-30 accept (value high) by ann/1".to_string()),
        "{p:?}"
    );
}

fn constraint(slug: &str, enforced_by: &[(&str, bool)], untested: Option<&str>) -> Constraint {
    Constraint {
        id: format!("architecture/{slug}"),
        slug: slug.into(),
        version: "00000000".into(),
        statement: "Git SHALL hold it.".into(),
        text: String::new(),
        enforced_by: enforced_by
            .iter()
            .map(|(rule, current)| Enforcing {
                rule: rule.to_string(),
                current: Some(*current),
            })
            .collect(),
        untested: untested.map(str::to_string),
        rules: enforced_by.iter().map(|(r, _)| r.to_string()).collect(),
        nodes: Vec::new(),
        spend: Spend::default(),
        changes: 0,
        approvals: 0,
        flags_settled: 0,
        burdens: Vec::new(),
        flags: Vec::new(),
        overlap: Overlap::default(),
        decisions: Vec::new(),
        flagged: false,
    }
}

fn flag(settled: bool) -> Flag {
    Flag {
        slug: "fr".into(),
        requirement: "a/y".into(),
        title: "Y is hard".into(),
        text: "It needed a fake.".into(),
        settled,
        added: None,
    }
}

#[test]
fn architecture_list_lines() {
    // req:effort/architecture-list@a45a68a0
    let mut e = constraint("e", &[("a/x", true), ("a/y", false)], None);
    e.flags = vec![flag(false)];
    e.flagged = true;
    let g = constraint("g", &[], Some("A judgement."));
    let mut h = constraint("h", &[("a/x", true)], Some("Partly."));
    h.approvals = 2;
    assert_eq!(
        architecture_text(&[e, g, h]),
        [
            "architecture 3 · 1 held · 1 ⚑ · costs are estimates: an entry's cost is its enforcing rules' share, counted apart from the capabilities'",
            "⚑ e  held 1/2             ≈$0.00  0 tok  0 reviews  1 workarounds  never decided",
            "  g  untested             ≈$0.00  0 tok  0 reviews  0 workarounds  never decided",
            "  h  held 1/1 + untested  ≈$0.00  0 tok  2 reviews  0 workarounds  never decided",
        ]
    );
}

#[test]
fn architecture_page_sections() {
    // req:effort/architecture-page@8bc35522
    let enforced = [("a/x", true), ("a/y", false)];
    let mut e = constraint("e", &enforced, Some("Partly judgement."));
    e.nodes = vec!["a".into()];
    e.flags = vec![flag(false)];
    e.flagged = true;
    e.overlap = Overlap {
        only: 1,
        shared: vec![("architecture/f".into(), 1)],
    };
    assert_eq!(
        constraint_text(&e),
        [
            "e ⚑",
            "Git SHALL hold it.",
            "2 enforcing rules · components a",
            "",
            "is it held?",
            "  current a/x",
            "  pending a/y",
            "  untested: Partly judgement.",
            "what it costs (an estimate: its enforcing rules' share, counted apart from the capabilities')",
            "  ≈$0.00 · 0 tokens · 0.0h active · 0 cache reads",
            "  0 changes",
            "  your attention: 0 approvals · 0 flags settled",
            "  declared burdens: none",
            "workarounds (1 open, 1 unsettled)",
            "  ⚑ fr a/y Y is hard",
            "    It needed a fake.",
            "can we live without it?",
            "  1 of its 2 enforcing rules enforce only it; the rest also enforce f (1)",
            "decisions",
            "  never decided",
            "decide: clm architecture decide e --value high|medium|low --decision keep|cut|redesign|accept",
        ]
    );
    let g = constraint_text(&constraint("g", &[], Some("A judgement.")));
    assert!(
        g.contains(&"0 enforcing rules · no components".to_string()),
        "{g:?}"
    );
    assert!(g.contains(&"  no rule enforces it".to_string()), "{g:?}");
    let alone = constraint_text(&constraint("h", &[("a/x", true)], None));
    let line = "  all its enforcing rules enforce only it".to_string();
    assert!(alone.contains(&line), "{alone:?}");
}

const ENFORCED: &str = "## Constraints\n\n### Constraint: e\nGit SHALL hold e.\n\n#### Scenario: s\n- ok\n\n#### Enforced by\n- a/x\n- a/y\n\n### Constraint: f\nGit SHALL hold f.\n\n#### Scenario: s\n- ok\n\n#### Enforced by\n- a/x\n\n### Constraint: g\nGit SHALL hold g.\n\n#### Scenario: s\n- ok\n\n#### Untested\nA judgement.\n";

fn constraint_of<'a>(list: &'a [Constraint], slug: &str) -> &'a Constraint {
    list.iter().find(|c| c.slug == slug).unwrap()
}

fn architecture_in(
    architecture_md: &str,
    fold: &Fold,
    current: Option<&BTreeSet<String>>,
) -> Vec<Constraint> {
    let graph = graph(&[("ARCHITECTURE.md", architecture_md)]);
    architecture(
        &Inputs {
            graph: &graph,
            fold,
            history: None,
            flag_text: &BTreeMap::new(),
        },
        current,
    )
}

fn architecture_of(fold: &Fold, current: Option<&BTreeSet<String>>) -> Vec<Constraint> {
    architecture_in(ENFORCED, fold, current)
}

fn fold_of_nothing() -> Fold {
    fold(&[])
}

fn slugs(list: &[Constraint]) -> Vec<&str> {
    list.iter().map(|c| c.slug.as_str()).collect()
}

#[test]
fn architecture_splits_a_rule_over_the_entries_naming_it() {
    // req:effort/architecture-report@0e43b5f8
    let list = architecture_of(&fold(&[effort("e1", "impl/a", 1)]), None);
    assert_eq!(constraint_of(&list, "e").spend.dollars, 3.0);
    assert_eq!(constraint_of(&list, "f").spend.dollars, 1.0);
    assert_eq!(constraint_of(&list, "g").spend, Spend::default());
    // A spec branch gives `a` half of $4, so `e` $1.5 and `f` $0.5.
    let list = architecture_of(&fold(&[effort("e1", "spec/c", 1)]), None);
    assert_eq!(constraint_of(&list, "e").spend.dollars, 1.5);
    assert_eq!(constraint_of(&list, "f").spend.dollars, 0.5);
}

#[test]
fn architecture_ranks_by_dollars_then_file_order() {
    // req:effort/architecture-report@0e43b5f8
    // `f` comes first in the file but `e` costs more; `g` and `h` tie at nothing.
    let first_f = "## Constraints\n\n### Constraint: f\nGit SHALL hold f.\n\n#### Scenario: s\n- ok\n\n#### Enforced by\n- a/x\n\n### Constraint: g\nGit SHALL hold g.\n\n#### Scenario: s\n- ok\n\n#### Untested\nOne.\n\n### Constraint: h\nGit SHALL hold h.\n\n#### Scenario: s\n- ok\n\n#### Untested\nTwo.\n\n### Constraint: e\nGit SHALL hold e.\n\n#### Scenario: s\n- ok\n\n#### Enforced by\n- a/x\n- a/y\n";
    let list = architecture_in(first_f, &fold(&[effort("e1", "impl/a", 1)]), None);
    assert_eq!(slugs(&list), ["e", "f", "g", "h"]);
    let list = architecture_in(first_f, &fold(&[]), None);
    assert_eq!(slugs(&list), ["f", "g", "h", "e"]);
}

#[test]
fn architecture_overlap_counts_shared_rules() {
    // req:effort/architecture-report@0e43b5f8
    let list = architecture_of(&fold(&[]), None);
    let e = constraint_of(&list, "e");
    assert_eq!(e.overlap.only, 1);
    assert_eq!(e.overlap.shared, [("architecture/f".to_string(), 1)]);
    let f = constraint_of(&list, "f");
    assert_eq!(f.overlap.only, 0);
    assert_eq!(f.overlap.shared, [("architecture/e".to_string(), 1)]);
    let g = constraint_of(&list, "g");
    assert_eq!((g.overlap.only, g.overlap.shared.len()), (0, 0));
}

#[test]
fn architecture_says_whether_it_is_held() {
    // req:effort/architecture-report@0e43b5f8
    let current: BTreeSet<String> = ["a/x".to_string()].into();
    let list = architecture_of(&fold(&[]), Some(&current));
    let e = constraint_of(&list, "e");
    let held: Vec<(&str, Option<bool>)> = e
        .enforced_by
        .iter()
        .map(|r| (r.rule.as_str(), r.current))
        .collect();
    assert_eq!(held, [("a/x", Some(true)), ("a/y", Some(false))]);
    assert_eq!(e.rules, ["a/x", "a/y"]);
    assert_eq!(e.nodes, ["a"]);
    assert_eq!(e.statement, "Git SHALL hold e.");
    let g = constraint_of(&list, "g");
    assert_eq!(g.untested.as_deref(), Some("A judgement."));
    assert!(g.rules.is_empty() && g.nodes.is_empty());
    assert_eq!(g.spend, Spend::default());
    let list = architecture_of(&fold(&[]), None);
    let unknown = constraint_of(&list, "e")
        .enforced_by
        .iter()
        .map(|r| r.current);
    assert!(unknown.eq([None, None]));
}

#[test]
fn architecture_counts_changes_approvals_and_burdens() {
    // req:effort/architecture-report@0e43b5f8
    let approved = |id: &str, branch: &str| {
        serde_json::json!({
            "id": id, "kind": "approved", "writer": "ann/1", "at": "2026-10-30T00:00:00.000Z",
            "fingerprint": "f", "branch": branch, "change": null,
        })
        .to_string()
    };
    let lines = [approved("a1", "spec/c"), approved("a2", "impl/b")];
    let list = architecture_of(&fold(&lines), None);
    // Change `c` and the archived `d` name node `a`; only `spec/c` reaches it.
    let e = constraint_of(&list, "e");
    assert_eq!((e.changes, e.approvals), (2, 1));
    assert_eq!(e.burdens, ["one approval"]);
    let g = constraint_of(&list, "g");
    assert_eq!((g.changes, g.approvals, g.burdens.len()), (0, 0, 0));
}

#[test]
fn capabilities_report_ignores_the_architecture_entries() {
    // req:effort/architecture-report@0e43b5f8
    let fold = fold(&[effort("e1", "impl/a", 1)]);
    let plain = run(&graph(&[]), &fold);
    let enforced = run(&graph(&[("ARCHITECTURE.md", ENFORCED)]), &fold);
    assert_eq!(plain, enforced);
}

#[test]
fn architecture_flags_and_decisions() {
    // req:effort/architecture-report@0e43b5f8
    let item = "---\ntitle: Y is hard\nkind: friction\ncomponents: [a]\nrequirement: a/y@00000000\n---\n\nIt needed a fake.\n";
    let graph = graph(&[("ARCHITECTURE.md", ENFORCED), ("backlog/fr.md", item)]);
    let decided = serde_json::json!({
        "id": "d1", "kind": "decided", "writer": "ann/1", "at": "2026-10-30T00:00:00.000Z",
        "capability": "architecture/e", "value": "low", "decision": "accept", "settles": ["fr"],
    })
    .to_string();
    let fold = fold(&[decided]);
    let flag_text = BTreeMap::from([("fr".to_string(), "It needed a fake.".to_string())]);
    let list = architecture(
        &Inputs {
            graph: &graph,
            fold: &fold,
            history: None,
            flag_text: &flag_text,
        },
        None,
    );
    let e = constraint_of(&list, "e");
    assert!(e.flags[0].settled && !e.flagged);
    assert_eq!(
        (e.flags[0].text.as_str(), e.flags_settled),
        ("It needed a fake.", 1)
    );
    assert_eq!(e.decisions.len(), 1);
    // `f` names `a/x`, not `a/y`.
    assert!(constraint_of(&list, "f").flags.is_empty());
    // Without the decision the flag is open and the entry flagged.
    let fold = fold_of_nothing();
    let list = architecture(
        &Inputs {
            graph: &graph,
            fold: &fold,
            history: None,
            flag_text: &flag_text,
        },
        None,
    );
    let e = constraint_of(&list, "e");
    assert!(!e.flags[0].settled && e.flagged);
}

#[test]
fn a_capabilitys_text_is_its_whole_entry() {
    // req:effort/entry-text@c8d13ca6
    let capabilities = "## Capabilities\n\n### Capability: x\nA person runs `clm land`.\n\n#### Why\nx why\n\n#### Scenario: s\n- ok\n\n### Capability: y\nA person can y.\n\n#### Why\ny why\n\n#### Scenario: s\n- ok\n";
    let graph = graph(&[("CAPABILITIES.md", capabilities)]);
    let fold = fold(&[]);
    let report = run(&graph, &fold);
    let cost = |slug: &str| report.capabilities.iter().find(|c| c.slug == slug).unwrap();
    let text = &cost("x").text;
    assert!(text.starts_with("A person runs `clm land`."), "{text}");
    assert!(text.contains("#### Why") && text.contains("#### Scenario: s"));
    assert!(!text.contains("y why") && !text.contains("Capability: y"));
    assert_eq!(text.trim(), text);
    // The JSON a client reads carries it.
    let json = serde_json::to_value(cost("x")).unwrap();
    assert_eq!(json["text"], *text);
}

#[test]
fn a_text_stops_at_the_next_section() {
    // req:effort/entry-text@c8d13ca6
    let capabilities = "## Capabilities\n\n### Capability: x\nA person can x.\n\n#### Why\nx why\n\n#### Scenario: s\n- ok\n\n## Notes\n\nOther prose.\n";
    let graph = graph(&[("CAPABILITIES.md", capabilities)]);
    let fold = fold(&[]);
    let report = run(&graph, &fold);
    let x = report.capabilities.iter().find(|c| c.slug == "x").unwrap();
    assert!(x.text.ends_with("- ok"), "{}", x.text);
    assert!(!x.text.contains("Other prose"));
}

#[test]
fn a_constraints_text_is_its_whole_entry() {
    // req:effort/entry-text@c8d13ca6
    let list = architecture_of(&fold_of_nothing(), None);
    let e = constraint_of(&list, "e");
    assert!(e.text.starts_with("Git SHALL hold e."), "{}", e.text);
    assert!(e.text.contains("#### Enforced by") && e.text.contains("- a/x"));
    let g = constraint_of(&list, "g");
    assert!(g.text.contains("#### Untested") && g.text.contains("A judgement."));
    assert!(!g.text.contains("Constraint: "));
}
