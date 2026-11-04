//! `replica/capabilities-report` and `replica/architecture-report`: the reports `clm
//! capabilities` and `clm architecture` give, over the replica's tip and ledgers.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use clm_replica::{Change, Replica};
use common::*;
use serde_json::{Value, json};

const CAPABILITIES: &str = "## Capabilities\n\n### Capability: x\nA person can x.\n\n#### Why\nx why\n\n#### Scenario: s\n- ok\n";
const ARCHITECTURE: &str = "## Constraints\n\n### Constraint: e\nGit SHALL hold it.\n\n#### Scenario: s\n- ok\n\n#### Enforced by\n- c/r\n- c/u\n";

pub fn graph(files: &FileMap) -> clm_model::Graph {
    let files: clm_model::Files = files
        .iter()
        .map(|(path, bytes)| (PathBuf::from(path), bytes.clone()))
        .collect();
    clm_model::load_files(&files).0
}

/// A flag on `c/r` at its version in `f`.
fn flag(f: &mut FileMap) {
    let version = graph(f).requirement("c/r").unwrap().version().to_string();
    f.insert(
        "backlog/fr.md".into(),
        format!(
            "---\ntitle: R is hard\nkind: friction\ncomponents: [c]\nrequirement: c/r@{version}\n---\n\nIt needed a fake.\n"
        )
        .into_bytes(),
    );
}

/// `clm_repo` with `CAPABILITIES.md` holding `x`, a component `c` serving it with requirement
/// `r`, and a workaround flag on `c/r`.
fn capability_files() -> FileMap {
    let mut f = clm_repo();
    f.extend(files(&[
        ("CAPABILITIES.md", CAPABILITIES),
        (
            "specs/c/SPEC.md",
            "---\ntitle: C\nstatus: active\nowners: []\nserves: [capability/x]\n---\n\n## Contract\nc\n\n## Requirements\n\n### Requirement: r\nIt SHALL r.\n\n#### Scenario: s\n- ok\n",
        ),
    ]));
    flag(&mut f);
    f
}

/// `clm_repo` with `ARCHITECTURE.md` holding `e`, enforced by `c/r` (cited at its current
/// version, with a flag) and `c/u`.
fn architecture_files() -> FileMap {
    let mut f = clm_repo();
    f.extend(files(&[
        ("ARCHITECTURE.md", ARCHITECTURE),
        (
            "specs/c/SPEC.md",
            "---\ntitle: C\nstatus: active\nowners: []\n---\n\n## Contract\nc\n\n## Requirements\n\n### Requirement: r\nIt SHALL r.\n\n#### Scenario: s\n- ok\n\n#### Serves\n- architecture/e\n\n### Requirement: u\nIt SHALL u.\n\n#### Scenario: s\n- ok\n\n#### Serves\n- architecture/e\n",
        ),
    ]));
    flag(&mut f);
    let v = version_of(&f, "c/r");
    f.insert("tests/t.rs".into(), cites("c/r", &v).into_bytes());
    f
}

/// An `effort` record on `impl/c` worth a million Opus 5.5 input tokens.
fn effort() -> Value {
    json!({
        "id": "e1", "kind": "effort", "writer": "ann/1", "at": "2026-10-30T00:00:00.000Z",
        "branch": "impl/c", "session": "s1",
        "models": {"claude-opus-5-5": {"input": 1_000_000, "output": 0, "cache_read": 0, "cache_write": 0}},
        "active_seconds": 3600,
    })
}

fn with_effort(r: &mut Replica) {
    r.apply(Change::Ledger {
        branch: "clm/ledger/ann/1".into(),
        text: Some(format!("{}\n", effort())),
    });
}

fn fold() -> clm_ledger::Fold {
    clm_ledger::fold_records(&[("clm/ledger/ann/1", &format!("{}\n", effort()))])
}

#[test]
fn the_capabilities_report_clm_gives() {
    // req:replica/capabilities-report@2b1b6dac
    let f = capability_files();
    let mut r = scratch(&f, "111");
    with_effort(&mut r);
    r.recompute(100);
    let answer = view(&r, "capabilities");
    let flag_text = BTreeMap::from([("fr".to_string(), "It needed a fake.".to_string())]);
    let g = graph(&f);
    let want = clm_effort::report(&clm_effort::Inputs {
        graph: &g,
        fold: &fold(),
        history: None,
        flag_text: &flag_text,
    });
    assert_eq!(answer["report"], serde_json::to_value(&want).unwrap());
    assert_eq!(answer["signed"], false);
    assert_eq!(answer.as_object().unwrap().len(), 2);
    let x = &answer["report"]["capabilities"][0];
    assert_eq!(x["slug"], "x");
    assert_eq!(x["spend"]["dollars"], 4.0);
    assert_eq!(x["flags"][0]["text"], "It needed a fake.");
}

#[test]
fn a_tip_holding_allowed_signers_is_signed() {
    // req:replica/capabilities-report@2b1b6dac
    let mut f = capability_files();
    f.insert(
        "docs/allowed_signers".into(),
        b"dana ssh-ed25519 AAAA\n".to_vec(),
    );
    let r = ready(&f, "111", 100);
    assert_eq!(view(&r, "capabilities")["signed"], true);
    assert_eq!(view(&r, "architecture")["signed"], true);
}

#[test]
fn the_architecture_report_clm_gives() {
    // req:replica/architecture-report@03e92222
    let f = architecture_files();
    let mut r = scratch(&f, "111");
    with_effort(&mut r);
    r.recompute(100);
    let answer = view(&r, "architecture");
    let flag_text = BTreeMap::from([("fr".to_string(), "It needed a fake.".to_string())]);
    let g = graph(&f);
    let current = BTreeSet::from(["c/r".to_string()]);
    let want = clm_effort::architecture(
        &clm_effort::Inputs {
            graph: &g,
            fold: &fold(),
            history: None,
            flag_text: &flag_text,
        },
        Some(&current),
    );
    assert_eq!(answer["architecture"], serde_json::to_value(&want).unwrap());
    assert_eq!(answer["indexing"], false);
    assert_eq!(answer["signed"], false);
    assert_eq!(answer.as_object().unwrap().len(), 3);
    let e = &answer["architecture"][0];
    assert_eq!(e["enforced_by"][0]["current"], true);
    assert_eq!(
        e["enforced_by"][1]["current"], false,
        "c/u is cited by no test"
    );
    assert_eq!(e["spend"]["dollars"], 4.0);
}

#[test]
fn the_architecture_while_indexing_leaves_current_unknown() {
    // req:replica/architecture-report@03e92222
    let f = architecture_files();
    let mut r = Replica::default();
    r.apply(tree("111", &f));
    let spec: Vec<String> = r
        .pending_fetch()
        .into_iter()
        .filter(|p| p.ends_with(".md"))
        .collect();
    r.apply(Change::Tip {
        sha: "111".into(),
        files: spec.iter().map(|p| file(p, &f[p])).collect(),
        removed: Vec::new(),
    });
    r.recompute(100);
    assert!(!r.pending_fetch().is_empty(), "tests/t.rs is still to scan");
    let answer = view(&r, "architecture");
    assert_eq!(answer["indexing"], true);
    for rule in answer["architecture"][0]["enforced_by"].as_array().unwrap() {
        assert!(rule["current"].is_null(), "{rule}");
    }
}
