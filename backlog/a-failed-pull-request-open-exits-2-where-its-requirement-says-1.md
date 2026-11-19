---
title: "A failed pull request open exits 2 where its requirement says 1"
kind: bug
components: [host]
origin: 20261119-the-cuts-rules-written-down
---

`host/keep-unopened-body` says that when the host's `pr open` fails, the error ends `; the body is kept at <path>`, "so `clm pr open` exits 1 naming it", and its scenario for Azure DevOps says "it exits 1". On `github`, a `gh pr create` that fails makes `clm pr open` exit 2, which `a_body_that_fails_to_open_is_kept_with_its_cuts` in `crates/clm-host/tests/command.rs` asserts. Either the requirement or the code is wrong for `github`; `host`'s Contract should say which status a host's failure gives, and the requirement and the test should agree with it.
