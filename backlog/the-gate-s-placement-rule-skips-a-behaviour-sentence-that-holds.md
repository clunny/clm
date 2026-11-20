---
title: "The gate's placement rule skips a behaviour sentence that holds the words is left in"
kind: bug
components: [gate, check.health]
origin: 20261119-wave-1-by-the-rule
---

`gate/cuts` leaves out of its count any sentence holding `SHALL sit`, `sat in` or `is left in`, as a sentence that only places a test (`cuts-are-what-a-program-lists`), and `clm health`'s `test-placement` signal reports the same phrases. A requirement's sentence about behaviour can hold one of them: one states that a directory is deleted "once nothing else is left in it". Dropping that sentence would lose behaviour and the gate would list nothing, so `removal-needs-a-yes` is unchecked for it, and the health signal reports it as a placement sentence.

Expected: the phrases match only where they place a test, for example together with the word `test`, or the gate counts every sentence and the change declares the placement ones.

Found in wave 1 of phase 5 (2026-11-19) by the pass's reader; the sentence was kept and reworded, so nothing was lost.
