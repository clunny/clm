---
component: check.affected
---

## ADDED Requirements
### Requirement: overlap-selection
`clm affected --overlap <a> <b>` SHALL print what both changes can break. With `<m>` the merge base of the commits `<a>` and `<b>` name, each *side* is the selection, readers included, of the paths that differ between `<m>` and that side's commit, computed as `affected-files`, `affected-all`, `affected-components` and `affected-readers` compute it but from the two commits alone, with no working tree and no untracked file. Its output SHALL be:
- when both sides are `all`, the first side's `all` line;
- when one side is `all`, the other side's output unchanged;
- otherwise the component lines of the components both sides affect, then the `file` lines of the files both sides select, each group sorted as `affected-components` sorts it, and nothing at all when no file is in both.

The library SHALL offer the pure `overlap(a, b) -> Selection` that applies these rules to two selections, and `affected_between(root, from, to, all)`, the selection between two commits. `--overlap` with `--base`, `--readers` or `--why` SHALL exit 1 naming both flags. A `<a>` or `<b>` that doesn't resolve to a commit SHALL exit 1 with `unknown revision <rev>`, two commits with no merge base SHALL exit 1 with `no merge base between <a> and <b>`, and any other git failure SHALL exit 2, as `affected-base` says for `--base`.

#### Scenario: independent changes
- GIVEN components `a` and `b` with no `depends_on` between them, owning `crates/a/**` and `crates/b/**`, a commit `m`, and commits `x` and `y` on top of `m`, `x` changing `crates/a/src/lib.rs` and `y` changing `crates/b/src/lib.rs`
- WHEN `clm affected --overlap x y` runs
- THEN it prints nothing and exits 0

#### Scenario: a dependant of both
- GIVEN the same, and a component `c` owning `crates/c/**` with `depends_on: [a, b]` and a test `crates/c/tests/t.rs`
- WHEN `clm affected --overlap x y` runs
- THEN it prints the component line for `c` and `file crates/c/tests/t.rs`, and no line for `a` or `b`

#### Scenario: one side selects everything
- GIVEN `[affected] all = ["Cargo.lock"]`, `x` changing `Cargo.lock` and `y` changing `crates/b/src/lib.rs`
- WHEN `clm affected --overlap x y` runs
- THEN it prints exactly what `clm affected --base m` prints with `y` checked out

#### Scenario: a reader on one side, its owner on the other
- GIVEN `x` changing `specs/a/SPEC.md` and `y` changing `crates/a/src/lib.rs`, and `crates/a/tests/spec.rs` holding a `clm:reads specs/**` marker
- WHEN `clm affected --overlap x y` runs
- THEN its output holds `file crates/a/tests/spec.rs`

#### Scenario: flags that don't combine
- GIVEN any repository
- WHEN `clm affected --overlap x y --readers` runs
- THEN it exits 1 naming `--overlap` and `--readers`
