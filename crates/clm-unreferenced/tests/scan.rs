//! `scan`: the public items of a workspace's libraries that nothing outside them uses.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

fn write(dir: &Path, rel: &str, content: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// A git repository holding `files`, all added to the index.
fn repo(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"]);
    for (rel, content) in files {
        write(dir.path(), rel, content);
    }
    git(dir.path(), &["add", "-A"]);
    dir
}

/// A package manifest named `name`.
fn manifest(name: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n")
}

/// The full names `scan` reports, in its order.
fn names(dir: &Path, allow: &[&str]) -> Vec<String> {
    lines(dir, allow, &[])
        .iter()
        .filter_map(|line| {
            let (_, rest) = line.split_once(": ")?;
            rest.strip_suffix(" is public but nothing outside its library uses it")
                .map(str::to_string)
        })
        .collect()
}

/// Every line `scan` prints with these `--allow` and `--entry` values.
fn lines(dir: &Path, allow: &[&str], entry: &[&str]) -> Vec<String> {
    let strings = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    clm_unreferenced::scan(dir, &strings(allow), &strings(entry)).unwrap()
}

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

#[test]
fn scenario_kinds_listed() {
    // req:check.unreferenced/public-items@7833fc92
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a-lib")),
        (
            "crates/a/src/lib.rs",
            "mod x;\npub fn f() {}\npub struct S;\npub(crate) fn g() {}\npub use x::Y;\n\
             impl S {\n    pub fn new() -> Self { S }\n}\n\
             impl Default for S {\n    fn default() -> Self { S }\n}\n\
             pub fn outer() {\n    pub fn inner() {}\n}\n",
        ),
        ("crates/a/src/x.rs", "pub(crate) struct Y;\n"),
    ]);
    assert_eq!(
        sorted(names(dir.path(), &[])),
        ["a_lib::S", "a_lib::S::new", "a_lib::f", "a_lib::outer"]
    );
}

#[test]
fn every_listed_kind_and_restricted_visibility() {
    // req:check.unreferenced/public-items@7833fc92
    let dir = repo(&[
        ("Cargo.toml", &manifest("a")),
        (
            "src/lib.rs",
            "pub enum E { V }\npub union U { x: u32 }\npub trait T { fn m(); }\n\
             pub type Al = u32;\npub const C: u32 = 1;\npub static ST: u32 = 1;\n\
             pub(super) fn ps() {}\npub(self) fn pself() {}\npub(in crate) fn pin() {}\n\
             pub mod m {\n    pub fn deep() {}\n    mod n {\n        pub fn deeper() {}\n    }\n}\n\
             pub struct F { pub field: u32 }\nmacro_rules! mac { () => {} }\n\
             impl F {\n    pub const K: u32 = 1;\n    pub(crate) fn hidden() {}\n    pub type Nope = u32;\n}\n\
             impl T for F {\n    pub fn m() {}\n    pub const TK: u32 = 1;\n}\n\
             const _: () = { pub fn in_block() {} };\n",
        ),
    ]);
    assert_eq!(
        sorted(names(dir.path(), &[])),
        [
            "a::Al",
            "a::C",
            "a::E",
            "a::F",
            "a::F::K",
            "a::ST",
            "a::T",
            "a::U",
            "a::deep",
            "a::deeper",
        ]
    );
}

#[test]
fn scenario_impl_type_names() {
    // req:check.unreferenced/public-items@7833fc92
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "pub struct W<T>(T);\npub trait Tr {}\n\
             impl<T> crate::W<T> {\n    pub fn wrap() {}\n}\n\
             impl dyn Tr {\n    pub fn dynamic() {}\n}\n\
             impl<'a> &'a W<u8> {\n    pub fn by_ref() {}\n}\n",
        ),
        ("crates/b/Cargo.toml", &manifest("b")),
        (
            "crates/b/src/main.rs",
            "fn main() { a::W(1); let _: &dyn a::Tr; }\n",
        ),
    ]);
    assert_eq!(
        sorted(names(dir.path(), &[])),
        ["a::Tr::dynamic", "a::W::by_ref", "a::W::wrap"]
    );
}

#[test]
fn scenario_test_code_skipped() {
    // req:check.unreferenced/public-items@7833fc92
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "mod sub;\nmod inner;\nmod imp;\n#[cfg(test)]\nmod tests {\n    pub fn helper() {}\n}\n\
             #[cfg(all(test, unix))]\npub fn unix_helper() {}\n\
             #[cfg(any(test, all(test, unix)))]\npub fn any_helper() {}\n\
             #[cfg(test)]\nmod fixtures;\npub fn kept() {}\n\
             mod inline {\n    #![cfg(test)]\n    pub fn inline_fixture() {}\n}\n",
        ),
        (
            "crates/a/src/fixtures.rs",
            "pub fn fixture() {}\nmod deep;\n",
        ),
        ("crates/a/src/fixtures/deep.rs", "pub fn deeper() {}\n"),
        ("crates/a/src/sub.rs", "#[cfg(test)]\nmod t;\n"),
        ("crates/a/src/sub/t/mod.rs", "pub fn sub_fixture() {}\n"),
        (
            "crates/a/src/inner.rs",
            "#![cfg(test)]\npub fn inner_fixture() {}\nmod below;\n",
        ),
        ("crates/a/src/inner/below.rs", "pub fn below_fixture() {}\n"),
        (
            "crates/a/src/imp.rs",
            "pub struct I;\n#[cfg(test)]\nimpl I {\n    pub fn test_new() {}\n}\n",
        ),
    ]);
    assert_eq!(names(dir.path(), &[]), ["a::I", "a::kept"]);
}

#[test]
fn scenario_not_test_is_checked() {
    // req:check.unreferenced/public-items@7833fc92
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "#[cfg(not(test))]\npub fn prod_only() {}\n\
             #[cfg(any(test, feature = \"x\"))]\npub fn shared() {}\n\
             #[cfg(any())]\npub fn off() {}\n\
             #[cfg(feature = \"test\")]\npub fn named_test() {}\n\
             #[cfg(all(unix, feature = \"a,test)\"))]\npub fn quoted() {}\n\
             mod real;\n",
        ),
        (
            "crates/a/src/real.rs",
            "#![cfg(not(test))]\npub fn real_only() {}\n",
        ),
    ]);
    assert_eq!(
        sorted(names(dir.path(), &[])),
        [
            "a::named_test",
            "a::off",
            "a::prod_only",
            "a::quoted",
            "a::real_only",
            "a::shared"
        ]
    );
}

#[test]
fn raw_strings_in_cfg_and_a_module_declared_both_ways() {
    // req:check.unreferenced/public-items@7833fc92
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "#[cfg(all(test, feature = r\"x\"))]\npub fn raw() {}\n\
             #[cfg(all(test, feature = r#\"y\"#))]\npub fn hashed() {}\n\
             #[cfg(not(test))]\nmod imp;\n#[cfg(test)]\nmod imp;\n",
        ),
        ("crates/a/src/imp.rs", "pub fn in_imp() {}\n"),
    ]);
    assert_eq!(names(dir.path(), &[]), ["a::in_imp"]);
}

#[test]
fn test_items_of_an_impl_and_modules_of_another_package_are_not_listed() {
    // req:check.unreferenced/public-items@7833fc92
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "pub struct S;\nimpl S {\n    #[cfg(test)]\n    pub fn test_new() {}\n}\nmod nested;\n",
        ),
        ("crates/a/src/nested/Cargo.toml", &manifest("nested")),
        ("crates/a/src/nested/mod.rs", "pub fn in_nested() {}\n"),
        ("crates/b/Cargo.toml", &manifest("b")),
        ("crates/b/src/main.rs", "fn main() { let _ = a::S; }\n"),
    ]);
    assert_eq!(names(dir.path(), &[]), Vec::<String>::new());
}

#[test]
fn scenario_module_tree_of_lib_rs() {
    // req:check.unreferenced/public-items@7833fc92
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "pub mod x;\nmod outer {\n    mod inner;\n}\n",
        ),
        ("crates/a/src/x/mod.rs", "pub fn in_x() {}\nmod y;\n"),
        ("crates/a/src/x/y.rs", "pub fn in_y() {}\n"),
        ("crates/a/src/outer/inner.rs", "pub fn in_inner() {}\n"),
        ("crates/a/src/stray.rs", "pub fn in_stray() {}\n"),
    ]);
    assert_eq!(
        sorted(names(dir.path(), &[])),
        ["a::in_inner", "a::in_x", "a::in_y"]
    );
}

#[test]
fn path_attributes_blocks_and_untracked_files_are_not_followed() {
    // req:check.unreferenced/public-items@7833fc92
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "#[path = \"elsewhere.rs\"]\nmod p;\nfn body() {\n    mod b;\n}\nmod both;\nmod gone;\n",
        ),
        ("crates/a/src/p.rs", "pub fn in_p() {}\n"),
        ("crates/a/src/elsewhere.rs", "pub fn in_elsewhere() {}\n"),
        ("crates/a/src/b.rs", "pub fn in_b() {}\n"),
        ("crates/a/src/both.rs", "pub fn in_both_rs() {}\n"),
        ("crates/a/src/both/mod.rs", "pub fn in_both_mod() {}\n"),
    ]);
    write(dir.path(), "crates/a/src/gone.rs", "pub fn in_gone() {}\n");
    assert_eq!(
        sorted(names(dir.path(), &[])),
        ["a::in_both_mod", "a::in_both_rs"]
    );
}

#[test]
fn scenario_binary_modules() {
    // req:check.unreferenced/public-items@7833fc92
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
        (
            "crates/a/src/main.rs",
            "mod cli;\n#[cfg(test)]\nmod tests;\nfn main() {}\n",
        ),
        ("crates/a/src/cli.rs", "pub fn run() {\n    a::f();\n}\n"),
        ("crates/a/src/tests.rs", "pub fn t_helper() {}\n"),
    ]);
    assert_eq!(names(dir.path(), &[]), Vec::<String>::new());
}

#[test]
fn scenario_entry_points_skipped() {
    // req:check.unreferenced/entry-attributes@69e092f6
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "pub struct S;\n\
             #[no_mangle]\npub extern \"C\" fn c_entry() {}\n\
             #[unsafe(no_mangle)]\npub extern \"C\" fn c2() {}\n\
             #[unsafe(export_name = \"e\")]\npub extern \"C\" fn c3() {}\n\
             #[wasm_bindgen(js_name = jsE)]\npub fn js_entry() {}\n\
             #[wasm_bindgen]\nimpl S {\n    pub fn js_method() {}\n}\n\
             #[proc_macro]\npub fn m1(_: TokenStream) -> TokenStream { todo!() }\n\
             #[proc_macro_derive(D)]\npub fn m2(_: TokenStream) -> TokenStream { todo!() }\n\
             #[proc_macro_attribute]\n\
             pub fn m3(_: TokenStream, _: TokenStream) -> TokenStream { todo!() }\n\
             impl S {\n    #[no_mangle]\n    pub fn one() {}\n    pub fn two() {}\n}\n",
        ),
        ("crates/b/Cargo.toml", &manifest("b")),
        ("crates/b/src/main.rs", "fn main() { let _ = a::S; }\n"),
    ]);
    assert_eq!(names(dir.path(), &[]), ["a::S::two"]);
}

#[test]
fn scenario_entry_given_as_an_argument() {
    // req:check.unreferenced/entry-attributes@69e092f6
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "#[event(fetch)]\npub async fn fetch() {}\n\
             #[worker::event(scheduled)]\npub async fn tick() {}\n",
        ),
    ]);
    assert_eq!(names(dir.path(), &[]), ["a::fetch", "a::tick"]);
    assert!(lines(dir.path(), &[], &["event"]).is_empty());
}

#[test]
fn scenario_stale_entry() {
    // req:check.unreferenced/entry-attributes@69e092f6
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "pub struct S;\n#[cfg(test)]\n#[napi]\npub fn t() {}\n\
             #[napi]\nfn private() {}\n#[napi]\nimpl S {}\n\
             fn body() {\n    #[napi]\n    pub fn inner() {}\n}\n",
        ),
        ("crates/b/Cargo.toml", &manifest("b")),
        ("crates/b/src/main.rs", "fn main() { let _ = a::S; }\n"),
    ]);
    assert_eq!(
        lines(dir.path(), &["zz"], &["napi"]),
        [
            "clm.toml: allow zz skips no finding",
            "clm.toml: entry napi marks no item",
        ]
    );
}

#[test]
fn scenario_entry_on_a_used_item() {
    // req:check.unreferenced/entry-attributes@69e092f6
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "#[napi]\npub fn used_entry() {}\n#[napi]\npub fn idle_entry() {}\n",
        ),
        ("crates/b/Cargo.toml", &manifest("b")),
        ("crates/b/src/main.rs", "fn main() { a::used_entry(); }\n"),
    ]);
    assert_eq!(
        lines(dir.path(), &["a::idle_entry"], &["napi", "worker::napi"]),
        [
            "clm.toml: allow a::idle_entry skips no finding",
            "clm.toml: entry worker::napi marks no item",
        ]
    );
}

#[test]
fn entry_on_an_impl_block_marks_its_items() {
    // req:check.unreferenced/entry-attributes@69e092f6
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "pub struct S;\n#[napi]\nimpl S {\n    pub fn method() {}\n}\n",
        ),
        ("crates/b/Cargo.toml", &manifest("b")),
        ("crates/b/src/main.rs", "fn main() { let _ = a::S; }\n"),
    ]);
    assert_eq!(names(dir.path(), &[]), ["a::S::method"]);
    assert!(lines(dir.path(), &[], &["napi"]).is_empty());
}

#[test]
fn scenario_binaries_and_lib_name() {
    // req:check.unreferenced/public-items@7833fc92
    let dir = repo(&[
        (
            "crates/a/Cargo.toml",
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[lib]\nname = \"core\"\n",
        ),
        ("crates/a/src/lib.rs", "pub fn lib_item() {}\n"),
        (
            "crates/a/src/main.rs",
            "pub fn in_main() {}\nfn main() {}\n",
        ),
        (
            "crates/a/src/bin/tool.rs",
            "pub fn in_bin() {}\nfn main() {}\n",
        ),
        // A package without a tracked `src/lib.rs` is no library.
        ("crates/app/Cargo.toml", &manifest("app")),
        (
            "crates/app/src/main.rs",
            "pub fn app_item() {}\nfn main() {}\n",
        ),
        ("crates/app/src/util.rs", "pub fn app_util() {}\n"),
    ]);
    assert_eq!(names(dir.path(), &[]), ["core::lib_item"]);
}

#[test]
fn nested_package_owns_its_files() {
    // req:check.unreferenced/public-items@7833fc92
    let dir = repo(&[
        ("Cargo.toml", &manifest("outer")),
        ("src/lib.rs", "pub fn outer_item() {}\n"),
        ("src/inner/Cargo.toml", &manifest("inner")),
        ("src/inner/src/lib.rs", "pub fn inner_item() {}\n"),
    ]);
    assert_eq!(
        names(dir.path(), &[]),
        ["inner::inner_item", "outer::outer_item"]
    );
}

#[test]
fn scenario_used_by_another_crate() {
    // req:check.unreferenced/references@daf06376
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        ("crates/a/src/lib.rs", "pub fn f() {}\npub fn h() {}\n"),
        ("crates/b/Cargo.toml", &manifest("b")),
        ("crates/b/src/main.rs", "fn main() {\n    a::f();\n}\n"),
    ]);
    assert_eq!(names(dir.path(), &[]), ["a::h"]);
}

#[test]
fn scenario_used_only_in_its_own_library() {
    // req:check.unreferenced/references@daf06376
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        ("crates/a/src/lib.rs", "mod x;\nfn g() {\n    x::f();\n}\n"),
        ("crates/a/src/x.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(names(dir.path(), &[]), ["a::f"]);
}

#[test]
fn scenario_comments_strings_doctests_and_tests() {
    // req:check.unreferenced/references@daf06376
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "pub fn f() {}\npub fn g() {}\npub fn d() {}\npub fn k() {}\npub fn e() {}\n\
             pub fn i1() {}\npub fn i2() {}\npub fn bs() {}\npub fn be() {}\npub fn ex() {}\n\
             pub fn bi() {}\n",
        ),
        ("crates/a/tests/t.rs", "#[test]\nfn t() {\n    a::k();\n}\n"),
        ("crates/a/build.rs", "fn main() { a::bs(); }\n"),
        ("crates/a/benches/b.rs", "fn main() { a::be(); }\n"),
        ("crates/a/examples/x.rs", "fn main() { a::ex(); }\n"),
        ("crates/a/src/bin/tool.rs", "fn main() { a::bi(); }\n"),
        ("crates/d/Cargo.toml", &manifest("d")),
        ("crates/d/src/lib.rs", "//! a::i1()\n/*! a::i2() */\n"),
        ("crates/b/Cargo.toml", &manifest("b")),
        (
            "crates/b/src/lib.rs",
            "// f is unused\n/* f */\n//// f\n/**/\n\
             #[derive(Deserialize)]\npub struct X {\n    #[serde(default = \"g\")]\n    x: u32,\n}\n\
             /// a::d()\npub fn uses() {}\n\
             /** e */\npub fn more() {}\n",
        ),
        ("crates/c/Cargo.toml", &manifest("c")),
        (
            "crates/c/src/main.rs",
            "fn main() { b::X; b::uses(); b::more(); }\n",
        ),
    ]);
    assert_eq!(names(dir.path(), &[]), ["a::f"]);
}

#[test]
fn scenario_through_an_exported_macro() {
    // req:check.unreferenced/references@daf06376
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "pub fn helper() {}\npub fn inner_helper() {}\npub fn idle_helper() {}\n\
             pub fn plain_helper() {}\n\
             #[macro_export]\nmacro_rules! outer { () => { $crate::helper(); $crate::inner!() } }\n\
             #[macro_export]\nmacro_rules! inner { () => { $crate::inner_helper() } }\n\
             #[macro_export]\nmacro_rules! idle { () => { $crate::idle_helper() } }\n\
             macro_rules! local { () => { $crate::plain_helper() } }\n",
        ),
        ("crates/b/Cargo.toml", &manifest("b")),
        ("crates/b/src/main.rs", "fn main() { a::outer!(); }\n"),
    ]);
    assert_eq!(
        names(dir.path(), &[]),
        ["a::idle_helper", "a::plain_helper"]
    );
}

#[test]
fn macro_carriers_at_any_depth_with_matchers_outside_comments_and_tests() {
    // req:check.unreferenced/references@daf06376
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "pub fn in_body() {}\npub fn in_matcher() {}\npub fn commented() {}\n\
             pub fn in_test() {}\npub fn second_rule() {}\n\
             fn host() {\n    #[macro_export(local_inner_macros)]\n    #[doc(hidden)]\n\
             macro_rules! deep { (in_matcher) => { $crate::in_body() /* commented */ };\n\
             () => { second_rule() } }\n}\n\
             #[cfg(test)]\nmod t {\n    #[macro_export]\n    macro_rules! tested { () => { in_test() } }\n}\n",
        ),
        ("crates/b/Cargo.toml", &manifest("b")),
        (
            "crates/b/src/main.rs",
            "fn main() { a::deep!(); a::tested!(); }\n",
        ),
    ]);
    assert_eq!(names(dir.path(), &[]), ["a::commented", "a::in_test"]);
}

#[test]
fn scenario_through_a_renamed_re_export() {
    // req:check.unreferenced/references@daf06376
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "mod inner;\npub use inner::{orig as renamed, other as alias};\n",
        ),
        (
            "crates/a/src/inner.rs",
            "pub fn orig() {}\npub fn other() {}\n",
        ),
        ("crates/b/Cargo.toml", &manifest("b")),
        ("crates/b/src/main.rs", "fn main() { a::renamed(); }\n"),
    ]);
    assert_eq!(names(dir.path(), &[]), ["a::other"]);
}

#[test]
fn re_exports_chain_and_skip_restricted_self_and_underscore() {
    // req:check.unreferenced/references@daf06376
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "mod inner;\npub use inner::deep as mid;\npub use self::mid as top;\n\
             pub(crate) use inner::hidden as shown;\npub use inner::under as _;\n\
             pub use inner::{self as module};\nuse inner::private as public;\n",
        ),
        (
            "crates/a/src/inner.rs",
            "pub fn deep() {}\npub fn hidden() {}\npub fn under() {}\npub fn private() {}\n",
        ),
        ("crates/b/Cargo.toml", &manifest("b")),
        (
            "crates/b/src/main.rs",
            "fn main() { a::top(); a::shown(); a::_(); a::module(); a::public(); }\n",
        ),
    ]);
    assert_eq!(
        names(dir.path(), &[]),
        ["a::hidden", "a::under", "a::private"]
    );
}

#[test]
fn carriers_in_a_test_module_file_and_other_files_carry_nothing() {
    // req:check.unreferenced/references@daf06376
    // req:check.unreferenced/report@27b2d762
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "pub fn h() {}\npub fn in_readme() {}\n#[cfg(test)]\nmod t;\n",
        ),
        (
            "crates/a/src/t.rs",
            "#[macro_export]\nmacro_rules! tm { () => { $crate::h() } }\n",
        ),
        ("crates/a/README.md", "Call `a::in_readme()`.\n"),
        ("crates/b/Cargo.toml", &manifest("b")),
        ("crates/b/src/main.rs", "fn main() { a::tm!(); }\n"),
    ]);
    assert_eq!(names(dir.path(), &[]), ["a::h", "a::in_readme"]);
}

#[test]
fn renames_of_raw_identifiers_carry() {
    // req:check.unreferenced/references@daf06376
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "pub fn r#match() {}\npub fn orig() {}\n\
             pub use self::r#match as found;\npub use self::orig as r#async;\n",
        ),
        ("crates/b/Cargo.toml", &manifest("b")),
        (
            "crates/b/src/main.rs",
            "fn main() { a::found(); a::r#async(); }\n",
        ),
    ]);
    assert_eq!(names(dir.path(), &[]), Vec::<String>::new());
}

#[test]
fn scenario_whole_words_only() {
    // req:check.unreferenced/references@daf06376
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        ("crates/a/src/lib.rs", "pub fn get() {}\npub fn put() {}\n"),
        ("crates/b/Cargo.toml", &manifest("b")),
        (
            "crates/b/src/main.rs",
            "fn main() {\n    get_all();\n    x.put();\n}\n",
        ),
    ]);
    assert_eq!(names(dir.path(), &[]), ["a::get"]);
}

#[test]
fn scenario_untracked_user() {
    // req:check.unreferenced/references@daf06376
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
    ]);
    write(dir.path(), "crates/b/Cargo.toml", &manifest("b"));
    write(
        dir.path(),
        "crates/b/src/main.rs",
        "fn main() { a::f(); }\n",
    );
    // An untracked library declares nothing either.
    write(dir.path(), "crates/c/Cargo.toml", &manifest("c"));
    write(dir.path(), "crates/c/src/lib.rs", "pub fn untracked() {}\n");
    assert_eq!(names(dir.path(), &[]), ["a::f"]);
}

#[test]
fn tracked_file_missing_from_the_working_tree_is_skipped() {
    // req:check.unreferenced/references@daf06376
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
        ("crates/b/Cargo.toml", &manifest("b")),
        ("crates/b/src/main.rs", "fn main() { a::f(); }\n"),
    ]);
    fs::remove_file(dir.path().join("crates/b/src/main.rs")).unwrap();
    assert_eq!(names(dir.path(), &[]), ["a::f"]);
}

#[test]
fn scenario_allowed_item_and_crate() {
    // req:check.unreferenced/allow-list@8bbdff3e
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        ("crates/a/src/lib.rs", "pub fn f() {}\npub fn g() {}\n"),
        ("crates/b/Cargo.toml", &manifest("b")),
        ("crates/b/src/lib.rs", "pub fn h() {}\n"),
    ]);
    assert_eq!(names(dir.path(), &["a::f", "b"]), ["a::g"]);
}

#[test]
fn allow_entry_skips_every_item_with_its_full_name() {
    // req:check.unreferenced/allow-list@8bbdff3e
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "pub mod x {\n    pub fn f() {}\n}\npub mod y {\n    pub fn f() {}\n}\n",
        ),
    ]);
    assert_eq!(names(dir.path(), &[]), ["a::f", "a::f"]);
    let out = clm_unreferenced::scan(dir.path(), &["a::f".to_string()], &[]).unwrap();
    assert!(out.is_empty(), "{out:?}");
}

#[test]
fn scenario_stale_entries() {
    // req:check.unreferenced/allow-list@8bbdff3e
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        ("crates/a/src/lib.rs", "pub fn used() {}\n"),
        ("crates/b/Cargo.toml", &manifest("b")),
        ("crates/b/src/main.rs", "fn main() { a::used(); }\n"),
    ]);
    let allow = ["a::gone", "zz", "a::used", "a"].map(String::from);
    assert_eq!(
        clm_unreferenced::scan(dir.path(), &allow, &[]).unwrap(),
        [
            "clm.toml: allow a::gone skips no finding",
            "clm.toml: allow zz skips no finding",
            "clm.toml: allow a::used skips no finding",
            "clm.toml: allow a skips no finding",
        ]
    );
}

#[test]
fn findings_sorted_by_path_then_line_number() {
    // req:check.unreferenced/report@27b2d762
    let mut lib = String::from("mod x;\n\npub fn f() {}\n");
    lib.push_str(&"\n".repeat(6));
    lib.push_str("pub fn z() {}\n");
    let mut x = "\n".repeat(11);
    x.push_str("pub fn h() {}\n");
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        ("crates/a/src/lib.rs", &lib),
        ("crates/a/src/x.rs", &x),
    ]);
    let tail = "is public but nothing outside its library uses it";
    assert_eq!(
        clm_unreferenced::scan(dir.path(), &[], &[]).unwrap(),
        [
            format!("crates/a/src/lib.rs:3: a::f {tail}"),
            format!("crates/a/src/lib.rs:10: a::z {tail}"),
            format!("crates/a/src/x.rs:12: a::h {tail}"),
        ]
    );
}

#[test]
fn invalid_utf8_and_parse_errors_are_used_as_far_as_they_parse() {
    // req:check.unreferenced/report@27b2d762
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "pub fn f() {}\npub fn unused() {}\npub fn broken( {\n",
        ),
    ]);
    let mut bytes = b"fn main() { a::f(); }\n// \xff\xfe\n".to_vec();
    bytes.extend_from_slice(b"fn x( {\n");
    fs::create_dir_all(dir.path().join("crates/b/src")).unwrap();
    fs::write(dir.path().join("crates/b/Cargo.toml"), manifest("b")).unwrap();
    fs::write(dir.path().join("crates/b/src/main.rs"), bytes).unwrap();
    git(dir.path(), &["add", "-A"]);
    let out = clm_unreferenced::scan(dir.path(), &[], &[]).unwrap();
    assert!(!out.iter().any(|l| l.contains("a::f ")), "{out:?}");
    assert!(out.iter().any(|l| l.contains("a::unused ")), "{out:?}");
}

#[cfg(unix)]
#[test]
fn unreadable_tracked_file_is_an_error_naming_it() {
    // req:check.unreferenced/report@27b2d762
    use std::os::unix::fs::PermissionsExt;
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
    ]);
    let lib = dir.path().join("crates/a/src/lib.rs");
    fs::set_permissions(&lib, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read(&lib).is_ok() {
        return; // Running as root, which reads it anyway.
    }
    let err = clm_unreferenced::scan(dir.path(), &[], &[]).unwrap_err();
    fs::set_permissions(&lib, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(err.starts_with("crates/a/src/lib.rs: "), "{err}");
}

#[test]
fn broken_manifests_are_errors_naming_the_file() {
    // req:check.unreferenced/report@27b2d762
    let dir = repo(&[("crates/a/Cargo.toml", "[package\nname = 1\n")]);
    let err = clm_unreferenced::scan(dir.path(), &[], &[]).unwrap_err();
    assert!(err.starts_with("crates/a/Cargo.toml: "), "{err}");
    assert!(!err.contains('\n'), "{err}");
    let dir = repo(&[("crates/a/Cargo.toml", "[package]\nversion = \"1\"\n")]);
    let err = clm_unreferenced::scan(dir.path(), &[], &[]).unwrap_err();
    assert!(err.starts_with("crates/a/Cargo.toml: "), "{err}");
    // A manifest without `[package]`, such as a virtual workspace's, is no package.
    let dir = repo(&[("Cargo.toml", "[workspace]\nmembers = []\n")]);
    assert!(
        clm_unreferenced::scan(dir.path(), &[], &[])
            .unwrap()
            .is_empty()
    );
}

#[test]
fn git_failing_is_an_error_naming_the_command() {
    // req:check.unreferenced/report@27b2d762
    let dir = TempDir::new().unwrap();
    let err = clm_unreferenced::scan(dir.path(), &[], &[]).unwrap_err();
    assert!(err.contains("git"), "{err}");
    assert!(!err.contains('\n'), "{err}");
}

#[test]
fn scenario_types_reached_through_used_items() {
    // req:check.unreferenced/signature-carriers@fa9799f5
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "pub struct Report {\n    pub count: Count,\n    hidden: Secret,\n}\n\
             pub struct Count;\npub struct Secret;\n\
             pub fn report() -> Report { todo!() }\n\
             pub struct Params;\npub fn take(p: Params) {}\n\
             pub enum Kind {\n    One(Inner),\n}\npub struct Inner;\n\
             impl Report {\n    pub fn tail(&self) -> Tail { Tail }\n}\npub struct Tail;\n\
             pub struct Idle;\npub fn idle_fn() -> Idle { Idle }\n",
        ),
        ("crates/b/Cargo.toml", &manifest("b")),
        (
            "crates/b/src/main.rs",
            "fn main() {\n    let r = a::report();\n    a::take(x);\n    r.tail();\n    \
             match k {\n        a::Kind::One(_) => {}\n    }\n}\n",
        ),
    ]);
    assert_eq!(
        sorted(names(dir.path(), &[])),
        ["a::Idle", "a::Secret", "a::idle_fn"]
    );
}

#[test]
fn scenario_chains_traits_aliases_and_impl_headers() {
    // req:check.unreferenced/signature-carriers@fa9799f5
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "pub fn view() -> View { todo!() }\n\
             pub struct View {\n    pub root: Root,\n}\n\
             pub struct Root {\n    pub held: Held,\n}\npub struct Held;\n\
             pub trait Tr {\n    fn out(&self) -> Out;\n}\npub struct Out;\n\
             pub type Alias = Base<Arg>;\npub struct Base<T>(T);\npub struct Arg;\n\
             pub struct Shown;\n\
             impl From<Shown> for View {\n    fn from(_: Shown) -> View { todo!() }\n}\n\
             #[no_mangle]\npub extern \"C\" fn entry() -> Exported { todo!() }\n\
             pub struct Exported;\n\
             pub(crate) fn hidden() -> Lonely { todo!() }\npub struct Lonely;\n",
        ),
        ("crates/b/Cargo.toml", &manifest("b")),
        (
            "crates/b/src/main.rs",
            "struct X;\nimpl a::Tr for X {}\nfn main() {\n    a::view();\n    \
             let _: a::Alias;\n}\n",
        ),
    ]);
    assert_eq!(names(dir.path(), &[]), ["a::Lonely"]);
}

#[test]
fn every_signature_part_carries() {
    // req:check.unreferenced/signature-carriers@fa9799f5
    // Each used item names one type in one part of its signature, and nothing else names it.
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "pub fn generic<T: Bound>(t: T) where T: Clause {}\n\
             pub const C: ConstTy = ConstTy;\npub static S: StaticTy = StaticTy;\n\
             pub union U {\n    pub f: UnionTy,\n}\n\
             pub struct G<T: StructBound>(T) where T: StructClause;\n\
             pub enum E {\n    V { field: VariantTy },\n}\n\
             pub trait T2: Super {\n    fn m(&self, x: ParamTy);\n}\n\
             pub struct Bound;\npub struct Clause;\npub struct ConstTy;\npub struct StaticTy;\n\
             pub struct UnionTy;\npub struct StructBound;\npub struct StructClause;\n\
             pub struct VariantTy;\npub struct Super;\npub struct ParamTy;\n",
        ),
        ("crates/b/Cargo.toml", &manifest("b")),
        (
            "crates/b/src/main.rs",
            "fn main() {\n    a::generic(1);\n    a::C;\n    a::S;\n    let _: a::U;\n    \
             let _: a::G<()>;\n    let _: a::E;\n    let _: &dyn a::T2;\n}\n",
        ),
    ]);
    assert_eq!(names(dir.path(), &[]), Vec::<String>::new());
}

#[test]
fn restricted_test_and_trait_impl_members_carry_nothing() {
    // req:check.unreferenced/signature-carriers@fa9799f5
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "pub struct W {\n    pub(crate) inner: Private,\n}\npub struct Private;\n\
             #[cfg(test)]\npub fn tested() -> FromTest { todo!() }\npub struct FromTest;\n\
             pub struct Thing;\n\
             impl Iterator for Thing {\n    type Item = Yielded;\n    \
             fn next(&mut self) -> Option<Yielded> { None }\n}\npub struct Yielded;\n\
             impl Thing {\n    pub(crate) fn crate_only(&self) -> CrateTy { todo!() }\n}\n\
             pub struct CrateTy;\n",
        ),
        ("crates/b/Cargo.toml", &manifest("b")),
        (
            "crates/b/src/main.rs",
            "fn main() {\n    let w: a::W;\n    tested();\n    next();\n    crate_only();\n}\n",
        ),
    ]);
    // `Yielded` is carried under `Thing` by the trait impl, and nothing outside uses `Thing`.
    assert_eq!(
        sorted(names(dir.path(), &[])),
        [
            "a::CrateTy",
            "a::FromTest",
            "a::Private",
            "a::Thing",
            "a::Yielded"
        ]
    );
}

#[test]
fn impl_headers_trait_impl_members_tuple_fields_and_parameter_names_carry() {
    // req:check.unreferenced/signature-carriers@fa9799f5
    // Each type below is carried by exactly one route.
    let dir = repo(&[
        ("crates/a/Cargo.toml", &manifest("a")),
        (
            "crates/a/src/lib.rs",
            "pub struct Holder<T>(T);\nimpl<T: HeaderBound> Holder<T> {}\npub struct HeaderBound;\n\
             pub struct Seq;\n\
             impl Iterator for Seq {\n    type Item = MemberTy;\n    \
             fn next(&mut self) -> Option<Self::Item> { None }\n}\npub struct MemberTy;\n\
             pub struct Pair(pub TupleTy, Hidden);\npub struct TupleTy;\npub struct Hidden;\n\
             pub fn named(param_name: u8) {}\npub fn param_name() {}\n\
             #[cfg(test)]\nimpl Seq {\n    pub fn test_only() -> TestImplTy { todo!() }\n}\n\
             pub struct TestImplTy;\n",
        ),
        ("crates/b/Cargo.toml", &manifest("b")),
        (
            "crates/b/src/main.rs",
            "fn main() {\n    let _: a::Holder<()>;\n    let _: a::Seq;\n    let _: a::Pair;\n    \
             a::named(1);\n    test_only();\n}\n",
        ),
    ]);
    assert_eq!(
        sorted(names(dir.path(), &[])),
        ["a::Hidden", "a::TestImplTy"]
    );
}
