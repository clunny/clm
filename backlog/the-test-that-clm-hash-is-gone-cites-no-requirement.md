---
title: "The test that clm hash is gone cites no requirement"
kind: deferred
components: [clm]
---

`apply/hash-command@752d167d` handed the rule that `clm hash` is no command, and its scenario `old name gone`, to the root's `clm/old-command-names-retired`. The build of `apply` took its own marker off the test that proves it, `hash_is_no_command` in `crates/clm/tests/cli.rs`, since that file is the root's and the build may re-cite only its own requirements there.

The test still runs and checks what `clm/old-command-names-retired` states of `clm hash` and `clm --help`, but no marker links it to that requirement. The root's build adds `req:clm/old-command-names-retired@<version>` to it and deletes this item in a commit with the trailer `Resolves: backlog/the-test-that-clm-hash-is-gone-cites-no-requirement`.
