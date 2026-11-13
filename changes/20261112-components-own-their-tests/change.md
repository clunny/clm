---
status: proposed
resolves: [a-build-has-no-way-to-give-its-component-a-new-test-file, compile-coupled-changes-across-components]
---

## Intent

Make `path-ownership` hold again without a growing list of exceptions: each component owns the tests that prove it, reaching the composition root through a test-only dependency, and a change that ties components together at compile time is built by one build that claims all of them. Draft in progress.
