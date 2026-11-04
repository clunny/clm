//! `no-dead-weight`: code nothing uses, and dependencies nothing needs, found.

use crate::{Repo, printed};

/// A crate's manifest, with `deps` as its dependencies.
fn manifest(deps: &str) -> String {
    format!(
        "[package]\nname = \"greeter\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n{deps}"
    )
}

#[test]
fn unused_dependency() {
    // req:capability/no-dead-weight@5b418b21
    // A Rust project brought under CLM with its language's detector, which runs
    // `cargo-machete` (installed where `clm land` runs, `ci/detector-tools`).
    let repo = Repo::empty();
    repo.write("Cargo.toml", &manifest("itoa = \"1\"\nryu = \"1\"\n"));
    repo.write(
        "src/lib.rs",
        "pub fn greet(n: u32) -> String {\n    itoa::Buffer::new().format(n).to_string()\n}\n",
    );
    repo.commit("a crate that uses itoa and not ryu");
    let init = repo.clm(&["init", "--lang", "rust"]);
    assert!(init.status.success(), "{}", printed(&init));
    repo.commit("adopt CLM");

    let out = repo.clm(&["check"]);
    let text = printed(&out);
    assert_eq!(out.status.code(), Some(1), "{text}");
    // The detector is the only failure.
    assert_eq!(
        text.lines().filter(|l| l.contains(": check/")).count(),
        1,
        "{text}"
    );
    assert!(text.contains("unused dependencies"), "{text}");
    assert!(text.contains("ryu"), "{text}");
    assert!(!text.contains("itoa"), "{text}");

    // Once the dependency is removed, the check passes.
    repo.write("Cargo.toml", &manifest("itoa = \"1\"\n"));
    repo.commit("drop ryu");
    let out = repo.clm(&["check"]);
    assert!(out.status.success(), "{}", printed(&out));
}
