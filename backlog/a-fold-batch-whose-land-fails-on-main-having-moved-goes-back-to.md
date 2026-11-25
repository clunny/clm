---
title: "A fold batch whose land fails on main having moved goes back to the human with no rebase run"
kind: bug
components: [orchestrator.loop]
---

Step: the build loop's `run land:<batch>` action for an approved fold batch, then `clm build returned` with the land's last line.

What happened: between the batch's fold and its land, a component build landed a workaround flag on a requirement the batch modifies. The batch's own `clm check` and gate had passed, but `clm land`'s check of the merge with main failed (`this_repository_is_clean`: the flag names the requirement's old version, "re-record the flag or delete it"). The loop answered with a `say` that the land failed and the batch "is back in your queue". Nothing in the queue says what to do: the fix is a `clm rebase` on the batch branch (which re-records or drops the stale flag), after which the diff's fingerprint changes and the human must approve again. No run is launched to do the rebase, so the batch waits on the human for work that is a session's.

Expected: a failed fold land whose cause is the merge with main gets a fold (or rebase) run that brings the batch branch up to date and resolves what `clm rebase` reports, and only then asks the human for the fresh approval the new fingerprint needs.

Rule involved: `clm land` merges main and checks the result; an approval holds only for an unchanged fingerprint; the loop decides every launch (`build-loop-in-clm`).
