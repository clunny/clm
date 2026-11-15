---
component: check.affected
---

## MODIFIED Requirements

### Requirement: affected-files <!-- base: 6074ba2c60b29f24 -->
`clm affected` SHALL print, after the components, `file <path>` for each changed file that still exists, and for each file `check`'s coverage scan reads that holds a `req:` marker, inside a test or outside one (`check.coverage/test-spans`, `scan`'s `outside`), whose requirement ID names an affected component, or that holds no `req:` marker at all and whose owner is affected, and for each reader (`affected-readers`), and, whenever any component is affected, for each file an exemption of its owner names for `undeclared-imports` (`model/exemptions`), since it imports code the graph doesn't connect it to and so can break from a change anywhere (`land-checks-stay-fast`, `temporary-exemptions`), sorted by path, and for no other file.

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

#### Scenario: a marker outside a test still selects
- GIVEN `tests/t.ts` owned by `b`, whose only marker is a file-header comment citing `a/x`, and a change to `a`'s code
- WHEN `clm affected` runs
- THEN it prints `file tests/t.ts`

#### Needs
- check.coverage/test-spans
- model/exemptions
- check.coverage/owner

### Requirement: affected-all <!-- base: c4a42acfc7076363 -->
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
- check.coverage/owner
- config/affected-setting

### Requirement: affected-components <!-- base: b287ecd8bfb9181c -->
`clm affected` SHALL print a component line (the Contract's) for each component that owns a changed file (`check.coverage/owner`) and for each component that depends on one of those, directly or indirectly, sorted by ID, and for no other component.

#### Scenario: dependants
- GIVEN a root with no dependencies, components `a`, `b` depending on `a`, `c` depending on `b`, and `d` depending on nothing, and a branch that changes only `crates/a/src/lib.rs`, which `a`'s `paths` hold
- WHEN `clm affected --base main` runs on the branch
- THEN it prints the component lines of `a`, `b` and `c`, and no other component line

#### Scenario: nothing changed
- GIVEN a branch with no commits beyond `main` and a clean working tree
- WHEN `clm affected --base main` runs
- THEN it prints nothing and exits 0

#### Needs
- check.coverage/owner

### Requirement: affected-readers <!-- base: 94edd10f5c1d9601 -->
`clm affected` SHALL treat as a *reader* each file `check`'s coverage scan reads that holds the text `clm:reads ` followed by a glob, up to the next whitespace, matching, as component `paths` match, a path that differs between the merge base and the working tree, doc-only paths and untracked files included. It SHALL print `file <path>` for each reader that still exists among its other `file` lines, sorted with them, whether or not the reader's owner is affected, and given `--readers` SHALL print those lines alone, with no component or `all` line, and nothing when no reader matches.

#### Scenario: reader of a doc-only file
- GIVEN `crates/e/tests/r.rs` holding `clm:reads ARCHITECTURE.md`, `crates/e/tests/s.rs` holding `clm:reads specs/**`, nothing depending on `e`, and a branch that changes only `ARCHITECTURE.md`
- WHEN `clm affected --base main` runs
- THEN it prints only `file crates/e/tests/r.rs`

#### Scenario: readers beside code
- GIVEN the same files and a branch that changes `ARCHITECTURE.md` and `crates/a/src/lib.rs`
- WHEN `clm affected --base main` runs
- THEN it prints the component lines and `file` lines a change to `crates/a/src/lib.rs` alone prints, and `file crates/e/tests/r.rs` among the `file` lines

#### Scenario: readers alone
- GIVEN the same files and branch
- WHEN `clm affected --base main --readers` runs
- THEN it prints only `file crates/e/tests/r.rs`

#### Scenario: no reader matches
- GIVEN the same files and a branch that changes only `backlog/y.md` and `tools/build.sh`, which no component's `paths` hold
- WHEN `clm affected --base main --readers` runs
- THEN it prints nothing and exits 0

#### Scenario: a marker in Markdown
- GIVEN `docs/notes.md` holding `clm:reads ARCHITECTURE.md` and a branch that changes only `ARCHITECTURE.md`
- WHEN `clm affected --base main --readers` runs
- THEN it prints nothing

#### Needs
- check.coverage/owner
- ledger/doc-only
