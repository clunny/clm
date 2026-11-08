//! `clm init` writes `clm.toml` with a commented-out `[verify]` section.

use std::fs;
use std::path::Path;
use std::process::Command;

use clm_init::{Options, init};
use tempfile::TempDir;

fn written() -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    let out = Command::new("git")
        .args(["init", "-q", "-b", "main"])
        .current_dir(dir.path())
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let options = Options {
        langs: vec!["rust".into()],
        detectors: true,
        ..Options::default()
    };
    init(dir.path(), &options).unwrap();
    let text = fs::read_to_string(dir.path().join("clm.toml")).unwrap();
    (dir, text)
}

fn has_verify_setting(root: &Path) -> bool {
    let none = |_: &str| None;
    clm_config::load(Some(root), &none)
        .unwrap()
        .verify
        .value
        .is_some()
}

#[test]
fn written_commented() {
    // req:init/init-verify-comment@ad40adc5
    let (dir, text) = written();
    assert!(text.lines().any(|l| l == "# [verify]"));
    assert!(!text.lines().any(|l| l.trim() == "[verify]"));
    assert!(text.contains("ci-and-deployment"));
    let section: Vec<&str> = text.lines().skip_while(|l| *l != "# [verify]").collect();
    assert!(section.iter().all(|l| l.starts_with("# ")), "{section:?}");
    let joined = section.join("\n");
    for word in [
        "check",
        "mode",
        "local",
        "pipeline",
        "import",
        "triggers",
        "change-done",
        "queue-drained",
        "work-done",
        "every",
        "workflow",
    ] {
        assert!(joined.contains(word), "section lacks {word}");
    }
    for key in ["check", "mode", "triggers", "every"] {
        let line = format!("# {key} = ");
        assert!(section.iter().any(|l| l.starts_with(&line)), "no {line}");
    }
    assert!(!has_verify_setting(dir.path()));
}

#[test]
fn uncommented_it_loads() {
    // req:init/init-verify-comment@ad40adc5
    let (dir, text) = written();
    let mut in_section = false;
    let uncommented: Vec<String> = text
        .lines()
        .map(|l| {
            in_section |= l == "# [verify]";
            match l.strip_prefix("# ") {
                Some(rest) if in_section => rest.to_owned(),
                _ => l.to_owned(),
            }
        })
        .collect();
    fs::write(dir.path().join("clm.toml"), uncommented.join("\n") + "\n").unwrap();
    assert!(has_verify_setting(dir.path()));
}
