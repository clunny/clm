//! `CONTRIBUTING.md` tells a contributor how to set a clone up (clm/contributing-doc).

use std::fs;
use std::path::Path;

#[test]
fn contributing_names_the_dev_install_setup() {
    // req:clm/contributing-doc@dca31868
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../CONTRIBUTING.md");
    let text = fs::read_to_string(path).unwrap();
    for needle in [
        "scripts/dev-install.sh",
        "--remove",
        "jq",
        "cargo",
        "claude",
        "clm-dev",
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
