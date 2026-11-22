---
component: rename
---

## MODIFIED Requirements

### Requirement: references <!-- base: dd6cdde7314e0e12 -->
`clm rename` SHALL rewrite every reference to a renamed ID: whole values in `depends_on`, `serves`, `#### Serves`, `#### Needs` and `#### Enforced by` and in backlog items' `components`, `blocks` and `requirement`, each item of a list a value whether the list is written in flow style (`[a, b]`) or, in a backlog item, in block style (a `- a` line per item), and a quoted value as a bare one, keeping its quotes; and, in spec text, each code span whose whole text is an old requirement or entry ID, or an old slug that no other requirement, entry or decision has. A component's ID in a code span is a word and stays, and no reference is rewritten on a line of fenced code, a line that opens or closes a code fence or lies inside one as `model/code-fences` reads fences. Beyond these references, the terms `term-swap` swaps, the markers `marker-rewrite` rewrites, the versions `flag-rerecord` re-records and the other changes' delta elements `other-deltas` rebases, it SHALL rewrite nothing else, the value of any other frontmatter key, such as `successor`, included, and it SHALL edit no file under `changes/archive/` or `docs/decisions/` but to rewrite a marker there as `marker-rewrite` does, which `check/dangling-markers` would otherwise report as naming no requirement.

#### Scenario: references across the repository
- GIVEN requirement `a/x` renamed to `a/y`, component `b` whose requirement `z` holds `#### Needs` `- a/x` and the text ``(`a/x`, `x`)``, no other requirement or decision with slug `x`, `ARCHITECTURE.md` naming `a/x` under `#### Enforced by`, and `backlog/q.md` with `blocks: [a/x]`
- WHEN `clm rename` runs
- THEN `b/z`'s `#### Needs` names `a/y` and its text reads ``(`a/y`, `y`)``, `ARCHITECTURE.md` names `a/y`, and `backlog/q.md` has `blocks: [a/y]`

#### Scenario: ambiguous slug
- GIVEN requirement `a/x` renamed to `a/y`, and component `b` with its own requirement `x` and a Design citing `` `x` ``
- WHEN `clm rename` runs
- THEN `b`'s Design still cites `` `x` ``

#### Scenario: a component's bare name
- GIVEN component `apply` renamed to `fold`, and a Design reading `` `apply` writes `specs/` ``
- WHEN `clm rename` runs
- THEN that text is unchanged

#### Scenario: an item's components
- GIVEN component `a.b`, a change whose `renames` holds `component: a.b` with `to: a.c`, and `backlog/x.md` with `components: [a.b]`
- WHEN `clm rename` runs on it
- THEN `backlog/x.md` holds `components: [a.c]`

#### Scenario: a reference in fenced code
- GIVEN requirement `a/x` renamed to `a/y`, and component `b` whose Design holds a block fenced by four backticks with the lines ```` ``` ````, `` see `a/x` `` and `~~~`, and, outside it, the line `` see `a/x` ``
- WHEN `clm rename` runs
- THEN the line in the block is unchanged, and the line outside it reads `` see `a/y` ``

#### Scenario: a list in block style
- GIVEN components `a` and `c`, `backlog/x.md` whose frontmatter holds `components:` followed by the lines `  - a` and `  - c`, and change `r`, `proposed`, with `renames` holding `component: a` and `to: d`
- WHEN `clm rename r` runs
- THEN `backlog/x.md`'s `components` lines read `  - d` and `  - c`, and the rest of the file is unchanged

#### Scenario: a key that holds no reference
- GIVEN component `a`, component `b` whose frontmatter holds `successor: "a"` and `depends_on: [a]`, and change `r`, `proposed`, with `renames` holding `component: a` and `to: d`
- WHEN `clm rename r` runs
- THEN `b`'s frontmatter holds `successor: "a"` and `depends_on: [d]`

#### Scenario: a marker in the archive
- GIVEN requirement `a/x`, an archived change `old` whose `changes/archive/old/assets/a/t.rs` holds a test citing `a/x` with no version, `docs/decisions/d.txt` reading `` `a/x` ``, and change `r`, `proposed`, with `renames` holding `requirement: a/x` and `to: y`
- WHEN `clm rename r` runs
- THEN `changes/archive/old/assets/a/t.rs` differs from before only in that its marker cites `a/y`, `docs/decisions/d.txt` is unchanged, and every marker the coverage reads in the tree names a requirement the spec holds

#### Needs
- model/code-fences
