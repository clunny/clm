//! `CONTRIBUTING.md` tells a contributor how to set a clone up and what CLM accepts
//! (clm/contributing-doc, clm/contributing-intent-only).

use std::fs;
use std::path::Path;

fn contributing() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../CONTRIBUTING.md");
    fs::read_to_string(path).unwrap()
}

#[test]
fn contributing_names_the_dev_install_setup() {
    // req:clm/contributing-doc@dc301880
    let text = contributing();
    for needle in [
        "scripts/dev-install.sh",
        "--remove",
        "jq",
        "cargo",
        "claude",
        "clm-dev",
        ".clm/dev-marketplace.json",
        "settings.local.json",
        "reference-transaction",
        "cargo run -q -p clm --",
        "clm-dev/bin",
    ] {
        assert!(
            text.contains(needle),
            "CONTRIBUTING.md doesn't name {needle}"
        );
    }
}

#[test]
fn contributing_takes_intent_only() {
    // req:clm/contributing-intent-only@c48e14b5
    let text = contributing();
    for needle in [
        "specs/",
        "changes/",
        "backlog/",
        "docs/",
        "ARCHITECTURE.md",
        "CAPABILITIES.md",
        "CLAUDE.md",
        "backlog/<slug>.md",
        "kind: idea",
        "priority",
        "/clm:propose",
        "Allow edits by maintainers",
        "clm guide contributions",
    ] {
        assert!(
            text.contains(needle),
            "CONTRIBUTING.md doesn't name {needle}"
        );
    }
    assert!(
        !text.contains("cargo clippy"),
        "CONTRIBUTING.md still tells a contributor to run the code checks"
    );
}
