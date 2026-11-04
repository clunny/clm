//! `pin-working-method`: every build follows the same fixed version of the AI's working
//! method, whatever the session has installed.

use std::fs;
use std::path::Path;

use crate::{Repo, printed};

/// The skills a build follows.
const SKILLS: [&str; 3] = [
    "writing-plans",
    "test-driven-development",
    "verification-before-completion",
];

/// Writes a `SKILL.md` for each of `SKILLS` under `root/skills`.
fn skills(root: &Path) {
    for skill in SKILLS {
        let dir = root.join("skills").join(skill);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), format!("# {skill}\n")).unwrap();
    }
}

/// The commit the project pins, the second word of its one-line pin file.
fn pinned_commit() -> String {
    let pin = Path::new(env!("CARGO_MANIFEST_DIR")).join("../clm-run/superpowers.pin");
    fs::read_to_string(pin)
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .to_string()
}

#[test]
fn pinned_method() {
    // req:capability/pin-working-method@3321b58b
    let repo = Repo::init();
    repo.land_on_origin("true");
    let home = repo.root.parent().unwrap().join("home");

    // The machine holds the pinned copy, and the session has another version installed.
    let cache = home.join(".cache");
    let pinned = cache.join("clm/superpowers").join(pinned_commit());
    skills(&pinned);
    let installed = home.join(".claude/plugins/cache/superpowers/superpowers/9.9.9");
    skills(&installed);

    // A build starts.
    let out = repo.clm_env(
        &["run", "a"],
        "",
        &[
            ("HOME", home.to_str().unwrap()),
            ("XDG_CACHE_HOME", cache.to_str().unwrap()),
            ("CLM_SUPERPOWERS_DIR", ""),
        ],
    );
    assert!(out.status.success(), "{}", printed(&out));

    // Its prompt sends the session to the pinned copy's skills, and to no other.
    let prompt = fs::read_to_string(repo.root.join(".clm/prompts/a.md")).unwrap();
    for skill in SKILLS {
        let path = pinned.join("skills").join(skill).join("SKILL.md");
        assert!(prompt.contains(path.to_str().unwrap()), "{skill}: {prompt}");
    }
    assert!(!prompt.contains("9.9.9"), "{prompt}");
}
