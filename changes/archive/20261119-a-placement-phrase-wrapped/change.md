---
status: done
resolves: [a-placement-phrase-wrapped-across-a-line-is-missed]
---

## Intent

A placement phrase (`SHALL sit`, `sat in`, `is left in`) is found however the statement is wrapped: any run of white space between its words, a line break included, reads as one space. A sentence that places a test across a line break is then left out of `gate/cuts`' count of stating sentences, as it is on one line, and `clm health`'s `test-placement` signal reports it (`backlog/a-placement-phrase-wrapped-across-a-line-is-missed`). Because the fix also makes some diffs need review that didn't, `RULES` becomes 6, and versions 4 and 5 keep the single-space match for deciding review (`audit-by-the-rules-in-force`).

## Problem

`gate/cuts` says the phrases are "matched as whole words separated by single spaces ... so a phrase broken across a line is not found", which `20261119-the-cuts-rules-written-down` wrote down from what the build did. A requirement's statement may span lines, and the sentence split already treats a line break as white space, so `Every test of a's requirements SHALL\nsit in a's files.` is one sentence that holds `SHALL\nsit` and no phrase. Two wrong answers follow:

- A branch that removes such a sentence is listed as `a/x states n sentences where it stated m` and needs review for a sentence the human agreed needs none (`cuts-are-what-a-program-lists`).
- A branch that drops a sentence of behaviour and adds a wrapped placement sentence keeps the count, so no cut is listed and the removal goes unseen (`removal-needs-a-yes`).

`clm health`'s `test-placement` calls `places_a_test` per sentence and misses the same sentence.

## Approaches

- **Taken: read any run of white space between a phrase's words as one space**, in the matching `places_a_test` and the stating-sentence count share. Scenario outcomes already read white space this way (`a-changed-outcome-is-a-cut`), so a reflowed statement is read as a reflowed outcome is: the same.
- **Taken: a new rules version.** The second wrong answer above is a diff the old gate passed on checks and the new one sends to review, so `audit-by-the-rules-in-force` needs `RULES` raised and the older answer kept behind `rules`: versions 4 and 5 count a shortened requirement with the single-space match, version 3 with it and the phrase alone. The cuts listed stay today's at every version, as `rules-version` already says.
- **Not taken: accept it.** This repository writes each statement on one line, so it rarely bites here, but nothing requires that of a project, and the second wrong answer hides a removal from the human.
- **Not taken: unwrap statements in `clm apply`.** It would rewrite the human's text and leave hand-edited and older specs unread the same way.

## Outcome

- `gate/cuts`: the placement phrases match across any run of white space between their words; the clause saying a phrase broken across a line is not found is replaced. New scenario "a sentence that places a test across a line".
- `gate/rules-version`: `RULES` is 6; versions 5 and 6 read paths as `paths-under-the-root` states; under versions 4 and 5 a shortened requirement is counted with the single-space match, and version 3 is defined from version 4. New scenario "a placement phrase across a line under each version".
- `gate`'s Design: the **Cuts** paragraph says a phrase matches across white space; the **Rules versions** paragraph gains version 6.
- `docs/decisions/cuts-are-what-a-program-lists.md` is amended.
- `check.health/rule-signals` is unchanged as written: it already reports each sentence `places_a_test` accepts, so `gate`'s build fixes the signal. No health requirement moves.

## Constraints

- `places_a_test`, `sentences` and `stating_sentences` still run no process and build for `wasm32-unknown-unknown`.
- The older matching is a branch on `rules` beside the rule that changed, not a copy of the gate (`rules-version`'s Design).

## Edge cases

- A phrase's words separated by a tab or by several spaces on one line match too; nothing in the specs writes them so, and treating them alike keeps one rule.
- A sentence whose phrase spans a line and holds `test` only on another line still holds both, since the sentence is the unit, not the line.

## Open questions

- No `check.health` test proves the `test-placement` signal on a wrapped sentence: the signal takes the fix through `clm_gate::places_a_test` with no code of its own changing, and `gate`'s scenario proves the match. A health scenario would make `check.health/rule-signals` pending for a test alone, so none is added.
