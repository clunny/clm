mod common;

use common::{env, repo, sh};

fn is_hex8(s: &str) -> bool {
    s.len() == 8 && s.chars().all(|c| c.is_ascii_hexdigit())
}

#[test]
fn first_use() {
    // req:ledger/writer-identity@6a854d4d
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), "Christopher Lunny");
    let file = dir.path().join(".git/clm/writer");
    assert!(!file.exists());
    let id = clm_ledger::writer(dir.path(), &env(dir.path(), &[])).unwrap();
    let clone = id.strip_prefix("christopher-lunny/").expect(&id);
    assert!(is_hex8(clone), "{id}");
    assert_eq!(std::fs::read_to_string(&file).unwrap().trim(), clone);
    // Read, not rewritten, on later use.
    assert_eq!(
        clm_ledger::writer(dir.path(), &env(dir.path(), &[])).unwrap(),
        id
    );
}

#[test]
fn shared_by_worktrees() {
    // req:ledger/writer-identity@6a854d4d
    let dir = tempfile::tempdir().unwrap();
    let main = dir.path().join("main");
    std::fs::create_dir_all(&main).unwrap();
    repo(&main, "Christopher Lunny");
    let linked = dir.path().join("linked");
    sh(&main, &["worktree", "add", "-q", linked.to_str().unwrap()]);
    let home = dir.path();
    let in_linked = clm_ledger::writer(&linked, &env(home, &[])).unwrap();
    let in_main = clm_ledger::writer(&main, &env(home, &[])).unwrap();
    assert_eq!(in_linked, in_main);
    assert!(main.join(".git/clm/writer").is_file());
}

#[test]
fn setting_wins() {
    // req:ledger/writer-identity@6a854d4d
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), "Christopher Lunny");
    let id = clm_ledger::writer(dir.path(), &env(dir.path(), &[("CLM_WRITER", "clunny")])).unwrap();
    assert!(id.starts_with("clunny/"), "{id}");
}

#[test]
fn an_empty_person_part_names_the_setting() {
    // req:ledger/writer-identity@6a854d4d
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), "!!!");
    let err = clm_ledger::writer(dir.path(), &env(dir.path(), &[])).unwrap_err();
    assert!(err.to_string().contains("`writer`"), "{err}");
}

#[test]
fn a_name_is_lowercased_and_runs_of_other_characters_become_one_dash() {
    // req:ledger/writer-identity@6a854d4d
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), "  O'Brien,  Ann ");
    let id = clm_ledger::writer(dir.path(), &env(dir.path(), &[])).unwrap();
    assert!(id.starts_with("o-brien-ann/"), "{id}");
}

#[test]
fn a_setting_that_is_not_a_slug_is_refused() {
    // req:ledger/writer-identity@6a854d4d
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), "T");
    let env = env(dir.path(), &[("CLM_WRITER", "Christopher/Lunny")]);
    let err = clm_ledger::writer(dir.path(), &env).unwrap_err();
    assert!(err.to_string().contains("`writer`"), "{err}");
}

#[test]
fn an_empty_clone_file_is_taken_over() {
    // req:ledger/writer-identity@6a854d4d
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), "T");
    let file = dir.path().join(".git/clm/writer");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "").unwrap();
    let id = clm_ledger::writer(dir.path(), &env(dir.path(), &[])).unwrap();
    let clone = id.strip_prefix("t/").expect(&id);
    assert_eq!(std::fs::read_to_string(&file).unwrap().trim(), clone);
}
