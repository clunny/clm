//! The guide's glossary defines every term CLM's commands and files use
//! (clm/glossary-covers-terms).
// clm:reads docs/guide/glossary.md

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use clm_check::STATE_NAMES;
use clm_model::{BACKLOG_KIND_TERMS, CHANGE_KEYS, CHANGE_STATUS_TERMS, STATUS_TERMS};

/// The text of every code span (backticks around exactly that text) on the page.
fn code_spans(glossary: &str) -> BTreeSet<&str> {
    glossary
        .split('`')
        .enumerate()
        .filter(|(i, _)| i % 2 == 1)
        .map(|(_, span)| span)
        .collect()
}

/// Each of `terms` that the glossary doesn't hold as a code span, in order.
fn missing<'a>(glossary: &str, terms: &'a [String]) -> Vec<&'a str> {
    let spans = code_spans(glossary);
    terms
        .iter()
        .map(String::as_str)
        .filter(|t| !spans.contains(t))
        .collect()
}

/// The top-level commands a help text lists under `Commands:`, but `help`.
fn commands(help: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_commands = false;
    for line in help.lines() {
        if line.starts_with("Commands:") {
            in_commands = true;
        } else if in_commands {
            if !line.starts_with(' ') {
                break;
            }
            if line.starts_with("   ") {
                continue;
            }
            if let Some(name) = line.split_whitespace().next()
                && name != "help"
            {
                names.push(format!("clm {name}"));
            }
        }
    }
    names
}

fn words<'a>(items: impl Iterator<Item = &'a str>) -> Vec<String> {
    items.map(str::to_string).collect()
}

#[test]
fn a_command_without_an_entry_is_reported() {
    // req:clm/glossary-covers-terms@0cecca49
    let glossary = "Seeing: `clm status`, `clm check`.\nA `clm syncs` and clm sync bare.";
    let terms = words(["clm status", "clm sync", "clm check"].into_iter());
    assert_eq!(missing(glossary, &terms), ["clm sync"]);
}

#[test]
fn a_span_must_hold_exactly_the_term() {
    // req:clm/glossary-covers-terms@0cecca49
    let glossary = "`clm sync --force` and `unbuilt node` and `human_burden`";
    let terms = words(["clm sync", "unbuilt", "human_burdens"].into_iter());
    assert_eq!(
        missing(glossary, &terms),
        ["clm sync", "unbuilt", "human_burdens"]
    );
}

#[test]
fn commands_come_from_the_help() {
    // req:clm/glossary-covers-terms@0cecca49
    let help = "Usage: clm\n\nCommands:\n  status  Show state\n          more words\n  help    Print\n\nOptions:\n  -h\n";
    assert_eq!(commands(help), ["clm status"]);
}

#[test]
fn the_glossary_defines_every_term() {
    // req:clm/glossary-covers-terms@0cecca49
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let glossary = fs::read_to_string(root.join("docs/guide/glossary.md")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    let help = String::from_utf8(out.stdout).unwrap();

    let mut terms = commands(&help);
    assert!(terms.contains(&"clm land".to_string()), "{terms:?}");
    terms.extend(words(STATE_NAMES.iter().map(|(_, w)| *w)));
    terms.extend(words(STATUS_TERMS.iter().map(|(_, w)| *w)));
    terms.extend(words(CHANGE_STATUS_TERMS.iter().map(|(_, w)| *w)));
    terms.extend(words(BACKLOG_KIND_TERMS.iter().map(|(_, w)| *w)));
    terms.extend(words(CHANGE_KEYS.iter().copied()));

    for sample in [
        "current",
        "unbuilt",
        "active",
        "applied",
        "bug",
        "human_burdens",
    ] {
        assert!(terms.iter().any(|t| t == sample), "no term {sample}");
    }

    let absent = missing(&glossary, &terms);
    assert!(
        absent.is_empty(),
        "docs/guide/glossary.md lacks a code span for: {absent:?}"
    );
}
