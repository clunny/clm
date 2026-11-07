---
title: "Parallel builds' test runs collide on fixed host resources the project's tests use"
kind: bug
components: [plugin, init, orchestrator.run]
issues: [266]
---

With several builds on one clone, a project whose check starts a preview server on a fixed port failed with "port already used", even though clm 0.20.0 already serializes every `[land] check` (`land/check-lock`). Builds also run the project's tests outside that lock, on every edit and in spot-checks, and serializing those would undo parallel builds. Nothing tells a CLM-built project that its tests must run concurrently on one host, and nothing gives a build a value to keep its ports and paths apart.

Expected: the method states, for every CLM-built project, that its tests can run concurrently on one host, with no fixed ports, paths or shared servers, and says so where an adopter sets up and builds. Each build and its checks get a distinct value derived from its build place to key such resources on. `check-runs-one-at-a-time` is amended: the lock stays for land checks, and the rejected per-run slot becomes the rule.
