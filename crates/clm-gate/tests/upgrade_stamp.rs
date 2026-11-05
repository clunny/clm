//! `upgrade_stamp_only` (gate/upgrade-stamp-left-out): whether `clm.toml` differs only in
//! the upgrade notes.

use clm_gate::upgrade_stamp_only;

const BEFORE: &[u8] = b"approval = \"other\"\nupgraded = \"0.17.0\"\n[land]\ncheck = \"a\"\n";

#[test]
fn only_the_notes_differ() {
    // req:gate/upgrade-stamp-left-out@5b8d7f0d
    let after = b"# stamped\napproval = \"other\"\nupgraded = \"0.18.0\"\ndeclined_migrations = [\"m\"]\n[land]\ncheck = \"a\"\n";
    assert!(upgrade_stamp_only(Some(BEFORE), Some(after)));
    // a missing file or key counts as unset
    assert!(upgrade_stamp_only(None, Some(b"upgraded = \"0.18.0\"\n")));
    assert!(upgrade_stamp_only(Some(b"upgraded = \"0.18.0\"\n"), None));
}

#[test]
fn any_other_key_differing_is_not_the_stamp() {
    // req:gate/upgrade-stamp-left-out@5b8d7f0d
    let other_setting = b"approval = \"other\"\nupgraded = \"0.18.0\"\n[land]\ncheck = \"b\"\n";
    assert!(!upgrade_stamp_only(Some(BEFORE), Some(other_setting)));
    let policy = b"approval = \"any\"\nupgraded = \"0.18.0\"\n[land]\ncheck = \"a\"\n";
    assert!(!upgrade_stamp_only(Some(BEFORE), Some(policy)));
}

#[test]
fn a_side_that_doesnt_parse_is_not_the_stamp() {
    // req:gate/upgrade-stamp-left-out@5b8d7f0d
    assert!(!upgrade_stamp_only(Some(BEFORE), Some(b"upgraded = [\n")));
    assert!(!upgrade_stamp_only(Some(BEFORE), Some(&[0xff, 0xfe])));
}
