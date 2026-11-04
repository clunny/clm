//! The repository's front page: `README.md`.

use std::fs;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

// req:clm/readme-front-page@dcb1846c
#[test]
fn readme_is_the_front_page() {
    let readme = fs::read_to_string(repo().join("README.md")).expect("README.md exists");
    for needle in [
        "/plugin marketplace add clunny/clm",
        "/plugin install clm@clm",
        "docs/guide/quickstart.md",
        "CONTRIBUTING.md",
        ".github/SECURITY.md",
        "[LICENSE](LICENSE)",
        "ARCHITECTURE.md",
        "CAPABILITIES.md",
        "arXiv:2606.27045",
        "](specs/)",
        "](changes/)",
        "](backlog/)",
        "](docs/decisions/)",
        "young and changes quickly",
        "](docs/guide/README.md)",
        "ai-native-sdlc-playbook",
    ] {
        assert!(readme.contains(needle), "README.md holds {needle}");
    }
    // Every relative link names an existing file or directory.
    for part in readme.split("](").skip(1) {
        let target = part.split(')').next().unwrap();
        if target.contains("://") || target.starts_with('#') {
            continue;
        }
        let path = target.split('#').next().unwrap();
        assert!(repo().join(path).exists(), "link {target} exists");
    }
}
