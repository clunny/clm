---
title: Unreferenced public items
status: active
owners: ["@clunny"]
depends_on: [vcs]
paths: ["crates/clm-unreferenced/**"]
successor: ""
serves: [capability/no-dead-weight, architecture/no-dead-code]
---

## Contract
`clm unreferenced [--root <dir>] [--allow <entry>]... [--entry <attribute>]...` lists the public items of a workspace's libraries that nothing outside their own library uses, one line per item, and exits 1 when it lists any, so a project can name it as one of its dead-code detectors in `clm.toml` (`dead-code-detectors`, `public-item-detector`): `command = ["clm", "unreferenced"]`, with an `--allow` pair per accepted exception and an `--entry` pair per Rust attribute that marks an item called from outside Rust beyond those the detector knows (`unreferenced-entry-attributes`). A library is a Rust package with `src/lib.rs`, whose public items are its `pub` items, or a TypeScript or Python package, whose public items are the names its entry modules export (`unreferenced-package-api`). rustc's `dead_code` lint, `knip` and `vulture` each assume or see one package; in an application workspace a public item rarely has users outside it, and this detector covers that gap. A type that a used public item names in its signature counts as used (`unreferenced-signature-carriers`). It exits 0 when it lists nothing, including in a repository with no library, and 2 when it can't read the repository. The library call `scan(root, allow, entry)` returns the findings it prints.

## Design
Crate `clm-unreferenced`, a library with `scan` and the command's text, dispatched from `crates/clm` as `clm unreferenced`; this component's build edits `crates/clm/src/main.rs`, `Cargo.toml` and `Cargo.lock` for it, as the root's Design says. `--root` defaults as `clm survey`'s does: the CLM repository found from the current directory, else the top of the git repository, so a project without CLM's specs gets its whole workspace. It lists tracked files with `git ls-files --cached` through `clm-git` and reads them from the working tree, skipping a tracked file missing from it, as `clm survey` does, and reading a source file's bytes as UTF-8 with invalid sequences replaced. It reads only the manifests, sources and embedding files `report` lists, and never parses an embedding file. It parses Rust, TypeScript (the `typescript` language for `.ts`, `.mts`, `.cts`, `.js`, `.mjs` and `.cjs`, the `tsx` language for `.tsx` and `.jsx`) and Python with the `tree-sitter`, `tree-sitter-rust`, `tree-sitter-typescript` and `tree-sitter-python` crates at the version strings `clm-init`'s `Cargo.toml` pins, copied rather than moved to the workspace, so the workspace builds one copy of each grammar without touching `init`'s files (`tree-sitter-analysis`); it walks each syntax tree with a cursor rather than queries, so it needs no `streaming-iterator`. Each language is a module of its own (`rust.rs`, `typescript.rs`, `python.rs`) that reads a file into one shape: the names it declares and exports, with their lines, its carriers, and its mentions; `scan` then matches each language's items against mentions in that language's files, so a Python name and a TypeScript name never hide each other. It reads `Cargo.toml` and `pyproject.toml` with the workspace's `toml` and `package.json` with the workspace's `serde_json`. A file tree-sitter parses with errors is used as far as it parses, never a failure. It never runs `cargo`, `npm`, Node.js or `python`: the tracked files are the workspace, so the result is the same on every machine and needs no build.

Tests that run the `clm` binary (`clm unreferenced`, and `clm check` naming it as a detector) live in `crates/clm/tests/unreferenced.rs`, since `CARGO_BIN_EXE_clm` exists only there, and build their fixtures as real git repositories with `git` on `PATH`; `scan`'s own tests may live in the crate. The `clm check` fixture needs a `specs/SPEC.md` that passes, as `crates/clm/tests/cli.rs`'s detector fixtures have.

**Libraries.** Each tracked `Cargo.toml` with a `[package]` table is a package rooted at its directory; a tracked file belongs to the package with the deepest root that holds it. A package is a library when it has a tracked `src/lib.rs`, named `[lib] name` when set, else `package.name` with `-` replaced by `_`. Its *library files* are the module tree of `src/lib.rs` as rustc finds it (`unreferenced-library-modules`), `m.rs` and `m/mod.rs` both when both are tracked, as `public-items` spells out; a declaration carrying `#[path]`, one inside a block, or one whose file isn't tracked isn't followed. Every other `.rs` file of the package (`src/main.rs` and the modules it declares, `src/bin/`, a file no module declares) is not a library file: its items aren't listed, and its mentions count as uses from outside the library.

**Public items.** In each library file, the detector lists the items whose visibility is exactly `pub` (not `pub(crate)`, `pub(super)`, `pub(self)` or `pub(in …)`) declared directly in a module, the file's or an inline `mod { }` at any depth: `fn`, `struct`, `enum`, `union`, `trait`, `type`, `const` and `static` items, and the `fn` and `const` items of an inherent `impl` block (one without `for`) declared directly in a module. Items inside function bodies or other blocks aren't listed. A full name carries no module path: `<crate>::<name>`, or `<crate>::<Type>::<name>` for an `impl` item, where `<Type>` is the last path segment of the `impl`'s type without generic arguments, references or `dyn`. Two items with one full name are two findings, and one `--allow` entry skips both.

It skips test code, as `public-items` defines it: items, `impl`s and inline modules under a `#[cfg(…)]` that holds only when `test` is set, files opening with such a `#![cfg(…)]`, and the module tree of a `#[cfg(test)] mod m;`; `not(…)` never counts, so `cfg(not(test))` code is checked. It skips entry points called from outside Rust: an item, or an item of an `impl` block, carrying an attribute whose last path segment (`#[worker::event(fetch)]` → `event`) is `no_mangle`, `export_name`, `wasm_bindgen`, `proc_macro`, `proc_macro_derive` or `proc_macro_attribute`, or one given with `--entry`, bare or inside `unsafe(…)`, with or without arguments. `cfg_attr` isn't looked into. `pub use`, `pub mod`, macros, fields, variants and trait members aren't listed: a re-export or a module is used through the names it exposes, which are listed where they are declared.

**References.** An item is used when its name, the last segment for a method, appears as a whole word (bounded by characters other than ASCII letters, digits and `_`) in the text of any tracked `.rs` file other than its own library's files, outside plain comments as tree-sitter parses them. Strings count, so a name that a derive attribute or a macro spells as a string is used, and doc comments (`///`, `//!`, `/** */`, `/*! */`) count, so a use in another crate's doctest is a use. That file may be in another package, or among the same package's files that aren't library files. Matching by name is shallow on purpose: two items sharing a name hide each other, which can only miss dead code, never report live code, so no `use` path or type has to be resolved.

A use from outside can also reach an item through a name its library declares for it, so a library file's *carriers* count too (`unreferenced-carriers`). Outside test code, and at any depth, function bodies and blocks included, a `macro_rules!` carrying `#[macro_export]` (bare, as `#[macro_export(local_inner_macros)]`, or among other attributes) carries every whole word of its body, the whole token tree after its name with matchers and every rule included, outside plain comments, under the macro's name, so `$crate::helper()` in an exported macro carries `helper`; and a `use` declaration whose visibility is exactly `pub` carries, for each `<path> as <alias>` in it (`pub use inner::orig as renamed;`, `pub use inner::{orig as renamed}`), the path's last segment under `<alias>`, unless that segment is `self`, `super`, `crate` or `*`, or the alias is `_`. A name is *used from outside* its library when it is mentioned outside the library's files, or when one of the library's carriers carries it under a name used from outside, followed to any depth (a macro calling another exported macro, a re-export of a re-export). An item is used when its name is used from outside its own library. A carrier never makes a name used by itself: an exported macro or renamed re-export nothing outside mentions carries nothing.

A public item's *signature* is a carrier too (`unreferenced-signature-carriers`): a type another crate receives from a function it calls, or reads as a public field, is used though the caller never spells it, and `pub(crate)` on the type would make the signature a `private_interfaces` warning. So every exactly-`pub` item of a listed kind carries the whole words of its signature's types under its own name, and an `impl` block the words of its header under its type's name, as `signature-carriers` lists them. A restricted-visibility field or item carries nothing, since only the library reaches what it names. The words are matched like any mention, so a parameter `file: PlaceFile` carries `file` too, which can only miss dead code. Carrying follows to any depth with the other carriers: `blockers_view` used outside makes `BlockersView` used, its public field `root: Root` makes `Root` used, and `Root`'s public field makes `Held` used.

**TypeScript and Python packages** (`unreferenced-package-api`). Neither language has a package-private visibility, so an `export`, or a Python name without a leading `_`, is also how a package's own modules share code; listing every one would report that sharing wholesale. A package's public items are therefore the names its *entry modules* expose, each listed where it is declared, `<package>::<name>` (`@acme/core::parse`, `acme_core::parse`) at the declaration's path and line, re-exports followed within the package. The rest of the rule is Rust's: a package's *files* are its sources other than its *test files*, which count as outside like a Rust package's `tests/`; an item is used when its name is a whole word, outside plain comments, in a file of its language outside the package, or carried under a used name.

A TypeScript package is a tracked `package.json` whose `name` is a string, rooted at its directory, a file belonging to the deepest package holding it and a file under any `node_modules` to none; its sources are `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs` and `.cjs` files, and its test files `*.test.<ext>`, `*.spec.<ext>` and files under `__tests__`, `test` or `tests`. Its entry modules are what `package.json` points at (`exports` at any depth, `main`, `module`, `browser`, `types`, `typings`), as `typescript-packages` resolves them: a built path (`dist/index.js`) isn't tracked, so it is tried with its extension and first directory swapped for the source's (`src/index.ts`); with no field resolving, `src/index.<ext>` then `index.<ext>`. A package with no entry module, as this repository's `web` and `plugin-panel` are, lists nothing, and its files count as outside every other package. Re-exports are followed within the package, a `.js` specifier also tried as `.ts` since TypeScript rewrites them. A default export isn't listed, since its importers name it as they like; `export =`, `module.exports` and `exports.x` aren't read. Mentions are also read from the whole text of `.vue`, `.svelte`, `.astro`, `.html` and `.mdx` files, comments included, since those embed scripts the detector doesn't parse and a live name used only there would otherwise be reported. Every declaration carries its signature as `typescript-uses` lists, listed or not, since the type a used function returns may itself be unexposed and still name a listed one.

A Python package is a directory with a tracked `__init__.py` whose parent has none, other than `test` or `tests`, named by the directory (`src/acme_core/` → `acme_core`); a directory with `__init__.py` below it is a subpackage within it. Its sources are its `.py` and `.pyi` files at any depth, and its test files `test_*.py`, `*_test.py`, `conftest.py` and files under `tests` or `test`. Its entry module is its `__init__.py`, and its API is `__all__` when that module assigns one (list or tuple literals, `=` and `+=`), else every top-level name without a leading `_` that a `def`, `class`, assignment or import binds, `from .m import *` bringing in `m`'s API by the same rules. An imported name is listed where the module the import names binds it, followed to any depth; a name bound to a module (`from . import sub`) isn't listed, like a Rust `pub mod`. Mentions are also read from the whole text of `.ipynb` files, which hold source as JSON strings, and from the string values of `pyproject.toml`'s `[project.scripts]`, `[project.gui-scripts]`, `[project.entry-points.<group>]`, `[tool.poetry.scripts]` and `[tool.poetry.plugins.<group>]` tables, since an entry point is called by name (`acme = "acme_core.cli:main"` uses `main`). Every top-level definition carries as `python-uses` lists, listed or not, string annotations included.

**Allow-list and entries.** Each `--allow` entry is `<crate>` or `<package>`, which skips that library or package (`clm_check`, `@acme/core`, `acme_core`, compared as written; one name that is both a crate's and a Python package's skips both, and is stale only when it skips nothing in either), or an item's full name, `<crate>::<name>`, `<crate>::<Type>::<name>` or `<package>::<name>`, which skips every item with that full name. An entry that skips no finding, because it names no library or package, no public item, or only items that are used, is itself a finding, so the list can't outlive the code it excuses (`no-dead-code`). Each `--entry` value is a Rust attribute name, compared with the attribute's last path segment as written, so a value spelled as a path (`worker::event`) matches nothing. One that no exactly-`pub` item of a listed kind in a library file, or `impl` block holding one, carries outside test code, used or not, is a finding `entry <attribute> marks no item`. Entry points are skipped before the allow-list applies: an entry's staleness ignores `--allow`, and an `--allow` entry that skips only entry points skips no finding.

**Output.** On stdout, the item findings, one per line, sorted by path, then line as a number, then text: `<path>:<line>: <full name> is public but nothing outside its library uses it` for a Rust item and `<path>:<line>: <full name> is exported but nothing outside its package uses it` for a TypeScript or Python item, with its path relative to the root and its 1-based line; then one line per stale `--allow` entry, `clm.toml: allow <entry> skips no finding`, in the order the entries were given; then one line per stale `--entry` value, `clm.toml: entry <attribute> marks no item`, in the order they were given. `clm check` copies that output to its stderr and reports `<name> failed (exit 1)` (`check/detectors`). Errors go to stderr as one line naming the file or command: a `Cargo.toml` that isn't valid TOML or whose `[package]` has no `name`, a `package.json` that isn't valid JSON, a `pyproject.toml` that isn't valid TOML, a tracked file present in the working tree that can't be read, or git failing. A `package.json` with no string `name` is no package and no error, as a workspace root's often is.

## Requirements

### Requirement: public-items
`scan` SHALL list, in each library's library files, every item whose visibility is exactly `pub` declared directly in a module (the file's or an inline `mod` at any depth) that is a `fn`, `struct`, `enum`, `union`, `trait`, `type`, `const` or `static`, and every exactly-`pub` `fn` or `const` of an inherent `impl` block declared directly in a module, named `<crate>::<name>`, or `<crate>::<Type>::<name>` for an `impl` item, `<Type>` being the last path segment of the `impl`'s type without generic arguments, references or `dyn`. A library SHALL be a package (a tracked `Cargo.toml` with a `[package]` table) with a tracked `src/lib.rs`; its crate name SHALL be `[lib] name` when set, else its package name with `-` replaced by `_`; its library files SHALL be `src/lib.rs` and, transitively, the tracked file of each out-of-line module a library file declares without `#[path]` outside a function body or block (`m.rs` and `m/mod.rs` in the declaring module's directory: the declaring file's directory for `lib.rs` and `mod.rs`, `x/` for any other declaring file `x.rs`, with the names of enclosing inline modules appended), and no other file. It SHALL NOT list an item with a restricted visibility (`pub(crate)`, `pub(super)`, `pub(self)`, `pub(in …)`), an item inside a function body or other block, a `pub use`, a `pub mod`, a macro, a field, a variant, a trait member or an item in a trait `impl`. It SHALL NOT list test code: an item carrying, or inside an `impl` or inline module carrying, a `#[cfg(…)]` whose predicate holds only when `test` is set (`test`, an `all(…)` with such an argument, or a non-empty `any(…)` whose arguments all are such, never a `not(…)`), nor an item of a module whose file or inline body opens with such a `#![cfg(…)]`, nor an item in the library file of an out-of-line module declared with such an attribute or of any module below it. It SHALL list items under `cfg(not(test))` or `cfg(any())` and in a file opening with `#![cfg(not(test))]`.

#### Scenario: kinds listed
- GIVEN a tracked library `crates/a` (package `a-lib`, with `src/lib.rs`) declaring `pub fn f`, `pub struct S`, `pub(crate) fn g`, `pub use x::Y`, `impl S { pub fn new() }`, `impl Default for S { fn default() }` and `pub fn outer() { pub fn inner() {} }`, and nothing else in the repository naming them
- WHEN `scan` runs
- THEN it reports `a_lib::f`, `a_lib::S`, `a_lib::S::new` and `a_lib::outer`, and nothing for `g`, `Y`, `default` or `inner`

#### Scenario: impl type names
- GIVEN library `a` declaring `pub struct W<T>(T)`, `impl<T> crate::W<T> { pub fn wrap() }` and `impl dyn Tr { pub fn dynamic() }`, with `pub trait Tr {}`, and nothing else naming them
- WHEN `scan` runs
- THEN it reports `a::W::wrap` and `a::Tr::dynamic`

#### Scenario: test code skipped
- GIVEN library `a` whose `src/lib.rs` holds `#[cfg(test)] mod tests { pub fn helper() }`, `#[cfg(all(test, unix))] pub fn unix_helper()` and `#[cfg(test)] mod fixtures;`, and a tracked `src/fixtures.rs` declaring `pub fn fixture()` and `mod deep;`, with a tracked `src/fixtures/deep.rs` declaring `pub fn deeper()`
- WHEN `scan` runs
- THEN it reports none of `helper`, `unix_helper`, `fixture` and `deeper`

#### Scenario: not test is checked
- GIVEN library `a` whose `src/lib.rs` declares `#[cfg(not(test))] pub fn prod_only()`, `#[cfg(any(test, feature = "x"))] pub fn shared()`, `#[cfg(any())] pub fn off()` and `mod real;`, and a tracked `src/real.rs` opening with `#![cfg(not(test))]` and declaring `pub fn real_only()`, none of them named elsewhere
- WHEN `scan` runs
- THEN it reports `a::prod_only`, `a::shared`, `a::off` and `a::real_only`

#### Scenario: binaries and lib name
- GIVEN a package with `[lib] name = "core"`, whose `src/main.rs` declares `pub fn in_main`
- WHEN `scan` runs
- THEN `in_main` isn't listed, and the package's library items are named `core::<name>`

#### Scenario: module tree of lib.rs
- GIVEN library `a` whose `src/lib.rs` declares `pub mod x;` and `mod outer { mod inner; }`, with tracked `src/x/mod.rs` declaring `pub fn in_x()` and `mod y;`, `src/x/y.rs` declaring `pub fn in_y()`, `src/outer/inner.rs` declaring `pub fn in_inner()`, a tracked `src/stray.rs` that no module declares, holding `pub fn in_stray()`, and none of them named elsewhere
- WHEN `scan` runs
- THEN it reports `a::in_x`, `a::in_y` and `a::in_inner`, and not `in_stray`

#### Scenario: binary modules
- GIVEN library `a` with `pub fn f` in `src/lib.rs`, whose `src/main.rs` declares `mod cli;` and `#[cfg(test)] mod tests;`, a tracked `src/cli.rs` declaring `pub fn run()` and calling `a::f()`, a tracked `src/tests.rs` declaring `pub fn t_helper()`, and nothing else naming `f`, `run` or `t_helper`
- WHEN `scan` runs
- THEN it reports none of `a::f`, `run` and `t_helper`

### Requirement: references
`scan` SHALL count an item as used, and not report it, when its name (the last segment for an `impl` item) is used from outside its library: when it appears as a whole word, bounded by characters other than ASCII letters, digits and `_`, in any tracked `.rs` file other than the item's own library files, including the same package's files that aren't library files (`tests/`, `benches/`, `examples/`, `build.rs`, `src/main.rs` and the modules it declares, `src/bin/`), or when a carrier in its own library's files, outside test code and at any depth, carries it under a name used from outside its library, to any depth. A `macro_rules!` carrying `#[macro_export]` SHALL carry every whole word of the token tree after its name, matchers included, under the macro's name, and a `use` declaration whose visibility is exactly `pub` SHALL carry the last path segment of each `<path> as <alias>` in it under `<alias>`, unless that segment is `self`, `super`, `crate` or `*` or the alias is `_`. Text inside string literals and doc comments (`///`, `//!`, `/** */`, `/*! */`) SHALL count; text inside other comments SHALL NOT. A use within the item's own library files SHALL NOT count, nor a carrier whose name nothing outside the library uses. Untracked files SHALL NOT count, as declarations or references, and a tracked file missing from the working tree SHALL be skipped.

#### Scenario: used by another crate
- GIVEN library `a` with `pub fn f` and `pub fn h`, and a tracked `crates/b/src/main.rs` calling `a::f()`
- WHEN `scan` runs
- THEN it reports `a::h` and not `a::f`

#### Scenario: used only in its own library
- GIVEN library `a` with `pub fn f` in `src/x.rs` called only from `src/lib.rs`
- WHEN `scan` runs
- THEN it reports `a::f`

#### Scenario: comments, strings, doctests and tests
- GIVEN library `a` with `pub fn f`, `pub fn g`, `pub fn d` and `pub fn k`, another crate whose only mention of `f` is in a `// f is unused` comment, a `#[serde(default = "g")]` attribute in another crate, a `/// a::d()` doctest in another crate, and `crates/a/tests/t.rs` calling `k()`
- WHEN `scan` runs
- THEN it reports `a::f` and not `a::g`, `a::d` or `a::k`

#### Scenario: whole words only
- GIVEN library `a` with `pub fn get`, and another crate whose only mention is `get_all()`
- WHEN `scan` runs
- THEN it reports `a::get`

#### Scenario: untracked user
- GIVEN library `a` with `pub fn f`, used only by an untracked `crates/b/src/main.rs`
- WHEN `scan` runs
- THEN it reports `a::f`

#### Scenario: through an exported macro
- GIVEN library `a` with `pub fn helper`, `pub fn inner_helper`, `pub fn idle_helper` and `pub fn plain_helper`, a `#[macro_export] macro_rules! outer { () => { $crate::helper(); $crate::inner!() } }`, a `#[macro_export] macro_rules! inner { () => { $crate::inner_helper() } }`, a `#[macro_export] macro_rules! idle { () => { $crate::idle_helper() } }` and a `macro_rules! local { () => { $crate::plain_helper() } }` without `#[macro_export]`, and another crate whose only mention of any of them is `a::outer!()`
- WHEN `scan` runs
- THEN it reports `a::idle_helper` and `a::plain_helper`, and not `a::helper` or `a::inner_helper`

#### Scenario: through a renamed re-export
- GIVEN library `a` whose `src/lib.rs` holds `mod inner;` and `pub use inner::{orig as renamed, other as alias};`, a tracked `src/inner.rs` declaring `pub fn orig()` and `pub fn other()`, and another crate calling `a::renamed()` and naming nothing else
- WHEN `scan` runs
- THEN it reports `a::other` and not `a::orig`

### Requirement: allow-list
`scan` SHALL skip every item of a library or package named by an `--allow <crate>` or `--allow <package>` entry and every item whose full name an `--allow <full name>` entry gives, whatever its language. An entry that skips no finding, because it names no library or package, no public item, or only items that are used, SHALL be reported as a finding `clm.toml: allow <entry> skips no finding`.

#### Scenario: allowed item and crate
- GIVEN libraries `a` (with unused `pub fn f` and `pub fn g`) and `b` (with unused `pub fn h`)
- WHEN `clm unreferenced --allow a::f --allow b` runs
- THEN it reports `a::g` only

#### Scenario: stale entries
- GIVEN library `a` with no item `gone` and a `pub fn used` that another crate calls
- WHEN `clm unreferenced --allow a::gone --allow zz --allow a::used` runs
- THEN it prints `clm.toml: allow a::gone skips no finding`, `clm.toml: allow zz skips no finding` and `clm.toml: allow a::used skips no finding`, in that order, and exits 1

#### Scenario: allowed package and item
- GIVEN TypeScript package `@acme/core` whose entry module exports unused `f` and `g`, and Python package `acme_core` whose `__init__.py` exposes unused `h` and `k`
- WHEN `clm unreferenced --allow @acme/core::f --allow acme_core` runs
- THEN it reports `@acme/core::g` only

### Requirement: report
`clm unreferenced` SHALL print on stdout one line per item finding, sorted by path, then line as a number, then text, each `<path>:<line>: <full name> is public but nothing outside its library uses it` for a Rust item and `<path>:<line>: <full name> is exported but nothing outside its package uses it` for a TypeScript or Python item, with the path relative to the root and a 1-based line, then the stale `--allow` lines in the order the entries were given, then the stale `--entry` lines in the order they were given. It SHALL exit 0 when there is no finding, including in a repository with no library or package, 1 when there is any, and 2, printing one line on stderr naming the file or command, when a tracked `Cargo.toml` isn't valid TOML or its `[package]` has no `name`, a tracked `package.json` isn't valid JSON, a tracked `pyproject.toml` isn't valid TOML, a tracked file present in the working tree can't be read, or git fails. A source file that isn't valid UTF-8, or that tree-sitter parses with errors, SHALL NOT be an error: it is read with invalid bytes replaced and used as far as it parses. It SHALL take `--root <dir>`, defaulting to the CLM repository found from the current directory, else the top of the git repository, as `clm survey`'s does, and `--allow <entry>` and `--entry <attribute>` any number of times each; it SHALL read only `Cargo.toml`, `package.json` and `pyproject.toml` files, source files with the extensions `.rs`, `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs`, `.cjs`, `.py` and `.pyi`, and embedding files with the extensions `.vue`, `.svelte`, `.astro`, `.html`, `.mdx` and `.ipynb`, and SHALL run no `cargo`, `npm`, Node.js or `python`.

#### Scenario: findings
- GIVEN library `a` whose `src/lib.rs` declares an unused `pub fn f` on line 3, `mod x;` and an unused `pub fn z` on line 10, and whose `src/x.rs` declares an unused `pub fn h` on line 12
- WHEN `clm unreferenced` runs
- THEN stdout is `crates/a/src/lib.rs:3: a::f is public but nothing outside its library uses it`, then the line for `a::z` at line 10, then the line for `a::h`, and the exit code is 1

#### Scenario: clean or no Rust
- GIVEN a repository whose public items are all used, or one with no `Cargo.toml`, `package.json` or `__init__.py`
- WHEN `clm unreferenced` runs
- THEN it prints nothing and exits 0

#### Scenario: broken manifest
- GIVEN a tracked `Cargo.toml` that isn't valid TOML, or a tracked `package.json` that isn't valid JSON, or a tracked `pyproject.toml` that isn't valid TOML
- WHEN `clm unreferenced` runs
- THEN it exits 2 with a stderr line naming that file

#### Scenario: from a subdirectory of a project without CLM
- GIVEN a git repository with no `specs/SPEC.md`, holding library `a` with an unused `pub fn f`
- WHEN `clm unreferenced` runs from `crates/a/src`
- THEN it prints the line for `a::f` with the path `crates/a/src/lib.rs`

#### Scenario: named as a detector
- GIVEN a CLM fixture repository whose own checks pass, with a `clm.toml` detector `unreferenced-public-items` whose command is the `clm` binary under test with `unreferenced`, and a library with an unused `pub fn f`
- WHEN `clm check` runs
- THEN stdout contains `clm.toml: check/detectors: unreferenced-public-items failed (exit 1)` and stderr contains the line naming `f`

### Requirement: git-through-layer
`clm-unreferenced` SHALL list `clm-git` among its dependencies and SHALL run no `git` or `gh` process itself: it lists tracked files through `clm-git`'s calls (`git-plumbing-layer`).

#### Scenario: covered by the scan
- GIVEN the workspace
- WHEN a test in `clm-unreferenced` reads `crates/clm-unreferenced/Cargo.toml` and every file under `crates/clm-unreferenced/src`
- THEN `clm-git` is a dependency and no file holds `Command::new("git")` or `Command::new("gh")`

#### Serves
- architecture/reuse-first

### Requirement: entry-attributes
`scan` SHALL NOT list an entry point: an item, or an item of an `impl` block, carrying an attribute whose name, its last path segment, is `no_mangle`, `export_name`, `wasm_bindgen`, `proc_macro`, `proc_macro_derive`, `proc_macro_attribute` or a name given with `--entry`, bare or inside `unsafe(…)`, with or without arguments. An `--entry` name that no exactly-`pub` item of a listed kind in a library file carries, directly or on its `impl` block, outside test code, whether that item is used or not, SHALL be reported as a finding `clm.toml: entry <attribute> marks no item`; the name SHALL be compared with the attribute's last path segment as given, so a value holding `::` marks no item. Entry points SHALL be skipped before the allow-list applies: an `--entry`'s staleness ignores `--allow`, and an `--allow` entry that skips only entry points skips no finding.

#### Scenario: entry points skipped
- GIVEN library `a` declaring `#[no_mangle] pub extern "C" fn c_entry()`, `#[unsafe(no_mangle)] pub extern "C" fn c2()`, `#[unsafe(export_name = "e")] pub extern "C" fn c3()`, `#[wasm_bindgen(js_name = jsE)] pub fn js_entry()`, `#[wasm_bindgen] impl S { pub fn js_method() }`, `#[proc_macro] pub fn m1(_: TokenStream) -> TokenStream`, `#[proc_macro_derive(D)] pub fn m2(_: TokenStream) -> TokenStream` and `#[proc_macro_attribute] pub fn m3(_: TokenStream, _: TokenStream) -> TokenStream`
- WHEN `scan` runs
- THEN it reports none of `c_entry`, `c2`, `c3`, `js_entry`, `js_method`, `m1`, `m2` and `m3`

#### Scenario: entry given as an argument
- GIVEN library `a` declaring `#[event(fetch)] pub async fn fetch()` and `#[worker::event(scheduled)] pub async fn tick()`, neither named elsewhere
- WHEN `clm unreferenced` runs, and then `clm unreferenced --entry event`
- THEN the first reports `a::fetch` and `a::tick`, and the second neither

#### Scenario: stale entry
- GIVEN library `a` with no item carrying an attribute named `napi`, and an `--allow` entry `zz` naming no library
- WHEN `clm unreferenced --entry napi --allow zz` runs
- THEN it prints `clm.toml: allow zz skips no finding` and then `clm.toml: entry napi marks no item`, and exits 1

#### Scenario: entry on a used item
- GIVEN library `a` declaring `#[napi] pub fn used_entry()`, which another crate calls, and `#[napi] pub fn idle_entry()`, with an `--allow` entry `a::idle_entry`
- WHEN `clm unreferenced --entry napi --allow a::idle_entry --entry worker::napi` runs
- THEN it prints `clm.toml: allow a::idle_entry skips no finding` and then `clm.toml: entry worker::napi marks no item`, and nothing for `--entry napi`

### Requirement: signature-carriers
`scan` SHALL count a name as used from outside a library when a carrier of the library carries it under a name used from outside, where, outside test code, every exactly-`pub` item of a listed kind declared directly in a module carries under its own name (the last segment for an `impl` item) every whole word, outside comments, of the types in its signature: a `fn`'s generic parameters and bounds, parameters (names included), return type and `where` clause; a `const`'s or `static`'s type; a `type` alias's right-hand side; a `struct`'s or `union`'s generic parameters, `where` clause and exactly-`pub` fields' types; an `enum`'s generic parameters, `where` clause and every variant field's type; a `trait`'s generic parameters, supertraits, `where` clause and members' signatures; and an inherent `impl`'s exactly-`pub` `fn` or `const` as a `fn` or `const`. An `impl` block declared directly in a module, outside test code, SHALL carry every word between `impl` and its body under the last path segment of its type, and a trait `impl` its members' signatures too; an entry point (`entry-attributes`) SHALL carry as an item used from outside does. A field or item with restricted visibility, and a trait `impl`'s members under their own names, SHALL carry nothing; a carrier whose name nothing outside the library uses SHALL make nothing used.

#### Scenario: types reached through used items
- GIVEN library `a` declaring `pub struct Report { pub count: Count, hidden: Secret }`, `pub struct Count`, `pub struct Secret`, `pub fn report() -> Report`, `pub struct Params`, `pub fn take(p: Params)`, `pub enum Kind { One(Inner) }`, `pub struct Inner`, `impl Report { pub fn tail(&self) -> Tail }`, `pub struct Tail`, `pub struct Idle` and `pub fn idle_fn() -> Idle`, and another crate that calls `a::report()`, `a::take(x)`, `.tail()` and matches `a::Kind::One(_)`, naming nothing else
- WHEN `scan` runs
- THEN it reports `a::Secret`, `a::Idle` and `a::idle_fn`, and none of `Report`, `Count`, `Params`, `Kind`, `Inner`, `take`, `tail` and `Tail`

#### Scenario: chains, traits, aliases and impl headers
- GIVEN library `a` declaring `pub fn view() -> View`, `pub struct View { pub root: Root }`, `pub struct Root { pub held: Held }`, `pub struct Held`, `pub trait Tr { fn out(&self) -> Out; }`, `pub struct Out`, `pub type Alias = Base<Arg>`, `pub struct Base<T>(T)`, `pub struct Arg`, `pub struct Shown`, `impl From<Shown> for View { … }`, `#[no_mangle] pub extern "C" fn entry() -> Exported`, `pub struct Exported`, `pub(crate) fn hidden() -> Lonely` and `pub struct Lonely`, and another crate that calls `a::view()`, implements `a::Tr` and names `a::Alias` and nothing else
- WHEN `scan` runs
- THEN it reports `a::Lonely` and none of `View`, `Root`, `Held`, `Tr`, `Out`, `Alias`, `Base`, `Arg`, `Shown` and `Exported`

### Requirement: typescript-packages
`scan` SHALL treat each tracked `package.json` whose `name` is a string as a package rooted at its directory, named by that string, a tracked file belonging to the package with the deepest root that holds it and a file with `node_modules` in its path to none; a package's files SHALL be its tracked `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs` and `.cjs` files other than test files (name ending in `.test.<ext>` or `.spec.<ext>`, or a directory `__tests__`, `test` or `tests` in the path below the root). Its entry modules SHALL be the package files resolved from the string values of `package.json`'s `exports` (at any depth), `main`, `module`, `browser` (a string only), `types` and `typings`, arrays included, a value holding `*` naming nothing, each with a leading `./` dropped and tried as given, then with an extension `.js`, `.mjs`, `.cjs`, `.jsx`, `.d.ts`, `.d.mts` or `.d.cts` replaced by `.ts`, `.tsx`, `.mts` and `.cts`, then each with `src` in place of its first directory or prepended when it has none, a path without extension also with the source extensions appended and as a directory holding `index`; when none resolves, the first tracked of `src/index.<ext>` then `index.<ext>` for the source extensions in the order above. It SHALL list, as `<package>::<name>` at the declaration's path and line, each name an entry module exports: an exported declaration (`function`, `class`, `interface`, `type`, `enum`, `namespace`, and each name a `const`, `let` or `var` pattern binds), a local declaration an `export { x as y }` clause exports, under `y`, or an import binding it exports, followed to its declaration as a re-export is, and a name re-exported by `export { x as y } from` or `export * from` a relative specifier resolving to a package file (as given, with a source extension appended, as a directory holding `index`, or with `.js`, `.mjs`, `.cjs` or `.jsx` replaced by `.ts`, `.mts`, `.cts` or `.tsx`), followed to any depth; declarations sharing a name in one file (overloads, an ambient declaration) SHALL be one item at the first. It SHALL NOT list a default export, a default re-exported under a name, a namespace re-export (`export * as ns from`), a name from a bare specifier, from a test file, from another package's file or from no tracked file, a name whose declaration isn't found, an `export =`, `module.exports` or `exports.x`, or anything of a package with no entry module.

#### Scenario: entry module and re-exports
- GIVEN package `packages/core` whose `package.json` is `{"name": "@acme/core", "main": "dist/index.js"}`, whose `src/index.ts` holds `export function parse() {}` on line 1, `export { helper as util } from './helper.js'`, `export * from './types'`, `export default function main() {}`, `function local() {}` on line 5 and `export { local }`, whose `src/helper.ts` holds `export function helper() {}` on line 2 and `export function internal() {}`, and whose `src/types.ts` holds `export interface Report {}` on line 1, and nothing outside the package naming any of them
- WHEN `clm unreferenced` runs
- THEN it prints `packages/core/src/helper.ts:2: @acme/core::util is exported but nothing outside its package uses it`, then the lines for `@acme/core::parse` at `packages/core/src/index.ts:1`, `@acme/core::local` at `packages/core/src/index.ts:5` and `@acme/core::Report` at `packages/core/src/types.ts:1`, and nothing for `internal`, `main` or `default`

#### Scenario: exports map, default entry and no entry
- GIVEN package `@acme/lib` whose `package.json` has `"exports": {".": {"import": "./dist/index.mjs", "types": "./dist/index.d.ts"}, "./utils": "./dist/utils.js", "./*": "./dist/*.js"}` and tracked `src/index.ts` exporting `a`, `src/utils.ts` exporting `b` and `src/other.ts` exporting `c`; package `@acme/plain` with neither `main` nor `exports`, whose `src/index.ts` exports `d`; and package `web` whose `main` is `dist/bundle.js`, with a tracked `src/app.ts` exporting `e` and no `bundle` or `index` file; none of the names used outside its package
- WHEN `scan` runs
- THEN it reports `@acme/lib::a`, `@acme/lib::b` and `@acme/plain::d`, and nothing for `c` or `e`

#### Scenario: test files, node_modules and nameless manifests
- GIVEN package `@acme/core` whose `src/index.ts` holds `export * from './x.test'`, `export * from '../node_modules/dep'` and `export * from '@acme/other'`, whose `src/x.test.ts` exports `t`, with a tracked `node_modules/dep/index.ts` exporting `n`, a root `package.json` `{"private": true, "workspaces": ["packages/*"]}` and a tracked root `index.ts` exporting `r`
- WHEN `scan` runs
- THEN it reports none of `t`, `n` and `r`

### Requirement: typescript-uses
`scan` SHALL count a TypeScript package's item as used, and not report it, when its exported name appears as a whole word, bounded by characters other than ASCII letters, digits and `_`, in a tracked `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs` or `.cjs` file outside the package's files (another package's files, the package's own test files, or a file in no package) outside plain comments (`//`, `/* */`, `<!-- -->`), strings, JSX text and `/** */` comments counting; or anywhere in the text of a tracked `.vue`, `.svelte`, `.astro`, `.html` or `.mdx` file outside the package's files; or when a declaration in a package file, listed or not, carries it under its name and that name is used from outside, to any depth: a function's type parameters, parameters' types and return type; a class's type parameters, `extends` and `implements` clauses and the annotations and signatures of its members not marked `private` or named with `#`, constructor parameters included; an interface's type parameters, `extends` clause and body; a type alias's type parameters and right-hand side; a variable's type annotation and the signature of an arrow or function expression it is initialised with; nothing for an enum or namespace. A mention within the package's own files SHALL NOT count, nor one inside a plain comment of a parsed file, nor one in an untracked file.

#### Scenario: used from another package, a test file and an embedding file
- GIVEN package `@acme/core` whose entry module exports `parse`, `build`, `Report` and `stale`, `apps/web/src/main.ts` holding `import { parse } from '@acme/core'`, `packages/core/src/parse.test.ts` calling `build()`, `apps/web/src/App.vue` whose `<script setup lang="ts">` names `Report`, and `stale` named only in a `// stale()` comment of `apps/web/src/main.ts` and in `packages/core/src/other.ts`
- WHEN `scan` runs
- THEN it reports `@acme/core::stale` only

#### Scenario: types reached through used items
- GIVEN package `@acme/core` whose entry module exports `function load(): Loaded`, `type Item = Row | Column`, `class Row {}`, `class Column {}`, `class Store { private memo: Memo; constructor(opts: Options) {} }`, `interface Memo {}`, `interface Options {}`, `const fetchAll = (q: Query): Row[] => []`, `interface Query {}`, `class Lonely {}` and `function lonely(): Lonely`, and holds an unexported `interface Loaded { items: Item[] }`, and another package that calls `load()`, `new Store({})` and `fetchAll(q)` and names nothing else
- WHEN `scan` runs
- THEN it reports `@acme/core::Memo`, `@acme/core::Lonely` and `@acme/core::lonely`, and none of `Loaded`, `Item`, `Row`, `Column`, `Store`, `Options`, `fetchAll` and `Query`

### Requirement: python-packages
`scan` SHALL treat each directory holding a tracked `__init__.py` whose parent directory holds none, other than one named `test` or `tests`, as a package named by the directory, whose files are its tracked `.py` and `.pyi` files at any depth other than test files (`test_*.py`, `*_test.py`, `conftest.py`, or a directory `tests` or `test` in the path below the root), and whose entry module is that `__init__.py`, never a `.pyi`. It SHALL list, as `<package>::<name>` at the declaration's path and line, each name of the entry module's API: the string literals of its top-level `__all__ = […]`, `__all__: … = […]` and `__all__ += […]` lists or tuples when it assigns `__all__`, else every name without a leading `_` bound at its top level (compound statements at that level, `if`, `try`, `with`, `for`, `while` and `match` with their clauses, nested to any depth, included; function and class bodies not) by a `def`, `class`, assignment or annotated assignment (every name of its targets, chains and tuples included), `import m as n`, `from .m import x`, `from .m import x as y` or `from .m import *` (the API of `m` by the same rules); an imported name SHALL be listed at the `def`, `class` or assignment that binds it in the module the import names, followed to any depth, where `.m` is `m.py` or `m/__init__.py` beside the importing file, `..m` one directory up, and an absolute import whose first segment is the package's name is resolved from the package's directory. It SHALL NOT list a name bound to a module, a name from a module outside the package or from no tracked file, a name whose binding isn't found, or anything of a directory with `__init__.py` below a package.

#### Scenario: names an init exposes
- GIVEN package `packages/core/src/acme_core` whose `__init__.py` holds `from .parse import parse`, `from .parse import render as show`, `from ._impl import helper`, `from . import sub`, `import os`, `def top(): ...` on line 6, `_private = 1` and `VERSION = "1"` on line 8, whose `parse.py` holds `def parse(): ...` on line 1, `def render(): ...` on line 4 and `def unused_in_parse(): ...`, whose `_impl.py` holds `def helper(): ...` on line 1, and whose `sub/__init__.py` holds `def g(): ...`, none of them named outside the package
- WHEN `clm unreferenced` runs
- THEN it prints `packages/core/src/acme_core/__init__.py:6: acme_core::top is exported but nothing outside its package uses it`, the line for `acme_core::VERSION` at `__init__.py:8`, the lines for `acme_core::parse` at `parse.py:1` and `acme_core::show` at `parse.py:4`, and the line for `acme_core::helper` at `_impl.py:1`, and nothing for `_private`, `sub`, `os`, `unused_in_parse` or `g`

#### Scenario: __all__ and star imports
- GIVEN package `acme_core` whose `__init__.py` holds `from .a import *`, `from .b import x, y`, `__all__ = ["x", "z"]` and `__all__ += ["w"]`, whose `a.py` holds `__all__ = ["z", "zz"]`, `def z(): ...`, `def zz(): ...` and `def hidden(): ...`, and whose `b.py` holds `def x(): ...` and `def y(): ...`, none of them named outside the package
- WHEN `scan` runs
- THEN it reports `acme_core::x` at `b.py`'s line of `x` and `acme_core::z` at `a.py`'s line of `z`, and nothing for `y`, `zz`, `hidden` or `w`

#### Scenario: packages, scripts and test files
- GIVEN `src/acme_core/__init__.py` holding `def f(): ...` and `def k(): ...`, `src/acme_core/tests/test_f.py` calling `f()`, `src/acme_core/conftest.py` holding `def fixture(): ...`, and `scripts/run.py` holding `def h(): ...` with no `__init__.py` beside it
- WHEN `scan` runs
- THEN it reports `acme_core::k` only

### Requirement: python-uses
`scan` SHALL count a Python package's item as used, and not report it, when its name appears as a whole word, bounded by characters other than ASCII letters, digits and `_`, in a tracked `.py` or `.pyi` file outside the package's files (another package's files, the package's own test files, or a file in no package) outside `#` comments, strings and docstrings counting; or anywhere in the text of a tracked `.ipynb` file outside the package's files; or as a whole word in a string value of a tracked `pyproject.toml`'s `[project.scripts]`, `[project.gui-scripts]`, `[project.entry-points.<group>]`, `[tool.poetry.scripts]` or `[tool.poetry.plugins.<group>]` table; or when a top-level `def`, `class` or annotated assignment of a package file, listed or not, carries it under its name and that name is used from outside, to any depth: a function's parameter and return annotations, a class's header arguments and every annotation in its body, a variable's annotation, string annotations' words included. A mention within the package's own files SHALL NOT count, nor one inside a `#` comment, nor one in an untracked file.

#### Scenario: used from another package, a test, an entry point and a notebook
- GIVEN package `packages/core/src/acme_core` whose `__init__.py` exposes `parse`, `render`, `main`, `Cfg` and `stale`, `packages/api/src/acme_api/app.py` holding `from acme_core import parse`, `packages/core/tests/test_render.py` calling `render()`, `packages/core/pyproject.toml` holding `[project.scripts]` with `acme = "acme_core.cli:main"`, a tracked `notebooks/demo.ipynb` whose source names `Cfg`, and `stale` named only in a `# stale()` comment of `app.py` and in `packages/core/src/acme_core/other.py`
- WHEN `scan` runs
- THEN it reports `acme_core::stale` only

#### Scenario: types reached through used items
- GIVEN package `acme_core` whose `__init__.py` holds `from ._impl import load, Item, Base, save, Options, Lonely, lonely`, whose `_impl.py` holds `def load() -> Loaded`, `class Loaded:` with `items: list[Item]`, `class Item(Base):`, `class Base:`, `def save(cfg: "Options") -> None`, `class Options:`, `class Lonely:` and `def lonely() -> Lonely`, and another package that calls `load()` and `save(...)` and names nothing else
- WHEN `scan` runs
- THEN it reports `acme_core::Lonely` and `acme_core::lonely`, and none of `Item`, `Base`, `save` and `Options`
