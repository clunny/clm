//! `verify`'s files build every `mutated` body through `Body::mutated`, and name the variant
//! only in patterns ending in `..`, so a field `ledger` adds breaks none of them.

use std::fs;
use std::path::{Path, PathBuf};

/// Every `.rs` file under `dir`, recursively, leaving out a `target` directory.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if !path.ends_with("target") {
                rust_files(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The text inside the braces after each whole-word `name` in `text` that whitespace alone
/// parts from a `{`, up to its matching `}`: the name after `Body::`, after a `use` of the
/// variant, with no space before the brace or with it on the next line all count.
fn bodies<'a>(text: &'a str, name: &str) -> Vec<&'a str> {
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    let mut found = Vec::new();
    for (start, _) in text.match_indices(name) {
        if text[..start].chars().next_back().is_some_and(ident) {
            continue;
        }
        let after = text[start + name.len()..].trim_start();
        let Some(rest) = after.strip_prefix('{') else {
            continue;
        };
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
            .unwrap_or(rest.len());
        found.push(&rest[..end]);
    }
    found
}

#[test]
fn no_literal_of_the_variant() {
    // req:verify/mutated-through-constructor@899e8820
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let name = concat!("Muta", "ted");
    let mut files = Vec::new();
    rust_files(crate_dir, &mut files);
    assert!(!files.is_empty());
    let mut literals = Vec::new();
    for path in &files {
        let text = fs::read_to_string(path).unwrap();
        for body in bodies(&text, name) {
            if !body.trim_end().ends_with("..") {
                literals.push(format!("{}: {name} {{{body}}}", path.display()));
            }
        }
    }
    assert!(
        literals.is_empty(),
        "literals of the variant:\n{literals:#?}"
    );
    let call = concat!("Body::", "mutated(");
    let mutate = fs::read_to_string(crate_dir.join("src/mutate.rs")).unwrap();
    assert!(
        mutate
            .lines()
            .any(|l| l.contains(call) && !l.trim_start().starts_with("//")),
        "src/mutate.rs makes no call of the constructor"
    );
}

/// The search behind `no_literal_of_the_variant`: whole words only, any whitespace before
/// the brace, nested braces matched.
#[test]
fn the_search_finds_each_form() {
    let name = concat!("Muta", "ted");
    let text = format!(
        "Body::{name} {{ a: X {{ b }}, .. }} {name}{{ c }} B::{name}\n  {{ d }} {name} => Un{name} {{ e }}"
    );
    assert_eq!(bodies(&text, name), [" a: X { b }, .. ", " c ", " d "]);
}
