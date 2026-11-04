//! `know-health`: one advisory report of the project's health.

use crate::{Repo, printed};

#[test]
fn health_report() {
    // req:capability/know-health@311ae878
    let repo = Repo::init();
    // A spec budget that node `a`'s spec is over.
    repo.write(
        "clm.toml",
        "host = \"none\"\n\n[component_size]\nspec_tokens = 10\ncode_tokens = 40000\n",
    );
    repo.commit("a small spec budget");
    let flag = repo.clm_stdin(
        &["workaround", "a/x", "--title", "Faked the clock"],
        "x reads the time, so the test faked the clock.\n",
    );
    assert!(flag.status.success(), "{}", printed(&flag));
    repo.commit("a workaround flag");

    let out = repo.clm(&["health"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{}", printed(&out));
    assert!(
        stdout
            .lines()
            .any(|l| l.starts_with("specs/a/SPEC.md: health/oversized: spec ")),
        "{stdout}"
    );
    assert!(
        stdout
            .lines()
            .any(|l| l.ends_with("health/workaround: a/x Faked the clock")),
        "{stdout}"
    );
}
