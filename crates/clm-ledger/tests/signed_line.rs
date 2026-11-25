//! `Fold::carries_sig` and `Fold::cleared_by_sig`, with or without the `git` feature
//! (`ledger/cleared-by-a-signed-line`).

use clm_ledger::{Fold, fold_records};

const REQUEST: &str = r#"{"id":"r","kind":"changes_requested","writer":"alice/web","at":"2026-10-30T07:10:00.000Z","branch":"spec/x","fingerprint":"f","text":"Split the requirement"}"#;

/// An approval `a` for `f` by `writer` whose `clears` holds `r`, with `sig` as given
/// (none when `None`).
fn approval(writer: &str, sig: Option<&str>) -> String {
    let sig = sig.map_or(String::new(), |s| format!(r#","sig":"{s}""#));
    format!(
        r#"{{"id":"a","kind":"approved","writer":"{writer}","at":"2026-10-30T07:11:00.000Z","fingerprint":"f","branch":"spec/x","change":null,"clears":["r"]{sig}}}"#
    )
}

fn fold_of(approval: &str) -> Fold {
    let text = format!("{REQUEST}\n{approval}\n");
    let fold = fold_records(&[("origin/clm/ledger/alice/web", &text)]);
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    fold
}

#[test]
fn a_signature_carried_not_checked() {
    // req:ledger/cleared-by-a-signed-line@55449c0e
    let cases = [
        (approval("alice/1a2b3c4d", None), false, false),
        (approval("alice/1a2b3c4d", Some("")), false, false),
        (approval("alice/1a2b3c4d", Some("x")), true, true),
        (approval("dana/9f9f9f9f", Some("x")), true, false),
    ];
    for (line, carries, cleared) in cases {
        let fold = fold_of(&line);
        let request = &fold.change_requests["spec/x"][0];
        assert_eq!(fold.carries_sig("a"), carries, "{line}");
        assert_eq!(fold.cleared_by_sig(request), cleared, "{line}");
        // `cleared` counts the unsigned ones too, so only the signature tells them apart.
        assert_eq!(fold.cleared(request), !line.contains("dana"), "{line}");
    }
    // A record the fold never read carries nothing.
    assert!(!fold_of(&approval("alice/1a2b3c4d", Some("x"))).carries_sig("nope"));
}
