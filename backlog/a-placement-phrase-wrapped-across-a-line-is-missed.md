---
title: "A placement phrase wrapped across a line break is never found"
kind: bug
components: [gate]
---

Step: building `check.health/rule-signals@015eed9a`. The code review found this; it was already true before the build.

What happens: `placement_phrase` in `crates/clm-gate/src/cuts.rs` looks for each phrase of `PLACEMENT_PHRASES` (`SHALL sit`, `sat in`, `is left in`) with a single space between its words. A statement whose lines are joined with `\n` and wrapped inside a phrase, such as `Every test of a's requirements SHALL\nsit in a's files.`, holds `SHALL\nsit`, so no phrase is found. `gate/cuts` then counts that sentence as one that states something, and if a later branch removes it, the gate lists a `Shortened` cut and asks for review of a sentence that only places a test. `clm health`'s `test-placement` signal, which calls `places_a_test`, misses the same sentence.

Fix: treat any run of whitespace between a phrase's words as one space before matching, in `placement_phrase` (and in `places_a_test`'s check for `test`/`tests` if needed), with a test of a wrapped phrase.
