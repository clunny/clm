---
component: init
base: {}
---

## MODIFIED Requirements

### Requirement: survey-components <!-- base: 9d709f1dcfe5e4a3 -->
`clm survey --components <file>` SHALL read the map's list of components from its top-level key `components` and an optional top-level `outside`, a list of globs like the root's `outside` (`model/outside-globs`). Its *code files* SHALL be those `check/unowned-files` reads: the files the index holds under the repository, binary files and `.github/` included, other than those under `specs/`, `changes/`, `backlog/` or `docs/` and other than `ARCHITECTURE.md`, `CAPABILITIES.md` and `CLAUDE.md`. Over them it SHALL report each code file no component covers and no `outside` glob matches (`survey/uncovered`), each code file two components cover (`survey/overlap`), and each `outside` glob that matches no code file outside every component (`survey/stale-outside`, with the message `outside glob <glob> matches no file outside every component`); a file a component covers and an `outside` glob matches is the component's. It SHALL report each import edge from one component's file to another component's file that its `depends_on` doesn't allow (`survey/undeclared-edge`), and each `depends_on` cycle (`survey/cycle`); an edge with an end in a file no component covers SHALL NOT be reported. A glob matches a file as the git pathspec `:(glob)<glob>` does. A map whose `outside` isn't a list of non-empty strings SHALL be refused as a map that doesn't parse is. The code files are listed apart from the files `clm survey`'s map reads for import edges, which still skip `.github/` and binary files (`survey-map`). A component of the map MAY set `tests` and `test_depends_on` as a component's frontmatter does (`model/test-ownership-keys`): a file a component's `tests` match is that component's whatever `paths` match it, a file two components' `tests` match is `survey/overlap`, an edge from a file of a component's `tests` is judged against its `depends_on` and `test_depends_on` together, and `test_depends_on` forms no `survey/cycle` (`components-own-their-tests`). It SHALL exit 1 when it reports any and 0 with no output otherwise. A map that sets `nodes`, the key `20261102-rename-node-to-component` kept beside `components`, SHALL fail with exit 1, naming the file, `nodes` and `components`, and the old flag SHALL fail as clap fails for an unknown argument (`clm/node-names-retired`). No `clm:remove-by 20261102-rename-node-to-component-contract` marker SHALL be left.

#### Scenario: tests carved out of a suite
- GIVEN a component map with `app` (`app/**`, `depends_on: [feat]`), `feat` (`feat/**`, tests `["tests/feat/**"]`, `test_depends_on: [app]`) and `harness` (`tests/**`), where `tests/feat/test_x.py` imports a file under `app`
- WHEN `clm survey --components` runs
- THEN it reports no `survey/overlap`, `survey/undeclared-edge` or `survey/cycle` line

#### Scenario: uncovered file
- GIVEN a component map whose components cover `crates/a/**` only, in the two-crate workspace
- WHEN `clm survey --components` runs
- THEN it exits 1 with a `survey/uncovered` line naming a file under `crates/b`

#### Scenario: undeclared edge
- GIVEN components `a` (`crates/a/**`, `depends_on: []`) and `b` (`crates/b/**`) in the two-crate workspace
- WHEN `clm survey --components` runs
- THEN it exits 1 with a `survey/undeclared-edge` line naming `a` and `b`

#### Scenario: good map
- GIVEN components `a` (`crates/a/**`, `depends_on: [b]`) and `b` (`crates/b/**`), covering every code file
- WHEN `clm survey --components` runs
- THEN it exits 0 with no output

#### Scenario: part left for later
- GIVEN a component map with component `a` (`crates/a/**`, `depends_on: []`), a component covering the workspace's top-level files, and `outside: ["crates/b/**"]`, in the two-crate workspace, where a file under `crates/a` imports one under `crates/b`
- WHEN `clm survey --components` runs
- THEN it exits 0 with no output

#### Scenario: outside glob that exempts nothing
- GIVEN components `a` (`crates/a/**`, `depends_on: [b]`), `b` (`crates/b/**`) and one covering the workspace's top-level files, together covering every code file, and `outside: ["crates/c/**"]`
- WHEN `clm survey --components` runs
- THEN it exits 1 with a `survey/stale-outside` line naming `crates/c/**`

#### Scenario: the code files check reads
- GIVEN the two-crate workspace with tracked files `docs/notes.md` and `.github/workflows/ci.yml` besides its own, and components `a` (`crates/a/**`, `depends_on: [b]`), `b` (`crates/b/**`) and one covering the workspace's top-level files
- WHEN `clm survey --components` runs
- THEN it exits 1 with a `survey/uncovered` line naming `.github/workflows/ci.yml`, and no line naming `docs/notes.md`

#### Scenario: the old key
- GIVEN a map that lists its components under `nodes`
- WHEN `clm survey --components` runs on it
- THEN it exits 1 naming the file, `nodes` and `components`

#### Needs
- clm/component-flags

#### Serves
- capability/adopt-clm
- architecture/incremental-adoption
- architecture/plain-terms
