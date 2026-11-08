---
title: "clm land regenerates the specs/ of a spec branch that is behind main in them"
date: 2026-11-07
status: accepted
---

`derived-specs-fingerprint` considered regenerating `specs/` at land instead of refusing, and rejected it: the landed tree would differ from the head the gate rated and the author pushed, and `gate/up-to-date` already sends the author to `clm rebase`. That held while spec lands were rare. With a triage session landing several drafts' spec branches at once, it fails in practice. Each draft regenerates the same `specs/<component>/SPEC.md` and the root `specs/SPEC.md`. A land whose check runs for minutes (the readers of `specs/`, `doc-readers-marked`) finds `main` moved by a faster land when it pushes, and its next attempt meets `gate/up-to-date`. `clm rebase`, `clm branch push` and `clm land` again lost the same race four times in a row: each round cost a full check, and nothing queued (`backlog/concurrent-spec-lands-starve-each-other-on-gate-up-to-date-for`).

So `clm land` regenerates them itself (`land/regenerates-behind-specs`). When the head adds or applies a change, `specs/` changed on both sides, and every conflict the merge reports lies under `specs/`, the land does what `clm rebase` would have done, in its scratch tree, and lands the result. Both objections are met:

- **What the gate rated.** The gate on the head would report `gate/up-to-date`, so for such a head the land runs that gate on the merge commit instead. Step 5 already gates every land's merge commit, since a file both sides touched can change whether it needs review, and its verdict is the one the `landed` record states.
- **What the author pushed.** What lands under `specs/` differs from the head's only by what `main` brought, regenerated from sources `main` already holds (`regenerate-dont-merge`). The fingerprint skips `specs/` on a head that adds a change, so the approval stands. To keep `land/derived-specs`' guard against a hand edit under `specs/`, the land first regenerates the head's own `specs/` onto its merge base and refuses a difference, as it refuses one on a head that isn't behind.

What stays refused: a conflict outside `specs/`, a delta stale against `main` (its regenerate stops, naming `clm rebase`, whose stale-delta merge is authored text a human may need to approve), and a head that adds or applies no change, whose `specs/` edits are authored, not derived.

What it costs: two regenerates on such a land instead of one comparison, a few seconds. A land can still lose the push to a faster one, but its next attempt now regenerates and retries on its own, up to ten times, instead of stopping.

Alternatives considered:
- **Run the gate's up-to-date rule before the long check.** It already runs first, on each attempt. The race is lost during the check, so checking earlier changes nothing.
- **Hold one lock across every land of a clone.** Lands from other clones and machines don't share it, and a spec land with nothing to check would wait minutes behind another's tests.
- **Retry inside `clm land` by running `clm rebase` on the branch.** It would commit to the author's branch and push it from the lander, and `clm rebase` stops for a session to resolve what it reports.
