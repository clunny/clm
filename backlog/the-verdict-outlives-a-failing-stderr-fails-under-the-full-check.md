---
title: "the_verdict_outlives_a_failing_stderr fails under the full check run but passes alone"
kind: bug
components: [verify]
---

`clm-verify::run the_verdict_outlives_a_failing_stderr` (crates/clm-verify/tests/run.rs:301) failed in the full `clm land --check-only` run of impl/config (4541 of 4542 passed; other builds ran tests on the host at the same time), and passed three times in a row run alone with CLM_SLOT=1. The test re-runs itself in a child whose stderr is `/dev/full`, so the child's panic message is lost: the parent's assertion shows only "0 passed; 1 failed", and the cause can't be seen. It may be timing under load or a resource shared with a concurrent run (`tests-run-concurrently`). The test should make the child's failure reason visible (e.g. write it to stdout or a file) so the next failure can be diagnosed, and then fix whatever it shows.
