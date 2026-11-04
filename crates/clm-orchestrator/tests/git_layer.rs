//! `clm-orchestrator` runs `git` and `gh` only through `clm-git` (`git-through-layer`), and
//! takes the remote's name and the clock from it (`shared-plumbing`).

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

/// The `file:line` of every line under the crate's `src` that holds one of `needles`.
fn lines_holding(needles: &[&str]) -> Vec<String> {
    let mut files = Vec::new();
    rust_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    assert!(!files.is_empty());
    files
        .iter()
        .flat_map(|f| {
            let text = fs::read_to_string(f).unwrap();
            text.lines()
                .enumerate()
                .filter(|(_, l)| needles.iter().any(|c| l.contains(c)))
                .map(|(n, _)| format!("{}:{}", f.display(), n + 1))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn covered_by_the_scan() {
    // req:orchestrator/git-through-layer@ef43c283
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
        "clm-orchestrator doesn't list clm-git"
    );
    let found = lines_holding(&["Command::new(\"git\")", "Command::new(\"gh\")"]);
    assert!(found.is_empty(), "direct git or gh runs: {found:?}");
}

#[test]
fn scenario_no_copy_of_the_remote_name_or_the_clock() {
    // req:orchestrator/shared-plumbing@aeadf0f1
    let found = lines_holding(&[concat!("\"", "origin"), concat!("fn ", "now(")]);
    assert!(
        found.is_empty(),
        "copies of the remote's name or the clock: {found:?}"
    );
}
