---
title: "Tests written before the mutation pass were never judged by it"
kind: idea
components: [verify, ci]
origin: 20261114-proof-is-real
---

## Intent

`verify/mutation-pass` judges a requirement when one of its citing tests is added or changed. The 4,400 tests that existed when it was built are judged only if they change, and a requirement that is only re-cited is never judged, so an empty test written before the pass still reads as proof.

A baseline would judge every requirement once. On 2026-11-14 a mutant cost about 1 second with a crate's own tests and 10 to 18 seconds with the root's CLI tests, which puts a baseline of 848 requirements at 50 to 80 machine-hours on one core.

## Open questions

- Is a one-time run worth that, or should it wait until each crate proves its requirements through its own entry point (phase 4 of `backlog/a-passing-check-proves-less-than-the-spec-claims`), which makes each mutant about ten times cheaper?
- A sample of 17 mutants of one crate was noticed by the tests of seven of its ten requirements often and by three almost never, all sound. A baseline has no diff to mutate first, so what verdict can it give a requirement whose code is a small part of its component?
