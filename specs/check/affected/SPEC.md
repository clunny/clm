---
title: Affected
status: active
owners: ["@clunny"]
depends_on: [model, check, ledger, vcs]
paths: ["crates/clm-affected/**"]
successor: ""
serves: [capability/land-safely]
---

## Contract
`clm affected [--base <ref>] [--root <dir>]` prints what a change can break, read from the spec graph (`graph-test-selection`): the components it affects and the files whose tests a check should run, or that every test must run. It knows no test runner; the project's check command turns the files into its runner's arguments (`ci`), and `clm land` hands it the output (`land`).

- The *changed files* are the paths that differ between the merge base of `<ref>` (default `origin/main`) and `HEAD`, and the working tree, with the untracked files git doesn't ignore, leaving out every path `ledger`'s `doc_only` names. A deleted file is a changed file.
- When a changed file matches a glob of `config`'s `[affected] all`, or no component's `paths` hold it (`check`'s `owner` gives none), the output is one line, `all <file>`, naming the first such file in path order.
- Otherwise the *affected* components are the owners of the changed files, and every component that depends on one of them, directly or indirectly (`depends_on`). The output is a *component line* for each affected component, the word `component-words` keeps for these lines followed by a space and the component's ID, which `ci`'s check script and `clm land` read, then `file <path>` for each selected file, each group sorted, and nothing when no file changed.
- The *selected* files are the changed files that still exist, whatever their content, and the files `check`'s coverage scan reads (tracked, or untracked and not ignored; UTF-8; Markdown left out) that hold a `req:` marker whose requirement ID names an affected component (the part before `/`, whether or not the requirement still exists), or that hold no `req:` marker and whose `owner` is affected.

It exits 0 with its output, 1 with the reason when `<ref>` doesn't resolve to a commit, when it has no merge base with `HEAD`, or when the graph or the settings can't be loaded, and 2 when git fails otherwise.

## Design
Crate `clm-affected`. `select(graph, changed, markers, all) -> Selection` is pure: it takes the graph (`model`), the changed paths with whether each still exists, the markers of every scanned file (`check`'s `scan`, each with its path) and the `[affected] all` globs, and returns `All(path)` or `Selection::Nodes`, holding the affected components' IDs and the selected files. Owners and globs are matched with `check`'s `owner` and `path_matches`, so a file belongs to the same component as in `clm pending`; paths are disjoint across components, and `owner`'s deepest-first rule settles any overlap. A `clm:remove-by` marker isn't a `req:` marker: a file holding only those belongs to its owner. Dependants are found by walking `depends_on` backwards from the changed components until no component is added. Doc-only paths come from `ledger`'s `doc_only`, the one list `code_key` uses, so a change whose code key is unchanged selects nothing.

`affected(root, base, all)` gathers the inputs through `git` (`git-plumbing-layer`): `git rev-parse --verify <base>^{commit}` (failing: exit 1, `unknown base <ref>`), `git merge-base <base> HEAD` (exit 1, `no merge base between <ref> and HEAD`, when it prints nothing with status 1), then `git diff --name-only --no-renames -z <merge base>` (the working tree against the merge base, so a rename counts as its old and new path) and `git ls-files --others --exclude-standard -z`. Any other git failure is exit 2. A changed path is matched as git lists it, relative to the repository root. A deleted file still selects through its path, its owner being affected, but has no `file` line; a deleted file no component holds selects everything.

`crates/clm` dispatches `clm affected`, with `--root` as every command has it, loading `config`'s `affected` and passing its globs in, as it passes `detectors` to `clm check`, so this crate doesn't depend on `clm-config`; settings that can't be loaded exit 1 naming the file and key, as for `clm size`. This component's build adds the dispatch (the root's Design), and its scenarios run the command, so they wait on `check`'s, `ledger`'s and `config`'s requirements below (`#### Needs`). Fixture graphs in the tests give their root component no `depends_on`, so the root is affected only when a scenario says so.

## Requirements

### Requirement: affected-files
`clm affected` SHALL print, after the components, `file <path>` for each changed file that still exists, and for each file `check`'s coverage scan reads that holds a `req:` marker whose requirement ID names an affected component, or that holds no `req:` marker and whose owner is affected, sorted by path, and for no other file.

#### Scenario: cited, owned and changed
- GIVEN the components of `affected-components` and a component `e` depending on nothing with `paths: ["crates/e/**"]`; `crates/e/tests/x.rs` citing a requirement of `c` and one of `d`; `crates/e/tests/y.rs` citing only `d`; `crates/a/src/util.rs` with no marker; `crates/d/src/lib.rs` with no marker; and a branch that changes `crates/a/src/lib.rs` and `crates/e/tests/z.rs`, which cites only `d`
- WHEN `clm affected --base main` runs
- THEN the `file` lines are `crates/a/src/lib.rs`, `crates/a/src/util.rs`, `crates/e/tests/x.rs` and `crates/e/tests/z.rs`, and not `crates/e/tests/y.rs` or `crates/d/src/lib.rs`

#### Scenario: marker naming a removed requirement
- GIVEN `crates/e/tests/old.rs` citing `b/gone`, which no longer exists, and a branch that affects `b`
- WHEN `clm affected --base main` runs
- THEN it prints `file crates/e/tests/old.rs`

#### Scenario: uncommitted, untracked and deleted
- GIVEN a clean branch equal to `main`, then an uncommitted edit to `crates/a/src/lib.rs`, a new untracked, unignored file `crates/a/src/new.rs`, and `crates/a/src/old.rs` deleted
- WHEN `clm affected --base main` runs
- THEN it prints the component line of `a`, `file crates/a/src/lib.rs` and `file crates/a/src/new.rs`, and no line for `crates/a/src/old.rs`

#### Needs
- check/path-ownership

### Requirement: affected-all
When a changed file matches a glob of the `[affected] all` setting, or lies in no component's `paths`, `clm affected` SHALL print exactly one line, `all <path>`, naming the first such file in path order, and exit 0.

#### Scenario: file no node holds
- GIVEN a branch that changes `crates/a/src/lib.rs` and `tools/build.sh`, which no component's `paths` hold
- WHEN `clm affected --base main` runs
- THEN it prints only `all tools/build.sh`

#### Scenario: setting's glob
- GIVEN a repository `clm.toml` with `[affected]` and `all = ["Cargo.lock"]`, the root component's `paths` holding `Cargo.lock`, and a branch that changes only `Cargo.lock`
- WHEN `clm affected --base main` runs
- THEN it prints only `all Cargo.lock`

#### Scenario: deleted file no node holds
- GIVEN a branch that deletes `tools/build.sh`, which no component's `paths` hold
- WHEN `clm affected --base main` runs
- THEN it prints only `all tools/build.sh`

#### Needs
- check/path-ownership
- config/affected-setting

### Requirement: doc-only-selects-nothing
`clm affected` SHALL leave out of the changed files every path `ledger`'s `doc_only` names, so a change only to those paths prints nothing, and a change to those paths and to code prints what the code alone would.

#### Scenario: docs and backlog only
- GIVEN a branch that changes only `docs/decisions/x.md`, `backlog/y.md` and `specs/a/SPEC.md`
- WHEN `clm affected --base main` runs
- THEN it prints nothing and exits 0

#### Scenario: docs beside code
- GIVEN a branch that changes `backlog/y.md` and `crates/d/src/lib.rs`, and nothing depends on `d`
- WHEN `clm affected --base main` runs
- THEN it prints the component line of `d` and `d`'s files, and no `all` line

#### Needs
- ledger/doc-only

### Requirement: affected-base
`clm affected` SHALL compare the working tree, untracked files included, with the merge base of `--base <ref>` and `HEAD`, defaulting `<ref>` to `origin/main`. It SHALL exit 1 naming the ref when `<ref>` doesn't resolve to a commit or has no merge base with `HEAD`, and 1 with the error when the graph can't be loaded.

#### Scenario: main moved on
- GIVEN a branch that changes `crates/a/src/lib.rs`, and `main` that has since changed `crates/d/src/lib.rs`
- WHEN `clm affected --base main` runs on the branch
- THEN it prints the component line of `a` and none of `d`

#### Scenario: unknown base
- GIVEN a repository with no ref `nope`
- WHEN `clm affected --base nope` runs
- THEN it exits 1 naming `nope`

#### Scenario: invalid graph
- GIVEN a repository whose `specs/` has a dependency cycle
- WHEN `clm affected --base main` runs
- THEN it exits 1 with the error and prints no component, `file` or `all` line

### Requirement: git-through-layer
`clm-affected` SHALL list `clm-git` among its dependencies and SHALL run no `git` or `gh` process itself: every call goes through `clm-git`'s `git`, `run` or repo queries (`git-plumbing-layer`).

#### Scenario: covered by the scan
- GIVEN the workspace
- WHEN a test in `clm-affected` reads `crates/clm-affected/Cargo.toml` and every file under `crates/clm-affected/src`
- THEN `clm-git` is a dependency and no file holds `Command::new("git")` or `Command::new("gh")`

#### Serves
- architecture/reuse-first

### Requirement: affected-components
`clm affected` SHALL print a component line (the Contract's) for each component whose `paths` hold a changed file and for each component that depends on one of those, directly or indirectly, sorted by ID, and for no other component.

#### Scenario: dependants
- GIVEN a root with no dependencies, components `a`, `b` depending on `a`, `c` depending on `b`, and `d` depending on nothing, and a branch that changes only `crates/a/src/lib.rs`, which `a`'s `paths` hold
- WHEN `clm affected --base main` runs on the branch
- THEN it prints the component lines of `a`, `b` and `c`, and no other component line

#### Scenario: nothing changed
- GIVEN a branch with no commits beyond `main` and a clean working tree
- WHEN `clm affected --base main` runs
- THEN it prints nothing and exits 0

#### Needs
- check/path-ownership
