//! The `[env]` a `[land] check` runs with (land/check-env).

mod common;

use common::Fixture;

/// `clm.toml` on `main` with `[env]` set to `A = "1"` appended.
fn with_env_a(f: &Fixture) -> String {
    let text = std::fs::read_to_string(f.work().join("clm.toml")).unwrap();
    format!("{text}\n[env]\nA = \"1\"\n")
}

#[test]
fn the_clones_value_wins_and_no_table_leaves_the_callers() {
    // req:land/check-env@a52574c4
    // SAFETY: the only test of this binary that sets or reads this variable's value in its
    // process; the others never name it.
    unsafe { std::env::set_var("RELAY_SERVER_IMAGE", "relay:shell") };

    let f = Fixture::new("[env]\nRELAY_SERVER_IMAGE = \"relay:amd64\"\n");
    f.write(
        "work/clm.local.toml",
        "[env]\nRELAY_SERVER_IMAGE = \"relay:arm64\"\n",
    );
    let out = clm_land::check_only(&f.work(), None, &f.env(&[])).unwrap();
    assert_eq!(out.lines.last().unwrap(), "checks pass");
    assert_eq!(f.lines("check-relay.log"), ["relay:arm64"]);
    let named = f.checked_commands();
    assert_eq!(
        named,
        [format!(
            "RELAY_SERVER_IMAGE=relay:arm64 {}",
            f.check_command()
        )]
    );

    let g = Fixture::new("");
    clm_land::check_only(&g.work(), None, &g.env(&[])).unwrap();
    assert_eq!(g.lines("check-relay.log"), ["relay:shell"]);
    assert_eq!(g.checked_commands(), [g.check_command()]);
    unsafe { std::env::remove_var("RELAY_SERVER_IMAGE") };
}

#[test]
fn variables_are_named_in_byte_order() {
    // req:land/check-env@a52574c4
    let f = Fixture::new("[env]\nb = \"2\"\nB = \"1\"\nA = \"x\"\n");
    clm_land::check_only(&f.work(), None, &f.env(&[])).unwrap();
    assert_eq!(
        f.checked_commands(),
        [format!("A=x B=1 b=2 {}", f.check_command())]
    );
}

#[test]
fn a_record_under_other_variables() {
    // req:land/check-env@a52574c4
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f.mark_checked("impl/a");
    f.write("work/clm.local.toml", "[env]\nA = \"1\"\n");
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
    assert_eq!(f.lines("check.log").len(), 1, "the check ran once");
    assert_eq!(f.lines("check-a.log"), ["1"]);
    let named = f.checked_commands();
    assert_eq!(named.last().unwrap(), &format!("A=1 {}", f.check_command()));
}

#[test]
fn a_record_under_the_same_variables_counts() {
    // req:land/check-env@a52574c4
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f.write("work/clm.local.toml", "[env]\nA = \"1\"\n");
    let key = clm_ledger::code_key(&f.work(), "impl/a").unwrap();
    f.append(
        &f.work(),
        clm_ledger::Body::Checked {
            code_key: key,
            command: format!("A=1 {}", f.check_command()),
            basis: None,
        },
    );
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(
        f.lines("check.log").is_empty(),
        "the check ran despite a record"
    );
}

#[test]
fn a_changed_variable_runs_every_test() {
    // req:land/check-env@a52574c4
    let f = Fixture::new("");
    let changed = with_env_a(&f);
    f.branch("impl/a", &[("clm.toml", &changed)]);
    f.mark_checked("origin/main");
    f.write("affected-out", "file crates/e/tests/r.rs\n");
    f.write("readers-out", "file crates/e/tests/r.rs\n");
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(f.lines("check-selection.log"), ["CLM_SELECTION=unset"]);
    assert_eq!(f.lines("check-a.log"), ["1"]);
}
