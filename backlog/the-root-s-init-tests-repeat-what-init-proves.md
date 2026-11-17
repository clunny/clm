---
title: "The root's init tests repeat what init proves in its own crate"
kind: idea
components: [clm]
---

The root's tests in `crates/clm/tests/init.rs`, which cite only `clm/init-result-accepted` now, still assert everything `clm init` writes. `init` proves that in its own crate since its build of `commands-in-crate`, so each of those assertions exists twice. They could be cut down to the clauses that run another command (`clm check`, `clm hashes`, `clm pending`, `clm setup`, `clm config`). Split out of `doc-readers-still-falls-back-to-the-moved-guide-tests`, whose fix landed without it.
