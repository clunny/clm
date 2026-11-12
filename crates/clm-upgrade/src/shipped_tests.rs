//! Tests of the migrations the binary holds, over the texts `clm init` wrote in published
//! releases and writes today.

use std::fs;

use tempfile::TempDir;

use crate::{apply, latest, list};

const OLD_ARCHITECTURE: &str = include_str!("migrations/20261105-clm-upgrade/old.md");
const NEW_ARCHITECTURE: &str = include_str!("../../clm-init/templates/architecture.md");
const OLD_CLAUDE: &str = include_str!("migrations/20261105-clm-upgrade-2/old.md");
const NEW_CLAUDE: &str = include_str!("../../clm-init/templates/claude.md");

const CONSTRAINTS_ID: &str = "20261105-clm-upgrade";
const HELP_ID: &str = "20261105-clm-upgrade-2";

/// `text` with the stamp `clm init` writes, so the project stands for an up-to-date one.
fn stamped(text: &str) -> String {
    format!("upgraded = \"{}\"\n{text}", latest().unwrap())
}

fn project(architecture: &str, claude: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("clm.toml"), stamped("defects = \"ask\"\n")).unwrap();
    fs::write(dir.path().join("ARCHITECTURE.md"), architecture).unwrap();
    fs::write(dir.path().join("CLAUDE.md"), claude).unwrap();
    write_settings(dir.path(), "settings.json", &all_rules());
    dir
}

const SUBCOMMANDS: [&str; 12] = [
    "apply",
    "branch",
    "check",
    "claim",
    "defect",
    "gate",
    "land",
    "pr",
    "rebase",
    "setup",
    "verify",
    "workaround",
];

fn all_rules() -> Vec<String> {
    SUBCOMMANDS
        .iter()
        .flat_map(|s| [format!("Bash(clm {s})"), format!("Bash(clm {s} *)")])
        .collect()
}

fn write_settings(root: &std::path::Path, name: &str, allow: &[String]) {
    fs::create_dir_all(root.join(".claude")).unwrap();
    let text = serde_json::json!({ "enabledPlugins": {}, "permissions": { "allow": allow } });
    fs::write(root.join(".claude").join(name), text.to_string()).unwrap();
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
    assert_eq!(
        text,
        include_str!("migrations/20261105-clm-upgrade-2/new.md")
    );
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

const OLD_SCRIPT: &str = include_str!("migrations/20261106-corpus-entries-drift-from/old.sh");
const NEW_SCRIPT: &str = include_str!("migrations/20261106-corpus-entries-drift-from/new.sh");
const SCRIPT_ID: &str = "20261106-corpus-entries-drift-from";

fn with_script(text: &str) -> TempDir {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    fs::create_dir(dir.path().join("scripts")).unwrap();
    fs::write(dir.path().join("scripts/clm-check.sh"), text).unwrap();
    dir
}

// req:upgrade/migration-check-script-from-root@46e5be99
#[test]
fn an_unedited_check_script_is_rewritten_to_run_from_the_root() {
    assert!(!OLD_SCRIPT.contains("BASH_SOURCE"));
    let dir = with_script(OLD_SCRIPT);
    assert!(list(dir.path()).unwrap().contains(SCRIPT_ID));
    let out = apply(dir.path()).unwrap();
    assert!(out.contains(&format!("applied {SCRIPT_ID}\n")), "{out}");
    let text = fs::read_to_string(dir.path().join("scripts/clm-check.sh")).unwrap();
    // `migration-starter-land-dir` then brings it to the text written today.
    assert_eq!(text, NEW_RUST_SCRIPT);
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
}

// req:upgrade/migration-check-script-from-root@46e5be99
#[test]
fn an_edited_check_script_is_left_and_noted() {
    let edited = format!("{OLD_SCRIPT}echo mine\n");
    let dir = with_script(&edited);
    let out = apply(dir.path()).unwrap();
    assert_eq!(
        fs::read_to_string(dir.path().join("scripts/clm-check.sh")).unwrap(),
        edited
    );
    assert!(out.contains(&format!("note {SCRIPT_ID}\n")), "{out}");
    assert!(!out.contains("applied"), "{out}");
}

// req:upgrade/migration-check-script-from-root@46e5be99
#[test]
fn the_check_script_migration_is_done_when_absent_or_run_from_the_root() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
    let dir = with_script(NEW_RUST_SCRIPT);
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
    let dir = with_script(&format!("{NEW_RUST_SCRIPT}echo mine\n"));
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
}

// req:upgrade/migration-check-script-from-root@46e5be99
#[test]
fn the_check_script_note_says_what_changed_for_every_language() {
    let dir = with_script(&format!("{OLD_SCRIPT}echo mine\n"));
    let note = crate::note(dir.path(), SCRIPT_ID).unwrap();
    assert!(note.contains("BASH_SOURCE"), "{note}");
    assert!(note.contains("manifest"), "{note}");
}

const RULES_ID: &str = "20261107-projects-set-up-before";
const RULES_NOTE: &str = "Unattended sessions need clm's allow rules in .claude/settings.json";

// req:upgrade/migration-unattended-allow-rules@49231d4a
#[test]
fn a_project_without_settings_is_offered_the_rules_and_nothing_is_written() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    fs::remove_dir_all(dir.path().join(".claude")).unwrap();
    assert!(
        list(dir.path())
            .unwrap()
            .contains(&format!("{RULES_ID} advised note {RULES_NOTE}\n"))
    );
    let out = apply(dir.path()).unwrap();
    assert!(out.contains(&format!("note {RULES_ID}\n")), "{out}");
    assert!(!out.contains("applied"), "{out}");
    assert!(!dir.path().join(".claude").exists());
}

// req:upgrade/migration-unattended-allow-rules@49231d4a
#[test]
fn a_project_lacking_only_the_newest_rules_keeps_its_settings() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    let rules: Vec<String> = all_rules()
        .into_iter()
        .filter(|r| !r.contains("verify"))
        .collect();
    write_settings(dir.path(), "settings.json", &rules);
    let before = fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();
    let out = apply(dir.path()).unwrap();
    assert!(out.contains(&format!("note {RULES_ID}\n")), "{out}");
    let after = fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();
    assert_eq!(before, after);
}

// req:upgrade/migration-unattended-allow-rules@49231d4a
#[test]
fn each_subcommand_needs_both_its_bare_and_its_argument_rule() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    for keep in [
        |r: &String| !r.ends_with(" *)"),
        |r: &String| r.ends_with(" *)"),
        |r: &String| !r.contains("verify"),
    ] {
        let rules: Vec<String> = all_rules().into_iter().filter(keep).collect();
        write_settings(dir.path(), "settings.json", &rules);
        assert!(list(dir.path()).unwrap().contains(RULES_ID));
    }
}

// req:upgrade/migration-unattended-allow-rules@49231d4a
#[test]
fn the_rules_may_be_split_across_the_two_settings_files() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    let (land, rest): (Vec<_>, Vec<_>) =
        all_rules().into_iter().partition(|r| r == "Bash(clm land)");
    write_settings(dir.path(), "settings.json", &rest);
    assert!(list(dir.path()).unwrap().contains(RULES_ID));
    write_settings(dir.path(), "settings.local.json", &land);
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
}

// req:upgrade/migration-unattended-allow-rules@49231d4a
#[test]
fn the_older_rule_form_does_not_count() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    let mut rules: Vec<String> = all_rules()
        .into_iter()
        .filter(|r| !r.starts_with("Bash(clm land"))
        .collect();
    rules.push("Bash(clm land:*)".to_string());
    write_settings(dir.path(), "settings.json", &rules);
    assert!(list(dir.path()).unwrap().contains(RULES_ID));
}

// req:upgrade/migration-unattended-allow-rules@49231d4a
#[test]
fn a_settings_file_that_is_not_the_expected_json_allows_nothing() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    fs::write(dir.path().join(".claude/settings.json"), "not json").unwrap();
    assert!(list(dir.path()).unwrap().contains(RULES_ID));
    fs::write(
        dir.path().join(".claude/settings.json"),
        r#"{"permissions":{"allow":"Bash(clm apply)"}}"#,
    )
    .unwrap();
    assert!(list(dir.path()).unwrap().contains(RULES_ID));
}

// req:upgrade/migration-unattended-allow-rules@49231d4a
#[test]
fn a_declined_rules_migration_is_not_listed() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    fs::remove_dir_all(dir.path().join(".claude")).unwrap();
    fs::write(
        dir.path().join("clm.toml"),
        stamped(&format!(
            "declined_migrations = [\"{RULES_ID}\", \"20261112-a-draft-s-clm\"]\n"
        )),
    )
    .unwrap();
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
}

// req:upgrade/migration-unattended-allow-rules@49231d4a
#[test]
fn the_rules_note_lists_the_rules_and_the_ways_out() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    fs::remove_dir_all(dir.path().join(".claude")).unwrap();
    let note = crate::note(dir.path(), RULES_ID).unwrap();
    // The note of the frozen migration lists the eleven rules it shipped with; `clm setup`'s are
    // offered by their own migration.
    for rule in all_rules().iter().filter(|r| !r.contains("setup")) {
        assert!(note.contains(&format!("`{rule}`")), "{rule}: {note}");
    }
    assert!(note.contains("clm init --check-permissions"), "{note}");
    assert!(
        note.contains(&format!("clm upgrade --decline {RULES_ID}")),
        "{note}"
    );
    assert!(note.contains("`paths`"), "{note}");
    assert!(note.contains("by hand"), "{note}");
    assert!(note.contains("spec change"), "{note}");
}

const VERIFY_ID: &str = "20261107-slow-tests-after-land";
const VERIFY_LINE: &str = "advised apply Slow tests can run after the land: offer [verify]";
const OVERLAP_ID: &str = "20261107-projects-upgrading-past-the";
const OVERLAP_LINE: &str =
    "advised note Builds' checks now overlap: your tests must pass side by side";

fn read_toml(dir: &TempDir) -> String {
    fs::read_to_string(dir.path().join("clm.toml")).unwrap()
}

// req:upgrade/migration-offer-verify@3243c239
#[test]
fn an_older_project_is_offered_the_commented_verify_section() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    fs::write(
        dir.path().join("clm.toml"),
        format!("defects = \"ask\"\nupgraded = \"{RULES_ID}\"\n\n[land]\ncheck = \"true\"\n"),
    )
    .unwrap();
    assert!(
        list(dir.path())
            .unwrap()
            .contains(&format!("{VERIFY_ID} {VERIFY_LINE}\n"))
    );
    let out = apply(dir.path()).unwrap();
    assert!(out.contains(&format!("applied {VERIFY_ID}\n")), "{out}");
    let text = read_toml(&dir);
    let section = include_str!("migrations/20261107-slow-tests-after-land/verify.toml");
    assert!(
        text.ends_with(&format!("check = \"true\"\n\n{section}")),
        "{text}"
    );
    let parsed: toml::Table = text.parse().unwrap();
    assert!(!parsed.contains_key("verify"), "{text}");
    // Every line of the section is a comment, and removing the `# ` leaves a valid table.
    assert!(section.lines().all(|l| l.starts_with("# ")));
    let uncommented: String = section
        .lines()
        .skip(1)
        .map(|l| format!("{}\n", l.strip_prefix("# ").unwrap()))
        .collect();
    let table: toml::Table = uncommented.parse().unwrap();
    assert!(table.contains_key("verify"));
    // Run again, it changes nothing.
    let before = read_toml(&dir);
    apply(dir.path()).unwrap();
    assert_eq!(read_toml(&dir), before);
}

// req:upgrade/migration-offer-verify@3243c239
#[test]
fn a_clm_toml_without_a_final_newline_gets_one_blank_line() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    fs::write(
        dir.path().join("clm.toml"),
        format!("upgraded = \"{RULES_ID}\""),
    )
    .unwrap();
    apply(dir.path()).unwrap();
    assert!(
        read_toml(&dir).starts_with(&format!(
            "upgraded = \"{}\"\n\n# Slow tests",
            latest().unwrap()
        )),
        "{}",
        read_toml(&dir)
    );
}

// req:upgrade/migration-offer-verify@3243c239
#[test]
fn the_verify_migration_is_done_once_set_or_commented_or_without_a_clm_toml() {
    for line in ["# [verify]", "[verify]", "  # [verify]  "] {
        let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
        fs::write(
            dir.path().join("clm.toml"),
            format!("upgraded = \"{RULES_ID}\"\n{line}\n"),
        )
        .unwrap();
        assert!(
            !list(dir.path()).unwrap().contains(VERIFY_ID),
            "{line:?} should count"
        );
    }
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    fs::remove_file(dir.path().join("clm.toml")).unwrap();
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
    // A lookalike is not the section.
    fs::write(dir.path().join("clm.toml"), "# [verifying]\n").unwrap();
    assert!(list(dir.path()).unwrap().contains(VERIFY_ID));
}

// req:upgrade/migration-tests-overlap@a2b3fa99
#[test]
fn a_project_stamped_before_the_lift_is_told_and_then_up_to_date() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    fs::write(
        dir.path().join("clm.toml"),
        format!("upgraded = \"{RULES_ID}\"\n\n[land]\ncheck = \"true\"\n# [verify]\n"),
    )
    .unwrap();
    // Later notes may follow it; the overlap note comes first.
    assert!(
        list(dir.path())
            .unwrap()
            .starts_with(&format!("{OVERLAP_ID} {OVERLAP_LINE}\n"))
    );
    let out = apply(dir.path()).unwrap();
    assert!(out.starts_with(&format!("note {OVERLAP_ID}\n")), "{out}");
    assert_eq!(
        read_toml(&dir),
        format!(
            "upgraded = \"{}\"\n\n[land]\ncheck = \"true\"\n# [verify]\n",
            latest().unwrap()
        )
    );
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
    assert_eq!(apply(dir.path()).unwrap(), "");
}

// req:upgrade/migration-tests-overlap@a2b3fa99
#[test]
fn a_never_stamped_or_unparseable_project_is_told_but_a_missing_one_is_not() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    fs::write(dir.path().join("clm.toml"), "# [verify]\n").unwrap();
    assert!(list(dir.path()).unwrap().contains(OVERLAP_ID));
    fs::write(dir.path().join("clm.toml"), "upgraded = [\n").unwrap();
    assert!(list(dir.path()).unwrap().contains(OVERLAP_ID));
    fs::remove_file(dir.path().join("clm.toml")).unwrap();
    assert!(!list(dir.path()).unwrap().contains(OVERLAP_ID));
}

// req:upgrade/migration-tests-overlap@a2b3fa99
#[test]
fn a_project_stamped_with_the_migration_itself_is_done() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    fs::write(
        dir.path().join("clm.toml"),
        format!("upgraded = \"{OVERLAP_ID}\"\n"),
    )
    .unwrap();
    assert!(!list(dir.path()).unwrap().contains(OVERLAP_ID));
}

// req:upgrade/migration-tests-overlap@a2b3fa99
#[test]
fn the_overlap_note_names_what_a_project_needs_to_know() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    fs::write(dir.path().join("clm.toml"), "").unwrap();
    let note = crate::note(dir.path(), OVERLAP_ID).unwrap();
    for needle in [
        "CLM_SLOT",
        "tests-run-concurrently",
        "clm guide constraints",
        "#### Replaces",
        "max_builds",
        "`bug`",
        "localhost",
        "127.0.0.1",
        "different `CLM_SLOT` values",
        "one at a time",
    ] {
        assert!(note.contains(needle), "{needle}: {note}");
    }
}

const LAND_DIR_ID: &str = "20261108-toolchain-neutral-land-builds";
const LAND_DIR_LINE: &str =
    "advised note Lands hand your check CLM_LAND_DIR and re-check only the overlap";

// req:upgrade/migration-land-dir@7f47023b
#[test]
fn a_project_stamped_before_the_land_dir_note_is_told_and_then_up_to_date() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    fs::write(
        dir.path().join("clm.toml"),
        format!("upgraded = \"{OVERLAP_ID}\"\n\n[land]\ncheck = \"true\"\n# [verify]\n"),
    )
    .unwrap();
    assert_eq!(
        list(dir.path()).unwrap(),
        format!("{LAND_DIR_ID} {LAND_DIR_LINE}\n{BUILD_SETUP_ID} {BUILD_SETUP_LINE}\n")
    );
    let out = apply(dir.path()).unwrap();
    assert_eq!(out, format!("note {LAND_DIR_ID}\nnote {BUILD_SETUP_ID}\n"));
    assert_eq!(
        read_toml(&dir),
        format!(
            "upgraded = \"{}\"\n\n[land]\ncheck = \"true\"\n# [verify]\n",
            latest().unwrap()
        )
    );
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
}

// req:upgrade/migration-land-dir@7f47023b
#[test]
fn the_land_dir_note_is_done_once_stamped_or_without_a_clm_toml_and_pending_otherwise() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    fs::write(
        dir.path().join("clm.toml"),
        format!("upgraded = \"{LAND_DIR_ID}\"\n"),
    )
    .unwrap();
    assert_eq!(
        list(dir.path()).unwrap(),
        format!("{BUILD_SETUP_ID} {BUILD_SETUP_LINE}\n")
    );
    fs::write(dir.path().join("clm.toml"), "# [verify]\n").unwrap();
    assert!(list(dir.path()).unwrap().contains(LAND_DIR_ID));
    fs::write(dir.path().join("clm.toml"), "upgraded = [\n").unwrap();
    assert!(list(dir.path()).unwrap().contains(LAND_DIR_ID));
    fs::remove_file(dir.path().join("clm.toml")).unwrap();
    assert!(!list(dir.path()).unwrap().contains(LAND_DIR_ID));
}

// req:upgrade/migration-land-dir@7f47023b
#[test]
fn the_land_dir_note_names_what_a_project_needs_to_know() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    fs::write(dir.path().join("clm.toml"), "").unwrap();
    let note = crate::note(dir.path(), LAND_DIR_ID).unwrap();
    for needle in [
        "CLM_LAND_DIR",
        "CARGO_TARGET_DIR",
        "land-checks-stay-fast",
        "clm guide constraints",
        "clm:reads",
        "[verify]",
    ] {
        assert!(note.contains(needle), "{needle}: {note}");
    }
}

const STARTER_ID: &str = "20261109-land-still-sets-cargo";
const OLD_RUST_SCRIPT: &str = include_str!("migrations/20261109-land-still-sets-cargo/old.sh");
const NEW_RUST_SCRIPT: &str = include_str!("migrations/20261109-land-still-sets-cargo/new.sh");

// req:upgrade/migration-starter-land-dir@131020e2
#[test]
fn an_unedited_v0_23_script_is_rewritten_with_the_land_dir_lines() {
    assert_eq!(OLD_RUST_SCRIPT, NEW_SCRIPT);
    let dir = with_script(OLD_RUST_SCRIPT);
    assert!(list(dir.path()).unwrap().contains(STARTER_ID));
    let out = apply(dir.path()).unwrap();
    assert!(out.contains(&format!("applied {STARTER_ID}\n")), "{out}");
    let text = fs::read_to_string(dir.path().join("scripts/clm-check.sh")).unwrap();
    assert_eq!(text, NEW_RUST_SCRIPT);
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
}

// req:upgrade/migration-starter-land-dir@131020e2
#[test]
fn an_unedited_v0_14_script_is_rewritten_by_both_migrations() {
    let dir = with_script(OLD_SCRIPT);
    let out = apply(dir.path()).unwrap();
    assert!(out.contains(&format!("applied {STARTER_ID}\n")), "{out}");
    let text = fs::read_to_string(dir.path().join("scripts/clm-check.sh")).unwrap();
    assert_eq!(text, NEW_RUST_SCRIPT);
}

// req:upgrade/migration-starter-land-dir@131020e2
#[test]
fn an_edited_v0_23_script_is_left_and_noted() {
    let edited = format!("{OLD_RUST_SCRIPT}echo mine\n");
    let dir = with_script(&edited);
    let out = apply(dir.path()).unwrap();
    assert_eq!(
        fs::read_to_string(dir.path().join("scripts/clm-check.sh")).unwrap(),
        edited
    );
    assert!(out.contains(&format!("note {STARTER_ID}\n")), "{out}");
}

// req:upgrade/migration-starter-land-dir@131020e2
#[test]
fn the_starter_migration_is_done_without_a_script_a_land_dir_or_cargo() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    assert!(!list(dir.path()).unwrap().contains(STARTER_ID));
    let dir = with_script(NEW_RUST_SCRIPT);
    assert!(!list(dir.path()).unwrap().contains(STARTER_ID));
    let typescript = "#!/usr/bin/env bash\nset -euo pipefail\ncd \"$(dirname \"${BASH_SOURCE[0]}\")/..\"\nif [ -f package.json ]; then\n    npm ci && npm test\nfi\n";
    let dir = with_script(typescript);
    assert!(!list(dir.path()).unwrap().contains(STARTER_ID));
}

// req:upgrade/migration-starter-land-dir@131020e2
#[test]
fn the_starter_note_names_what_a_project_needs_to_know() {
    let dir = with_script(&format!("{OLD_RUST_SCRIPT}echo mine\n"));
    let note = crate::note(dir.path(), STARTER_ID).unwrap();
    assert!(
        note.starts_with(
            "Lands no longer set CARGO_TARGET_DIR: point cargo at CLM_LAND_DIR in your check\n"
        ),
        "{note}"
    );
    for needle in [
        "CLM_LAND_DIR",
        "CARGO_TARGET_DIR",
        "CARGO_INCREMENTAL",
        "--decline",
    ] {
        assert!(note.contains(needle), "{needle}: {note}");
    }
}

const BUILD_SETUP_ID: &str = "20261109-a-build-s-tests";
const BUILD_SETUP_LINE: &str =
    "advised note Builds run your tests in a fresh worktree: declare [build] setup";

// req:upgrade/migration-build-setup@92303a2e
#[test]
fn a_project_stamped_before_the_build_setup_note_is_told_and_then_up_to_date() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    let before = "upgraded = \"20261109-land-still-sets-cargo\"\n\n[land]\ncheck = \"true\"\n";
    fs::write(dir.path().join("clm.toml"), before).unwrap();
    assert_eq!(
        list(dir.path()).unwrap(),
        format!("{BUILD_SETUP_ID} {BUILD_SETUP_LINE}\n")
    );
    let out = apply(dir.path()).unwrap();
    assert_eq!(out, format!("note {BUILD_SETUP_ID}\n"));
    assert_eq!(
        read_toml(&dir),
        before.replace("20261109-land-still-sets-cargo", latest().unwrap())
    );
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
}

// req:upgrade/migration-build-setup@92303a2e
#[test]
fn the_build_setup_note_is_done_once_stamped_or_without_a_clm_toml_and_pending_otherwise() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    fs::write(
        dir.path().join("clm.toml"),
        format!("upgraded = \"{BUILD_SETUP_ID}\"\n"),
    )
    .unwrap();
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
    fs::write(dir.path().join("clm.toml"), "# [verify]\n").unwrap();
    assert!(list(dir.path()).unwrap().contains(BUILD_SETUP_ID));
    fs::write(dir.path().join("clm.toml"), "upgraded = [\n").unwrap();
    assert!(list(dir.path()).unwrap().contains(BUILD_SETUP_ID));
    fs::remove_file(dir.path().join("clm.toml")).unwrap();
    assert!(!list(dir.path()).unwrap().contains(BUILD_SETUP_ID));
}

// req:upgrade/migration-build-setup@92303a2e
#[test]
fn the_build_setup_note_names_what_a_project_needs_to_know() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    fs::write(dir.path().join("clm.toml"), "").unwrap();
    let note = crate::note(dir.path(), BUILD_SETUP_ID).unwrap();
    assert!(
        note.starts_with("Builds run your tests in a fresh worktree: declare [build] setup\n"),
        "{note}"
    );
    for needle in [
        "tests-run-in-their-own-tree",
        "clm guide constraints",
        "[build]",
        "setup",
        "clm setup",
        "[land] check",
        "npm ci",
        "leaves nothing to decline",
    ] {
        assert!(note.contains(needle), "{needle}: {note}");
    }
}

const AGENTS_ID: &str = "20261112-agents-md";

/// A project no migration has stamped, so the apply migrations are pending.
fn unstamped(architecture: &str, claude: &str) -> TempDir {
    let dir = project(architecture, claude);
    fs::write(dir.path().join("clm.toml"), "defects = \"ask\"\n").unwrap();
    dir
}

fn read(dir: &TempDir, name: &str) -> String {
    fs::read_to_string(dir.path().join(name)).unwrap()
}

// req:upgrade/migration-agents-md@41da745b
#[test]
fn an_unedited_claude_md_moves_to_agents_md_as_init_writes_it() {
    let dir = unstamped(NEW_ARCHITECTURE, OLD_CLAUDE);
    assert!(list(dir.path()).unwrap().contains(AGENTS_ID));
    let out = apply(dir.path()).unwrap();
    assert!(out.contains(&format!("applied {AGENTS_ID}\n")), "{out}");
    assert!(!out.contains(&format!("note {HELP_ID}")), "{out}");
    assert_eq!(
        read(&dir, "AGENTS.md"),
        include_str!("../../clm-init/templates/agents.md")
    );
    assert_eq!(read(&dir, "CLAUDE.md"), "@AGENTS.md\n");
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
}

// req:upgrade/migration-agents-md@41da745b
#[test]
fn an_edited_claude_md_moves_whole() {
    let edited = format!("{OLD_CLAUDE}\nMy own rule.\n");
    let dir = unstamped(NEW_ARCHITECTURE, &edited);
    apply(dir.path()).unwrap();
    assert_eq!(read(&dir, "AGENTS.md"), edited);
    assert_eq!(read(&dir, "CLAUDE.md"), "@AGENTS.md\n");
}

// req:upgrade/migration-agents-md@41da745b
#[test]
fn both_files_keep_every_line_and_gain_the_import_first() {
    let dir = unstamped(NEW_ARCHITECTURE, "# Ours\n");
    fs::write(dir.path().join("AGENTS.md"), "# Theirs\n").unwrap();
    apply(dir.path()).unwrap();
    assert_eq!(read(&dir, "AGENTS.md"), "# Theirs\n");
    assert_eq!(read(&dir, "CLAUDE.md"), "@AGENTS.md\n\n# Ours\n");
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
}

// req:upgrade/migration-agents-md@41da745b
#[test]
fn the_agents_md_migration_is_advised_and_done_once_moved_or_without_claude_md() {
    let dir = unstamped(NEW_ARCHITECTURE, "# Ours\n");
    let out = list(dir.path()).unwrap();
    assert!(
        out.contains(&format!("{AGENTS_ID} advised apply ")),
        "{out}"
    );
    fs::write(dir.path().join("CLAUDE.md"), "@AGENTS.md\n").unwrap();
    assert!(!list(dir.path()).unwrap().contains(AGENTS_ID));
    fs::remove_file(dir.path().join("CLAUDE.md")).unwrap();
    assert!(!list(dir.path()).unwrap().contains(AGENTS_ID));
}

// req:upgrade/migration-agents-md@41da745b
#[test]
fn a_claude_md_that_imports_agents_md_has_nothing_pending() {
    let dir = project(NEW_ARCHITECTURE, "# Ours\n@AGENTS.md\n");
    assert_eq!(list(dir.path()).unwrap(), "up to date\n");
}
