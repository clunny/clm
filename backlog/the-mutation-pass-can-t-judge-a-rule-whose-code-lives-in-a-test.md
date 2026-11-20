---
title: "The mutation pass can't judge a rule whose code lives in a test file"
kind: bug
components: [ci]
---

The pass filed `vcs/one-git-layer` as unproven (`backlog/unproven-vcs-75853921`) after mutating `crates/clm-git/src/`, which its tests never run: the requirement's rule is the scan in `crates/clm-git/tests/scan.rs`, and cargo-mutants never mutates a test target. No test change could kill those mutants, so the item asked for work that can't be done. Mutating the scan by hand (21 mutants in the vcs build of 2026-11-19) killed every one. The build added `clm:reads` markers for the crates the workspace test reads, which leaves the next pass `not-judged`, so a later weakening of those tests goes unseen. The pass should tell a requirement whose rule is code in a test file from an empty test, or mutate that file, rather than file it as unproven.
