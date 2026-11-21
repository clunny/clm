use std::fs;
use std::path::Path;

use clm_check::{Detector, detectors, run};
use tempfile::TempDir;

fn detector(name: &str, command: &[&str], hint: Option<&str>) -> Detector {
    Detector {
        name: name.into(),
        command: command.iter().map(|s| s.to_string()).collect(),
        hint: hint.map(Into::into),
        timeout: 600,
        requires: None,
    }
}

/// `(path, rule, message)` of each violation.
fn found(root: &Path, list: &[Detector]) -> Vec<(String, &'static str, String)> {
    detectors(root, list, &mut std::io::sink())
        .into_iter()
        .map(|v| (v.path.display().to_string(), v.rule, v.message))
        .collect()
}

#[test]
fn reports_a_tool_that_cannot_run_with_its_hint() {
    // req:check/detectors@4dd73f3d
    let dir = TempDir::new().unwrap();
    let list = [
        detector(
            "machete",
            &["no-such-tool-xyz"],
            Some("cargo install cargo-machete"),
        ),
        detector("bare", &["no-such-tool-xyz"], None),
    ];
    let found = found(dir.path(), &list);
    assert_eq!(found.len(), 2, "{found:?}");
    let (path, rule, message) = &found[0];
    assert_eq!((path.as_str(), *rule), ("clm.toml", "check/detectors"));
    assert!(message.starts_with("machete could not run: "), "{message}");
    assert!(
        message.ends_with("; install with: cargo install cargo-machete"),
        "{message}"
    );
    assert!(found[1].2.starts_with("bare could not run: "), "{found:?}");
    assert!(!found[1].2.contains("install with"), "{found:?}");
}

#[test]
fn run_skips_detectors() {
    // req:check/detectors@4dd73f3d
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("specs")).unwrap();
    fs::write(
        dir.path().join("specs/SPEC.md"),
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("clm.toml"),
        "[[detectors]]\nname = \"bad\"\ncommand = [\"sh\", \"-c\", \"echo unused-dep; exit 3\"]\n",
    )
    .unwrap();
    let found: Vec<_> = run(dir.path())
        .into_iter()
        .filter(|v| v.rule == "check/detectors")
        .collect();
    assert!(found.is_empty(), "{found:?}");
}
