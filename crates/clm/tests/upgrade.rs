//! `clm upgrade` as the binary dispatches it (`upgrade-command`).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

fn clm(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap()
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
}

/// A repository `clm init` has just set up.
fn initialised() -> TempDir {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"]);
    let init = clm(
        dir.path(),
        &["init", "--lang", "rust", "--no-detectors", "--host", "none"],
    );
    assert!(init.status.success(), "{init:?}");
    dir
}

// req:clm/upgrade-command@86c8b58e
#[test]
fn upgrade_in_a_subdirectory_of_a_fresh_project_is_up_to_date() {
    let dir = initialised();
    let sub = dir.path().join("specs");
    let out = clm(&sub, &["upgrade"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "up to date\n");
}

// req:clm/upgrade-command@86c8b58e
#[test]
fn the_flags_are_dispatched_and_exclusive() {
    let dir = initialised();
    let before = fs::read_to_string(dir.path().join("clm.toml")).unwrap();
    let apply = clm(dir.path(), &["upgrade", "--apply"]);
    assert_eq!(apply.status.code(), Some(0), "{apply:?}");
    assert!(
        apply.stdout.is_empty(),
        "apply is not the listing: {apply:?}"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("clm.toml")).unwrap(),
        before,
        "nothing to change writes nothing"
    );
    let unknown = clm(dir.path(), &["upgrade", "--note", "no-such-migration"]);
    assert_eq!(unknown.status.code(), Some(1), "{unknown:?}");
    let decline = clm(dir.path(), &["upgrade", "--decline", "no-such-migration"]);
    assert_eq!(decline.status.code(), Some(1), "{decline:?}");
    let both = clm(dir.path(), &["upgrade", "--apply", "--note", "x"]);
    assert_eq!(both.status.code(), Some(2), "{both:?}");
}

/// Sets `clm.toml`'s `upgraded` to a migration no binary holds.
fn stamp_too_new(root: &Path) {
    let toml = root.join("clm.toml");
    let text = fs::read_to_string(&toml).unwrap();
    let mut lines: Vec<String> = text
        .lines()
        .filter(|l| !l.starts_with("upgraded"))
        .map(String::from)
        .collect();
    lines.insert(0, "upgraded = \"99990101-future\"".to_string());
    fs::write(&toml, lines.join("\n") + "\n").unwrap();
}

// req:clm/upgrade-command@86c8b58e
#[test]
fn lock_runs_on_the_upgrade_crate_and_refuses_a_too_new_project() {
    let dir = initialised();
    // The project holds the migrations' directories, as the CLM repository does.
    let migrations = dir.path().join("crates/clm-upgrade/src/migrations");
    fs::create_dir_all(&migrations).unwrap();
    let copied = Command::new("cp")
        .arg("-r")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../clm-upgrade/src/migrations/."))
        .arg(&migrations)
        .status()
        .unwrap();
    assert!(copied.success());
    let out = clm(dir.path(), &["upgrade", "--lock", "0.1.0"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        dir.path()
            .join("crates/clm-upgrade/migrations.lock")
            .exists()
    );

    stamp_too_new(dir.path());
    let too_new = clm(dir.path(), &["upgrade", "--lock", "0.1.0"]);
    assert_eq!(too_new.status.code(), Some(1), "{too_new:?}");
    assert!(String::from_utf8_lossy(&too_new.stderr).contains("newer `clm`"));
}

// req:clm/upgrade-command@86c8b58e
#[test]
fn upgrade_from_a_subdirectory_reads_the_project_root() {
    let dir = initialised();
    stamp_too_new(dir.path());
    let out = clm(&dir.path().join("specs"), &["upgrade"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("newer `clm`"));
}

/// The files `clm init` writes today into a directory named `project`, as the corpus compares
/// them: every file but the two only a first run writes (`corpus-skips-first-run-files`).
fn init_files() -> BTreeMap<String, Vec<u8>> {
    let parent = TempDir::new().unwrap();
    let project = parent.path().join("project");
    fs::create_dir(&project).unwrap();
    git(&project, &["init", "-q", "-b", "main"]);
    let init = clm(
        &project,
        &["init", "--lang", "rust", "--no-detectors", "--host", "none"],
    );
    assert!(init.status.success(), "{init:?}");
    let mut files = BTreeMap::new();
    collect(&project, &project, &mut files);
    files.remove(".claude/settings.json");
    files.remove("specs/SPEC.md");
    files
}

fn collect(root: &Path, dir: &Path, into: &mut BTreeMap<String, Vec<u8>>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.file_name().unwrap() == ".git" {
            continue;
        }
        if path.is_dir() {
            collect(root, &path, into);
        } else {
            let name = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .to_string();
            into.insert(name, fs::read(&path).unwrap());
        }
    }
}

/// `clm.toml` as parsed TOML, its `upgraded` and `declined_migrations` keys aside.
fn settings(bytes: &[u8]) -> Option<toml::Table> {
    let mut table: toml::Table = std::str::from_utf8(bytes).ok()?.parse().ok()?;
    table.remove("upgraded");
    table.remove("declined_migrations");
    Some(table)
}

/// Upgrades a copy of one corpus entry with the built `clm` and checks it against `expected`,
/// the files a fresh `clm init` writes, giving the first difference as the error.
fn check_entry(entry: &Path, expected: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    let name = entry.file_name().unwrap().to_string_lossy().to_string();
    let dir = TempDir::new().unwrap();
    let unpacked = Command::new("tar")
        .arg("-xzf")
        .arg(entry)
        .arg("-C")
        .arg(dir.path())
        .status()
        .unwrap();
    assert!(unpacked.success(), "unpack {name}");
    git(dir.path(), &["init", "-q", "-b", "main"]);
    git(dir.path(), &["add", "-A"]);
    git(
        dir.path(),
        &[
            "-c",
            "user.name=corpus",
            "-c",
            "user.email=corpus@example.invalid",
            "commit",
            "-q",
            "-m",
            "entry",
        ],
    );
    let apply = clm(dir.path(), &["upgrade", "--apply"]);
    if !apply.status.success() {
        return Err(format!("{name}: clm upgrade --apply failed: {apply:?}"));
    }
    // The allow-rules migration is a hand edit, so the entry declines it.
    let decline = clm(dir.path(), &["upgrade", "--decline", ALLOW_RULES_ID]);
    if !decline.status.success() {
        return Err(format!("{name}: clm upgrade --decline failed: {decline:?}"));
    }
    let listing = clm(dir.path(), &["upgrade"]);
    if String::from_utf8_lossy(&listing.stdout) != "up to date\n" {
        return Err(format!("{name}: not up to date: {listing:?}"));
    }
    let check = clm(dir.path(), &["check"]);
    if !check.status.success() {
        return Err(format!("{name}: clm check fails: {check:?}"));
    }
    for (file, want) in expected {
        let got = fs::read(dir.path().join(file))
            .map_err(|e| format!("{name}: {file} is missing after the upgrade ({e})"))?;
        let same = if file == "clm.toml" {
            settings(&got).is_some() && settings(&got) == settings(want)
        } else {
            &got == want
        };
        if !same {
            return Err(format!(
                "{name}: {file} differs from what `clm init` writes"
            ));
        }
    }
    Ok(())
}

/// The note migration that offers the allow rules `clm init` seeds on a first run.
const ALLOW_RULES_ID: &str = "20261107-projects-set-up-before";

fn entries() -> Vec<PathBuf> {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../clm-upgrade/corpus");
    let mut entries: Vec<PathBuf> = fs::read_dir(corpus)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".tar.gz"))
        .collect();
    entries.sort();
    entries
}

// req:clm/upgrade-corpus@dfc50a6d
#[test]
fn every_corpus_entry_upgrades_to_what_init_writes() {
    let expected = init_files();
    let entries = entries();
    assert!(entries.len() >= 5, "the corpus holds v0.14.0 to v0.18.0");
    assert!(
        entries.iter().any(|e| e.ends_with("v0.14.0.tar.gz")),
        "the corpus starts at v0.14.0"
    );
    let failures: Vec<String> = entries
        .iter()
        .filter_map(|entry| check_entry(entry, &expected).err())
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// req:clm/upgrade-corpus@dfc50a6d
#[test]
fn a_template_changed_without_a_migration_fails_naming_file_and_entry() {
    let mut expected = init_files();
    let claude = expected.get_mut("CLAUDE.md").unwrap();
    let text = String::from_utf8(claude.clone()).unwrap();
    let changed = text.replacen("CLM", "CLX", 1);
    assert_ne!(changed, text, "the template names CLM");
    *claude = changed.into_bytes();
    let entry = entries().pop().unwrap();
    let err = check_entry(&entry, &expected).unwrap_err();
    assert!(err.contains("CLAUDE.md"), "{err}");
    assert!(
        err.contains(&entry.file_name().unwrap().to_string_lossy().to_string()),
        "{err}"
    );
}

/// Removes `rule` from the `.claude/settings.json` of the repository at `dir`, which `clm init`
/// has just set up, and checks that `clm upgrade` then lists a pending migration, giving the rule
/// as the error when none does.
fn check_rule_is_offered(dir: &Path, rule: &str) -> Result<(), String> {
    let path = dir.join(".claude/settings.json");
    let mut json: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap())
        .map_err(|e| format!("{rule}: settings.json isn't JSON ({e})"))?;
    let allow = json
        .pointer_mut("/permissions/allow")
        .and_then(|a| a.as_array_mut())
        .ok_or_else(|| format!("{rule}: settings.json holds no permissions.allow"))?;
    allow.retain(|r| r != rule);
    fs::write(&path, json.to_string()).unwrap();
    let listing = clm(dir, &["upgrade"]);
    if String::from_utf8_lossy(&listing.stdout) == "up to date\n" {
        return Err(format!(
            "{rule}: no migration offers it to an older project"
        ));
    }
    Ok(())
}

// req:clm/upgrade-covers-allow-rules@bca597ec
#[test]
fn every_seeded_rule_is_offered_to_older_projects() {
    let failures: Vec<String> = clm_init::UNATTENDED_RULES
        .iter()
        .filter_map(|rule| check_rule_is_offered(initialised().path(), rule).err())
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// req:clm/upgrade-covers-allow-rules@bca597ec
#[test]
fn a_rule_no_migration_offers_fails_naming_it() {
    let dir = initialised();
    let path = dir.path().join(".claude/settings.json");
    let mut json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    json["permissions"]["allow"]
        .as_array_mut()
        .unwrap()
        .push("Bash(clm example)".into());
    fs::write(&path, json.to_string()).unwrap();
    let err = check_rule_is_offered(dir.path(), "Bash(clm example)").unwrap_err();
    assert!(err.contains("Bash(clm example)"), "{err}");
}
