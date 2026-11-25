---
title: "The mutation pass's StrykerJS sandbox copies the site tests' build directory and fails when a fixture in it is gone"
kind: bug
components: [ci]
---

Step: `clm verify --due` (run by the build loop), in its mutation pass, `.github/scripts/mutate.sh` judging a `web` requirement with StrykerJS.

What happened: StrykerJS failed with ENOENT scanning `site/.test-dist/drift-src/docs` inside its sandbox copy of the `web` npm directory (`.stryker-tmp-clm-<n>/sandbox-<id>/`). `mutate.sh` then reported that StrykerJS failed for the requirement and exited 1, and the pass ended with "26 killed, 0 unproven, 20 not judged, 58 left". `verify` itself recorded passed. `web/site/.test-dist/` is git-ignored output that the site's own tests create and delete (fixture guides and blogs under `.test-dist/<name>-src`), so the sandbox copy picks up whatever the verify tree holds of it at that moment, and a directory removed mid-copy breaks the run.

Expected: the sandbox holds only what the judged tests need, never another suite's transient build output, so a tool failure can't come from files outside the requirement's code and tests; the requirements it would have judged are judged.

Rule involved: `mutate-script` (a tool that fails for any reason but its mutants' outcomes fails the script, leaving the requirements due) and `mutation-after-land`.
