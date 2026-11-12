---
component: check
---

## MODIFIED Requirements

### Requirement: unowned-files <!-- base: d7dc8ebd0e7d9e3d -->
When the root sets `outside` (`model/outside-globs`), even to `[]`, `run` SHALL check path ownership; when the root has no `outside` key it SHALL NOT, and reports no `check/unowned-files` violation. It SHALL read the files the index holds under the CLM repository (`git ls-files --cached`, through the `git` layer), every entry the index lists, symlinks and submodules included; in a directory that isn't a git work tree it SHALL read no file and report no `check/unowned-files` violation; `run_listed` (`run-listed`) SHALL read the files of the listing it is given instead, wherever the directory is. It SHALL leave out the spec paths: every file under `specs/`, `changes/`, `backlog/` or `docs/`, and `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and `AGENTS.md`, the same spec paths `gate` names. Of the rest, the *code files*, it SHALL report each that no component's `paths` match, whatever the component's status, and no glob of the root's `outside` matches, as `<file>: check/unowned-files: in no component's paths; add it to a component's paths, or to outside in specs/SPEC.md until a change adopts it`. It SHALL report each `outside` glob that matches no code file that no component's `paths` match, as `specs/SPEC.md: check/unowned-files: outside glob <glob> matches no file outside every component`. A glob matches a file as the git pathspec `:(glob)<glob>` does; matching in memory by the same rules is enough. A code file that a component's `paths` and an `outside` glob both match is the component's, and SHALL NOT be reported. A file the index doesn't hold, untracked or ignored, SHALL NOT be read or reported.

#### Scenario: file in no node
- GIVEN component `a` with `paths: ["crates/a/**"]`, a root with `outside: []`, and tracked files `crates/a/lib.rs` and `crates/b/lib.rs`
- WHEN `clm check` runs
- THEN it exits 1 with the line `crates/b/lib.rs: check/unowned-files: in no component's paths; add it to a component's paths, or to outside in specs/SPEC.md until a change adopts it`, and none for `crates/a/lib.rs`

#### Scenario: not opted in
- GIVEN the same files, and a root with no `outside` key
- WHEN `clm check` runs
- THEN it reports no `check/unowned-files` violation

#### Scenario: declared not yet adopted
- GIVEN the same files, and the root with `outside: ["crates/b/**"]`
- WHEN `clm check` runs
- THEN it reports no `check/unowned-files` violation

#### Scenario: part of an outside directory adopted
- GIVEN component `api` with `paths: ["src/api/**"]`, the root with `outside: ["src/**"]`, and tracked files `src/api/x.rs` and `src/web/y.rs`
- WHEN `clm check` runs
- THEN it reports no `check/unowned-files` violation

#### Scenario: outside glob left behind
- GIVEN components `a` and `b` with `paths` `["crates/a/**"]` and `["crates/b/**"]`, the root with `outside: ["crates/b/**"]`, and tracked files `crates/a/lib.rs` and `crates/b/lib.rs`
- WHEN `clm check` runs
- THEN it exits 1 with the line `specs/SPEC.md: check/unowned-files: outside glob crates/b/** matches no file outside every component`

#### Scenario: spec paths and untracked files
- GIVEN component `a` with `paths: ["crates/a/**"]`, a root with `outside: []`, tracked files `docs/guide/x.md`, `backlog/y.md`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `AGENTS.md` and `crates/a/lib.rs`, and an untracked file `notes/todo.txt`
- WHEN `clm check` runs
- THEN it reports no `check/unowned-files` violation

#### Scenario: extracted tree
- GIVEN a directory that isn't a git work tree, holding `specs/SPEC.md` with `outside: [".ci/**"]` and component `a` with `paths: ["crates/a/**"]`, and the files `crates/b/lib.rs` and `.ci/build.yml`
- WHEN `run_listed` is called on it with the listing `specs/SPEC.md`, `crates/b/lib.rs` and `.ci/build.yml`
- THEN it reports `crates/b/lib.rs` as `check/unowned-files`, and no `outside` glob

#### Scenario: no work tree, no listing
- GIVEN the same directory
- WHEN `run` is called on it
- THEN it reports no `check/unowned-files` violation

#### Serves
- capability/adopt-clm
- architecture/incremental-adoption

#### Needs
- model/outside-globs
