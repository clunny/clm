---
status: done
resolves:
  - guide-says-lockfiles-don-t-count-toward-size-but-they-count
---

## Intent

The guide says what counts toward a component's code size as `check` counts it: lockfiles count in full until `.gitattributes` marks them, and a session checks for an unmarked one before it treats a component as oversized. `/clm:init` proposes a `linguist-generated` line for each lockfile when it brings a repository under CLM, and this repository marks every lockfile it tracks, not three of six (`backlog/guide-says-lockfiles-don-t-count-toward-size-but-they-count`).

## Problem

`docs/guide/shaping-the-graph.md` said "Lockfiles, binaries and anything `.gitattributes` marks `linguist-generated` or `linguist-vendored` don't count". `check`'s **Size** leaves out only binary files and marked files, as designed (`node-size-budgets`, `reuse-first`): a lockfile counts in full unless the project marks it. In a Python project a `uv.lock` made a component read as about twice its code budget, a false oversized state that could lead a session to split a component that needs no split. Nothing in adoption told the project to mark its lockfiles.

The same gap is in this repository: `.gitattributes` marks `Cargo.lock`, `web/package-lock.json` and `plugin-panel/package-lock.json`, but not `capabilities/panel/package-lock.json`, `capabilities/web/package-lock.json` or `web/site/package-lock.json`, added since.

The size rule is right; the guide was wrong, and adoption never sets up the convention the rule reads.

## Approaches

- **Leave out known lockfile names in `check`'s size count.** Rejected: `node-size-budgets` and `web-size-from-listing` chose git's `linguist` attributes so the count reuses a convention GitHub and git already read (`reuse-first`), and the Worker's count must agree with `clm size`; a list of names in CLM would be a second convention to keep in step.
- **`clm init` prints a next step naming each unmarked tracked lockfile.** Deterministic, but it misses a new project, which has no lockfile yet when `clm init` runs, and a `.gitattributes` the human then creates by hand after `clm init` lies in no component's `paths` and outside the root's `outside`, so the bootstrap commit fails `check/unowned-files`.
- **`clm init` writes or appends `.gitattributes` itself.** Covers a new project only by guessing lockfile names from `--lang`, which the skill doesn't always pass and which can't know the Python package manager; and it would add a file `clm init` owns to `init-root-ownership`'s `paths`, changing that rule's scenarios.
- **`/clm:init` proposes the lines before it runs `clm init` (chosen).** The skill knows the stack from the onboarding conversation, so it can name a new project's lockfiles, and for a repository with code it asks `git check-attr` which tracked lockfiles are unmarked. Written before `clm init`, `.gitattributes` is among the files the bootstrap commit holds, so `clm init` names it in the root's `outside` (or keeps it there when tracked) and the bootstrap passes path ownership with no change to `init`. Lines are written bare by file name, marking a file of that name at any depth, so a later package's lockfile is covered too.
- **Fix the guide only.** Necessary, and part of the change, but it only helps a session that reads the page; adoption is when the convention is cheapest to set up.

## Outcome

- `docs/guide/shaping-the-graph.md` says binary and marked files are left out and everything else counts, lockfiles included, shows the bare-name line, and, in the oversized section, has a session check for an unmarked lockfile or generated file before splitting or setting a budget (a hand edit on this branch).
- `plugin.init/init-skill-marks-lockfiles`: the init skill tells the human why lockfiles need marking and proposes the lines before `clm init` runs.
- `clm/lockfiles-generated` (modified): every tracked `Cargo.lock` and `package-lock.json` is marked, written by bare file name, and its test lists them with `git ls-files` instead of naming three paths, so a new npm package's lockfile can't be missed. The root's Design says the same. The root's build edits `.gitattributes`.

## Constraints

- `reuse-first`: the count keeps reading git's `linguist` attributes; nothing in `check` changes.
- `incremental-adoption`: the bootstrap commit must pass `check/unowned-files`, which is why the skill writes `.gitattributes` before `clm init` rather than after.

## Edge cases and workarounds

- A repository whose existing `.gitattributes` marks a lockfile `-linguist-generated` on purpose: `git check-attr` reports it unset, so the skill proposes the line; the human says no. No workaround needed.
- A lockfile name the skill doesn't know (a less common package manager): the skill proposes from what the repository tracks and the stack, and the guide covers anything later.
