---
status: proposed
---

# Keep the required-migration workaround until a real required migration ships

## Intent

The human answered `backlog/required-migration-test-hook` by choosing to wait. Until a real `required` migration ships, `capability/upgrade-clm`'s test keeps reaching the exit-4 refusal through a too-new `upgraded` stamp. Once one ships, the test uses it and checks that the message points at `clm upgrade`. No test hook or boundary migration list is added. This change only edits the backlog. The answer leaves the item open, waiting on an event the human judges (the first required migration shipping), so it is left out of `resolves` and turned from a `question` into a `deferred` item with that condition, keeping its slug. The flag `backlog/workaround-capability-upgrade-clm-1` now names the human's choice instead of asking for a spec or a test hook. No requirement, scenario or design changes, so there is nothing to build.

## The human's answers

> The human picked: "A: wait" — Keep the workaround until a real required migration ships, then use it in the test.

## Problem

- The question asked whether a required migration should ship for the test, or the spec should name a way to supply a migration list at the boundary. The human chose the first and to wait for it, so nothing is designed now, but the follow-up work must stay recorded.

## Approaches

- **A. Turn the question into a `deferred` item (chosen).** The work is decided (use the first real required migration in the test) and waits on a condition the human judges in `clm triage`. When it is drafted, the test change is code only, since `clm/upgrade-blocks-commands`'s scenario "required migration pending" already states the behaviour, so a build closes it with a `Resolves:` trailer (`deferred-work-resolves-in-code`).
- **B. Resolve the question and record a decision.** The follow-up would then live only in the workaround flag, which asks for a spec or a test hook, the opposite of the answer.
- **C. Turn it into a bug.** Nothing can be fixed until a required migration exists, so a bug would sit unfixable in every triage.

## Outcome

- `backlog/required-migration-test-hook.md` is `kind: deferred`, components `[capability]`, with the condition and the work.
- `backlog/workaround-capability-upgrade-clm-1.md` names the human's choice and the deferred item.

## Constraints

- `workaround-flags`: the flag stays, at its current version, since the workaround still applies.
- `yagni`: no test hook for a migration list is added to the binary.

## Edge cases and workarounds

- The existing workaround (the too-new `upgraded` stamp) stays until the deferred item is done.
