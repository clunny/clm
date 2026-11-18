---
title: "clm-verify's mutate tests fail under a full check and pass alone"
kind: bug
components: [verify]
---

Step: `clm land --check-only` on `impl/land` (slot 2), with other builds' checks running on the host (`tests-run-concurrently`).

What happened: in the whole-workspace nextest run, three tests of `crates/clm-verify/tests/mutate.rs` failed together: `a_left_requirement_no_longer_cited_is_dropped` (line 556, `lists(&p.f).iter().all(Vec::is_empty)` false), `a_new_test_that_notices` (line 152, `[]` where `["a/thing"]` was expected) and `a_line_cut_short` (line 437, the same `[]` against `["a/thing"]`). Rerun alone with `CLM_SLOT=2 cargo nextest run -p clm-verify --test mutate`, all 27 passed. The branch touched no file of `verify`.

Expected: the mutate tests pass however loaded the host is and whatever else runs beside them; whatever they share (a path, a port, the environment, or a timing assumption in the fake mutation run) is keyed on `CLM_SLOT` or the fixture's own directory, or the wait has a deadline instead of a race.
