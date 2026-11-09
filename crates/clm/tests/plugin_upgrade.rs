//! The `upgrade` skill, `plugin/skills/upgrade/SKILL.md`: what it tells Claude to do.
// clm:reads plugin/skills/upgrade/SKILL.md

use std::fs;
use std::path::{Path, PathBuf};

fn plugin() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugin")
}

/// The YAML frontmatter and the body of a markdown file.
fn frontmatter(path: &Path) -> (serde_yaml::Value, String) {
    let text = fs::read_to_string(path).unwrap();
    let rest = text.strip_prefix("---\n").expect("frontmatter");
    let (yaml, body) = rest.split_once("\n---\n").expect("frontmatter end");
    (serde_yaml::from_str(yaml).unwrap(), body.to_owned())
}

/// The first sentence of `text` holding `needle`: sentences end at a newline or at a
/// period followed by a space, so file names such as `change.md` don't cut one short.
fn sentence_with<'a>(text: &'a str, needle: &str) -> &'a str {
    text.lines()
        .flat_map(|l| l.split(". "))
        .find(|s| s.contains(needle))
        .unwrap_or_else(|| panic!("no sentence mentions {needle}"))
}

/// What a skill lacks of `must` and names of `must_not`, one line per gap naming the skill
/// and the text. Empty when it says it all.
fn text_gaps(skill: &str, body: &str, must: &[&str], must_not: &[&str]) -> Vec<String> {
    let mut gaps: Vec<String> = must
        .iter()
        .filter(|needle| !body.contains(*needle))
        .map(|needle| format!("{skill}: SKILL.md must mention {needle}"))
        .collect();
    gaps.extend(
        must_not
            .iter()
            .filter(|needle| body.contains(*needle))
            .map(|needle| format!("{skill}: SKILL.md must not mention {needle}")),
    );
    gaps
}

/// The body of `skill`'s `SKILL.md`.
fn skill_body(skill: &str) -> String {
    frontmatter(&plugin().join(format!("skills/{skill}/SKILL.md"))).1
}

/// Asserts `gaps` reports `needle` for `skill`, and nothing else's.
fn assert_reports(gaps: &[String], skill: &str, needle: &str) {
    assert!(
        !gaps.is_empty()
            && gaps
                .iter()
                .all(|g| g.starts_with(&format!("{skill}:")) && g.contains(needle)),
        "{needle}: {gaps:?}"
    );
}

/// What `upgrade-skill` asks of the `upgrade` skill.
fn upgrade_skill_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "upgrade",
        body,
        &[
            "clm upgrade --apply",
            "clm upgrade --note",
            "clm upgrade --decline",
            "clm defect",
            "skip_migrations",
            "refuses together as `mixed`",
            "clm land",
            "clm defect --component upgrade",
            "never setting it without their yes",
            "clm branch push",
            "origin/main",
            "When it prints `up to date`, say so and stop",
            "Cut the branch `upgrade/<YYYYMMDD>` from `origin/main` in a worktree of its own (`session-worktrees`)",
            "upgrade/<YYYYMMDD>-notes",
            "is one the human then approves again with `clm approve`",
            "ask for the human's `clm approve` first and run `clm land` on their yes",
            "beside the file's current text",
            "Run `clm check`",
            "`upgraded`",
            "clm.local.toml",
            "declined_migrations",
            "clm approve",
            "clm gate --base origin/main",
        ],
        &[],
    )
}

#[test]
fn upgrade_skill() {
    // req:plugin.upgrade/upgrade-skill@75544804
    let (fm, body) = frontmatter(&plugin().join("skills/upgrade/SKILL.md"));
    assert!(
        fm["description"]
            .as_str()
            .is_some_and(|d| d.contains("up to date with the clm it runs"))
    );
    assert_eq!(upgrade_skill_gaps(&body), Vec::<String>::new());
    // a skill that leaves out a command
    let gaps = upgrade_skill_gaps(&body.replace("clm upgrade --decline", "elided"));
    assert_reports(&gaps, "upgrade", "clm upgrade --decline");
    let gaps = upgrade_skill_gaps(&body.replace("skip_migrations", "elided"));
    assert_reports(&gaps, "upgrade", "skip_migrations");
    // `[verify]` settings are a change outside spec paths: they go on the
    // first branch, never the notes branch, which the gate would refuse as
    // `mixed` beside a note's spec edit.
    let verify = body
        .lines()
        .find(|l| l.contains("applied 20261107-slow-tests-after-land"))
        .expect("the [verify] step");
    assert!(!verify.contains("notes branch"), "{verify}");
    for needle in [
        "outside spec paths",
        "first branch",
        "before committing the first branch",
        "a code branch of its own",
    ] {
        assert!(verify.contains(needle), "{needle}: {verify}");
    }
    let split = sentence_with(&body, "split `clm.toml`'s edit");
    assert!(split.contains("`upgraded`"), "{split}");
}

#[test]
fn upgrade_skill_notes_without_edit() {
    // req:plugin.upgrade/upgrade-skill-notes-without-edit@7cb403fe
    let body = skill_body("upgrade");
    let must = ["edits no file", "stamp-taken-notes"];
    assert_eq!(
        text_gaps("upgrade", &body, &must, &[]),
        Vec::<String>::new()
    );
    for needle in must {
        let gaps = text_gaps("upgrade", &body.replace(needle, "elided"), &must, &[]);
        assert_reports(&gaps, "upgrade", needle);
    }
    let note = sentence_with(&body, "A note that says it edits no file");
    for needle in [
        "asks for no edit",
        "offer to do what it advises",
        "`bug` backlog item",
        "on their yes",
        "run no `clm upgrade --decline` for it",
    ] {
        assert!(note.contains(needle), "{needle}: {note}");
    }
}

#[test]
fn upgrade_skill_settings_by_hand() {
    // req:plugin.upgrade/upgrade-skill-settings-by-hand@9902cf03
    let body = skill_body("upgrade");
    let must = [
        ".claude/settings.json",
        "clm init --check-permissions",
        "by hand",
    ];
    assert_eq!(
        text_gaps("upgrade", &body, &must, &[]),
        Vec::<String>::new()
    );
    for needle in must {
        let gaps = text_gaps("upgrade", &body.replace(needle, "elided"), &must, &[]);
        assert_reports(&gaps, "upgrade", needle);
    }
    let never = sentence_with(&body, "never edit that file yourself");
    for needle in [
        "protected path",
        "unattended-steps-run-clm",
        "add those they want",
    ] {
        assert!(never.contains(needle), "{needle}: {never}");
    }
    let third = sentence_with(&body, "When the root component's `paths` don't list");
    for needle in [
        "`upgrade/<YYYYMMDD>-settings`",
        "delta for the root's frontmatter",
        "only after that",
        "no component's `paths` hold",
    ] {
        assert!(third.contains(needle), "{needle}: {third}");
    }
    assert!(body.contains("takes the human's settings edit on the first, code, branch"));
}

#[test]
fn upgrade_skill_offers_verify() {
    // req:plugin.upgrade/upgrade-skill-offers-verify@4e293808
    let body = skill_body("upgrade");
    let must = ["[verify]", "Slow tests can run after the land"];
    assert_eq!(
        text_gaps("upgrade", &body, &must, &[]),
        Vec::<String>::new()
    );
    for needle in must {
        let gaps = text_gaps("upgrade", &body.replace(needle, "elided"), &must, &[]);
        assert_reports(&gaps, "upgrade", needle);
    }
    let offer = sentence_with(&body, "ask the human the questions `/clm:init` asks");
    for needle in ["`init-skill-asks-verify`", "recommending `change-done`"] {
        assert!(offer.contains(needle), "{needle}: {offer}");
    }
    assert!(body.contains("On a no, leave the section commented."));
}
