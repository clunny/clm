//! The Worker reads its build-time assets through the binding `web/wrangler.toml` names
//! (web.worker/invited-sign-in).

use std::fs;
use std::path::Path;

use toml::Value;

/// A file by its path from the repository root.
fn repo_file(path: &str) -> String {
    let full = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path);
    fs::read_to_string(&full).unwrap_or_else(|e| panic!("can't read {}: {e}", full.display()))
}

fn wrangler() -> Value {
    toml::from_str(&repo_file("web/wrangler.toml")).expect("web/wrangler.toml parses")
}

#[test]
fn the_worker_reads_the_build_time_assets_through_its_binding() {
    // req:web.worker/invited-sign-in@6950d0c1
    // `_clm/` holds the invited list, which the Worker reads through the binding;
    // the Worker answers `/_clm/*` 404 itself (sign_in.rs).
    let w = wrangler();
    assert_eq!(w["assets"]["binding"].as_str(), Some("ASSETS"));
    let worker = repo_file("crates/clm-web/src/worker.rs");
    assert!(worker.contains("const ASSETS_BINDING: &str = \"ASSETS\";"));
    assert!(worker.contains("self.env.assets(ASSETS_BINDING)"));
}
