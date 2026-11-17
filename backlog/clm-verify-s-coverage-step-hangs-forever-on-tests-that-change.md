---
title: "clm verify's coverage step hangs forever on tests that change the process environment"
kind: bug
components: [orchestrator.checkpoint, land, ci]
---

What happened: a build loop ran `clm verify --due` as the loop's `run` action. Its mutation step runs the coverage script, which runs the workspace tests through `cargo llvm-cov`, that is plain `cargo test`, with each test binary's tests on threads of one process. One test binary never finished: after 1h40m with no output, about 20 child processes were spinning at ~68% CPU each (load ~30 on 24 cores), and the verify had run 4h40m in all. The human had to ask why, and the session killed the process tree by hand; verify then reported "mutation ... failed with status 101; nothing recorded".

Cause: test helpers that set a process-wide environment variable (PATH) for one test, documented as safe on the assumption "one test per process". That holds under a one-process-per-test runner but not under `cargo test`, which the coverage step uses. Parallel tests stacked each other's fake-`git` directories onto PATH; each fake finds the real `git` by stripping only its own directory from the front of PATH, so with another test's directory first, two fakes exec each other forever. The same helper and assumption exist in at least the checkpoint and land command test files, and other test files set PATH too.

Expected: verify's steps finish or fail in bounded time. Two gaps: (1) tests that set process-wide environment must hold a shared lock or pass the environment to the command under test, not rely on one test per process; (2) the coverage/mutation step has no timeout, so a hang is never reported, and the loop then offers to re-run the same verify (`--event work-done`), which would hang again.
