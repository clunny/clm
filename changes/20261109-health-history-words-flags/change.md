---
status: applied
from: backlog/health-history-words-flags-no-longer-when-it-describes-a-present
resolves:
  - health-history-words-flags-no-longer-when-it-describes-a-present
---

## Intent

Stop `clm health`'s `history-words` signal flagging lines that hold "no longer" in a present-tense condition. The phrase leaves the crate's constant list of history phrases, `check.health`'s Design says why, and `check.health/spec-bloat-signals` gains a scenario in which a Design line stating a present condition with "no longer" gives no `history-words` line.

## Problem

`history-words` matches its phrases at word boundaries, with no reading of the sentence. "no longer" is common in Designs as a condition on the present: a local branch "that no longer contains" its remote tip, "once that component no longer uses it", "a tree the session no longer has". The spec-slimmer that `/clm:health` launched on such a line reported it as a false positive. The signal advises (`health-advises`), so every misfire costs the reader's time and invites a cut to text that is correct and current (`spec-is-current`).

## Approaches

1. **Drop "no longer" from the list.** Chosen. The other phrases ("used to", "as before", "formerly", "previously", "was removed", "is gone", ...) still catch a line that narrates a change, and a date still does. A sentence such as "the old flag no longer runs" goes unflagged; it is rarer than the conditions above, and the slimmer reads the whole spec anyway when a component is flagged for other reasons.
2. **Match "no longer" only in forms that narrate change** (at the start of a clause about the spec or the code). Rejected (`yagni`): telling a clause's subject apart takes a grammar the crate doesn't have, any heuristic would misfire in new ways, and the signal only advises.
3. **Keep the phrase and let the slimmer judge.** Rejected: it is the misfire the item reports, and it repeats on every run.

## Outcome

- `check.health`'s Design: the fenced phrase list loses `no longer`, and a sentence states the rule for a phrase on the list (it narrates a change wherever it appears) with "no longer" as the example kept off.
- `check.health/spec-bloat-signals`: a new scenario, *a present condition*. Its text is otherwise unchanged; the existing scenario *history in a design* still holds, since its line's first match is the date.
- Code (built by `check.health`): remove `"no longer"` from `HISTORY_PHRASES` in `crates/clm-health/src/spec.rs`, and add a test for the new scenario citing the requirement at its new version.

## Constraints

- No edge cases or workarounds are known.
- No other component reads the phrase list; `plugin`'s `health` skill names the signal only.

## Open questions

None.
