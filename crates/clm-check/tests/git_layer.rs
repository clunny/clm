//! `clm-check` runs `git` and `gh` only through `clm-git` (`git-through-layer`).

use std::fs;
use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn covered_by_the_scan() {
    // req:check/git-through-layer@3909a133
    let krate = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest: toml::Table = fs::read_to_string(krate.join("Cargo.toml"))
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        manifest["dependencies"]
            .as_table()
            .unwrap()
            .contains_key("clm-git"),
        "clm-check doesn't list clm-git"
    );
    let mut files = Vec::new();
    rust_files(&krate.join("src"), &mut files);
    assert!(!files.is_empty());
    let calls = ["Command::new(\"git\")", "Command::new(\"gh\")"];
    let found: Vec<String> = files
        .iter()
        .flat_map(|f| {
            let text = fs::read_to_string(f).unwrap();
            text.lines()
                .enumerate()
                .filter(|(_, l)| calls.iter().any(|c| l.contains(c)))
                .map(|(n, _)| format!("{}:{}", f.display(), n + 1))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(found.is_empty(), "direct git or gh runs: {found:?}");
}

#[test]
fn size_lists_tracked_files_through_the_layer() {
    // req:check/size-tracked-through-git@551f5d19
    let size = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/size.rs");
    let text = fs::read_to_string(size).unwrap();
    let needle = ["ls", "-files"].concat();
    assert!(!text.contains(&needle), "size.rs runs git {needle} itself");
}
