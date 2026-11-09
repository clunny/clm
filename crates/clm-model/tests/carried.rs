//! CLM's carried constraints: the compiled copy of `ARCHITECTURE.md`, the source, and a
//! project's `#### Replaces`.
// clm:reads ARCHITECTURE.md

use std::path::{Path, PathBuf};

use clm_model::{Files, Graph, Tier, Violation, carried, load_files};

const ROOT: &str = "---\ntitle: T\nstatus: active\nowners: []\n---\n\n## Contract\nc\n";

fn entry(slug: &str, extra: &str) -> String {
    format!(
        "### Constraint: {slug}\nIt SHALL hold.\n\nMore.\n\n{extra}#### Scenario: s\n- GIVEN a\n- THEN b\n\n"
    )
}

fn arch(entries: &[String]) -> String {
    format!("# Architecture\n\n## Constraints\n\n{}", entries.concat())
}

fn load(architecture: Option<&str>, capabilities: Option<&str>) -> (Graph, Vec<Violation>) {
    let mut files = Files::new();
    files.insert(PathBuf::from("specs/SPEC.md"), ROOT.into());
    for (path, text) in [
        ("ARCHITECTURE.md", architecture),
        ("CAPABILITIES.md", capabilities),
    ] {
        if let Some(text) = text {
            files.insert(PathBuf::from(path), text.as_bytes().to_vec());
        }
    }
    load_files(&files)
}

fn slugs(entries: &[clm_model::CarriedEntry]) -> Vec<&str> {
    entries.iter().map(|e| e.slug.as_str()).collect()
}

/// The violations, each as its rule, its path and the message, for the assertions.
fn reports(violations: &[Violation], path: &str, wanted: &[&[&str]]) {
    assert_eq!(violations.len(), wanted.len(), "{violations:?}");
    for names in wanted {
        let found = violations.iter().any(|v| {
            v.rule == "model/human-layers"
                && v.path == Path::new(path)
                && names.iter().all(|n| v.message.contains(n))
        });
        assert!(found, "{names:?} in {violations:?}");
    }
}

#[test]
fn the_source() {
    // req:model/carried-constraints@7b7f8a74
    let a = entry("a", "#### Carries\n- method\n\n");
    let b = entry(
        "b",
        "#### Carries\n- replaceable\n\n#### Guarantee\nIt stays green.\n\n",
    );
    let c = entry("c", "");
    let (graph, violations) = load(Some(&arch(&[a, b, c])), None);
    assert!(violations.is_empty(), "{violations:?}");
    let carried = &graph.carried;
    assert!(carried.source);
    assert_eq!(slugs(&carried.entries), ["a", "b"]);
    assert_eq!(carried.entries, carried.in_force);
    assert_eq!(carried.entries[0].tier, Tier::Method);
    assert_eq!(carried.entries[0].statement, "It SHALL hold.\n\nMore.");
    assert_eq!(carried.entries[0].guarantee, None);
    assert_eq!(carried.entries[1].tier, Tier::Replaceable);
    assert_eq!(
        carried.entries[1].guarantee.as_deref(),
        Some("It stays green.")
    );
}

#[test]
fn a_project() {
    // req:model/carried-constraints@7b7f8a74
    let x = entry("x", "#### Replaces\n- expand-contract\n- yagni\n\n");
    let (graph, violations) = load(Some(&arch(&[x])), None);
    assert!(violations.is_empty(), "{violations:?}");
    let carried = &graph.carried;
    assert!(!carried.source);
    let all = slugs(&carried.entries);
    for slug in ["proven-by-tests", "expand-contract", "yagni"] {
        assert!(all.contains(&slug), "{slug}");
    }
    let force = slugs(&carried.in_force);
    assert!(force.contains(&"proven-by-tests"));
    assert!(!force.contains(&"expand-contract") && !force.contains(&"yagni"));
    assert_eq!(carried.entries, clm_model::carried());
    assert_eq!(carried.entries.len() - 2, carried.in_force.len());
}

#[test]
fn no_architecture_file() {
    // req:model/carried-constraints@7b7f8a74
    let (graph, violations) = load(None, None);
    assert!(violations.is_empty(), "{violations:?}");
    assert!(!graph.carried.source);
    assert!(!graph.carried.entries.is_empty());
    assert_eq!(graph.carried.entries, graph.carried.in_force);
    assert_eq!(graph.carried.entries, carried());
}

#[test]
fn the_compiled_copy_is_this_repositorys_file() {
    // req:model/carried-constraints@7b7f8a74
    let own = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ARCHITECTURE.md"),
    )
    .unwrap();
    let (graph, violations) = load(Some(&own), None);
    // The entries' `Enforced by` IDs name rules of a graph this one doesn't hold.
    let violations: Vec<_> = violations
        .iter()
        .filter(|v| !v.message.contains("is enforced by"))
        .collect();
    assert!(violations.is_empty(), "{violations:?}");
    assert!(graph.carried.source);
    assert_eq!(graph.carried.entries, carried());
    for e in carried().iter().filter(|e| e.tier == Tier::Replaceable) {
        assert!(e.guarantee.is_some(), "{}", e.slug);
    }
}

#[test]
fn bad_replaces() {
    // req:model/carried-constraints@7b7f8a74
    let x = entry("x", "#### Replaces\n- proven-by-tests\n- no-such-entry\n\n");
    let own = entry("yagni", "");
    let (_, violations) = load(Some(&arch(&[x, own])), None);
    reports(
        &violations,
        "ARCHITECTURE.md",
        &[
            &["`x`", "proven-by-tests"],
            &["`x`", "no-such-entry"],
            &["`yagni`"],
        ],
    );
}

#[test]
fn malformed_replaces() {
    // req:model/carried-constraints@7b7f8a74
    let empty = entry("e", "#### Replaces\n\n");
    let junk = entry("j", "#### Replaces\n- yagni\nreuse-first\n\n");
    let two = entry(
        "t",
        "#### Replaces\n- yagni\n\n#### Replaces\n- reuse-first\n\n",
    );
    let (graph, violations) = load(Some(&arch(&[empty, junk, two])), None);
    reports(
        &violations,
        "ARCHITECTURE.md",
        &[
            &["`e`", "empty"],
            &["`j`", "reuse-first"],
            &["`t`", "more than one"],
        ],
    );
    assert!(!graph.carried.in_force.iter().any(|e| e.slug == "yagni"));
}

#[test]
fn bad_carries() {
    // req:model/carried-constraints@7b7f8a74
    let a = entry("a", "#### Carries\n- optional\n\n");
    let b = entry("b", "#### Carries\n- replaceable\n\n");
    let c = entry(
        "c",
        "#### Carries\n- method\n\n#### Guarantee\nIt holds.\n\n",
    );
    let d = entry("d", "#### Replaces\n- yagni\n\n");
    let e = entry("e", "#### Guarantee\nIt holds.\n\n");
    let f = entry(
        "f",
        "#### Carries\n- replaceable\n\n#### Guarantee\ng\n\n#### Guarantee\nh\n\n",
    );
    let (_, violations) = load(Some(&arch(&[a, b, c, d, e, f])), None);
    reports(
        &violations,
        "ARCHITECTURE.md",
        &[&["`a`"], &["`b`"], &["`c`"], &["`d`"], &["`e`"], &["`f`"]],
    );
}

#[test]
fn duplicate_and_two_line_carries() {
    // req:model/carried-constraints@7b7f8a74
    let twice = entry("a", "#### Carries\n- method\n\n#### Carries\n- default\n\n");
    let lines = entry("b", "#### Carries\n- method\n- default\n\n");
    let (_, violations) = load(Some(&arch(&[twice, lines])), None);
    reports(
        &violations,
        "ARCHITECTURE.md",
        &[&["`a`", "more than one"], &["`b`", "exactly one line"]],
    );
}

#[test]
fn capabilities_hold_none_of_the_blocks() {
    // req:model/carried-constraints@7b7f8a74
    let cap = |extra: &str| {
        format!(
            "# Capabilities\n\n## Capabilities\n\n### Capability: c\nA person can.\n\n#### Why\nw\n\n{extra}#### Scenario: s\n- GIVEN a\n- THEN b\n"
        )
    };
    for block in [
        "#### Carries\n- method\n\n",
        "#### Guarantee\ng\n\n",
        "#### Replaces\n- yagni\n\n",
    ] {
        let (_, violations) = load(None, Some(&cap(block)));
        reports(&violations, "CAPABILITIES.md", &[&["`c`"]]);
    }
}
