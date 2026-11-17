//! This repository's own detectors, as its `clm.toml` names them (`unreferenced-detector`,
//! `retired-terms-detector`).

// clm:reads clm.toml
// clm:reads specs/**
// clm:reads backlog/**
// clm:reads ARCHITECTURE.md
// clm:reads CAPABILITIES.md
// clm:reads CLAUDE.md
// clm:reads AGENTS.md
// clm:reads README.md
// clm:reads docs/guide/**
// clm:reads docs/spec-format.md
// clm:reads plugin/**
// clm:reads web/src/**
// clm:reads scripts/retired-terms.sh
// clm:reads scripts/retired-terms.txt

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

/// The root of this repository.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The detectors this repository's `clm.toml` names, loaded from a copy of it alone, so neither
/// a clone's `clm.local.toml` nor the developer's own settings leak in.
fn this_repository_detectors() -> Vec<clm_config::Detector> {
    let dir = TempDir::new().unwrap();
    fs::copy(repo_root().join("clm.toml"), dir.path().join("clm.toml")).unwrap();
    let empty = dir.path().join("no-user-config");
    let env = |key: &str| (key == "XDG_CONFIG_HOME").then(|| empty.display().to_string());
    clm_config::load(Some(dir.path()), &env)
        .unwrap()
        .detectors
        .value
}

fn unreferenced_detector() -> clm_config::Detector {
    this_repository_detectors()
        .into_iter()
        .find(|d| d.name == "unreferenced-public-items")
        .expect("unreferenced-public-items detector")
}

/// The lines inside the `unreferenced-public-items` detector's `command = [ ... ]` array, as
/// this repository's `clm.toml` writes them.
fn unreferenced_command_lines() -> Vec<String> {
    let text = fs::read_to_string(repo_root().join("clm.toml")).unwrap();
    let mut lines = text
        .lines()
        .skip_while(|l| l.trim() != r#"name = "unreferenced-public-items""#)
        .skip_while(|l| l.trim() != "command = [");
    assert!(lines.next().is_some(), "no multi-line command array");
    lines
        .take_while(|l| l.trim() != "]")
        .map(|l| l.trim().to_string())
        .collect()
}

/// The value of a `"<flag>", "<value>",` line.
fn pair_value<'a>(line: &'a str, flag: &str) -> Option<&'a str> {
    line.strip_prefix(&format!(r#""{flag}", ""#))?
        .strip_suffix(r#"","#)
        .filter(|v| !v.is_empty() && !v.contains('"'))
}

/// The ID, with `.` replaced by `-`, of the node whose `paths` hold the library `krate`
/// (`clm_web` -> `crates/clm-web/**` -> `specs/web/worker/SPEC.md` -> `web-worker`).
fn crate_node_slug(krate: &str) -> String {
    let path = format!("\"crates/{}/**\"", krate.replace('_', "-"));
    let specs = repo_root().join("specs");
    let mut dirs = vec![specs.clone()];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap().path();
            if entry.is_dir() {
                dirs.push(entry);
            } else if entry.file_name().is_some_and(|n| n == "SPEC.md")
                && fs::read_to_string(&entry)
                    .unwrap()
                    .lines()
                    .any(|l| l.starts_with("paths:") && l.contains(&path))
            {
                let node = dir.strip_prefix(&specs).unwrap();
                let parts: Vec<_> = node.iter().map(|p| p.to_string_lossy()).collect();
                return parts.join("-");
            }
        }
    }
    panic!("no node owns {path}");
}

#[test]
fn unreferenced_detector_is_named_here() {
    // req:config/unreferenced-detector@d9f516a5
    let names: Vec<_> = this_repository_detectors()
        .into_iter()
        .map(|d| d.name)
        .collect();
    let machete = names.iter().position(|n| n == "unused-dependencies");
    let unreferenced = names.iter().position(|n| n == "unreferenced-public-items");
    assert!(
        matches!((machete, unreferenced), (Some(m), Some(u)) if m < u),
        "{names:?}"
    );

    let detector = unreferenced_detector();
    assert_eq!(detector.hint.as_deref(), Some("scripts/dev-install.sh"));
    let command = &detector.command;
    assert_eq!(command[..2], ["clm", "unreferenced"], "{command:?}");
    let pairs = command[2..].chunks(2);
    assert!(command.len().is_multiple_of(2), "{command:?}");
    let mut allowing = false;
    for pair in pairs {
        match pair[0].as_str() {
            "--entry" => assert!(!allowing, "--entry after --allow: {pair:?}"),
            "--allow" => {
                allowing = true;
                // `<crate>::<name>` or `<crate>::<Type>::<name>`, never a whole library.
                let parts: Vec<_> = pair[1].split("::").collect();
                assert!(
                    (2..=3).contains(&parts.len())
                        && parts.iter().all(|p| {
                            !p.is_empty() && p.chars().all(|c| c.is_alphanumeric() || c == '_')
                        }),
                    "not one item: {pair:?}"
                );
            }
            _ => panic!("not an --entry or --allow pair: {pair:?}"),
        }
    }
}

#[test]
fn unreferenced_allow_list_gives_every_entry_a_reason() {
    // req:config/unreferenced-detector@d9f516a5
    let lines = unreferenced_command_lines();
    let mut lines = lines.iter().filter(|l| !l.is_empty());
    assert_eq!(
        lines.next().map(String::as_str),
        Some(r#""clm", "unreferenced","#)
    );
    // The comment lines since the last pair: the reason the next pairs come under.
    let mut reason: Vec<&str> = Vec::new();
    let mut after_pair = false;
    let mut allowing = false;
    for line in lines {
        if let Some(comment) = line.strip_prefix('#') {
            if after_pair {
                reason.clear();
                after_pair = false;
            }
            reason.push(comment);
            assert!(
                !comment.contains("narrow-public-items-config"),
                "a pending config item: {line}"
            );
            let mut rest = comment;
            while let Some(at) = rest.find("backlog/narrow-public-items-") {
                rest = &rest[at + "backlog/".len()..];
                let slug: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
                    .collect();
                let item = repo_root().join("backlog").join(format!("{slug}.md"));
                assert!(item.is_file(), "{line}: no {}", item.display());
            }
            continue;
        }
        assert!(!reason.is_empty(), "a pair before any comment: {line}");
        after_pair = true;
        if pair_value(line, "--entry").is_some() {
            assert!(!allowing, "--entry after --allow: {line}");
        } else if let Some(entry) = pair_value(line, "--allow") {
            allowing = true;
            let reason = reason.join(" ");
            let krate = entry.split("::").next().unwrap();
            if krate == "clm_config" || !reason.contains("narrow-public-items-") {
                assert!(
                    reason.contains("Kept"),
                    "{entry} is neither kept nor pending"
                );
            } else {
                let item = format!("backlog/narrow-public-items-{}", crate_node_slug(krate));
                assert!(reason.contains(&item), "{entry} is pending, not on {item}");
            }
        } else {
            panic!("neither a comment nor one pair: {line}");
        }
    }
    // An empty list is fine; a reason with nothing under it isn't.
    assert!(
        after_pair || reason.is_empty(),
        "no pairs after the last comment"
    );
}

#[test]
fn the_retired_terms_detector_passes_here() {
    // req:config/retired-terms-detector@1e881e15
    // The detector's command as `clm check` runs it, from the repository's root.
    let detector = this_repository_detectors()
        .into_iter()
        .find(|d| d.name == "retired-terms")
        .expect("retired-terms detector");
    let out = Command::new(&detector.command[0])
        .args(&detector.command[1..])
        .current_dir(repo_root())
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
