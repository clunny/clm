---
title: "Renaming a citing test reads to the land as deleting it, so a test keeps a name that has become false"
kind: bug
components: [land]
---

On `impl/host`, a build of `host/keep-unopened-body` changed what `clm pr open` exits with on a failed `gh` from 2 to 1, as the requirement states. Two tests named for the old status (`page_open_exits_2_when_gh_fails`, `a_failing_gh_pr_view_exits_2_and_opens_nothing` in `crates/clm-host/tests/command.rs`) were renamed to say 1. `clm land --check-only` then refused under `land/deleted-tests-approved`: each test "is gone and no approved list names it", although a test with the same body and the same citations sat under a new name in the same file. The build had to restore the false names and add a comment explaining them. A test that keeps every citation it had, renamed in place, loses nothing a removal yes protects; the check could match a new test in the same file that cites everything the gone one cited, or the build needs a way to record a rename.
