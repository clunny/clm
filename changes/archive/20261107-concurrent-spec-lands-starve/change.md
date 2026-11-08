---
status: done
resolves: [concurrent-spec-lands-starve-each-other-on-gate-up-to-date-for]
---

# clm land regenerates the specs/ of a spec branch that is behind main in them

## Intent

Let a spec branch that is behind `main` only in derived `specs/` files land without a `clm rebase` round. When the head adds or applies a change, a file under `specs/` changed on both sides, and every conflict the merge reports lies under `specs/`, `clm land` regenerates the merged `specs/` itself, as `clm rebase` would, and lands that. It first proves the head's own `specs/` derived on its merge base, so a hand edit there is still refused, and it runs the gate on the merge commit for such a head, where `gate/up-to-date` can't fire.

## Problem

During a triage session several drafts land spec branches at once, and each regenerates the same `specs/<component>/SPEC.md` and the root `specs/SPEC.md`. A land whose check runs for minutes (the readers of `specs/`) finds `main` moved by a faster land when it pushes, and its next attempt meets `gate/up-to-date: specs/plugin/SPEC.md changed on both this branch and origin/main`. `clm rebase`, `clm branch push` and `clm land` again lost the same race four times in a row. Each round costs a full check, and nothing queues or retries. `derived-specs-fingerprint` had rejected regenerating at land, because the landed tree would differ from the head the gate rated; under concurrent spec lands that refusal starves them.

## Approaches

- **Regenerate at land (chosen, `land-regenerates-behind-specs`).** `clm land` already regenerates `specs/` to check them (`land/derived-specs`); for a head behind in `specs/` it lands the regenerated tree instead of comparing. The land's own retry loop then carries a land through a moved `main` with no session round. It reuses `sync.regenerate`'s `regenerated` as it stands.
- **Check the up-to-date rule before the long check.** Rejected: it already runs first on each attempt; the race is lost during the check.
- **One lock across every land.** Rejected: lands from other clones and machines don't share it, and a quick spec land would wait behind another's tests.
- **Run `clm rebase` on the branch from inside `clm land`.** Rejected: it would commit to and push the author's branch from the lander, and `clm rebase` stops for a session to resolve what it reports.

## Outcome

- `land/regenerates-behind-specs` (added): the condition, the two regenerates, the gate on the merge commit, and what stays refused.
- `land/land` (modified): the gate runs on the merge commit for such a head, and a merge conflict refuses only outside what that requirement regenerates.
- `land`'s Design: steps 2 and 3 point to a new **Behind in `specs/`** paragraph.
- The decision `land-regenerates-behind-specs`; `derived-specs-fingerprint`'s rejected alternative notes that it was taken later for this case.

## Constraints

- What stays refused: a conflict outside `specs/`, a delta stale against `main` (its stale-delta merge is authored text, so `clm rebase` and possibly a fresh approval), and a head that adds or applies no change, whose `specs/` edits are authored.
- The approval stands: the fingerprint skips `specs/` on a head that adds a change; on one that only applies a change it holds the head's `specs/`, which the first regenerate proves derived.
- `gate` is unchanged: a session's own `clm gate --base origin/main` still reports `gate/up-to-date` for such a branch, and `clm rebase` still clears it.

## Edge cases and workarounds

- The merged tree `git merge-tree` writes can hold conflict markers under `specs/`; the regenerate reads `specs/` only from `onto`, so they don't matter, and they are replaced.
- `git merge-tree --write-tree` exits 1 on a conflict but still prints the tree; the land reads it, and the conflicted paths from `--name-only`, through `clm-git` rather than dropping a failed run's output.
- `derived.rs` reads the applied changes against the fetched base and the merged checkout; the head-side regenerate needs the same reading against the merge base and the head's checkout.
- A land can still lose the push to a faster one; its next attempt regenerates again, within the ten attempts `land` allows.

## Open questions

None.
