//! `clm-gate` runs `git` only through `clm-git` (gate/git-through-layer).

use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn git_runs_through_the_layer() {
    // req:gate/git-through-layer@58c3fcf0
    let krate = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest: toml::Table = std::fs::read_to_string(krate.join("Cargo.toml"))
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        manifest["dependencies"]
            .as_table()
            .unwrap()
            .contains_key("clm-git"),
        "clm-gate must list clm-git among its dependencies"
    );
    let mut files = Vec::new();
    rust_files(&krate.join("src"), &mut files);
    assert!(!files.is_empty());
    // The direct runs, and the gate's old `git` wrapper (`no-dead-code`).
    let calls = [
        "Command::new(\"git\")",
        "Command::new(\"gh\")",
        "fn git(",
        "fn gh(",
    ];
    let direct: Vec<String> = files
        .iter()
        .flat_map(|file| {
            let text = std::fs::read_to_string(file).unwrap();
            text.lines()
                .enumerate()
                .filter(|(_, line)| calls.iter().any(|c| line.contains(c)))
                .map(|(n, _)| format!("{}:{}", file.display(), n + 1))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(direct.is_empty(), "direct git or gh runs: {direct:?}");
}

#[test]
fn no_process_of_its_own() {
    // req:gate/base-specs-as-checkout@a340ed25
    let mut files = Vec::new();
    rust_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    assert!(!files.is_empty());
    let found: Vec<String> = files
        .iter()
        .filter(|file| {
            let text = std::fs::read_to_string(file).unwrap();
            text.contains("Command::new(") || text.contains("fn extract_specs")
        })
        .map(|file| file.display().to_string())
        .collect();
    assert!(found.is_empty(), "a process of its own: {found:?}");
}
