//! `clm-gate` offers `clm-fingerprint`'s items under the same names (gate/fingerprint-reexported).

use std::path::Path;

fn listing(second_blob: &str) -> Vec<u8> {
    format!(
        "100644 blob {}\tsrc/a.rs\0100644 blob {second_blob}\tsrc/b.rs\0",
        "1".repeat(40)
    )
    .into_bytes()
}

#[test]
fn same_function_through_the_gate() {
    // req:gate/fingerprint-reexported@f5d8facf
    let base: Vec<clm_gate::Entry> = clm_gate::parse_ls_tree(&listing(&"2".repeat(40)));
    let head = clm_gate::parse_ls_tree(&listing(&"3".repeat(40)));
    assert_eq!(base.len(), 2);
    let through_gate = clm_gate::fingerprint_of(&base, &head);
    let direct = clm_fingerprint::fingerprint_of(
        &clm_fingerprint::parse_ls_tree(&listing(&"2".repeat(40))),
        &clm_fingerprint::parse_ls_tree(&listing(&"3".repeat(40))),
    );
    assert_eq!(through_gate, direct);
    // The other re-exports name the same items: coercing each to the original's
    // function type fails to compile if one is missing or a local copy.
    let _: fn(&[clm_fingerprint::Entry], &[clm_fingerprint::Entry]) -> bool = clm_gate::adds_change;
    let _: fn(&[clm_fingerprint::Entry], &[clm_fingerprint::Entry]) -> Vec<Vec<u8>> =
        clm_gate::added_changes;
    let _: fn(&Path, &str) -> Result<String, String> = clm_gate::fingerprint;
    assert_eq!(
        clm_gate::adds_change(&base, &head),
        clm_fingerprint::adds_change(&base, &head)
    );
}
