---
title: "check_lock test a_land_with_the_callers_target fails when other builds' checks run"
kind: bug
components: [land]
---

clm-land::check_lock a_land_with_the_callers_target failed once in `clm land --check-only` with "waiting for another land's check ... 1 ahead" and event order ["start", "end"] mismatching its expectation, while other builds were running checks on the host. It passed on an immediate rerun. The test seems to depend on no other check holding the lock (tests-run-concurrently).
