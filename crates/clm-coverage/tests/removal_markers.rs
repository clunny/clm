//! `removal-markers`: the `clm:remove-by` markers `scan` finds, the ones
//! `clm unreferenced` accepts.

use std::fs;

use tempfile::TempDir;

// Fixture markers are built so the literal never appears whole in this file.
const MARK: &str = concat!("clm", ":remove-by ");

#[test]
fn scenario_what_may_come_before_a_marker() {
    // req:check.coverage/removal-markers@d4fd38e1
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    // SAFETY: nextest runs each test in a process of its own.
    unsafe {
        for var in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"] {
            std::env::remove_var(var);
        }
        std::env::set_var("GIT_CEILING_DIRECTORIES", d.parent().unwrap());
    }
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

    let scan = clm_coverage::scan(d);
    let mut ids: Vec<&str> = scan.remove_by.iter().map(|m| m.id.as_str()).collect();
    ids.sort_unstable();
    assert_eq!(
        ids,
        ["20260101-accented", "20260101-bracketed", "20260101-plain"]
    );
}
