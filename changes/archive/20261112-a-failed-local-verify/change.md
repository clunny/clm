---
status: done
resolves: [a-failed-local-verify-records-no-log-of-which-tests-failed]
---

## Intent

Let a triage session show the human the failure itself when it puts a red slow-test result to them (`backlog/a-failed-local-verify-records-no-log-of-which-tests-failed`). `clm intake` prints, after each `verify <sha> new <culprit> <component>...` line, a line `verify <sha> log <path>` naming the log this clone kept of that failed run, found through a new `clm-verify` library call, `kept_log`. The triage skill reads that log and shows the last 40 lines and the step open as it ended beside the line, and names them and the log's path in the body of the item it writes when the human takes it in, as `/clm:verify` already does for a run it made. When this clone kept no log, the skill says so.

## Problem

A failed `local` `clm verify` already keeps its whole output, at `.clm/verify-logs/<short sha>.log` in the clone's main worktree (`verify/verify-run`), and prints the step and last lines once, to whoever ran it: at the end of a `/clm:build` or `/clm:triage` session that may be long gone. The `verified` record holds the result, culprit, components and mode only, and `clm intake`'s line names none of the log, so the triage session the human later answers could not say which test failed or judge whether a culprit that only touched a retry loop was real or a flaky probe. `plugin/skills-run-verify` asks for an item body naming "the verify log", which nothing tells the session how to find. No requirement states that intake or triage reaches the kept output, so the fix is a spec change (`code-fix-needs-a-written-rule`).

## Approaches

1. **`clm intake` names the kept log; the skill reads it** (chosen). Reuses the log and its `::group::` step markers, which already exist (`reuse-first`); `verify` keeps owning where its logs live (`path-ownership`) through `kept_log`, which matches by prefix of the full commit ID, so a name made with a shorter abbreviation is still found.
2. **The `verified` record carries the failing step and last lines.** Rejected: the ledger is shared, signed and append-only, so every red record would push 40 lines of the project's test output, whatever they hold, to `origin` for good, while a `local` run is nearly always read on the clone that ran it.
3. **The record carries the log's path.** Rejected: a path on one machine means nothing to another clone.
4. **The skill builds `.clm/verify-logs/<sha>.log` itself.** Rejected: it copies verify's layout into a skill and misses a log once the repository's abbreviations grow.
5. **A command of its own (`clm verify --log <sha>`).** Rejected (`yagni`): `clm intake` already lists the red result.

The decision is `verify-log-through-intake`.

## Outcome

- `verify` Contract: names the library call `kept_log(root, commit)`.
- `verify/kept-log` (added): the path of the kept log of a failed run of `commit` in this clone's main worktree, by the longest file name that is a prefix of the full ID (at least 7 characters), or none; reads only that directory. Callable from a linked worktree.
- `orchestrator.triage` Frontmatter: `depends_on` gains `verify`, since `clm-triage` calls `clm_verify::kept_log` (a new crate dependency, `clm-verify` depending on nothing of `clm-triage`). Contract: `clm intake` also reads the clone's kept verify logs and names the log after a red result no item has taken in.
- `orchestrator.triage/intake-verify-failures` (modified): `verify <sha> log <path>` after each `new` line that has a kept log, none after an `imported` line or when none is kept; scenarios `a new failure` (modified), `a kept log` (added) and `taken in` (modified). Needs `verify/kept-log`.
- `plugin.triage/triage-verify-log` (added): the triage skill names the `log` line, shows the last 40 lines and the open step beside a `new` line, says so when no log was kept, and the body names the step, the lines and the log's path when there was one. Needs `orchestrator.triage/intake-verify-failures`. It is `plugin.triage`'s, whose paths hold the skill, not a change to `plugin/skills-run-verify`, which stays as written and true (`path-ownership`).
- The capability test of `slow-tests-after-land` sees the new `log` line after its `new` line; `intake-verify-failures` names it, and its build checks that line.

## Constraints

- `verify-after-land`, `path-ownership`, `reuse-first`, `yagni`, `code-fix-needs-a-written-rule`, `untrusted-text-read-only-verifier`: the `log` line's path is built from the clone's own root and a commit-ID prefix, so no host text reaches intake's lines, and the log is the project's own test output on this clone; `verify-log-through-intake` records this.
- Edge cases known: a red result from another clone or from `pipeline`/`import` mode has no log here, and the skill says so; a log named with a shorter abbreviation than `git rev-parse --short` now gives is found by prefix. No workarounds known.
