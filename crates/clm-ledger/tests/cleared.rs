//! `Fold::cleared` and `person`, with or without the `git` feature (`ledger/request-cleared`).

use clm_ledger::{Body, Fold, Record, fold_records, person};

const REQUESTED: &str = "2026-10-30T07:10:00.000Z";
const LATER: &str = "2026-10-30T07:11:00.000Z";

fn record(id: &str, writer: &str, at: &str, body: Body) -> Record {
    Record {
        id: id.into(),
        writer: writer.into(),
        at: at.into(),
        body,
    }
}

fn request() -> Record {
    record(
        "r1",
        "alice/web",
        REQUESTED,
        Body::ChangesRequested {
            branch: "spec/x".into(),
            fingerprint: "f".into(),
            text: "Split the requirement".into(),
        },
    )
}

fn approval(id: &str, writer: &str, at: &str, fingerprint: &str) -> Record {
    record(
        id,
        writer,
        at,
        Body::Approved {
            fingerprint: fingerprint.into(),
            branch: "spec/x".into(),
            change: None,
        },
    )
}

fn fold_of(records: &[Record]) -> Fold {
    let text: String = records
        .iter()
        .map(|r| serde_json::to_string(r).unwrap() + "\n")
        .collect();
    let fold = fold_records(&[("origin/clm/ledger/alice/web", &text)]);
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    fold
}

#[test]
fn the_requester_approved_later() {
    // req:ledger/request-cleared@0fe654c6
    let fold = fold_of(&[request(), approval("a1", "alice/1a2b3c4d", LATER, "f")]);
    assert!(fold.cleared(&request()));
}

#[test]
fn not_cleared() {
    // req:ledger/request-cleared@0fe654c6
    let fold = fold_of(&[
        request(),
        approval("a1", "dana/9f9f9f9f", LATER, "f"),
        approval("a2", "alice/1a2b3c4d", REQUESTED, "f"),
        approval("a3", "alice/1a2b3c4d", LATER, "g"),
    ]);
    assert!(!fold.cleared(&request()));
}

#[test]
fn a_record_that_is_no_request_is_never_cleared() {
    // req:ledger/request-cleared@0fe654c6
    let earlier = approval("a0", "alice/web", REQUESTED, "f");
    let fold = fold_of(&[
        earlier.clone(),
        approval("a1", "alice/1a2b3c4d", LATER, "f"),
    ]);
    assert!(!fold.cleared(&earlier));
}

#[test]
fn person_of_a_writer() {
    // req:ledger/request-cleared@0fe654c6
    assert_eq!(person("alice/1a2b3c4d"), "alice");
    assert_eq!(person("alice"), "alice");
}

#[test]
fn a_signed_approval_counts_as_well() {
    // req:ledger/request-cleared@0fe654c6
    let mut line = serde_json::to_value(approval("a1", "alice/1a2b3c4d", LATER, "f")).unwrap();
    line["sig"] = "-----BEGIN SSH SIGNATURE-----\nx\n-----END SSH SIGNATURE-----".into();
    let text = format!("{}\n{}\n", serde_json::to_string(&request()).unwrap(), line);
    let fold = fold_records(&[("origin/clm/ledger/alice/web", &text)]);
    assert!(fold.lines["a1"].contains_key("sig"));
    assert!(fold.cleared(&request()));
}

#[test]
fn no_other_person_parser() {
    // req:ledger/request-cleared@0fe654c6
    // Splitting on `/` is how a writer's person is taken apart: only `person` does it.
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&src).unwrap() {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap();
        let file = path.file_name().unwrap().to_string_lossy().into_owned();
        for (n, line) in text.lines().enumerate() {
            if ["split('/')", "split_once('/')", "splitn(2, '/')"]
                .iter()
                .any(|call| line.contains(call))
            {
                found.push(format!("{file}:{}", n + 1));
            }
        }
    }
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("fold.rs:"), "{found:?}");
}
