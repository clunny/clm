---
title: "A build has no way to give its component a new test file, so its tests land in another component's files"
kind: idea
components: [orchestrator.run, check, plugin]
---

## Intent

In the first CLM project on Azure DevOps, every build that needed a new test file found `check/unowned-files` rejecting it, because its component's `paths` listed existing files rather than a directory glob. Each build instead added its tests to files a test-harness component owned and listed that under "Fixes in other components". Over one `/clm:build` session the proofs for five components ended up in the harness component's files. The build prompt gives a build no route to a new file of its own, so the method steers every build to the same cross-component placement.

Reported from that project's session on 2026-11-10.

## Open questions

- Does the init skill draft `paths` as file lists when it recovers an existing project, and should it prefer directory globs (or a declared test glob per component) so new files are owned from the start?
- Should a build be allowed to add a file under its component's own directory without a spec change, and if so, what keeps that from widening a component unseen?
- Or should the build prompt tell builds up front to ask for a new file through a spec branch, and build the rest meanwhile (`requirement-blocks`)?
