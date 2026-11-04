//! `web/.npmrc`, which keeps npm from running dependencies' install scripts.

use std::fs;
use std::path::Path;

/// Whether the last line of `npmrc` that sets `ignore-scripts` sets it to `true`.
fn ignores_scripts(npmrc: &str) -> bool {
    npmrc
        .lines()
        .filter_map(|l| {
            let (key, value) = l.trim().split_once('=')?;
            (key.trim() == "ignore-scripts").then(|| value.trim().to_string())
        })
        .next_back()
        .is_some_and(|v| v == "true")
}

// req:web/no-install-scripts@0d751f57
#[test]
fn npmrc() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/.npmrc");
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("can't read {}: {e}", path.display()));
    assert!(
        ignores_scripts(&text),
        "web/.npmrc must end up with ignore-scripts=true"
    );
}

#[test]
fn a_later_line_can_turn_scripts_back_on() {
    assert!(ignores_scripts("ignore-scripts=true\n"));
    assert!(!ignores_scripts(""));
    assert!(!ignores_scripts("ignore-scripts=false\n"));
    assert!(!ignores_scripts(
        "ignore-scripts=true\nignore-scripts=false\n"
    ));
}
