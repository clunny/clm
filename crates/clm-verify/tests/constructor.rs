//! `verify`'s files build every `mutated` body through `Body::mutated`, and name the variant
//! only in patterns ending in `..`, so a field `ledger` adds breaks none of them.

use std::fs;
use std::path::{Path, PathBuf};

/// Every `.rs` file under `dir`, recursively.
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

/// The text after each `needle` in `text` up to its matching `}`, the needle's own `{`
/// opening the count.
fn bodies<'a>(text: &'a str, needle: &str) -> Vec<&'a str> {
    let mut found = Vec::new();
    for (start, _) in text.match_indices(needle) {
        let rest = &text[start + needle.len()..];
        let mut depth = 1usize;
        let end = rest
            .char_indices()
            .find(|&(_, c)| {
                match c {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
                depth == 0
            })
            .map(|(i, _)| i)
            .expect("an unmatched brace");
        found.push(&rest[..end]);
    }
    found
}

#[test]
fn no_literal_of_the_variant() {
    // req:verify/mutated-through-constructor@899e8820
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let needle = concat!("Body::", "Mutated {");
    let mut files = Vec::new();
    rust_files(crate_dir, &mut files);
    files.retain(|p| !p.strip_prefix(crate_dir).unwrap().starts_with("target"));
    assert!(!files.is_empty());
    let mut literals = Vec::new();
    for path in &files {
        let text = fs::read_to_string(path).unwrap();
        for body in bodies(&text, needle) {
            if !body.trim_end().ends_with("..") {
                literals.push(format!("{}: {needle}{body}}}", path.display()));
            }
        }
    }
    assert!(
        literals.is_empty(),
        "literals of the variant:\n{literals:#?}"
    );
    let mutate = fs::read_to_string(crate_dir.join("src/mutate.rs")).unwrap();
    assert!(mutate.contains(concat!("Body::", "mutated(")));
}

#[test]
fn the_brace_count_finds_the_match() {
    // req:verify/mutated-through-constructor@899e8820
    let needle = concat!("Body::", "Mutated {");
    let text = format!("{needle} a: X {{ b }}, .. }} {needle} c }}");
    let found = bodies(&text, needle);
    assert_eq!(found, [" a: X { b }, .. ", " c "]);
}
