---
title: "An empty test in a file that reads the repository escapes the mutation pass"
kind: idea
components: [verify, ci]
---

`verify`'s plan gives a block a `reads <glob>` line for every `clm:reads` marker anywhere in a citing test's file (`crates/clm-verify/src/mutate.rs`), and `ci/mutate-script` (at `1ac5990f`) then writes `not-judged`, not `survived`, for a block with a `reads` line whose tests run none of its code. Several shared test files carry a file-level marker (`crates/clm/tests/cli.rs`, `crates/clm-check/tests/cli.rs`, `crates/clm-health/tests/command.rs`, `crates/clm-config/tests/this_repository.rs`), so a test in one of them that asserts nothing and runs none of the code is never reported `unproven`, which is the case the pass exists to catch (`proven-by-tests`).

A tighter signal would key on what the citing test itself reads, such as a marker within the test's own span, rather than any marker in its file. Found by the code review of the `impl/ci` build that brought `ci/mutate-script` to `1ac5990f`.
