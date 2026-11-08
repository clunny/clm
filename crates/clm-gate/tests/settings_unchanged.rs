//! `settings_unchanged` (gate/settings-unchanged): whether `clm.toml`'s settings are the
//! same once the upgrade notes, comments and formatting are set aside.

use clm_gate::{settings_unchanged, upgrade_stamp_only};

const BEFORE: &[u8] = b"approval = \"other\"\nupgraded = \"0.17.0\"\n[land]\ncheck = \"a\"\n";

#[test]
fn the_stamp_only() {
    // req:gate/settings-unchanged@e141f56b
    let after = b"approval = \"other\"\nupgraded = \"0.18.0\"\ndeclined_migrations = [\"m\"]\n[land]\ncheck = \"a\"\n";
    assert!(settings_unchanged(Some(BEFORE), Some(after)));
    assert!(upgrade_stamp_only(Some(BEFORE), Some(after)));
}

#[test]
fn a_comment_only() {
    // req:gate/settings-unchanged@e141f56b
    let mut after = BEFORE.to_vec();
    after.extend_from_slice(b"# [verify]\n# check = \"x\"\n");
    assert!(settings_unchanged(Some(BEFORE), Some(&after)));
    assert!(!upgrade_stamp_only(Some(BEFORE), Some(&after)));
}

#[test]
fn no_change_and_missing_files() {
    // req:gate/settings-unchanged@e141f56b
    assert!(settings_unchanged(Some(BEFORE), Some(BEFORE)));
    assert!(settings_unchanged(None, None));
    assert!(settings_unchanged(None, Some(b"upgraded = \"1\"\n")));
    assert!(!settings_unchanged(None, Some(BEFORE)));
}

#[test]
fn a_setting() {
    // req:gate/settings-unchanged@e141f56b
    let after = b"approval = \"other\"\nupgraded = \"0.17.0\"\n[land]\ncheck = \"b\"\n";
    assert!(!settings_unchanged(Some(BEFORE), Some(after)));
}

#[test]
fn only_top_level_notes_are_left_out() {
    // req:gate/settings-unchanged@e141f56b
    let a = b"[land]\nupgraded = \"1\"\n";
    let b = b"[land]\nupgraded = \"2\"\n";
    assert!(!settings_unchanged(Some(a), Some(b)));
    assert!(settings_unchanged(
        Some(b"declined_migrations = [\"m\"]\n"),
        Some(b"")
    ));
}

#[test]
fn unreadable() {
    // req:gate/settings-unchanged@e141f56b
    assert!(!settings_unchanged(Some(BEFORE), Some(b"upgraded = [\n")));
    assert!(!settings_unchanged(Some(&[0xff, 0xfe]), Some(BEFORE)));
    assert!(!settings_unchanged(
        Some(&[0xff, 0xfe]),
        Some(&[0xff, 0xfe])
    ));
}
