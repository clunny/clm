---
title: "One block's StrykerJS failure leaves every other component's requirements unjudged, filed on their own components"
kind: bug
components: [ci, verify]
origin: 20261124-unjudged-web.worker-88470edf
---

In the mutation passes after `bf0d9028` and `88470edf`, `.github/scripts/mutate.sh` failed with `StrykerJS failed for requirement web/<slug>` for five `web` blocks (StrykerJS: "Vitest failed to find test files related to mutated files ... No tests were executed", `vitest.related` finding no test that imports the mutated files, as for tests that read the repository's files). `ci/mutate-script` then starts no further block and exits non-zero, so the pass recorded `0 killed, 0 unproven, 0 not judged, 29 left` (`.clm/verify-run.out` in the main checkout).

Two things go wrong from there:

1. `ci/mutate-script` treats a block-specific StrykerJS failure (no test run for this block's files) like a broken tool, though the fixture already proved the tool works. Since `verify/mutation-pass` puts left requirements first in the next pass, the same failing `web` blocks come first again and every other requirement due stays unjudged pass after pass.
2. `verify/files-unproven` files an `unjudged-<component>-<sha>` item for each component whose requirements were left, naming that component, even when they were left only because another block's tool failure stopped the pass. One cause became seven items (`unjudged-web.worker`, `-web.worker.replica`, `-web.views`, `-model`, `-ci`, `-check.coverage`, `-web`), triage drafted six of them in parallel, and each draft must find on its own that its component holds no fix (`unjudged-web.worker-88470edf`'s components were corrected to `ci` by its draft).
