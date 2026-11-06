//! Tests of the migrations the binary holds, over the texts `clm init` wrote in published
//! releases and writes today.

use std::fs;

use tempfile::TempDir;

use crate::{apply, list};

const OLD_ARCHITECTURE: &str = include_str!("migrations/20261105-clm-upgrade/old.md");
const NEW_ARCHITECTURE: &str = include_str!("../../clm-init/templates/architecture.md");
const OLD_CLAUDE: &str = include_str!("migrations/20261105-clm-upgrade-2/old.md");
const NEW_CLAUDE: &str = include_str!("../../clm-init/templates/claude.md");

const CONSTRAINTS_ID: &str = "20261105-clm-upgrade";
const HELP_ID: &str = "20261105-clm-upgrade-2";

fn project(architecture: &str, claude: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("clm.toml"), "defects = \"ask\"\n").unwrap();
    fs::write(dir.path().join("ARCHITECTURE.md"), architecture).unwrap();
    fs::write(dir.path().join("CLAUDE.md"), claude).unwrap();
    dir
}

// req:upgrade/migration-carried-constraints-copy@42c5fa08
#[test]
fn an_unedited_copy_of_the_constraints_becomes_the_pointer() {
    assert!(OLD_ARCHITECTURE.contains("## Enforced by clm"));
    assert!(!NEW_ARCHITECTURE.contains("## Enforced by clm"));
    let dir = project(OLD_ARCHITECTURE, NEW_CLAUDE);
    assert!(list(dir.path()).unwrap().contains(CONSTRAINTS_ID));
    let out = apply(dir.path()).unwrap();
    assert!(
        out.contains(&format!("applied {CONSTRAINTS_ID}\n")),
        "{out}"
    );
    let text = fs::read_to_string(dir.path().join("ARCHITECTURE.md")).unwrap();
    assert_eq!(text, NEW_ARCHITECTURE);
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
}

// req:upgrade/migration-carried-constraints-copy@42c5fa08
#[test]
fn an_edited_copy_of_the_constraints_is_left_and_noted() {
    let edited = format!("{OLD_ARCHITECTURE}\n### Constraint: mine\nMine SHALL hold.\n");
    let dir = project(&edited, NEW_CLAUDE);
    let out = apply(dir.path()).unwrap();
    assert_eq!(
        fs::read_to_string(dir.path().join("ARCHITECTURE.md")).unwrap(),
        edited
    );
    assert!(out.contains(&format!("note {CONSTRAINTS_ID}\n")), "{out}");
    assert!(!out.contains("applied"), "{out}");
}

// req:upgrade/migration-carried-constraints-copy@42c5fa08
#[test]
fn the_constraints_migration_is_done_without_the_heading() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
    fs::remove_file(dir.path().join("ARCHITECTURE.md")).unwrap();
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
}

// req:upgrade/migration-claude-md-names-help@d8931de0
#[test]
fn an_unedited_claude_md_gains_the_help_pointer() {
    assert!(!OLD_CLAUDE.contains("/clm:help"));
    let dir = project(NEW_ARCHITECTURE, OLD_CLAUDE);
    assert!(list(dir.path()).unwrap().contains(HELP_ID));
    let out = apply(dir.path()).unwrap();
    assert!(out.contains(&format!("applied {HELP_ID}\n")), "{out}");
    let text = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    assert_eq!(text, NEW_CLAUDE);
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
}

// req:upgrade/migration-claude-md-names-help@d8931de0
#[test]
fn the_help_migration_is_done_when_claude_md_names_help_or_is_missing() {
    let dir = project(NEW_ARCHITECTURE, "Use /clm:help.\n");
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
    fs::remove_file(dir.path().join("CLAUDE.md")).unwrap();
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
}

// req:upgrade/migration-claude-md-names-help@d8931de0
#[test]
fn an_edited_claude_md_without_help_is_left_and_noted() {
    let edited = format!("{OLD_CLAUDE}\nMy own rule.\n");
    let dir = project(NEW_ARCHITECTURE, &edited);
    let out = apply(dir.path()).unwrap();
    assert_eq!(
        fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap(),
        edited
    );
    assert!(out.contains(&format!("note {HELP_ID}\n")), "{out}");
}
