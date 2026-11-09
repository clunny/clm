---
title: "health/history-words flags 'no longer' when it describes a present state"
kind: bug
components: [check.health]
origin: 20261108-plugin-skill-children
---

Step: `clm health`, the `history-words` signal, read through `/clm:health`.

What happened: the signal flagged a Design line because it holds the phrase "no longer", although the sentence describes a present state rather than narrating history: a local branch "that no longer contains" its remote tip. The spec-slimmer that read the line reported it as a false positive, and other "no longer" phrases in the same spec ("once that component no longer uses it", "a tree the session no longer has") describe conditions too.

Expected: `history-words` reports lines that narrate how the system changed (`spec-is-current`), not every line holding the phrase. "no longer" is common in present-tense conditions, so a plain phrase match on it misfires; either the phrase leaves the list, or it is matched only in forms that narrate change (such as "no longer" at the start of a clause about the spec or the code).

Rule involved: `check.health`'s `history-words` signal and its constant phrase list; `health-advises` (the signals advise, so a misfire costs reader time and invites wrong cuts).
