---
status: proposed
resolves:
  - a-land-check-that-builds-nothing-still-waits-for-one-that-does
  - only-cargo-projects-get-a-shared-land-build-and-its-lock
---

## Intent

A land whose code is already checked, and that only has to run the tests reading the files it changes, shouldn't wait behind another branch's full build and test run; and the shared land build place and its lock should serve any toolchain a project declares, not only cargo. Design in progress.
