---
title: "A failed land check's refusal shows a tail of output that may not hold the reason"
kind: bug
issues: [262]
components: [land]
origin: 20261107-slow-tests-after-land
---

Step: `clm land` when `[land] check` fails, in a project whose check runs several steps (install, unit tests, a browser end-to-end suite).

What happened: the refusal printed the last 40 lines of the check's output, which were cut off before the failing step's own message, so the reason wasn't shown. The session reran the suite separately to tell a flake from a real failure. Reported from another project as part of GitHub issue #262.

Expected: the refusal names the step that failed and its summary, or points the session at where in the kept log the failure is, rather than a fixed tail.

Rule involved: `land-check-log`.
