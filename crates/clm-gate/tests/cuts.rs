//! What a spec branch cuts (gate/cuts), read from two spec graphs and the lists a change
//! declares, and the exemptions it widens (gate/needs-review-of).

use std::path::PathBuf;

use clm_gate::{
    Cut, Side, TestList, cuts, cuts_or_unloaded, list_lines, places_a_test, sentences,
    stating_sentences, widened_exemptions,
};
use clm_model::Graph;

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n";

/// A component spec with `extra` frontmatter lines and `requirements` under its heading.
fn component(id: &str, extra: &str, requirements: &str) -> String {
    format!(
        "---\ntitle: {id}\nstatus: active\nowners: []\npaths: [\"src/{id}/**\"]\n{extra}---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n{requirements}"
    )
}

/// A requirement whose statement is `statement`, with one scenario per name.
fn requirement(slug: &str, statement: &str, scenarios: &[&str]) -> String {
    let mut out = format!("\n### Requirement: {slug}\n{statement}\n");
    for name in scenarios {
        out.push_str(&format!(
            "\n#### Scenario: {name}\n- GIVEN g\n- WHEN w\n- THEN t\n"
        ));
    }
    out
}

/// The graph of a root and the components `(id, extra frontmatter, requirements)`.
fn graph(components: &[(&str, &str, String)]) -> Graph {
    let mut files = clm_model::Files::new();
    files.insert(PathBuf::from("specs/SPEC.md"), ROOT.as_bytes().to_vec());
    for (id, extra, reqs) in components {
        files.insert(
            PathBuf::from(format!("specs/{id}/SPEC.md")),
            component(id, extra, reqs).into_bytes(),
        );
    }
    let (graph, violations) = clm_model::load_files(&files);
    assert!(violations.is_empty(), "{violations:?}");
    graph
}

fn lines(cuts: &[Cut]) -> Vec<String> {
    cuts.iter().map(ToString::to_string).collect()
}

const X: &str = "X SHALL hold.";

#[test]
fn scenario_from_two_graphs() {
    // req:gate/cuts@35c052d4
    let base = graph(&[("a", "", requirement("x", X, &["first", "second"]))]);
    let head = graph(&[("a", "", requirement("x", X, &["first"]))]);
    let found = cuts(&base, &head, &[]);
    assert_eq!(
        found,
        [Cut::Scenario {
            spec: PathBuf::from("specs/a/SPEC.md"),
            id: "a/x".into(),
            name: "second".into(),
        }]
    );
    assert_eq!(
        lines(&found),
        ["specs/a/SPEC.md: gate/cut: scenario \"second\" of a/x is gone"]
    );
    // nothing cut the other way round, or between equal graphs
    assert_eq!(cuts(&head, &base, &[]), []);
    assert_eq!(cuts(&base, &base, &[]), []);
}

#[test]
fn a_gone_requirement_and_its_order() {
    // req:gate/cuts@35c052d4
    let two = format!(
        "{}{}",
        requirement("x", X, &["first"]),
        requirement("y", "Y SHALL hold. It SHALL exit 1.", &["s"])
    );
    let base = graph(&[
        ("a", "", two),
        ("b", "", requirement("z", X, &["one", "two"])),
    ]);
    let head = graph(&[
        ("a", "", requirement("y", "Y SHALL hold.", &["s"])),
        ("b", "", requirement("z", X, &["one"])),
    ]);
    assert_eq!(
        lines(&cuts(&base, &head, &[])),
        [
            "specs/a/SPEC.md: gate/cut: requirement a/x is gone",
            "specs/b/SPEC.md: gate/cut: scenario \"two\" of b/z is gone",
            "specs/a/SPEC.md: gate/cut: a/y states 1 sentences where it stated 2",
        ]
    );
}

#[test]
fn moved_to_another_component() {
    // req:gate/cuts@35c052d4
    let base = graph(&[
        ("a", "", requirement("x", X, &["the chain"])),
        ("b", "", requirement("y", X, &["other"])),
    ]);
    let head = graph(&[
        ("a", "", String::new()),
        ("b", "", requirement("y", X, &["other", "the chain"])),
    ]);
    assert_eq!(cuts(&base, &head, &[]), []);
}

#[test]
fn two_requirements_merged() {
    // req:gate/cuts@35c052d4
    let base = graph(&[(
        "a",
        "",
        format!(
            "{}{}",
            requirement("x", X, &["can edit"]),
            requirement("y", X, &["can edit"])
        ),
    )]);
    let head = graph(&[("a", "", requirement("z", X, &["can edit"]))]);
    assert_eq!(
        lines(&cuts(&base, &head, &[])),
        ["specs/a/SPEC.md: gate/cut: requirement a/y is gone"]
    );
}

#[test]
fn a_name_another_requirement_already_held() {
    // req:gate/cuts@35c052d4
    let base = graph(&[
        ("a", "", requirement("x", X, &["s", "not switched on"])),
        ("b", "", requirement("y", X, &["not switched on"])),
    ]);
    let head = graph(&[
        ("a", "", requirement("x", X, &["s"])),
        ("b", "", requirement("y", X, &["not switched on"])),
    ]);
    assert_eq!(
        lines(&cuts(&base, &head, &[])),
        ["specs/a/SPEC.md: gate/cut: scenario \"not switched on\" of a/x is gone"]
    );
}

#[test]
fn a_renamed_scenario_is_listed() {
    // req:gate/cuts@35c052d4
    let base = graph(&[("a", "", requirement("x", X, &["first", "second"]))]);
    let head = graph(&[("a", "", requirement("x", X, &["first", "the second one"]))]);
    assert_eq!(
        lines(&cuts(&base, &head, &[])),
        ["specs/a/SPEC.md: gate/cut: scenario \"second\" of a/x is gone"]
    );
}

#[test]
fn sentences_that_state_and_sentences_that_place() {
    // req:gate/cuts@35c052d4
    assert_eq!(
        stating_sentences(
            "The command SHALL print the list. It SHALL exit 1 when the list is empty."
        ),
        [
            "The command SHALL print the list.",
            "It SHALL exit 1 when the list is empty."
        ]
    );
    // a `.` not followed by a space ends no sentence; a sentence without SHALL states nothing
    assert_eq!(
        stating_sentences("It reads `a.b` first. Then it SHALL stop!\nWhy? It SHALL not?"),
        ["Then it SHALL stop!", "It SHALL not?"]
    );
    // SHALL as a word, not within one
    assert_eq!(
        stating_sentences("A SHALLOW pool. MARSHALL it."),
        Vec::<&str>::new()
    );
    // a sentence that places a test isn't counted, whatever its case
    assert_eq!(
        stating_sentences(
            "X SHALL hold. Every test of a's requirements SHALL sit in a's own files."
        ),
        ["X SHALL hold."]
    );
    assert_eq!(
        places_a_test("Every test SHALL sit in a's files."),
        Some("SHALL sit")
    );
    assert_eq!(
        places_a_test("Tests that Sat in the root SHALL move."),
        Some("sat in")
    );
    assert_eq!(
        places_a_test("The test that IS LEFT IN the file SHALL sit there."),
        Some("is left in")
    );
    assert_eq!(places_a_test("X SHALL hold."), None);
    assert_eq!(
        stating_sentences("Its test is left in `t.rs`, which SHALL run."),
        Vec::<&str>::new()
    );
}

#[test]
fn a_phrase_places_a_test_only_beside_the_word_test() {
    // req:gate/cuts@35c052d4
    // a phrase and neither word: a sentence of behaviour, which states
    assert_eq!(
        places_a_test("The directory SHALL be deleted once nothing else is left in it."),
        None
    );
    assert_eq!(
        stating_sentences(
            "The command SHALL print the list. The directory SHALL be deleted once nothing else is left in it."
        ),
        [
            "The command SHALL print the list.",
            "The directory SHALL be deleted once nothing else is left in it."
        ]
    );
    // `Tests`, and the `tests` of a path, are the word, whatever the case
    assert_eq!(
        places_a_test("Its TESTS SHALL sit in `crates/a`."),
        Some("SHALL sit")
    );
    assert_eq!(
        places_a_test("What SHALL sit in crates/clm/tests/ moves."),
        Some("SHALL sit")
    );
    // `tested` and `places_a_test` hold neither word
    assert_eq!(places_a_test("What is tested SHALL sit in a."), None);
    assert_eq!(places_a_test("`places_a_test` SHALL sit in a."), None);
    // the word in one sentence and the phrase in another: neither places a test
    let statement = "Every test SHALL pass. The file SHALL sit in a.";
    assert_eq!(stating_sentences(statement).len(), 2);
    assert!(
        sentences(statement)
            .iter()
            .all(|s| places_a_test(s).is_none())
    );
}

#[test]
fn sentences_are_every_sentence_that_isnt_empty() {
    // req:gate/cuts@35c052d4
    assert_eq!(
        sentences("It reads `a.b` first. Then it SHALL stop!\nWhy?  It SHALL not? "),
        [
            "It reads `a.b` first.",
            "Then it SHALL stop!",
            "Why?",
            "It SHALL not?"
        ]
    );
    assert_eq!(sentences("  "), Vec::<&str>::new());
    assert_eq!(sentences("No end"), ["No end"]);
}

#[test]
fn a_lost_sentence_and_a_reworded_one() {
    // req:gate/cuts@35c052d4
    let two = "The command SHALL print the list. It SHALL exit 1 when the list is empty.";
    let base = graph(&[("a", "", requirement("x", two, &["s"]))]);
    let head = graph(&[(
        "a",
        "",
        requirement("x", "The command SHALL print the list.", &["s"]),
    )]);
    assert_eq!(
        lines(&cuts(&base, &head, &[])),
        ["specs/a/SPEC.md: gate/cut: a/x states 1 sentences where it stated 2"]
    );
    // reworded, as many sentences: nothing
    let base = graph(&[(
        "a",
        "",
        requirement("x", "The command SHALL print the list.", &["s"]),
    )]);
    let head = graph(&[(
        "a",
        "",
        requirement("x", "`clm list` SHALL print one line per item.", &["s"]),
    )]);
    assert_eq!(cuts(&base, &head, &[]), []);
    // a sentence that places a test goes without a cut
    let placed = format!("{X} Every test of a's requirements SHALL sit in a's own files.");
    let base = graph(&[("a", "", requirement("x", &placed, &["s"]))]);
    let head = graph(&[("a", "", requirement("x", X, &["s"]))]);
    assert_eq!(cuts(&base, &head, &[]), []);
}

#[test]
fn list_lines_read_file_and_name() {
    // req:gate/cuts@35c052d4
    let text = "# tests to delete\n\nDELETE tests/t.rs::two\tmerged into a/y\r\nremove the old ones\nDELETE b.rs::mod::three\n";
    let found: Vec<(String, String)> = list_lines(text)
        .into_iter()
        .map(|l| (l.file, l.name))
        .collect();
    assert_eq!(
        found,
        [
            ("tests/t.rs".to_string(), "two".to_string()),
            ("b.rs".to_string(), "mod::three".to_string()),
        ]
    );
}

#[test]
fn tests_a_list_adds() {
    // req:gate/cuts@35c052d4
    let g = graph(&[("a", "", requirement("x", X, &["s"]))]);
    let lists = [
        TestList {
            path: "changes/c/deleted-tests.txt",
            head: "DELETE tests/t.rs::one\tkept\nDELETE tests/t.rs::two\tnew\n",
            base: Some("DELETE tests/t.rs::one\told reason\n"),
        },
        // an archived list whose base text is the live one's adds nothing
        TestList {
            path: "changes/archive/d/deleted-tests.txt",
            head: "DELETE tests/u.rs::three\n",
            base: Some("DELETE tests/u.rs::three\n"),
        },
        TestList {
            path: "changes/b/deleted-tests.txt",
            head: "DELETE tests/v.rs::four\n",
            base: None,
        },
    ];
    assert_eq!(
        lines(&cuts(&g, &g, &lists)),
        [
            "changes/b/deleted-tests.txt: gate/cut: test tests/v.rs::four is to be deleted",
            "changes/c/deleted-tests.txt: gate/cut: test tests/t.rs::two is to be deleted",
        ]
    );
}

#[test]
fn the_unloaded_line() {
    // req:gate/cuts@35c052d4
    let cut = Cut::Unloaded {
        revision: "abc123".into(),
    };
    assert_eq!(
        cut.to_string(),
        "specs: gate/cut: the specs at abc123 don't load, so what the branch cuts isn't listed"
    );
}

#[test]
fn cuts_or_unloaded_lists_by_the_rule_for_specs_that_dont_load() {
    // req:gate/cuts@35c052d4
    let base = graph(&[("a", "", requirement("x", X, &["first", "second"]))]);
    let head = graph(&[("a", "", requirement("x", X, &["first"]))]);
    let broken = [clm_model::Violation {
        path: PathBuf::from("specs/a/SPEC.md"),
        rule: "model/scenario",
        message: "a requirement holds no scenario".into(),
    }];
    let side = |graph, violations, revision| Side {
        graph,
        violations,
        revision,
    };
    let list = [TestList {
        path: "changes/c/deleted-tests.txt",
        head: "DELETE tests/t.rs::two\n",
        base: None,
    }];
    // both load: what `cuts` gives
    assert_eq!(
        cuts_or_unloaded(&side(&base, &[], "b1"), &side(&head, &[], "h1"), &list),
        cuts(&base, &head, &list)
    );
    // the head doesn't load: the one line naming it, then the lists' tests
    let none = Graph::default();
    assert_eq!(
        lines(&cuts_or_unloaded(
            &side(&base, &[], "b1"),
            &side(&none, &broken, "h1"),
            &list
        )),
        [
            "specs: gate/cut: the specs at h1 don't load, so what the branch cuts isn't listed",
            "changes/c/deleted-tests.txt: gate/cut: test tests/t.rs::two is to be deleted",
        ]
    );
    // both fail: the merge base is named, once
    assert_eq!(
        cuts_or_unloaded(
            &side(&none, &broken, "b1"),
            &side(&none, &broken, "h1"),
            &[]
        ),
        [Cut::Unloaded {
            revision: "b1".into()
        }]
    );
}

const NARROW: &str = "exemptions:\n  - rule: undeclared-imports\n    files: [\"tests/a/**\"]\n    until: backlog/r\n";
const WIDE: &str = "exemptions:\n  - rule: undeclared-imports\n    files: [\"tests/a/**\", \"tests/b/**\"]\n    until: backlog/r\n";

#[test]
fn scenario_an_exemption_from_two_graphs() {
    // req:gate/needs-review-of@60afa4d1
    let base = graph(&[("a", NARROW, requirement("x", X, &["s"]))]);
    let head = graph(&[("a", WIDE, requirement("x", X, &["s"]))]);
    let widened = widened_exemptions(&base, &head);
    assert_eq!(widened.len(), 1);
    let (node, exemption) = widened[0];
    assert_eq!(node.id, "a");
    assert_eq!(exemption.files, ["tests/a/**", "tests/b/**"]);
    assert_eq!(exemption.until, "backlog/r");
    assert!(widened_exemptions(&head, &base).is_empty());
    // a new component counts every exemption; another `until` is another exemption
    let new = graph(&[
        ("a", NARROW, requirement("x", X, &["s"])),
        ("b", NARROW, requirement("x", X, &["s"])),
    ]);
    assert_eq!(widened_exemptions(&base, &new).len(), 1);
    let moved = graph(&[(
        "a",
        &NARROW.replace("backlog/r", "backlog/q"),
        requirement("x", X, &["s"]),
    )]);
    assert_eq!(widened_exemptions(&base, &moved).len(), 1);
    // only `why` changed
    let why = graph(&[(
        "a",
        &format!("{NARROW}    why: because\n"),
        requirement("x", X, &["s"]),
    )]);
    assert!(widened_exemptions(&base, &why).is_empty());
}

/// A requirement whose scenarios are `(name, lines)`, each block's lines as written.
fn raw(slug: &str, statement: &str, scenarios: &[(&str, &str)]) -> String {
    let mut out = format!("\n### Requirement: {slug}\n{statement}\n");
    for (name, body) in scenarios {
        out.push_str(&format!("\n#### Scenario: {name}\n{body}\n"));
    }
    out
}

/// The cut lines of `a/x` going from the scenarios `base` to `head`.
fn outcome_cuts(base: &[(&str, &str)], head: &[(&str, &str)]) -> Vec<String> {
    let base = graph(&[("a", "", raw("x", X, base))]);
    let head = graph(&[("a", "", raw("x", X, head))]);
    lines(&cuts(&base, &head, &[]))
}

const NONE: [String; 0] = [];

const EXITS_1: &str = "- GIVEN a test renamed in its file\n- WHEN the land runs\n- THEN it exits 1";

#[test]
fn a_changed_outcome_from_two_graphs() {
    // req:gate/cuts@35c052d4
    let changed =
        ["specs/a/SPEC.md: gate/cut: scenario \"a renamed test\" of a/x changes its outcome"];
    let base = [("a renamed test", EXITS_1)];
    // reversed
    let lands = EXITS_1.replace("it exits 1", "it lands");
    assert_eq!(outcome_cuts(&base, &[("a renamed test", &lands)]), changed);
    let found = cuts(
        &graph(&[("a", "", raw("x", X, &base))]),
        &graph(&[("a", "", raw("x", X, &[("a renamed test", &lands)]))]),
        &[],
    );
    assert_eq!(
        found,
        [Cut::Outcome {
            spec: PathBuf::from("specs/a/SPEC.md"),
            id: "a/x".into(),
            name: "a renamed test".into(),
        }]
    );
    // reflowed, its white space Unicode's, or only its setting changed: no cut
    let reflowed = EXITS_1.replace("it exits 1", "it\n  exits 1  ");
    assert_eq!(outcome_cuts(&base, &[("a renamed test", &reflowed)]), NONE);
    let nbsp = EXITS_1.replace("exits 1", "exits\u{a0}1");
    assert_eq!(outcome_cuts(&base, &[("a renamed test", &nbsp)]), NONE);
    let setting = EXITS_1.replace("renamed in its file", "renamed into another file");
    assert_eq!(outcome_cuts(&base, &[("a renamed test", &setting)]), NONE);
    // a word joined to the next is a change
    let joined = EXITS_1.replace("it exits 1", "itexits 1");
    assert_eq!(outcome_cuts(&base, &[("a renamed test", &joined)]), changed);
}

#[test]
fn an_outcome_runs_from_its_then_line_to_the_blocks_end() {
    // req:gate/cuts@35c052d4
    let changed = |name: &str| {
        vec![format!(
            "specs/a/SPEC.md: gate/cut: scenario \"{name}\" of a/x changes its outcome"
        )]
    };
    // an indented `- THEN` starts it, and the lines after it, an `- AND` among them, are in it
    let indented = "- GIVEN g\n  - THEN t\n- AND u";
    let and_v = indented.replace("- AND u", "- AND v");
    assert_eq!(
        outcome_cuts(&[("s", indented)], &[("s", &and_v)]),
        changed("s")
    );
    // a line before the first `- THEN` is not
    let two = "- GIVEN g\n- THEN t\n- THEN u";
    let given_h = two.replace("- GIVEN g", "- GIVEN h");
    assert_eq!(outcome_cuts(&[("s", two)], &[("s", &given_h)]), NONE);
    let then_v = two.replace("- THEN u", "- THEN v");
    assert_eq!(outcome_cuts(&[("s", two)], &[("s", &then_v)]), changed("s"));
    // case is kept: `- then` starts no outcome, so the scenario's is empty
    let lower = "- GIVEN g\n- then t";
    let lower_u = lower.replace("then t", "then u");
    assert_eq!(outcome_cuts(&[("s", lower)], &[("s", &lower_u)]), NONE);
    assert_eq!(
        outcome_cuts(&[("s", lower)], &[("s", "- GIVEN g\n- THEN t")]),
        changed("s")
    );
    // a heading inside a fence doesn't end the block, and the fence's lines are read as any
    let fenced = "- THEN it prints\n```\n#### not a heading\nold\n```";
    let new = fenced.replace("old", "new");
    assert_eq!(outcome_cuts(&[("s", fenced)], &[("s", &new)]), changed("s"));
    // the next heading ends the block: a `#### Needs` after the last scenario is not in it
    let needing = |other: &str, then: &str| {
        let b = format!(
            "{}{}",
            requirement("y", X, &["s"]),
            requirement("z", X, &["s"])
        );
        let a = format!("{}\n#### Needs\n- {other}\n", raw("x", X, &[("s", then)]));
        graph(&[("a", "", a), ("b", "", b)])
    };
    let base = needing("b/y", "- THEN t");
    assert_eq!(cuts(&base, &needing("b/z", "- THEN t"), &[]), []);
    assert_eq!(
        lines(&cuts(&base, &needing("b/z", "- THEN u"), &[])),
        changed("s")
    );
}

#[test]
fn scenarios_of_one_name_compare_by_their_place_among_it() {
    // req:gate/cuts@35c052d4
    let line =
        ["specs/a/SPEC.md: gate/cut: scenario \"not switched on\" of a/x changes its outcome"];
    let first = ("not switched on", "- THEN it exits 0");
    let second = ("not switched on", "- THEN it exits 1");
    let third = ("not switched on", "- THEN it exits 2");
    // two scenarios of one name, the second's outcome changed: one cut
    assert_eq!(outcome_cuts(&[first, second], &[first, third]), line);
    // a duplicate the head loses
    assert_eq!(outcome_cuts(&[first, first], &[first]), line);
    // one the head gains is no cut, nor one whose place among other names moves
    assert_eq!(outcome_cuts(&[first], &[first, second]), NONE);
    assert_eq!(
        outcome_cuts(&[("s", "- THEN u"), first], &[first, ("s", "- THEN u")]),
        NONE
    );
}

#[test]
fn changed_outcomes_follow_gone_scenarios_and_precede_shortened_requirements() {
    // req:gate/cuts@35c052d4
    let t = "- THEN t";
    let u = "- THEN u";
    let base = graph(&[
        (
            "a",
            "",
            raw(
                "x",
                "X SHALL hold. It SHALL exit 1.",
                &[("p", t), ("q", t), ("r", t)],
            ),
        ),
        ("b", "", raw("y", X, &[("s", t), ("gone", t)])),
        ("c", "", raw("z", X, &[("s", t)])),
    ]);
    let head = graph(&[
        (
            "a",
            "",
            raw("x", "X SHALL hold.", &[("r", u), ("q", t), ("p", u)]),
        ),
        ("b", "", raw("y", X, &[("s", u)])),
        ("c", "", String::new()),
    ]);
    assert_eq!(
        lines(&cuts(&base, &head, &[])),
        [
            "specs/c/SPEC.md: gate/cut: requirement c/z is gone",
            "specs/b/SPEC.md: gate/cut: scenario \"gone\" of b/y is gone",
            "specs/a/SPEC.md: gate/cut: scenario \"p\" of a/x changes its outcome",
            "specs/a/SPEC.md: gate/cut: scenario \"r\" of a/x changes its outcome",
            "specs/b/SPEC.md: gate/cut: scenario \"s\" of b/y changes its outcome",
            "specs/a/SPEC.md: gate/cut: a/x states 1 sentences where it stated 2",
        ]
    );
}

#[test]
fn a_moved_scenario_is_not_compared() {
    // req:gate/cuts@35c052d4
    let base = graph(&[
        ("a", "", raw("x", X, &[("the chain", "- THEN t")])),
        ("b", "", raw("y", X, &[("other", "- THEN t")])),
    ]);
    let head = graph(&[
        ("a", "", String::new()),
        (
            "b",
            "",
            raw("y", X, &[("other", "- THEN t"), ("the chain", "- THEN u")]),
        ),
    ]);
    assert_eq!(cuts(&base, &head, &[]), []);
}
