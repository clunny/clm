---
status: applied
resolves: [a-removal-marker-after-a-non-ascii-letter-skips-its-item-unchecked]
---

## Intent

Make the scan behind `check/remove-by-markers` and `clm unreferenced` agree on what a `clm:remove-by <change-id>` marker is, so no item is skipped for a marker that `clm check` never checks for liveness.

## Problem

`check.unreferenced/removal-marked` states a marker as `clm:remove-by ` not preceded by an ASCII letter, digit or `_`. `check.coverage`'s `scan`, whose `remove_by` list `check/remove-by-markers` checks, refuses one after any Unicode alphanumeric character (`follows_word` in `crates/clm-coverage/src/coverage.rs` uses `char::is_alphanumeric`). So `éclm:remove-by 20260101-x` before an item makes `clm unreferenced` skip it, while `clm check` never asks whether `20260101-x` is live, and the item stays skipped after its removal is abandoned. `check.unreferenced`'s Design promises the opposite ("every marker it accepts is one `check/remove-by-markers` checks"). No requirement of `check.coverage` states what may precede a removal marker: its Design names only the regex, and `check`'s Design says "text matching the regex", so the fix needs a written rule (`code-fix-needs-a-written-rule`).

## Approaches

- **The coverage scan reads removal markers by `removal-marked`'s ASCII rule (chosen).** One new requirement of `check.coverage`, `removal-markers`, states the marker exactly as `removal-marked` does, and its build makes `parse_removal_markers` refuse only an ASCII letter, digit or `_` before `clm:`. `removal-marked` already states ASCII and is built; `clm unreferenced` depends on `vcs` alone and reads the marker itself, so the coverage scan is the one to move.
- **Both scans Unicode.** Rejected: it modifies a requirement just built, and `check.unreferenced` would need Unicode classes for nothing a project writes; the ASCII rule matches how every other boundary in that detector is drawn (`References`).
- **`clm unreferenced` calls `clm-coverage`'s parser.** Rejected: `check.unreferenced` depends on `vcs` alone so it runs in projects without CLM's specs, and adding `check.coverage` (with `model`) to its `depends_on` for a twelve-line parser costs more than stating the rule twice and proving each side.
- **`follows_word` ASCII for `req:` markers too.** Rejected as out of scope: nothing reported a `req:` marker after a non-ASCII letter, and changing it moves every coverage reader (`rename`, `check.affected`, the Worker) for no defect (`yagni`). The Design now says the two boundaries differ, and why.

## Outcome

- `check.coverage` gains `removal-markers`: a removal marker is `clm:remove-by ` not preceded by an ASCII letter, digit or `_`, followed by eight ASCII digits, `-` and one or more of `a-z`, `0-9` and `-`. Its build splits the boundary test in `crates/clm-coverage/src/coverage.rs` so `parse_removal_markers` checks ASCII alone, `parse_markers` keeps `follows_word` as it is, and the date's eight digits stay ASCII; the scenario is proven through `scan` on a directory outside git.
- `check.coverage`'s Design names the marker by that rule, and `check`'s Design adds the boundary to its regex sentence. Neither adds a requirement to `check`, whose `remove-by-markers` reads the markers `scan` gives.

## Constraints

- `path-ownership`: the fix lies in `crates/clm-coverage/**`, `check.coverage`'s own paths.
- `expand-contract`: nothing calls `parse_removal_markers` outside the crate, so no contract moves.

## Edge cases and workarounds

- A full-width digit (`２`) is a Unicode digit; the date's digits must be ASCII, as the code already requires. The scenario's full-width line catches a build that loosens the characters of the ID it reads, not one that loosens only the date check, which the ID's ASCII characters already keep ASCII.
- `clm unreferenced` also asks where a marker sits (a comment directly before an item); `scan` lists a marker wherever its text is. The direction that matters holds: every marker `clm unreferenced` honours is one `scan` lists.
