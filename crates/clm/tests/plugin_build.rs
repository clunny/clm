//! The `build` skill, `plugin/skills/build/SKILL.md`: what its text tells Claude.
// clm:reads docs/guide/**
// clm:reads CLAUDE.md
// clm:reads README.md
// clm:reads AGENTS.md

use std::fs;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn plugin() -> PathBuf {
    repo().join("plugin")
}

/// The YAML frontmatter and the body of a markdown file.
fn frontmatter(path: &Path) -> (serde_yaml::Value, String) {
    let text = fs::read_to_string(path).unwrap();
    let rest = text.strip_prefix("---\n").expect("frontmatter");
    let (yaml, body) = rest.split_once("\n---\n").expect("frontmatter end");
    (serde_yaml::from_str(yaml).unwrap(), body.to_owned())
}

fn text_files_under(dir: &Path, out: &mut Vec<(String, String)>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            text_files_under(&path, out);
        } else if let Ok(text) = fs::read_to_string(&path) {
            let rel = path.strip_prefix(repo()).unwrap_or(&path);
            out.push((rel.to_string_lossy().into_owned(), text));
        }
    }
}

/// What `skill-named-build` asks of a file: no mention of the retired skill, with its line.
fn old_skill_name_gaps(files: &[(String, String)]) -> Vec<String> {
    let old = ["/clm:implement", "skills/implement/"];
    let mut gaps = Vec::new();
    for (name, text) in files {
        for (n, line) in text.lines().enumerate() {
            for word in old.iter().filter(|word| line.contains(**word)) {
                gaps.push(format!("{name}:{}: names the old skill `{word}`", n + 1));
            }
        }
    }
    gaps
}

#[test]
fn build_skill_is_named_build() {
    // req:plugin.build/skill-named-build@3434a2db
    // The skill is `skills/build/SKILL.md`, with `skills/implement/` gone, and its description
    // holds `approved specs` and `not compile`.
    assert!(plugin().join("skills/build/SKILL.md").is_file());
    assert!(!plugin().join("skills/implement").exists());
    let (front, body) = frontmatter(&plugin().join("skills/build/SKILL.md"));
    let description = front["description"].as_str().unwrap();
    assert!(description.starts_with("Build"), "{description}");
    for needle in ["approved specs", "not compile"] {
        assert!(description.contains(needle), "{needle}: {description}");
    }
    assert!(body.trim_start().starts_with("# Build"), "{body}");
    // A guide page or skill that still says `/clm:implement` is reported with its line.
    let stale = [(
        "docs/guide/using-clm.md".to_owned(),
        format!("one\nRun {}{} to build.\n", "/clm:", "implement"),
    )];
    assert_eq!(
        old_skill_name_gaps(&stale),
        ["docs/guide/using-clm.md:2: names the old skill `/clm:implement`"]
    );
    let stale = [("x.md".to_owned(), "see skills/implement/ here\n".to_owned())];
    assert_eq!(
        old_skill_name_gaps(&stale),
        ["x.md:1: names the old skill `skills/implement/`"]
    );
    // The shipped plugin, guide, CLAUDE.md and README say none.
    let mut files = Vec::new();
    text_files_under(&plugin(), &mut files);
    text_files_under(&repo().join("docs/guide"), &mut files);
    // `AGENTS.md` too, when the repository has one.
    for name in ["CLAUDE.md", "AGENTS.md", "README.md"] {
        if name == "AGENTS.md" && !repo().join(name).exists() {
            continue;
        }
        files.push((
            name.to_owned(),
            fs::read_to_string(repo().join(name)).unwrap(),
        ));
    }
    let gaps = old_skill_name_gaps(&files);
    assert!(gaps.is_empty(), "{gaps:#?}");
}

#[test]
fn build_skill_points_at_the_queue() {
    // req:plugin.build/build-keeps-the-human-in-sight@1964557c
    // The skill says what needs the human is in the loop's queue and points at it.
    let (_, body) = frontmatter(&plugin().join("skills/build/SKILL.md"));
    for needle in ["clm view queue", "Queue"] {
        assert!(body.contains(needle), "{needle} missing from the skill");
    }
}
