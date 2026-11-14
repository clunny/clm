---
status: proposed
resolves: [the-written-exception-to-path-ownership-lets-a-build-re-cite-a]
---

## Intent

Make the written exceptions to `path-ownership` agree with `removed-requirements-build-out`, so a build knows whether it may remove a test that cites its own removed requirement in a file another component holds.
