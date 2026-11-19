---
title: "clm verify's coverage step fails because .clm/verify-build doesn't exist"
kind: bug
components: [verify]
---

Step: the build loop printed `run verify clm verify --due --event work-done`, and the session ran it.

What happened: every test suite passed (4791 Rust tests and both vitest suites). Then the coverage step's `cp target/lcov.info "$CLM_LAND_DIR/lcov.info"` failed with "No such file or directory". `clm verify` sets CLM_LAND_DIR to `.clm/verify-build`, and that directory didn't exist. The run then printed `clm verify: mutation: .github/scripts/mutate.sh failed with status 1` and, as its last line, `clm verify: mutation: 9 killed, 0 unproven, 0 not judged`. It exited 0.

Expected: `clm verify` creates the build place it names in CLM_LAND_DIR before running the check (`verify-build-dir`), or the check creates it, so the lcov copy that `clm health` reads (`health-lcov-here`) is written. A step failing on a missing directory shouldn't leave a verify run with a mutation failure and exit 0 whose last line says everything was killed.
