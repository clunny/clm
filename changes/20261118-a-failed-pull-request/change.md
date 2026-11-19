---
status: applied
resolves: [a-failed-pull-request-open-exits-2-where-its-requirement-says-1]
---

## Intent

`host`'s commands give a host's failure one exit status on every host: 2 when `gh` or `az` fails or answers what the adapter can't read, and 1 only when the command refuses by its own checks. Today `github` exits 2 when `gh pr create` fails, while `azure` exits 1 when `az` fails, and `host/keep-unopened-body` says 1 for both, so its test for `github` asserts 2 against a requirement that says 1 (`backlog/a-failed-pull-request-open-exits-2-where-its-requirement-says-1`).

## Problem

- `keep-unopened-body` says a failed `pr open` exits 1 on `github` or `azure`; the `github` adapter's failures exit 2, and `a_body_that_fails_to_open_is_kept_with_its_cuts` asserts 2.
- `azure-selected` says a failure of `az` exits 1, because the `Azure` adapter maps `az`'s failure to a refusal, while every other failure of a host's tool, `gh`'s and `az`'s unreadable answers included, already exits 2.
- `host`'s Contract says nothing of which status a failure gives, so a caller can't rely on either.

## Approaches

- **Taken: a host's tool failing exits 2 on every host.** It is the status the workspace already gives a failing tool (`clm-git`'s `Failure { Refused, Git }`, `shared-plumbing-in-git`), the one `pr_command`'s own doc comment promises, and the one the `github` adapter and both adapters' unreadable answers already give. Only one mapping in the `Azure` adapter changes. Exit 1 keeps meaning that the command refused the work by its own checks, and 2 a failure that may pass when run again, as elsewhere in CLM (`land`'s push, `claim`'s switch).
- **Rejected: exit 1 on every host.** It matches the two requirements as written, but makes a failing `gh` a refusal unlike every other tool failure in the workspace, and changes every `github` operation's error mapping instead of one of `azure`'s.

## Outcome

- `host`'s Contract states the rule, and a new requirement, `host-tool-failure`, states it for `clm pr open`, `clm pr body` and `clm issue`, with a scenario each for `gh` failing, `az` failing and an answer that can't be read.
- `azure-selected` and `keep-unopened-body` say 2, and `keep-unopened-body`'s `github` scenario asserts the status too.
- The build changes the `Azure` adapter's mapping of a failed `az` from a refusal to a tool failure, and the tests that assert 1 for a failing `az` (the `az not signed in` and `the host refuses` scenarios) to assert 2; the `github` test the item names re-cites `keep-unopened-body`.

## Constraints

- No other command's status changes: `clm branch push`'s and `clm branch delete`'s refusals stay 1, and a refusal by the command's own checks (a blank body, no pull request, nowhere to file) stays 1 on every host.

## Edge cases and workarounds

- None known.
