//! `clm superpowers <skill>` as `clm-run`'s call (`commands-in-crate`): the path of a skill
//! in the pinned superpowers copy builds follow.

mod support;

use std::fs;

use clm_run::SUPERPOWERS_COMMIT;
use support::*;
use tempfile::TempDir;

#[test]
fn override_dir() {
    // req:orchestrator.run/superpowers-path@411708d0
    let dir = TempDir::new().unwrap();
    let skill = with_skill(dir.path());
    let o = clm_superpowers(
        "brainstorming",
        &[("CLM_SUPERPOWERS_DIR", dir.path().as_os_str())],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(stdout(&o), format!("{}\n", skill.display()));
}

#[test]
fn relative_override_prints_an_absolute_path() {
    // req:orchestrator.run/superpowers-path@411708d0
    let dir = TempDir::new().unwrap();
    let skill = with_skill(&dir.path().canonicalize().unwrap().join("sp"));
    // Run in `dir`, as the process's current directory reads it.
    let o = clm_superpowers_in(
        &dir.path().canonicalize().unwrap(),
        &["brainstorming"],
        &[("CLM_SUPERPOWERS_DIR", "sp".as_ref())],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(stdout(&o), format!("{}\n", skill.display()));
}

#[test]
fn empty_override_counts_as_unset() {
    // req:orchestrator.run/superpowers-path@411708d0
    let (cache, skill) = pinned_cache();
    let o = clm_superpowers(
        "brainstorming",
        &[
            ("XDG_CACHE_HOME", cache.path().as_os_str()),
            ("CLM_SUPERPOWERS_DIR", "".as_ref()),
        ],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(stdout(&o), format!("{}\n", skill.display()));
}

#[test]
fn unknown_skill() {
    // req:orchestrator.run/superpowers-path@411708d0
    let dir = TempDir::new().unwrap();
    with_skill(dir.path());
    // Only the name check keeps `skills/brainstorming` and `../brainstorming` from
    // reaching these files.
    with_skill(&dir.path().join("skills"));
    fs::create_dir_all(dir.path().join("brainstorming")).unwrap();
    fs::write(dir.path().join("brainstorming/SKILL.md"), "").unwrap();
    fs::write(dir.path().join("skills/SKILL.md"), "").unwrap();
    for skill in [
        "no-such-skill",
        "../brainstorming",
        "",
        "skills/brainstorming",
        ".",
    ] {
        let o = clm_superpowers(skill, &[("CLM_SUPERPOWERS_DIR", dir.path().as_os_str())]);
        assert_eq!(o.status.code(), Some(1), "{skill}: {}", stderr(&o));
        assert!(stderr(&o).contains(&format!("`{skill}`")), "{}", stderr(&o));
        assert_eq!(stdout(&o), "", "{skill}");
    }
}

#[test]
fn skill_dir_without_skill_md() {
    // req:orchestrator.run/superpowers-path@411708d0
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("skills/brainstorming")).unwrap();
    let o = clm_superpowers(
        "brainstorming",
        &[("CLM_SUPERPOWERS_DIR", dir.path().as_os_str())],
    );
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    assert!(stderr(&o).contains("`brainstorming`"), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
}

#[test]
fn symlinked_cache_prints_the_path_run_loads() {
    // req:orchestrator.run/superpowers-path@411708d0
    let (cache, _) = pinned_cache();
    let links = TempDir::new().unwrap();
    let link = links.path().join("cache");
    std::os::unix::fs::symlink(cache.path(), &link).unwrap();
    let o = clm_superpowers("brainstorming", &[("XDG_CACHE_HOME", link.as_os_str())]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(
        stdout(&o),
        format!(
            "{}\n",
            link.join("clm/superpowers")
                .join(SUPERPOWERS_COMMIT)
                .join("skills/brainstorming/SKILL.md")
                .display()
        )
    );
}
