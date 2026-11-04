//! `import_edges`: the file-to-file import edges `clm survey` finds, for `check`.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

fn write(root: &Path, files: &[(&str, &str)]) {
    for (rel, content) in files {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
}

const TWO_CRATES: &[(&str, &str)] = &[
    (
        "Cargo.toml",
        "[workspace]\nresolver = \"2\"\nmembers = [\"crates/*\"]\n",
    ),
    (
        "crates/a/Cargo.toml",
        "[package]\nname = \"a\"\n\n[dependencies]\nb = { path = \"../b\" }\n",
    ),
    ("crates/a/src/lib.rs", "mod parse;\n"),
    ("crates/a/src/parse.rs", "pub fn parse() {}\n"),
    ("crates/b/Cargo.toml", "[package]\nname = \"b\"\n"),
    ("crates/b/src/lib.rs", "pub fn b() {}\n"),
];

/// A git repository holding `files`, all added to the index.
fn repo(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    clm_git::git(dir.path(), &["init", "-q"]).unwrap();
    write(dir.path(), files);
    clm_git::git(dir.path(), &["add", "-A"]).unwrap();
    dir
}

fn pair(a: &str, b: &str) -> (String, String) {
    (a.to_owned(), b.to_owned())
}

#[test]
fn not_a_repository() {
    // req:init/import-edges@0d7cfd29
    let dir = TempDir::new().unwrap();
    write(dir.path(), TWO_CRATES);
    assert_eq!(clm_init::import_edges(dir.path()), Ok(Vec::new()));
}

#[test]
fn path_dependency() {
    // req:init/import-edges@0d7cfd29
    let repo = repo(TWO_CRATES);
    assert_eq!(
        clm_init::import_edges(repo.path()),
        Ok(vec![
            pair("crates/a/Cargo.toml", "crates/b/Cargo.toml"),
            pair("crates/a/src/lib.rs", "crates/a/src/parse.rs"),
        ])
    );
}

#[test]
fn typescript_file_edge() {
    // req:init/import-edges@0d7cfd29
    // Imported twice, listed once; a file that doesn't parse adds nothing and fails nothing.
    let repo = repo(&[
        (
            "web/src/app.ts",
            "import { u } from './lib/util';\nimport { v } from './lib/util';\n",
        ),
        ("web/src/lib/util.ts", "export const u = 1;\n"),
        ("web/src/broken.ts", "import { from '\n}}}{{{"),
        ("crates/x/Cargo.toml", "[package\nname ="),
    ]);
    assert_eq!(
        clm_init::import_edges(repo.path()),
        Ok(vec![pair("web/src/app.ts", "web/src/lib/util.ts")])
    );
}
