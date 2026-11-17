---
component: check.unreferenced
---

## MODIFIED Requirements



### Requirement: report <!-- base: 27b2d762f23ed6f3 -->
`clm unreferenced` SHALL print on stdout one line per item finding, sorted by path, then line as a number, then text, each `<path>:<line>: <full name> is public but nothing outside its library uses it` for a Rust item and `<path>:<line>: <full name> is exported but nothing outside its package uses it` for a TypeScript or Python item, with the path relative to the root and a 1-based line, then the stale `--allow` lines in the order the entries were given, then the stale `--entry` lines in the order they were given. It SHALL exit 0 when there is no finding, including in a repository with no library or package, 1 when there is any, and 2, printing one line on stderr naming the file or command, when a tracked `Cargo.toml` isn't valid TOML or its `[package]` has no `name`, a tracked `package.json` isn't valid JSON, a tracked `pyproject.toml` isn't valid TOML, a tracked file present in the working tree can't be read, or git fails. A source file that isn't valid UTF-8, or that tree-sitter parses with errors, SHALL NOT be an error: it is read with invalid bytes replaced and used as far as it parses. It SHALL take `--root <dir>`, whose default `crates/clm` finds and the root states (`clm/root-from-cwd`): the CLM repository found from the current directory, else the top of the git repository, as `clm survey`'s is; and `--allow <entry>` and `--entry <attribute>` any number of times each; it SHALL read only `Cargo.toml`, `package.json` and `pyproject.toml` files, source files with the extensions `.rs`, `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs`, `.cjs`, `.py` and `.pyi`, and embedding files with the extensions `.vue`, `.svelte`, `.astro`, `.html`, `.mdx` and `.ipynb`, and SHALL run no `cargo`, `npm`, Node.js or `python`.

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
