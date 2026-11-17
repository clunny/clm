---
title: "A verify can hang on a test until the plan's freeze ends"
kind: question
components: [ci]
blocks: [ci/tests-time-bounded, ci/coverage-script, ci/nextest-runner, ci/test-tools]
choices:
  - "Build it now, as an exception"
  - "Wait for the plan's end"
---

`20261116-clm-verify-s-coverage` runs every Rust test run through nextest, the coverage pass and the release's included, and has nextest end a test after 15 minutes (`one-runner-for-the-rust-tests`). It is not one of the plan's phases in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`, so building it needs an exception to the freeze, which is the human's to make.

For building it now: the mutation pass is phase 3's evidence, and each verify that reaches the coverage pass under `cargo test` can hang without end on the tests that set their own process's `PATH`, as it did for 100 minutes on 2026-11-16; the release's `cargo test --all` would hang the same way.

For waiting: until it is built, when a verify sits with test processes spinning and no output, kill its process tree; verify records nothing and runs again. Don't run a release before it is built.
