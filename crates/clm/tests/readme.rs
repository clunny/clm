//! The repository's front page: `README.md`.

// clm:reads README.md
// clm:reads docs/guide/why-clm.md

use std::fs;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

// req:clm/readme-front-page@d8ab8035
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

// req:clm/readme-front-page@d8ab8035
#[test]
fn readme_links_the_site_first_and_credits_its_sources() {
    let readme = fs::read_to_string(repo().join("README.md")).expect("README.md exists");
    let site = readme
        .find("https://clm-app.dev")
        .expect("README.md links the site");
    let first_section = readme.find("\n## ").expect("README.md has a section");
    assert!(
        site < first_section,
        "the site link comes before the first ## heading"
    );
    for needle in [
        "https://kubernetes.io/docs/concepts/architecture/controller/",
        "https://github.com/obra/superpowers",
        "https://github.com/Fission-AI/OpenSpec",
        "https://claude.com/blog/the-ai-native-sdlc-playbook",
    ] {
        assert!(readme.contains(needle), "README.md links {needle}");
    }
    // Every work the guide's "Where it comes from" credits is linked too.
    let why = fs::read_to_string(repo().join("docs/guide/why-clm.md")).expect("why-clm.md exists");
    let credits = why
        .split("## Where it comes from")
        .nth(1)
        .expect("why-clm.md credits its sources");
    let credits = credits.split("\n## ").next().unwrap();
    let urls: Vec<&str> = credits
        .split("](")
        .skip(1)
        .map(|part| part.split(')').next().unwrap())
        .filter(|target| target.starts_with("https://"))
        .collect();
    assert!(!urls.is_empty(), "why-clm.md links the works it credits");
    for url in urls {
        assert!(
            readme.contains(url),
            "README.md links {url}, which why-clm.md credits"
        );
    }
}
