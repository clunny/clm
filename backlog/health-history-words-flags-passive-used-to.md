---
title: "health/history-words flags passive 'used to' (\"may be used to\") as history"
kind: bug
components: [check.health]
issues: [493]
---

`clm health` reports a `history-words` line for any Contract or Design line that contains "used to", because the phrase is on the crate's history-phrase list. In "is used to", "are used to" or "may be used to" the phrase is a passive meaning "employed for" and describes the present system. These lines get flagged anyway, so `/clm:health` sends the spec-slimmer after text that is correct.

Expected: a line whose only match is the passive form gets no `history-words` line, and a line saying something "used to" behave a different way is still flagged or caught by another phrase. The Design's own rule admits a phrase to the list only when it narrates a change wherever it appears, which "used to" does not. The earlier "no longer" fix (`changes/archive/20261109-health-history-words-flags/`) dropped its phrase rather than parsing the sentence, and rejected a heuristic as `yagni`. The same choice applies here: probably drop "used to" from the list, and add a scenario to `check.health/spec-bloat-signals` with a passive "used to" line that yields no `history-words` line.
