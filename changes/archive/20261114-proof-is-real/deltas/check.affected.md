---
component: check.affected
---

## MODIFIED Requirements

### Requirement: affected-files <!-- base: 0b773abdf888713f -->
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
- check/path-ownership
