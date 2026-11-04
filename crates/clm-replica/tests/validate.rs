//! `replica/validate-writes`.

mod common;

use clm_replica::{Change, Refusal, Replica, Validated, Write};
use common::*;
use serde_json::json;

const CAPABILITIES: &str = "## Capabilities\n\n### Capability: x\nA person can x.\n\n#### Why\nx why\n\n#### Scenario: s\n- ok\n";

fn repo() -> FileMap {
    let mut f = clm_repo();
    f.extend(files(&[
        ("CAPABILITIES.md", CAPABILITIES),
        (
            "specs/c/SPEC.md",
            "---\ntitle: C\nstatus: active\nowners: []\nserves: [capability/x]\n---\n\n## Contract\nc\n\n## Requirements\n\n### Requirement: r\nIt SHALL r.\n\n#### Scenario: s\n- ok\n",
        ),
        ("backlog/export-csv.md", "---\ntitle: Export CSV\nkind: idea\ncomponents: [a]\n---\nBody.\n"),
        ("changes/c9/change.md", "---\nstatus: applied\nresolves: [spent-item]\n---\n\n## Intent\nI.\n"),
    ]));
    f
}

fn version(f: &FileMap, id: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    write_checkout(dir.path(), f);
    let (graph, _) = clm_model::load(dir.path());
    graph.requirement(id).unwrap().version().to_string()
}

fn decision(id: &str, version: &str) -> Write {
    Write::Decision {
        id: id.into(),
        version: version.into(),
    }
}

fn proposal(title: &str, nodes: &[&str]) -> Write {
    Write::Proposal {
        title: title.into(),
        nodes: nodes.iter().map(|n| n.to_string()).collect(),
    }
}

#[test]
fn a_reworded_capability_is_stale() {
    // req:replica/validate-writes@a4cd62e9
    let f = repo();
    let r = ready(&f, "111", 100);
    let current = version(&f, "capability/x");
    assert_eq!(
        r.validate(&decision("capability/x", "00000000")),
        Err(Refusal::Stale)
    );
    assert_eq!(
        r.validate(&decision("capability/x", &current)),
        Ok(Validated::Decision {
            settles: Vec::new()
        })
    );
}

#[test]
fn an_entry_the_tip_doesnt_hold_is_unknown() {
    // req:replica/validate-writes@a4cd62e9
    let f = repo();
    let r = ready(&f, "111", 100);
    assert_eq!(
        r.validate(&decision("capability/nope", "00000000")),
        Err(Refusal::Unknown)
    );
    assert_eq!(
        r.validate(&decision("architecture/x", "00000000")),
        Err(Refusal::Unknown)
    );
    // A tip whose specs don't load, and a replica with no tip yet.
    let mut broken = f.clone();
    broken.insert("specs/b/SPEC.md".into(), b"not a spec".to_vec());
    let r = ready(&broken, "111", 100);
    let v = version(&f, "capability/x");
    assert_eq!(
        r.validate(&decision("capability/x", &v)),
        Err(Refusal::Unknown)
    );
    assert_eq!(
        Replica::default().validate(&decision("capability/x", &v)),
        Err(Refusal::Unknown)
    );
}

#[test]
fn signed_comes_before_stale() {
    // req:replica/validate-writes@a4cd62e9
    let mut f = repo();
    f.insert(
        "docs/allowed_signers".into(),
        b"dana ssh-ed25519 AAAA\n".to_vec(),
    );
    let r = ready(&f, "111", 100);
    assert_eq!(
        r.validate(&decision("capability/x", "00000000")),
        Err(Refusal::Signed)
    );
    let current = version(&f, "capability/x");
    assert_eq!(
        r.validate(&decision("capability/x", &current)),
        Err(Refusal::Signed)
    );
    assert_eq!(
        r.validate(&decision("capability/nope", &current)),
        Err(Refusal::Unknown)
    );
}

#[test]
fn a_decision_gives_the_flags_it_settles() {
    // req:replica/validate-writes@a4cd62e9
    let mut f = repo();
    let v = version(&f, "c/r");
    f.insert(
        "backlog/fr.md".into(),
        format!("---\ntitle: R is hard\nkind: friction\ncomponents: [c]\nrequirement: c/r@{v}\n---\n\nIt needed a fake.\n")
            .into_bytes(),
    );
    let mut r = ready(&f, "111", 100);
    let current = version(&f, "capability/x");
    assert_eq!(
        r.validate(&decision("capability/x", &current)),
        Ok(Validated::Decision {
            settles: vec!["fr".into()]
        })
    );
    // A decision in the ledgers settles it, so the next decision settles nothing.
    let decided = json!({
        "id": "d1", "kind": "decided", "writer": "ann/web", "at": "2026-10-30T00:00:00.000Z",
        "capability": "capability/x", "value": "high", "decision": "keep", "settles": ["fr"],
    });
    r.apply(Change::Ledger {
        branch: "clm/ledger/ann/web".into(),
        text: Some(format!("{decided}\n")),
    });
    assert_eq!(
        r.validate(&decision("capability/x", &current)),
        Ok(Validated::Decision {
            settles: Vec::new()
        })
    );
}

#[test]
fn a_taken_slug() {
    // req:replica/validate-writes@a4cd62e9
    let mut r = ready(&repo(), "111", 100);
    assert_eq!(
        r.validate(&proposal("Export CSV", &["a"])),
        Err(Refusal::Taken)
    );
    // A slug a change's `resolves` holds is spent too.
    assert_eq!(
        r.validate(&proposal("Spent item", &["a"])),
        Err(Refusal::Taken)
    );
    assert_eq!(
        r.validate(&proposal("Dark mode", &["a", "b"])),
        Ok(Validated::Proposal {
            slug: "dark-mode".into()
        })
    );
    // An open proposal of the fold takes it, a withdrawn one doesn't.
    let proposed = json!({
        "id": "p1", "kind": "proposed", "writer": "ann/web", "at": "2026-10-30T00:00:00.000Z",
        "slug": "dark-mode", "title": "Dark mode", "body": "b", "nodes": [],
    });
    r.apply(Change::Ledger {
        branch: "clm/ledger/ann/web".into(),
        text: Some(format!("{proposed}\n")),
    });
    assert_eq!(r.validate(&proposal("Dark mode", &[])), Err(Refusal::Taken));
    let withdrawn = json!({
        "id": "p2", "kind": "withdrawn", "writer": "ann/web", "at": "2026-10-30T00:01:00.000Z",
        "proposal": "p1",
    });
    r.apply(Change::Ledger {
        branch: "clm/ledger/ann/web".into(),
        text: Some(format!("{proposed}\n{withdrawn}\n")),
    });
    assert_eq!(
        r.validate(&proposal("Dark mode", &[])),
        Ok(Validated::Proposal {
            slug: "dark-mode".into()
        })
    );
}

#[test]
fn a_component_the_graph_lacks_is_invalid() {
    // req:replica/validate-writes@a4cd62e9
    let r = ready(&repo(), "111", 100);
    assert_eq!(
        r.validate(&proposal("Dark mode", &["nope"])),
        Err(Refusal::Invalid)
    );
    assert_eq!(r.validate(&proposal("!!!", &[])), Err(Refusal::Invalid));
}

#[test]
fn slugs_as_clm_defect_does() {
    // req:replica/validate-writes@a4cd62e9
    let r = ready(&repo(), "111", 100);
    let slug = |title: &str| match r.validate(&proposal(title, &[])) {
        Ok(Validated::Proposal { slug }) => slug,
        other => panic!("{other:?}"),
    };
    assert_eq!(slug("  Dark mode: on/off!  "), "dark-mode-on-off");
    assert_eq!(slug("Ünïcode only"), "n-code-only");
    let long = "a".repeat(70);
    assert_eq!(slug(&long), "a".repeat(64));
    let words = format!("{} bb", "a".repeat(62));
    assert_eq!(
        slug(&words),
        "a".repeat(62),
        "a word that doesn't fit is left out"
    );
}
