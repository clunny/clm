---
status: proposed
resolves:
  - a-spec-branch-can-t-remove-a-capability-entry-a-test-cites
---

## Intent

A spec branch that removes a `CAPABILITIES.md` entry a capability test cites fails `clm check`, and can't delete the test itself. Let such a branch land, with the test removed by the build, as a removed requirement's tests are.
