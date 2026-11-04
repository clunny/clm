//! `scan` on TypeScript and JavaScript packages.

mod common;

use common::{TAIL, lines, names, repo};

#[test]
fn scenario_entry_module_and_re_exports() {
    // req:check.unreferenced/typescript-packages@95acf450
    let dir = repo(&[
        (
            "packages/core/package.json",
            r#"{"name": "@acme/core", "main": "dist/index.js"}"#,
        ),
        (
            "packages/core/src/index.ts",
            "export function parse() {}\n\
             export { helper as util } from './helper.js'\n\
             export * from './types'\n\
             export default function main() {}\n\
             function local() {}\n\
             export { local }\n",
        ),
        (
            "packages/core/src/helper.ts",
            "// helpers\nexport function helper() {}\nexport function internal() {}\n",
        ),
        ("packages/core/src/types.ts", "export interface Report {}\n"),
    ]);
    assert_eq!(
        lines(dir.path(), &[]),
        [
            format!("packages/core/src/helper.ts:2: @acme/core::util {TAIL}"),
            format!("packages/core/src/index.ts:1: @acme/core::parse {TAIL}"),
            format!("packages/core/src/index.ts:5: @acme/core::local {TAIL}"),
            format!("packages/core/src/types.ts:1: @acme/core::Report {TAIL}"),
        ]
    );
}

#[test]
fn scenario_exports_map_default_entry_and_no_entry() {
    // req:check.unreferenced/typescript-packages@95acf450
    let dir = repo(&[
        (
            "packages/lib/package.json",
            r#"{"name": "@acme/lib", "exports": {".": {"import": "./dist/index.mjs", "types": "./dist/index.d.ts"}, "./utils": "./dist/utils.js", "./*": "./dist/*.js"}}"#,
        ),
        ("packages/lib/src/index.ts", "export const a = 1;\n"),
        ("packages/lib/src/utils.ts", "export const b = 1;\n"),
        ("packages/lib/src/other.ts", "export const c = 1;\n"),
        ("packages/plain/package.json", r#"{"name": "@acme/plain"}"#),
        ("packages/plain/src/index.ts", "export function d() {}\n"),
        (
            "apps/web/package.json",
            r#"{"name": "web", "main": "dist/bundle.js"}"#,
        ),
        ("apps/web/src/app.ts", "export function e() {}\n"),
    ]);
    assert_eq!(
        names(dir.path()),
        ["@acme/lib::a", "@acme/lib::b", "@acme/plain::d"]
    );
}

#[test]
fn scenario_test_files_node_modules_and_nameless_manifests() {
    // req:check.unreferenced/typescript-packages@95acf450
    let dir = repo(&[
        (
            "packages/core/package.json",
            r#"{"name": "@acme/core", "main": "src/index.ts"}"#,
        ),
        (
            "packages/core/src/index.ts",
            "export * from './x.test'\n\
             export * from '../node_modules/dep'\n\
             export * from '@acme/other'\n",
        ),
        ("packages/core/src/x.test.ts", "export function t() {}\n"),
        (
            "packages/core/node_modules/dep/index.ts",
            "export function n() {}\n",
        ),
        (
            "package.json",
            r#"{"private": true, "workspaces": ["packages/*"]}"#,
        ),
        ("index.ts", "export function r() {}\n"),
    ]);
    assert_eq!(names(dir.path()), Vec::<String>::new());
}

#[test]
fn entry_values_resolve_as_given_with_extensions_and_as_directories() {
    // req:check.unreferenced/typescript-packages@95acf450
    let dir = repo(&[
        (
            "p/package.json",
            r#"{"name": "p", "module": "lib/esm", "types": ["types/main"], "browser": {"./x.js": false}, "typings": "./dist/t.d.mts"}"#,
        ),
        ("p/lib/esm/index.js", "export function viaDir() {}\n"),
        ("p/types/main.tsx", "export function viaExt() {}\n"),
        ("p/src/t.mts", "export function viaMts() {}\n"),
        ("p/x.ts", "export function notBrowser() {}\n"),
    ]);
    assert_eq!(names(dir.path()), ["p::viaDir", "p::viaExt", "p::viaMts"]);
}

#[test]
fn exported_declarations_patterns_imports_and_what_is_not_listed() {
    // req:check.unreferenced/typescript-packages@95acf450
    let dir = repo(&[
        ("p/package.json", r#"{"name": "p", "main": "index.ts"}"#),
        (
            "p/index.ts",
            "export class C {}\n\
             export interface I {}\n\
             export type T = number;\n\
             export enum E { A }\n\
             export namespace N {}\n\
             export const { o1, k: o2 } = obj, [a1] = arr;\n\
             export let l = 1;\n\
             export var v = 1;\n\
             export function over(): void;\n\
             export function over(x?: number) {}\n\
             export declare function amb(): void;\n\
             import { imp as local } from './m';\n\
             import dflt from './m';\n\
             export { local as renamed, dflt };\n\
             export * as ns from './m';\n\
             export { default as d2 } from './m';\n\
             export = legacy;\n\
             module.exports = { cjs };\n",
        ),
        (
            "p/m.ts",
            "export function imp() {}\nexport default function m() {}\n",
        ),
    ]);
    let lines = lines(dir.path(), &[]);
    let over: Vec<_> = lines.iter().filter(|l| l.contains("p::over ")).collect();
    assert_eq!(over, [&format!("p/index.ts:9: p::over {TAIL}")]);
    assert_eq!(
        names(dir.path()),
        [
            "p::C",
            "p::E",
            "p::I",
            "p::N",
            "p::T",
            "p::a1",
            "p::amb",
            "p::l",
            "p::o1",
            "p::o2",
            "p::over",
            "p::renamed",
            "p::v",
        ]
    );
}

#[test]
fn re_export_cycles_end() {
    // req:check.unreferenced/typescript-packages@95acf450
    let dir = repo(&[
        ("p/package.json", r#"{"name": "p", "main": "a.ts"}"#),
        ("p/a.ts", "export * from './b'\nexport function fa() {}\n"),
        ("p/b.ts", "export * from './a'\nexport function fb() {}\n"),
    ]);
    assert_eq!(names(dir.path()), ["p::fa", "p::fb"]);
}

#[test]
fn deepest_package_owns_a_file() {
    // req:check.unreferenced/typescript-packages@95acf450
    let dir = repo(&[
        ("package.json", r#"{"name": "outer", "main": "index.ts"}"#),
        (
            "index.ts",
            "export * from './inner/index'\nexport function o() {}\n",
        ),
        ("inner/package.json", r#"{"name": "inner"}"#),
        ("inner/index.ts", "export function i() {}\n"),
    ]);
    // `inner/index.ts` is `inner`'s file, so `outer` doesn't re-export it, and `outer`'s
    // mention of `inner` counts as a use from outside `inner`, which names nothing it uses.
    assert_eq!(names(dir.path()), ["inner::i", "outer::o"]);
}

#[test]
fn broken_package_json_is_an_error_naming_it() {
    // req:check.unreferenced/report@27b2d762
    let dir = repo(&[("web/package.json", "{\"name\": ")]);
    let err = clm_unreferenced::scan(dir.path(), &[], &[]).unwrap_err();
    assert!(err.starts_with("web/package.json: "), "{err}");
    assert!(!err.contains('\n'), "{err}");
}

#[test]
fn scenario_used_from_another_package_a_test_file_and_an_embedding_file() {
    // req:check.unreferenced/typescript-uses@18d0ccf0
    let dir = repo(&[
        ("packages/core/package.json", r#"{"name": "@acme/core"}"#),
        (
            "packages/core/src/index.ts",
            "export function parse() {}\nexport function build() {}\n\
             export interface Report {}\nexport function stale() {}\n",
        ),
        ("packages/core/src/other.ts", "stale();\n"),
        ("packages/core/src/parse.test.ts", "build();\n"),
        ("apps/web/package.json", r#"{"name": "web"}"#),
        (
            "apps/web/src/main.ts",
            "import { parse } from '@acme/core'\n// stale()\n/* stale */\n",
        ),
        (
            "apps/web/src/App.vue",
            "<script setup lang=\"ts\">\nimport type { Report } from '@acme/core'\n</script>\n",
        ),
    ]);
    assert_eq!(names(dir.path()), ["@acme/core::stale"]);
}

#[test]
fn strings_jsx_text_and_doc_comments_count_and_untracked_files_do_not() {
    // req:check.unreferenced/typescript-uses@18d0ccf0
    let dir = repo(&[
        ("p/package.json", r#"{"name": "p"}"#),
        (
            "p/index.ts",
            "export const inString = 1;\nexport const inJsx = 1;\nexport const inDoc = 1;\n\
             export const inUntracked = 1;\nexport const inHtml = 1;\n",
        ),
        ("app/a.ts", "const k = 'inString';\n/** inDoc */\n"),
        ("app/b.jsx", "const x = <p>inJsx</p>;\n"),
        ("app/page.html", "<!-- inHtml -->\n"),
    ]);
    std::fs::write(dir.path().join("app/c.ts"), "inUntracked;\n").unwrap();
    assert_eq!(names(dir.path()), ["p::inUntracked"]);
}

#[test]
fn scenario_types_reached_through_used_items() {
    // req:check.unreferenced/typescript-uses@18d0ccf0
    let dir = repo(&[
        ("packages/core/package.json", r#"{"name": "@acme/core"}"#),
        (
            "packages/core/src/index.ts",
            "export function load(): Loaded { return { items: [] } }\n\
             export type Item = Row | Column\n\
             export class Row {}\n\
             export class Column {}\n\
             export class Store { private memo: Memo; constructor(opts: Options) {} }\n\
             export interface Memo {}\n\
             export interface Options {}\n\
             export const fetchAll = (q: Query): Row[] => []\n\
             export interface Query {}\n\
             export class Lonely {}\n\
             export function lonely(): Lonely { return new Lonely() }\n\
             interface Loaded { items: Item[] }\n",
        ),
        ("packages/app/package.json", r#"{"name": "app"}"#),
        (
            "packages/app/src/main.ts",
            "load()\nnew Store({})\nfetchAll(q)\n",
        ),
    ]);
    assert_eq!(
        names(dir.path()),
        [
            "@acme/core::Lonely",
            "@acme/core::Memo",
            "@acme/core::lonely"
        ]
    );
}

#[test]
fn renamed_re_export_carries_its_declaration() {
    // req:check.unreferenced/typescript-uses@18d0ccf0
    let dir = repo(&[
        ("p/package.json", r#"{"name": "p"}"#),
        (
            "p/index.ts",
            "export { make as build, Made, Idle } from './make'\n",
        ),
        (
            "p/make.ts",
            "export function make(): Made { return {} }\nexport interface Made {}\n\
             export interface Idle {}\n",
        ),
        ("app/main.ts", "build()\n"),
    ]);
    // `build` is used, and is `make`, whose signature names `Made`.
    assert_eq!(names(dir.path()), ["p::Idle"]);
}

#[test]
fn own_exports_shadow_star_exports() {
    // req:check.unreferenced/typescript-packages@95acf450
    let dir = repo(&[
        ("p/package.json", r#"{"name": "p"}"#),
        ("p/index.ts", "export * from './a'\nexport const x = 1\n"),
        ("p/a.ts", "export const x = 2\nexport const y = 2\n"),
    ]);
    assert_eq!(
        lines(dir.path(), &[]),
        [
            format!("p/a.ts:2: p::y {TAIL}"),
            format!("p/index.ts:2: p::x {TAIL}"),
        ]
    );
}

#[test]
fn fallback_entry_prefers_src_index_and_the_first_extension() {
    // req:check.unreferenced/typescript-packages@95acf450
    let dir = repo(&[
        ("p/package.json", r#"{"name": "p"}"#),
        ("p/src/index.js", "export function fromSrcJs() {}\n"),
        ("p/src/index.ts", "export function fromSrcTs() {}\n"),
        ("p/index.ts", "export function fromRoot() {}\n"),
    ]);
    assert_eq!(names(dir.path()), ["p::fromSrcTs"]);
}

#[test]
fn heritage_interface_extends_and_variable_annotations_carry_private_members_do_not() {
    // req:check.unreferenced/typescript-uses@18d0ccf0
    let dir = repo(&[
        ("p/package.json", r#"{"name": "p"}"#),
        (
            "p/index.ts",
            "export class Shape extends Base implements Drawn { #hidden: Hashed; \
             constructor(private secret: Kept, open: Opened) {} }\n\
             export class Base {}\nexport interface Drawn {}\nexport interface Hashed {}\n\
             export interface Kept {}\nexport interface Opened {}\n\
             export interface Wide extends Narrow {}\nexport interface Narrow {}\n\
             export const config: Settings = {}\nexport interface Settings {}\n\
             export enum Mode { A = Value }\nexport const Value = 1\n",
        ),
        ("app/main.ts", "new Shape(); let w: Wide; config;\nMode.A\n"),
    ]);
    assert_eq!(names(dir.path()), ["p::Hashed", "p::Kept", "p::Value"]);
}

#[test]
fn every_embedding_file_counts_and_other_files_do_not() {
    // req:check.unreferenced/typescript-uses@18d0ccf0
    // req:check.unreferenced/report@27b2d762
    let dir = repo(&[
        ("p/package.json", r#"{"name": "p"}"#),
        (
            "p/index.ts",
            "export const inSvelte = 1\nexport const inAstro = 1\nexport const inMdx = 1\n\
             export const inMarkdown = 1\nexport const inPython = 1\n",
        ),
        ("app/A.svelte", "<script>inSvelte</script>\n"),
        ("app/b.astro", "---\ninAstro\n---\n"),
        ("docs/c.mdx", "<X v={inMdx} />\n"),
        ("docs/d.md", "inMarkdown\n"),
        ("tools/e.py", "inPython\n"),
    ]);
    assert_eq!(names(dir.path()), ["p::inMarkdown", "p::inPython"]);
}

#[test]
fn a_file_that_parses_with_errors_is_used_as_far_as_it_parses() {
    // req:check.unreferenced/report@27b2d762
    let dir = repo(&[
        ("p/package.json", r#"{"name": "p"}"#),
        (
            "p/index.ts",
            "export function kept() {}\nexport function idle() {}\nexport function broken( {\n",
        ),
        ("app/main.ts", "kept(\n"),
    ]);
    std::fs::write(dir.path().join("app/main.ts"), b"kept( \xff\n").unwrap();
    let names = names(dir.path());
    assert!(names.contains(&"p::idle".to_string()), "{names:?}");
    assert!(!names.contains(&"p::kept".to_string()), "{names:?}");
}
