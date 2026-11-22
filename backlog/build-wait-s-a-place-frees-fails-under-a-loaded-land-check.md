---
title: "build_wait's a_place_frees fails under a loaded land check"
kind: bug
components: [orchestrator.loop]
---

`crates/clm-loop/tests/build_wait.rs::a_place_frees` failed in `clm land --check-only` of `impl/web.worker.review` (a branch that touches only `crates/clm-web-review`), panicking at line 50 with `why` = `work is ready: build:a` instead of a reason naming `place b freed`. Run alone three times right after, it passed each time (about 5 s each). The test sleeps a fixed 700 ms and then frees the place, so under the land check's load (other builds' tests running beside it, `tests-run-concurrently`) `wait` can wake for the ready work before the place-freed event is seen, and the land check of an unrelated branch fails. The test should not depend on wall-clock ordering between the free and the wake.
