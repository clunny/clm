Lands hand your check CLM_LAND_DIR and re-check only the overlap
`clm land` now sets `CLM_LAND_DIR` for a check it runs in the clone's land tree, a place that lasts from land to land. If your check builds, point your toolchain's output there in your own check script. For cargo that is `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL=0`, set from `CLM_LAND_DIR` when it is set, since a later `clm` stops setting the cargo variables itself.

A land whose branch's head `clm land --check-only` checked whole now runs only the tests both the branch and what `main` gained since can affect, and none when there are none. Its verdict rests on your tests declaring what they read (`land-checks-stay-fast`, which `clm guide constraints` states): an owner and `depends_on`, `req:` markers, and `clm:reads` markers for the files they read.

Tests that are slow, flaky or need a resource no other run may share belong in `[verify]`, after the land. A failure there is reported with how the culprit's land re-checked, so you can judge whether a fact, the code or the test was wrong.

Taking this note edits no file and leaves nothing to decline: `clm upgrade --apply` stamps `upgraded`, which marks it taken.
