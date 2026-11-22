mod common;

use common::{clone_with_origin, env, repo, sh};

fn is_hex8(s: &str) -> bool {
    s.len() == 8 && s.chars().all(|c| c.is_ascii_hexdigit())
}

#[test]
fn first_use() {
    // req:ledger/writer-identity@39cf81bb
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
    // req:ledger/writer-identity@39cf81bb
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
    // req:ledger/writer-identity@39cf81bb
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), "Christopher Lunny");
    let id = clm_ledger::writer(dir.path(), &env(dir.path(), &[("CLM_WRITER", "clunny")])).unwrap();
    assert!(id.starts_with("clunny/"), "{id}");
}

#[test]
fn an_empty_person_part_names_the_setting() {
    // req:ledger/writer-identity@39cf81bb
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), "!!!");
    let err = clm_ledger::writer(dir.path(), &env(dir.path(), &[])).unwrap_err();
    assert!(err.to_string().contains("`writer`"), "{err}");
}

#[test]
fn a_name_is_lowercased_and_runs_of_other_characters_become_one_dash() {
    // req:ledger/writer-identity@39cf81bb
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), "  O'Brien,  Ann ");
    let id = clm_ledger::writer(dir.path(), &env(dir.path(), &[])).unwrap();
    assert!(id.starts_with("o-brien-ann/"), "{id}");
}

#[test]
fn a_setting_that_is_not_a_slug_is_refused() {
    // req:ledger/writer-identity@39cf81bb
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), "T");
    let env = env(dir.path(), &[("CLM_WRITER", "Christopher/Lunny")]);
    let err = clm_ledger::writer(dir.path(), &env).unwrap_err();
    assert!(err.to_string().contains("`writer`"), "{err}");
}

#[test]
fn an_empty_clone_file_is_taken_over() {
    // req:ledger/writer-identity@39cf81bb
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), "T");
    let file = dir.path().join(".git/clm/writer");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "").unwrap();
    let id = clm_ledger::writer(dir.path(), &env(dir.path(), &[])).unwrap();
    let clone = id.strip_prefix("t/").expect(&id);
    assert!(is_hex8(clone), "{id}");
    assert_eq!(std::fs::read_to_string(&file).unwrap().trim(), clone);
}

#[test]
fn a_setting_config_accepts_that_is_not_a_slug_is_refused() {
    // req:ledger/writer-identity@39cf81bb
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), "T");
    let env = env(dir.path(), &[("CLM_WRITER", "ann--lee")]);
    let err = clm_ledger::writer(dir.path(), &env).unwrap_err();
    assert_eq!(
        err.to_string(),
        "`writer` is `ann--lee`, not a slug: use lowercase letters, digits and `-`, such as \
         `ann-lee`"
    );
}

#[test]
fn a_clone_file_of_white_space_is_taken_over() {
    // req:ledger/writer-identity@39cf81bb
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), "T");
    let file = dir.path().join(".git/clm/writer");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, " \n\t\n").unwrap();
    let id = clm_ledger::writer(dir.path(), &env(dir.path(), &[])).unwrap();
    let clone = id.strip_prefix("t/").expect(&id);
    assert!(is_hex8(clone), "{id}");
    assert_eq!(std::fs::read_to_string(&file).unwrap().trim(), clone);
}

#[test]
fn a_clm_toml_that_cannot_be_loaded() {
    // req:ledger/writer-identity@39cf81bb
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_with_origin(dir.path(), "a", "Ann");
    // `[verify]` with no `check`: `config` refuses this file.
    std::fs::write(clone.join("clm.toml"), "[verify]\nmode = \"local\"\n").unwrap();
    let env = env(dir.path(), &[("CLM_WRITER", "clunny")]);
    assert!(clm_config::load(Some(&clone), &env).is_err());
    let id = clm_ledger::writer(&clone, &env).unwrap();
    let digits = id.strip_prefix("clunny/").expect(&id);
    assert!(is_hex8(digits), "{id}");
    let body = clm_ledger::Body::approved("f".into(), "spec/x".into(), Some("x".into()));
    let out = clm_ledger::append(&clone, body, &env).unwrap();
    assert_eq!(out.record.writer, id);
    let origin = dir.path().join("origin.git");
    let line = sh(
        &origin,
        &["show", &format!("clm/ledger/clunny/{digits}:records.jsonl")],
    );
    let record: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(record["kind"], "approved");
    assert_eq!(record["writer"], id.as_str());
}
