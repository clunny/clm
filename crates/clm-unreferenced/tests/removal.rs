//! `scan` skips an item a removal marker holds: the old path of an expand-contract.

mod common;

use std::path::Path;

use common::repo;

/// A removal marker naming a change, built so the whole marker never appears in this file,
/// which `check/remove-by-markers` would read as naming a change that doesn't exist.
const MARKER: &str = concat!("clm:remove", "-by 20260101-a-rename");

/// A package manifest named `name`.
fn manifest(name: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n")
}

/// Every line `scan` prints with these `--allow` entries.
fn lines(dir: &Path, allow: &[&str]) -> Vec<String> {
    common::lines(dir, allow)
}

/// The full names `scan` reports for Rust items and package items, sorted.
fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = lines(dir, &[])
        .iter()
        .filter_map(|line| {
            let (_, rest) = line.split_once(": ")?;
            rest.strip_suffix(" is public but nothing outside its library uses it")
                .map(str::to_string)
        })
        .chain(common::names(dir))
        .collect();
    names.sort();
    names
}

#[test]
fn scenario_an_old_path_marked_for_removal() {
    // req:check.unreferenced/removal-marked@e3696365
    let lib = format!(
        "/// The old form.\n// {MARKER}\npub fn old() {{}}\n\
         pub fn after() {{}}\n\
         // {MARKER}\nimpl S {{\n    pub fn m() {{}}\n}}\n\
         pub struct S;\n\
         // clm:remove-by soon\npub fn not_an_id() {{}}\n\
         // x{MARKER}\npub fn glued() {{}}\n\
         pub fn inside() {{\n    // {MARKER}\n}}\n\
         pub fn before() {{}} // {MARKER}\npub fn trailed() {{}}\n"
    );
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        ("crates/a/src/lib.rs", &lib),
        ("crates/b/Cargo.toml", &manifest("b")),
        (
            "crates/b/src/main.rs",
            "fn main() {\n    let _ = a::S;\n}\n",
        ),
    ]);
    assert_eq!(
        names(dir.path()),
        [
            "a::after",
            "a::before",
            "a::glued",
            "a::inside",
            "a::not_an_id",
            "a::trailed"
        ]
    );
}

#[test]
fn markers_before_attributes_hold_and_inner_ones_do_not() {
    // req:check.unreferenced/removal-marked@e3696365
    let lib = format!(
        "//! {MARKER}\npub fn inner_doc() {{}}\n\
         // {MARKER}\nmod m {{\n    pub fn in_mod() {{}}\n}}\n\
         // {MARKER}\n#[inline]\npub fn attributed() {{}}\n\
         /* {MARKER} */\npub fn block() {{}}\n\
         #[inline]\n/** {MARKER} */\npub fn doc_after_attribute() {{}}\n\
         pub fn stopped() {{}}\n\
         // {MARKER}\npub const C: u8 = 0;\npub fn after_const() {{}}\n"
    );
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        ("crates/a/src/lib.rs", &lib),
    ]);
    assert_eq!(
        names(dir.path()),
        ["a::after_const", "a::in_mod", "a::inner_doc", "a::stopped"]
    );
}

#[test]
fn scenario_marked_in_typescript_and_python() {
    // req:check.unreferenced/removal-marked@e3696365
    let index = format!("// {MARKER}\nexport function old() {{}}\nexport function g() {{}}\n");
    let init = format!("def first():\n    return 1\n# {MARKER}\n@decorator\ndef old_py(): ...\n");
    let dir = repo(&[
        ("packages/core/package.json", r#"{"name": "@acme/core"}"#),
        ("packages/core/src/index.ts", &index),
        ("src/acme_core/__init__.py", &init),
    ]);
    assert_eq!(names(dir.path()), ["@acme/core::g", "acme_core::first"]);
}

#[test]
fn a_marker_names_a_change_id() {
    // req:check.unreferenced/removal-marked@e3696365
    let key = concat!("clm:remove", "-by ");
    let lib = format!(
        "// {key}2026O101-a-rename\npub fn letter_in_date() {{}}\n\
         // {key}202601011-a\npub fn nine_digits() {{}}\n\
         // {key}20260101-Rename\npub fn upper_slug() {{}}\n\
         // {key}20260101-\npub fn no_slug() {{}}\n\
         // see _{key}20260101-a-rename\npub fn underscored() {{}}\n\
         // see ({key}20260101-a-rename)\npub fn in_parens() {{}}\n"
    );
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        ("crates/a/src/lib.rs", &lib),
    ]);
    assert_eq!(
        names(dir.path()),
        [
            "a::letter_in_date",
            "a::nine_digits",
            "a::no_slug",
            "a::underscored",
            "a::upper_slug"
        ]
    );
}

#[test]
fn typescript_and_python_markers_on_other_declarations() {
    // req:check.unreferenced/removal-marked@e3696365
    let index = format!(
        "// {MARKER}\nconst local = 1;\nexport {{ local }};\n\
         /** {MARKER} */\n@sealed\nexport class Old {{}}\n\
         // {MARKER}\nif (true) {{}}\nexport const kept = 2;\n"
    );
    let init = format!(
        "# {MARKER}\nOLD = 1\nNEW = 2\n\
         # {MARKER}\nif True:\n    def in_if(): ...\n\
         # {MARKER}\nclass Old: ...\n"
    );
    let dir = repo(&[
        ("packages/core/package.json", r#"{"name": "@acme/core"}"#),
        ("packages/core/src/index.ts", &index),
        ("src/acme_core/__init__.py", &init),
    ]);
    assert_eq!(
        names(dir.path()),
        ["@acme/core::kept", "acme_core::NEW", "acme_core::in_if"]
    );
}

#[test]
fn scenario_an_allowance_for_a_marked_item() {
    // req:check.unreferenced/removal-marked@e3696365
    let lib = format!("// {MARKER}\npub fn old() {{}}\n");
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        ("crates/a/src/lib.rs", &lib),
    ]);
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = clm_unreferenced::command(
        ["--allow", "a::old"],
        dir.path(),
        &|_| None,
        &mut out,
        &mut err,
    );
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "clm.toml: allow a::old skips no finding\n"
    );
    assert!(err.is_empty());
    assert_eq!(status, 1);
}

#[test]
fn markers_that_hold_nothing_in_every_language() {
    // req:check.unreferenced/removal-marked@e3696365
    let lib = format!(
        "/*! {MARKER} */\npub fn inner_block() {{}}\n\
         // {MARKER}\npub fn used() {{}}\npub struct OnlyOld;\n\
         // {MARKER}\npub fn old() -> OnlyOld {{ OnlyOld }}\n"
    );
    let index = format!(
        "// {MARKER}\ndeclare function ambient(): void;\n\
         export {{ ambient }};\n\
         function a() {{}}\n// {MARKER}\nexport {{ a }};\n\
         export function b() {{}} // {MARKER}\nexport function c() {{}}\n\
         export function d() {{\n  // {MARKER}\n}}\n"
    );
    let init = format!(
        "def e(): ...  # {MARKER}\ndef f(): ...\n\
         def g():\n    # {MARKER}\n    ...\n"
    );
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        ("crates/a/src/lib.rs", &lib),
        ("crates/b/Cargo.toml", &manifest("b")),
        ("crates/b/src/main.rs", "fn main() {\n    a::old();\n}\n"),
        ("packages/core/package.json", r#"{"name": "@acme/core"}"#),
        ("packages/core/src/index.ts", &index),
        ("src/acme_core/__init__.py", &init),
    ]);
    // A marked item still carries its signature while its name is used: `OnlyOld` is used
    // through `old`, and `used`, marked, is skipped though nothing uses it.
    assert_eq!(
        names(dir.path()),
        [
            "@acme/core::a",
            "@acme/core::b",
            "@acme/core::c",
            "@acme/core::d",
            "a::inner_block",
            "acme_core::e",
            "acme_core::f",
            "acme_core::g"
        ]
    );
}
