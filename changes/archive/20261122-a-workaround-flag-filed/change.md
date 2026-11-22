---
status: done
resolves: [a-workaround-flag-filed-in-error-can-only-be-withdrawn-by]
---

## Intent

The decision `workaround-flags` states how a flag filed in error is withdrawn, and the bug `a-workaround-flag-filed-in-error-can-only-be-withdrawn-by` is deleted. No requirement, scenario or Design changes, and nothing is built.

## Problem

`clm workaround` only files a flag, and the decision named no way to take back one whose trigger never happened. A session that filed one by mistake deleted its file by hand on a branch and, finding the route nowhere stated, reported it as a defect.

## Approaches

- **State the routes that already exist (taken).** Deleting the file is the right act; what was missing is who may do it, and when. Both routes already work and are already partly written: `clm run`'s prompt (`orchestrator.run/prompt-workaround`) has a build delete its own flag file on its branch when it re-records it, and says flags already on `main` aren't the build's to edit; the `triage` skill's step 6a offers the human a `workaround` line to draft, keep or drop, a dropped one becoming a backlog-only change that deletes it. The decision now says so in one paragraph: a flag only on the filing build's branch is deleted there before the land; a flag on `main` is withdrawn only by the human dropping it at triage, since a session deleting a landed flag would lower the count `redesign-before-workaround` stops a build on; a flag that was true is settled by a `decided` record (`effort/unsettled-flags`), never withdrawn.
- **A `clm workaround --withdraw` command (rejected).** It would only delete a file, which git already does, and it would give a session a way to take back a landed flag the human hasn't seen, the evidence `redesign-before-workaround` counts (`yagni`). The decision's Rejected list now names it.

## Outcome

`docs/decisions/workaround-flags.md` gains the paragraph above and the rejected command; the item is deleted. Nothing in `specs/` changes, so no component becomes pending.

## Constraints

- `redesign-before-workaround`: a flag on `main` is evidence for the human's threshold, so only the human removes it.
- `yagni`, `reuse-first`: no new command where deleting a file and triage's drop already do the job.
