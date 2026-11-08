//! The superpowers pin is one data file, read at compile time and moved by hand (`cut-evals`).

use std::fs;
use std::path::{Path, PathBuf};

use clm_run::{SUPERPOWERS_COMMIT, SUPERPOWERS_TAG};

/// Every file under `dir`, recursively.
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(files(&path));
        } else {
            out.push(path);
        }
    }
    out
}

#[test]
fn one_pin() {
    // req:orchestrator.run/pin-file@a682a3d8
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let pin = fs::read_to_string(crate_dir.join("superpowers.pin")).unwrap();
    let line = pin.strip_suffix('\n').unwrap_or(&pin);
    assert!(!line.contains('\n'), "one line: {pin:?}");
    let (tag, commit) = line.split_once(' ').unwrap();
    let numbers: Vec<&str> = tag.strip_prefix('v').unwrap().split('.').collect();
    assert_eq!(numbers.len(), 3, "{tag}");
    assert!(
        numbers
            .iter()
            .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    );
    assert_eq!(commit.len(), 40, "{commit}");
    assert!(
        commit
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    );
    assert_eq!(SUPERPOWERS_TAG, tag);
    assert_eq!(SUPERPOWERS_COMMIT, commit);

    // Nothing else in the crate names them, so moving the file moves the pin.
    let mut sources = files(&crate_dir.join("src"));
    sources.push(crate_dir.join("Cargo.toml"));
    for file in sources {
        let text = fs::read_to_string(&file).unwrap();
        for field in [tag, commit] {
            assert!(!text.contains(field), "{} names {field}", file.display());
        }
    }
}

#[test]
fn no_eval_in_the_crate() {
    // req:orchestrator.run/superpowers-path@411708d0
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    for file in files(&crate_dir.join("src")) {
        let text = fs::read_to_string(&file).unwrap();
        assert!(
            !text.contains("plugin/evals"),
            "{} names plugin/evals",
            file.display()
        );
    }
}
