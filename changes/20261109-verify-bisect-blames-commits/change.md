---
status: applied
resolves: [verify-bisect-blames-commits-it-could-not-test]
---

## Intent

`clm verify`'s bisect skips commits whose check cannot run, rather than counting them red, and records `culprit unknown` with the components of the lands that may hold the break when the untestable commits leave the culprit open.

## Problem

`verify-bisect` counts every probe whose `[verify] check` doesn't pass as red. A commit older than the project's slow-test script can't run the check at all (`sh` exits 127: command not found), yet it counts red. With no green record the search starts at the first commit on `main`'s first-parent chain, so the outcome can name the repository's first commit as the culprit and, since that diff touches no component's `paths`, the root component. Triage then takes a culprit and component that cannot be right, and `land/verify-stop` holds the wrong component's lands.

## Approaches

- **Untestable probes set aside, `git bisect skip`'s way (chosen).** A probe exiting 125, 126 or 127 is untestable: 126 and 127 are `sh`'s codes for a command it can't run or can't find, and 125 is `git bisect run`'s existing "can't test this commit" code, so a project's script can say so without CLM inventing a convention (`reuse-first`). The search probes around set-aside lands; when set-aside lands sit right before the first red one, any of them may hold the break, so the culprit is unknown and the record names the components of those lands and the first red one. That is the `culprit unknown` outcome the Contract already has.
- **Probe only commits that hold the check's script.** CLM can't tell which file a shell command needs, and a command can be missing for reasons other than a file (a tool the old tree didn't install). Rejected.
- **Count untestable as green.** Wrong in general: a commit set aside says nothing about the break, and counting it green could name a later land falsely.

## Outcome

`verify`'s Design and `verify-bisect` say what counts as untestable, how the search moves around it, and what the record holds when the culprit is left unknown. `land/shared-check-runner` also returns the command's exit code, since its callers today see only whether it passed. The red head's own run is unchanged: a head whose check can't run is still red, since that is a broken `[verify] check` the human must see.

## Constraints

- Edge cases known: concluding `culprit unknown` means probing every land set aside in the range left, each a checkout and a check that ends at once, so a long run of old commits before the slow-test script costs that many checkouts; the search assumes, as `git bisect` does, that a land after a red one stays red; a land with no first parent (the repository's first commit) diffs against the empty tree, as `changed_files` already does.
- The new exit code is a field added to `clm_land::CheckRun`, which only `clm-land` constructs, so no other crate's code breaks.
- No workaround is known to be needed.

## Open questions
