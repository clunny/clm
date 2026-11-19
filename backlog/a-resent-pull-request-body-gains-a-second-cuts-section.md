---
title: "A resent pull request body gains a second Cuts section, and keeping a body with its cuts is stated only in the Design"
kind: idea
components: [host]
---

Found reviewing the build of `host/pr-open` and `host/pr-body`.

- A body that already starts with the `## Cuts` section, such as the copy `keep-unopened-body` keeps for a spec branch with cuts, or a body read back from the pull request, gets a second section when it is sent again with `clm pr open --body-file` or `clm pr body`. A requirement could say that a leading `## Cuts` section of the given body is replaced rather than kept.
- That the kept copy holds the section is stated only in the Design ("The cuts in a body"); `keep-unopened-body` says the `--body-file`'s contents are kept. Nor does any text say that a gate that can't be evaluated keeps no body. The build follows the Design and keeps nothing on a gate refusal; `keep-unopened-body` could state both.
