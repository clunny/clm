---
status: done
resolves: [verify-bisect-blames-commits-it-could-not-test]
---

## Intent

`clm verify`'s bisect skips commits whose check cannot run, rather than counting them red, and records `culprit unknown` with the components of the lands that may hold the break when the untestable commits leave the culprit open.

## Problem

`verify-bisect` counts every probe whose `[verify] check` doesn't pass as red. A commit older than the project's slow-test script can't run the check at all (`sh` exits 127: command not found), yet it counts red. With no green record the search starts at the first commit on `main`'s first-parent chain, so the outcome can name the repository's first commit as the culprit and, since that diff touches no component's `paths`, the root component. Triage then takes a culprit and component that cannot be right, and `land/verify-stop` holds the wrong component's lands.

## Approaches

- **Untestable probes set aside, `git bisect skip`'s way (chosen).** A probe exiting 125, 126 or 127 is untestable: 126 and 127 are `sh`'s codes for a command it can't run or can't find, and 125 is `git bisect run`'s existing "can't test this commit" code, so a project's script can say so without CLM inventing a convention (`reuse-first`). The search probes around set-aside lands; when set-aside lands sit right before the first red one, any of them may hold the break, so the culprit is unknown and the record names the components of those lands and the first red one. That is the `culprit unknown` outcome the Contract already has.
- **`git bisect` itself, with `--first-parent` and `skip`.** Its search keeps state in the repository's `refs/bisect/*` and checks commits out in the tree it runs in, while each probe here must run in the verify tree through `land/shared-check-runner`, taking the clone's check lock afresh so lands go between probes, with the selection from the last green commit. Its "the first bad commit could be any of" result is prose meant for a person, not an interface to read the range from. The search CLM already has is a few lines, and skipping adds a few more; it reuses `git bisect run`'s exit code 125 and its rule that a set-aside land next to the first bad one leaves the culprit open. Rejected.
- **Probe only commits that hold the check's script.** CLM can't tell which file a shell command needs, and a command can be missing for reasons other than a file (a tool the old tree didn't install). Rejected.
- **Count untestable as green.** Wrong in general: a commit set aside says nothing about the break, and counting it green could name a later land falsely.

## Outcome

`verify`'s Design and `verify-bisect` say what counts as untestable, how the search moves around it, and what the record holds when the culprit is left unknown. `land/shared-check-runner` also returns the command's exit code, since its callers today see only whether it passed. `docs/guide/ci-and-deployment.md`'s **When one fails** says what exit 125, 126 and 127 mean to the bisect and what `culprit unknown` means, a hand edit in this change (`method-in-the-plugin`, from the architecture audit). The red head's own run is unchanged: a head whose check can't run is still red, since that is a broken `[verify] check` the human must see.

## Constraints

- Edge cases known: concluding `culprit unknown` means probing every land set aside in the range left, each a checkout and a check that ends at once, so a long run of old commits before the slow-test script costs that many checkouts; the search assumes, as `git bisect` does, that a land after a red one stays red; a land with no first parent (the repository's first commit) diffs against the empty tree, as `changed_files` already does; a check whose own tool is missing inside the script also exits 127, and that land is set aside too, which is the intent; a probe a signal ended has no exit code and stays red; a red head whose own check exits 127 is still red, so every land is probed and the outcome is `culprit unknown` over the range.
- From the builder check: `first_red` returns either the culprit or the range `lo..hi` left; `verify_at` and `bisect_red` skip `recheck_note` with no culprit and pass `chain[lo..=hi]` to `components`.
- The new exit code is a field added to `clm_land::CheckRun`, which only `clm-land` constructs, so no other crate's code breaks.
- No workaround is known to be needed.

## Open questions
