---
status: done
resolves: [triage-s-wrap-up-step-doesn-t-say-how-to-archive-the-changes-clm]
---

## Intent

Have `/clm:triage`'s wrap-up archive the changes `clm wrap-up` lists as `complete <id>`, the way `/clm:build`'s wrap-up does: on one `impl/archive/<id>` branch cut from `origin/main`, where `clm rebase` archives every complete change and deletes the items they resolve (`archive-branches`, `archive-on-sync`), landed with `clm land`. A triage session then never sets a change's `status` by hand or runs `clm archive` on an `applied` change.

## Problem

A draft that lands a change with nothing to build (a backlog-and-decision change, no deltas) leaves it complete on `main` within the same triage session, so `clm wrap-up` prints `complete <id>` there. The triage skill's wrap-up step says only "fix what it reports and run it again", and no `plugin.triage` requirement states what to do with a `complete` line. A session tried `clm archive <id>`, which refuses an `applied` change, then set `status: done` by hand and archived, bypassing `archive-on-sync`, which deletes the resolved items too.

## Approaches

1. **A `plugin.triage` requirement that states build's archive route for triage** (chosen). The skill text carries the steps, as build's does, adapted to triage's way of entering another worktree (the EnterWorktree tool, as for a fold's worktree, `triage-enters-branch-worktree`) and to an isolated session (`session-worktrees`). Its scenarios mirror `build-archive-branch`'s, so the nested branch name stays guarded against reading as a claim on a component `archive`.
2. **Point triage's step at `/clm:build`'s step.** No duplicated prose, but a skill reading another skill's numbered step is fragile: step numbers move, and the build step's worktree moves (`cd` into the archive worktree) don't match how triage enters worktrees. Rejected.
3. **Make `clm wrap-up`, or `clm archive`, archive complete `applied` changes itself.** Deterministic, but it would cut, push and land a branch from a checkpoint command, or let `clm archive` bypass the code branch that `archive-branches` puts archiving on. A redesign far bigger than the defect; rejected (`yagni`).
4. **Move the archive route into a shared `plugin` rule both skills follow.** Cleaner if a third skill ever archives, but today only `build` and `triage` do, and a shared rule needs a requirement in each child plus `#### Needs`; rejected for now (`yagni`).

## Outcome

- `plugin.triage/triage-archives-complete` (added): the triage skill archives `complete <id>` changes on `impl/archive/<id>` from `origin/main`, through `clm rebase`, `clm branch push`, `clm pr open` and `clm land`; removes a leftover archive worktree first; leaves archiving to another session's unlanded `impl/archive/<id>` or a refused push; cuts the branch in its own checkout when isolated; runs `clm wrap-up` again; never names `impl/archive` without `/<id>`.
- `plugin.triage` Design: the wrap-up paragraph says the session archives complete changes as `build` does, and the summary names what it archived or left to another session.

## Constraints

- `archive-branches`, `archive-on-sync`, `checkpoint-before-done`, `session-worktrees`, `land-locally`.
- No edge cases or workarounds known.

## Settled by the builder check and the audit

- `plugin` Contract: the `session-worktrees` line says `triage` cuts its archive branch the same way, a rule `triage-archives-complete` states. The shared requirement `plugin/session-worktrees` and its test stay as they are, since the triage requirement carries its own scenarios (architecture audit).
- A session isolated under `.claude/worktrees/` removes no leftover archive worktree, as `build`'s does (`session-worktrees` decision; architecture audit). The archive text sits after the wrap-up paragraph that `wrap-up-left-lines`' test reads, leaving it whole (builder check).

## Open questions

- None. The builder check found no requirement this one relies on that isn't current and no capability test it changes.
