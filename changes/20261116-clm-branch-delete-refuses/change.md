---
status: applied
resolves: [clm-branch-delete-refuses-a-pushed-spec-branch-after-the-draft]
---

## Intent

`clm branch delete` deletes a branch this clone pushed with `clm branch push` even when another session's fetch stored the pushed tip before the push did. `clm branch push` now records each tip it pushes in a local ref, `refs/clm/pushed/<branch>`, and `clm branch delete` takes either that record or the remote-tracking reflog's `update by push` as proof the tip is this clone's. A draft's cleanup then removes its stub `spec/<id>` from `origin`, so no empty change holds its items.

## Problem

The item blamed the order of the draft prompt's cleanup (local branch first, then `origin`'s). That isn't the cause: deleting a local branch leaves the remote-tracking ref's reflog alone, and a fresh repository with that order deletes both. The run that filed it (`spec/20261115-a-verify-test-fails`, pushed at 05:56:07Z on 2026-11-16) shows the cause in the clone's reflog for `refs/remotes/origin/spec/20261115-a-verify-test-fails`: its one entry is `0000… → 949fa39f fetch origin: storing head`, written at 05:56:10Z, three seconds after the push. Another session's fetch, in a worktree of the same clone, listed `origin` after the push landed there and stored the tip before the push moved the clone's remote-tracking ref; the push's own update was then to the value the ref already held, and git writes no reflog entry for that. So the newest entry named a fetch, and `branch-delete` refused, as written. Reproduced on 2026-11-16: with the remote-tracking ref already at the pushed tip, `git push` writes no entry, `git update-ref` to the same value writes none either, and `clm branch delete` exits 1. Every CLM session of a clone shares its remote-tracking refs and fetches often, so the race is the design's, not the draft's.

## Approaches

- **A record `clm branch push` writes, beside the reflog (taken).** `refs/clm/pushed/<branch>` at the pushed tip, written only after a push that moved `origin`'s branch (not one `git push --porcelain` flags `=`, already up to date, whose tip may be another clone's), and never touched by a fetch. `branch delete` accepts the record or the reflog, so nothing that deletes today stops deleting, and a tip another clone pushed later differs from the record and is still refused. A ref, like the checkpoint's review mark, because it names a commit and is shared by the clone's worktrees; nothing lands, approves or passes a check on it.
- **Reorder the draft prompt's cleanup.** Not taken: the order isn't the cause, as above.
- **Write the missing reflog entry after the push.** Not taken: git writes no entry for an update to the value a ref holds, and deleting and recreating the ref to force one races the same fetches.
- **Drop the ownership check, or accept a fetch entry.** Not taken: a fetch entry can't tell this clone's push from another clone's, which is what the check is for (`lease-claims`).
- **A record every push path writes (`clm claim`, `clm autosync`), in `vcs`.** Not taken now (`yagni`): it would change three more components for a race no run has hit on those paths. A claimed branch is pushed by `clm branch push` once its build pushes, which writes the record.

## Outcome

- `host`: `branch-push` sets `refs/clm/pushed/<branch>` when its push moved `origin`'s branch, and leaves it as it was when the push moved nothing or was refused; `branch-delete` takes the record or the reflog as proof and removes the record once `origin` has no such branch. The Design says why.

## Edge cases and workarounds

- `clm claim`'s first push of `impl/<component>` and `clm autosync`'s pushes of `autosync/spec/<id>` keep the reflog alone. A fetch winning the same race there leaves the branch on `origin` until a human deletes it; with the reflog's fallback that is the same as today. If a run meets it, a later change moves the record into a `vcs` helper those paths call.
- `clm land`'s delete of a landed branch goes through the library operation and leaves the branch's record: a ref per landed branch, naming a commit `main` holds. Accepted: it costs nothing, and a later `clm branch delete` of that name removes it.
- A clone that loses its record (a deleted ref) falls back to the reflog, as today.

## Builder check and audit

- The builder check found no requirement either modified one relies on, no code work outside them, and no capability test that sees a difference (`crates/clm/tests/capability/git_host.rs` pushes with plain `git`). Settled from its questions: how a test makes the race's state (store the tip with `git update-ref` before the push, now in the Design), that a refused push leaves the record, and land's records (above).
- The architecture audit found that writing the record when `origin` already held the commit would let a session re-push another clone's fetched tip and then delete it, against `lease-claims`. Fixed: only a push that moved `origin`'s branch writes the record, with a scenario (`another clone's tip pushed again`). It also found that `docs/decisions/unattended-steps-run-clm.md` described the reflog alone; its sentence now names the record too.

## Constraints

- `lease-claims`: the record never makes another clone's tip deletable, since a later tip differs from it.
- `git-is-truth`: the record is a clone's own cache, derived from its own pushes, like `refs/clm/tiers-verified`.
- `unattended-steps-run-clm`: no prompt changes; the draft cleanup's order stays.
