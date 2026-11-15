//! The calls that read the crate's own list in place of a caller's are gone, and nothing calls them.

// clm:reads crates/**/*.rs
use std::fs;
use std::path::{Path, PathBuf};

const FORMS: [&str; 7] = [
    "pending",
    "require_current",
    "list",
    "apply",
    "note",
    "decline",
    "latest",
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if path.is_dir() {
            if name != "target" && name != "node_modules" {
                rust_files(&path, out);
            }
        } else if name.ends_with(".rs") {
            out.push(path);
        }
    }
}

fn words(text: &str) -> Vec<&str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
        .collect()
}

/// What of the crate's sources, and of the others' use of it, still holds a form of the kind
/// `forms-without-a-list-gone` removes.
fn leftovers(crates: &Path, marker: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut files = Vec::new();
    rust_files(crates, &mut files);
    for file in files {
        let text = fs::read_to_string(&file).unwrap();
        let shown = file.strip_prefix(crates).unwrap().display().to_string();
        let own_source = shown.starts_with("clm-upgrade/src/");
        if text.contains(marker) {
            found.push(format!("{shown}: a `{marker}` marker is left"));
        }
        for form in FORMS {
            if own_source {
                let exported = text.contains(&format!("pub fn {form}("))
                    || text.split("pub use ").skip(1).any(|rest| {
                        let stmt = rest.split(';').next().unwrap();
                        words(&stmt.replace(&format!("{form}::"), "")).contains(&form)
                    });
                if exported {
                    found.push(format!("{shown}: exports `{form}`"));
                }
            } else {
                let called = text.contains(&format!("clm_upgrade::{form}("));
                let imported = text.split("use clm_upgrade::").skip(1).any(|rest| {
                    let stmt = rest.split(';').next().unwrap();
                    words(stmt).contains(&form)
                });
                if called || imported {
                    found.push(format!("{shown}: uses `{form}`"));
                }
            }
        }
    }
    found
}

fn marker() -> String {
    ["clm:remove-by", "20261114-migrations-with-templates"].join(" ")
}

// req:upgrade/forms-without-a-list-gone@991edcce
#[test]
fn no_form_without_a_list_is_exported_called_or_marked() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    // This file builds the names it looks for from parts or holds them as data, so it is
    // skipped by name rather than by what it says.
    let found: Vec<String> = leftovers(&crates, &marker())
        .into_iter()
        .filter(|l| !l.starts_with("clm-upgrade/tests/forms_gone.rs"))
        .collect();
    assert_eq!(found, Vec::<String>::new());
}

// req:upgrade/forms-without-a-list-gone@991edcce
#[test]
fn a_left_form_is_found_by_name_and_a_form_with_a_list_is_not() {
    let tree = tempfile::TempDir::new().unwrap();
    let src = tree.path().join("clm-upgrade/src");
    let other = tree.path().join("other/src");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&other).unwrap();
    fs::write(
        src.join("lib.rs"),
        format!(
            "pub fn pending(r: &Path) {{}}\npub use x::{{apply, list_in}};\n// {}\n",
            marker()
        ),
    )
    .unwrap();
    fs::write(
        other.join("main.rs"),
        "use clm_upgrade::{list_in, note};\nfn f() { clm_upgrade::latest(); clm_upgrade::decline_in(); }\n",
    )
    .unwrap();
    let found = leftovers(tree.path(), &marker()).join("\n");
    for expected in [
        "exports `pending`",
        "exports `apply`",
        "uses `note`",
        "uses `latest`",
        "marker is left",
    ] {
        assert!(found.contains(expected), "{expected}: {found}");
    }
    for unexpected in ["exports `list`", "uses `list`", "uses `decline`"] {
        assert!(!found.contains(unexpected), "{unexpected}: {found}");
    }
}
