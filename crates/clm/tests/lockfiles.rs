//! The repository's lockfiles are marked `linguist-generated` in `.gitattributes`, so
//! `clm size` leaves them out of every node's code count (clm/lockfiles-generated).

// clm:reads **/Cargo.lock
// clm:reads **/package-lock.json
// clm:reads **/.gitattributes

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const LOCKFILES: [&str; 3] = [
    "Cargo.lock",
    "web/package-lock.json",
    "plugin-panel/package-lock.json",
];

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}");
    String::from_utf8(out.stdout).unwrap()
}

fn write(root: &Path, rel: &str, content: &[u8]) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn lockfiles_are_marked_generated() {
    // req:clm/lockfiles-generated@59b8d4d8
    let repo = repo();
    let listed = git(&repo, &["ls-files"]);
    let files: Vec<&str> = listed
        .lines()
        .filter(|f| {
            matches!(
                f.rsplit('/').next(),
                Some("Cargo.lock" | "package-lock.json")
            )
        })
        .collect();
    assert!(!files.is_empty(), "no lockfile tracked");
    let mut args = vec!["check-attr", "linguist-generated"];
    args.extend(&files);
    let out = git(&repo, &args);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), files.len(), "{out}");
    for (line, file) in lines.iter().zip(&files) {
        assert_eq!(*line, format!("{file}: linguist-generated: set"));
    }
}

#[test]
fn lockfiles_dont_count() {
    // req:clm/lockfiles-generated@59b8d4d8
    // A repository holding this one's `.gitattributes` and lockfiles, and nodes `clm`, `web`
    // and `plugin.panel` with the paths that own them, so each lockfile is the only file
    // under its node. The graph is a fixture: a doc-only PR skips the tests, so none of them
    // may read this repository's spec files.
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    let repo = repo();
    git(root, &["init", "-q"]);
    let node =
        |paths: &str| format!("---\ntitle: N\nstatus: active\nowners: []\npaths: [{paths}]\n---\n");
    write(root, "specs/SPEC.md", node("\"Cargo.lock\"").as_bytes());
    write(root, "specs/web/SPEC.md", node("\"web/**\"").as_bytes());
    write(root, "specs/plugin/SPEC.md", node("").as_bytes());
    write(
        root,
        "specs/plugin/panel/SPEC.md",
        node("\"plugin-panel/**\"").as_bytes(),
    );
    let attributes = fs::read(repo.join(".gitattributes")).expect("no .gitattributes");
    write(root, ".gitattributes", &attributes);
    for file in LOCKFILES {
        let content = fs::read(repo.join(file)).unwrap();
        assert!(!content.is_empty(), "{file} is empty");
        write(root, file, &content);
    }
    let mut add = vec!["add", "--"];
    add.extend(LOCKFILES);
    git(root, &add);
    for node in ["clm", "web", "plugin.panel"] {
        let out = Command::new(env!("CARGO_BIN_EXE_clm"))
            .env(
                "XDG_CONFIG_HOME",
                Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-user-config"),
            )
            .args(["size", node, "--root"])
            .arg(root)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "clm size {node}: {stdout}{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            stdout.contains("\tcode 0/"),
            "clm size {node} counts its lockfile: {stdout}"
        );
    }
}
