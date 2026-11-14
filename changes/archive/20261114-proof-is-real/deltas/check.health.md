---
component: check.health
---

## MODIFIED Requirements

### Requirement: churn-signal <!-- base: 563e439fbc59fe28 -->
`clm health` SHALL report, under `churn`, each requirement of the graph whose ID the commits reachable from `HEAD` cite at three or more distinct versions, counting the versioned `req:` markers `marker_spans` finds in the added lines of every commit's patch for every file but Markdown, whatever their path and whether or not they sit in a test, since a patch line shows no test around it, as `specs/<component path>/SPEC.md: health/churn: <component>/<slug> cited at <n> versions`. It SHALL NOT count a marker without a version, a marker in a Markdown file, or a marker no commit holds, and SHALL skip an ID no component holds. The library's `churn(root)` SHALL return every cited ID, held by a component or not, with its distinct versions, and `None` when `HEAD` names no commit.

#### Scenario: re-cited across commits and a rename
- GIVEN component `a` with requirements `x` and `y`, and a history in which `tests/old.rs` cites `req:a/x@11111111`, then `req:a/x@22222222`, then is renamed to `tests/new.rs` where it cites `req:a/x@33333333`, and `tests/y.rs` cites `req:a/y@aaaaaaaa`, then `req:a/y@bbbbbbbb`
- WHEN `clm health` runs
- THEN it prints `specs/a/SPEC.md: health/churn: a/x cited at 3 versions` and no `churn` line for `a/y`

#### Scenario: what doesn't count
- GIVEN component `a` with requirement `z`, a history whose commits cite `req:a/z@cccccccc` and then `req:a/z@dddddddd` in `tests/z.rs`, `req:a/z@eeeeeeee` in `notes.md` only, and `req:a/z` without a version in `tests/z.rs`, and cite `a/gone`, which no component holds, at four versions in `tests/gone.rs`; and an uncommitted edit of `tests/z.rs` that cites `req:a/z@ffffffff`
- WHEN `clm health` runs, and `churn` is called for the same root
- THEN it prints no `churn` line for `a/z` or `a/gone`, and `churn` returns `a/z` with the two versions `cccccccc` and `dddddddd` and `a/gone` with four

#### Needs
- check.coverage/marker-spans
