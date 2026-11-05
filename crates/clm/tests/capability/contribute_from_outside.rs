//! `contribute-from-outside`: a suggestion from outside lands under its author's name, and
//! code from outside is refused before any of it runs.

use std::fs;
use std::os::unix::fs::PermissionsExt;

use crate::{Repo, printed};

/// The file a check command writes when it runs, outside the work tree.
fn marker(repo: &Repo) -> std::path::PathBuf {
    repo.root.parent().unwrap().join("check-ran")
}

/// A repository whose `main` has landed once, so its code is already checked, and the
/// contributor's clone of it.
fn project_and_contributor() -> (Repo, Repo) {
    let repo = Repo::init();
    repo.land_on_origin(&format!("echo ran >> {}", marker(&repo).display()));
    repo.write("src/lib.rs", "pub fn f() {}\n");
    repo.push_branch("impl/f", "add f");
    let first = repo.clm(&["land", "impl/f"]);
    assert!(first.status.success(), "{}", printed(&first));
    // The first land ran the check; forget that, to see only what the next one does.
    assert!(marker(&repo).exists(), "the first land ran the check");
    std::fs::remove_file(marker(&repo)).unwrap();
    repo.git(&["pull", "-q", "origin", "main"]);
    let bob = repo.clone_as("bob");
    (repo, bob)
}

#[test]
fn a_suggestion_lands() {
    // req:capability/contribute-from-outside@4c66fa8e
    let (repo, bob) = project_and_contributor();
    bob.write(
        "backlog/dark-mode.md",
        "---\ntitle: Dark mode\nkind: idea\ncomponents: []\n---\n\nAdd a dark mode.\n",
    );
    bob.push_branch("spec/x", "Suggest a dark mode");

    let out = repo.clm(&["land", "spec/x"]);
    assert!(out.status.success(), "{}", printed(&out));
    assert!(
        printed(&out).lines().any(|l| l.starts_with("landed ")),
        "{}",
        printed(&out)
    );

    let authors = repo.git(&["log", "--format=%an", &repo.origin_main()]);
    assert!(authors.lines().any(|a| a == "bob"), "{authors}");
    assert!(!marker(&repo).exists(), "the project's check command ran");
}

#[test]
fn code_from_outside() {
    // req:capability/contribute-from-outside@4c66fa8e
    let (repo, bob) = project_and_contributor();
    bob.write("src/lib.rs", "pub fn f() { std::process::abort() }\n");
    bob.push_branch("spec/x", "Improve f");
    let before = repo.origin_main();

    let out = repo.clm(&["land", "spec/x"]);
    let text = printed(&out);
    assert!(!out.status.success(), "{text}");
    assert!(
        text.contains("a spec branch holds only spec paths"),
        "{text}"
    );
    assert!(text.contains("src/lib.rs"), "{text}");
    assert!(!text.lines().any(|l| l.starts_with("landed ")), "{text}");
    assert!(!marker(&repo).exists(), "the project's check command ran");
    assert_eq!(repo.origin_main(), before, "{text}");
}

#[test]
fn a_fixed_issue_shows() {
    // req:capability/contribute-from-outside@4c66fa8e
    let repo = Repo::init();
    repo.write("clm.toml", "host = \"github\"\n");
    repo.write(
        "backlog/dark-mode.md",
        "---\ntitle: Dark mode\nkind: idea\ncomponents: []\nissues: [7]\n---\n\nAdd a dark mode.\n",
    );
    repo.commit("take in issue 7");
    repo.git(&["push", "-q", "origin", "main"]);
    // A change resolved the item and landed, deleting it.
    repo.write(
        "changes/archive/20261104-dark/change.md",
        "---\nstatus: done\nresolves: [dark-mode]\n---\n\n## Intent\n\nDark mode.\n",
    );
    fs::remove_file(repo.root.join("backlog/dark-mode.md")).unwrap();
    repo.commit("resolve dark mode");
    repo.git(&["push", "-q", "origin", "main"]);

    // A `gh` that lists issue 7 as still open, and no pull requests.
    let bin = repo.root.parent().unwrap().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let gh = bin.join("gh");
    fs::write(
        &gh,
        "#!/bin/sh\nif [ \"$1\" = issue ]; then echo '[{\"number\":7,\"author\":{\"login\":\"bob\"}}]'; else echo '[]'; fi\n",
    )
    .unwrap();
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let out = repo.clm_env(&["intake"], "", &[("PATH", &path)]);
    assert!(out.status.success(), "{}", printed(&out));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "issue 7 resolved dark-mode 20261104-dark",
        "{}",
        printed(&out)
    );
}
