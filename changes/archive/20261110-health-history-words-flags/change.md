---
status: done
from: backlog/health-history-words-flags-passive-used-to
resolves:
  - health-history-words-flags-passive-used-to
---

## Intent

Stop `clm health`'s `history-words` signal flagging lines that use "used to" as a passive ("is used to", "may be used to"). The phrase leaves the crate's constant list of history phrases, `check.health`'s Design names it beside "no longer" as a phrase kept off the list and says why, and `check.health/spec-bloat-signals` gains a scenario in which a Design line with a passive "used to" gives no `history-words` line.

## Problem

`history-words` matches its phrases at word boundaries, with no reading of the sentence. "used to" is on the list, but in "is used to", "are used to" or "may be used to" it means "employed for" and describes the present system. Such lines are flagged anyway, and `/clm:health` sends the spec-slimmer after text that is correct and current (`spec-is-current`). The signal advises (`health-advises`), so every misfire costs the reader's time and invites a wrong cut. The Design's own rule admits a phrase only when it narrates a change wherever it appears, which "used to" does not.

## Approaches

1. **Drop "used to" from the list.** Chosen, as the earlier change `20261109-health-history-words-flags` did for "no longer". The other phrases ("as before", "formerly", "previously", "was removed", "is gone", ...) and a date still catch most lines that narrate a change. A line such as "the flag used to run first" now goes unflagged unless it holds another phrase; that is accepted, as it was for "no longer": it is rarer than the passive, and the slimmer reads the whole spec when a component is flagged for any other reason.
2. **Match "used to" only when not preceded by a form of "be"** ("is", "are", "was", "be", "been"). Rejected (`yagni`): the matcher reads no grammar, and the heuristic misfires in new ways ("it was used to it", "the value it used to hold is used to ..."); the earlier change rejected the same kind of heuristic for "no longer".
3. **Keep the phrase and let the slimmer judge.** Rejected: it is the misfire the item reports, and it repeats on every run.

## Outcome

- `check.health`'s Design: the fenced phrase list loses `used to`, and the sentence stating the list's rule names "used to" beside "no longer" as kept off, with the reason (a passive describes the present; a phrase that is history only in some forms is dropped, not parsed).
- `check.health/spec-bloat-signals`: a new scenario, *a passive used to*. The requirement's text is otherwise unchanged, and its other scenarios still hold.
- Code (built by `check.health`): remove `"used to"` from `HISTORY_PHRASES` in `crates/clm-health/src/spec.rs` (the array's length drops to 8), and add a test for the new scenario citing the requirement at its new version; re-cite the requirement's other tests at the new version.

## Constraints

- No edge cases or workarounds are known.
- No other component reads the phrase list; `plugin`'s `health` skill names the signal only.

## Open questions

None.
