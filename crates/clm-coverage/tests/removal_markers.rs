//! `removal-markers`: the `clm:remove-by` markers `scan` finds, the ones
//! `clm unreferenced` accepts.

use std::fs;

use tempfile::TempDir;

mod common;
use common::outside_git;

// Fixture markers are built so the literal never appears whole in this file.
const MARK: &str = concat!("clm", ":remove-by ");

#[test]
fn scenario_what_may_come_before_a_marker() {
    // req:check.coverage/removal-markers@d4fd38e1
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    outside_git(d);
    let lines = [
        format!("// {MARK}20260101-plain"),
        format!("// é{MARK}20260101-accented"),
        format!("// ({MARK}20260101-bracketed"),
        format!("// x{MARK}20260101-letter"),
        format!("// 1{MARK}20260101-digit"),
        format!("// _{MARK}20260101-underscore"),
        format!("// {MARK}2026010-short"),
        format!("// {MARK}２０２６０１０１-wide"),
        format!("// {MARK}20260101-"),
        format!("// {MARK}soon"),
    ];
    fs::write(d.join("m.rs"), lines.join("\n") + "\n").unwrap();

    assert_eq!(
        removal_ids(d),
        ["20260101-accented", "20260101-bracketed", "20260101-plain"]
    );
}

#[test]
fn an_id_runs_from_eight_digits_to_its_last_id_character() {
    // req:check.coverage/removal-markers@d4fd38e1
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    outside_git(d);
    let lines = [
        format!("// {MARK}2026abcd-letters"),
        format!("// {MARK}2026-0101-dashes"),
        format!("x = \"{MARK}20260101-ends-here\"; {MARK}20260102-second.rs"),
    ];
    fs::write(d.join("m.rs"), lines.join("\n") + "\n").unwrap();
    assert_eq!(removal_ids(d), ["20260101-ends-here", "20260102-second"]);
}

/// The IDs of `scan`'s `remove_by` in `d`, sorted.
fn removal_ids(d: &std::path::Path) -> Vec<String> {
    let mut ids: Vec<String> = clm_coverage::scan(d)
        .remove_by
        .into_iter()
        .map(|m| m.id)
        .collect();
    ids.sort_unstable();
    ids
}
