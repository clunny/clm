---
status: applied
resolves: [the-gate-s-placement-rule-skips-a-behaviour-sentence-that-holds]
---

## Intent

A sentence counts as one that only places a test when it holds a placement phrase (`SHALL sit`, `sat in`, `is left in`) and also the word `test` or `tests`. A sentence of behaviour that holds a phrase and no such word, such as one deleting a directory "once nothing else is left in it", is then counted by `gate/cuts`, so losing it is listed as a cut and needs the human's yes (`removal-needs-a-yes`), and `clm health`'s `test-placement` signal no longer reports it.

## Problem

`gate/cuts` leaves out of its count of a requirement's stating sentences any sentence holding `SHALL sit`, `sat in` or `is left in`, as one that only places a test (`cuts-are-what-a-program-lists`). Those phrases also appear in sentences of behaviour. Wave 1 of phase 5 met one ("once nothing else is left in it"), and `gate/cuts` holds another in its own statement: the sentence that defines a stating sentence names the three phrases. A branch that dropped either sentence would have the gate list nothing, so the yes the removal needs would rest on the session alone. `clm health`'s `test-placement` signal calls `places_a_test` with a requirement's whole statement joined, so it reports such a requirement as placing a test.

## Approaches

- **Taken: a placement phrase counts only beside the word `test` or `tests`.** Every placement sentence in `specs/` today names the tests it places ("Every test of `gate`'s requirements that sat in ...", "The tests of `clm guide` among them SHALL sit in ..."), and neither behaviour sentence above holds the word. The rule stays a pure function of one sentence, and it narrows what goes uncounted, so it can only list more cuts, never fewer. A sentence of behaviour that holds both a phrase and the word stays a reader's, as the decision already says.
- **Taken: `test-placement` reads sentence by sentence.** `rule-signals` already says "a sentence `places_a_test` accepts"; with the word required, a statement read whole would match a phrase in one sentence and `test` in another. `clm-gate` offers `sentences(statement)`, the split `stating_sentences` already makes, so the signal and the gate split alike.
- **Not taken: count every sentence and have the change declare its placement ones.** It needs a new declaration format and makes every branch that cuts a placement sentence need review, which the human agreed on 2026-11-18 it doesn't.
- **Not taken: accept the behaviour as a reader's.** The decision lists "a sentence of behaviour worded like one that places a test" as a reader's, but a check that a cheap rule can make stronger shouldn't be left to judgement (`a passing check must mean the code does what the spec says`).

## Outcome

- `gate/cuts`: *places a test* is defined; a *stating sentence* holds `SHALL` and places no test; `clm-gate` offers `sentences(statement)` beside `stating_sentences` and `places_a_test`, which now returns a phrase only for a sentence that also holds `test` or `tests`. New scenario "a sentence of behaviour that holds a phrase". The Contract lists `sentences`.
- `check.health/rule-signals`: `test-placement` splits a statement with `sentences` and asks `places_a_test` of each. New scenario "a phrase in a sentence of behaviour". The Design names `sentences` among what it takes from `gate`.
- `docs/decisions/cuts-are-what-a-program-lists.md` is amended to the narrower rule.

## Constraints

- `sentences`, like `stating_sentences` and `places_a_test`, builds for `wasm32-unknown-unknown`, so the `web` Worker could list the same cuts.
- `check.health/rule-signals` already needs `gate/cuts`, so its build waits for the gate's.

## Edge cases and workarounds

- A sentence of behaviour that holds a placement phrase and the word `test` or `tests` (say, "a test run is left in the queue") is still left out of the count; that residue stays a reader's, as the decision says.

## What the requirements no longer state

Nothing: no sentence holding `SHALL` is removed.

## Open questions

None.
