---
status: applied
---

# The retired-terms test keeps only old names, and skips changes

## Intent

The retired-terms test (`clm/retired-terms`) keeps CLM's renamed commands, flags, keys and coined terms from coming back, but it also bans two plain English words outright, and it reads the prose of live changes, where a change that describes a rename has to name the old word. Both have made sessions reword ordinary sentences and rename backlog items to get past it, and `main` landed failing it. This change keeps the list to forms ordinary English never takes, drops the bare word for ancestors from it, and stops the test reading `changes/`.

## Problem

On 2026-11-04 the test failed `main` on a plain-English use of the old name of the `workaround` kind in a change's intent, then failed the change that freed that word, twice: once on its own description of the rule and once on a backlog item's slug that the description named. The cause is the same each time: a bare English word on the list, matched anywhere, including in `changes/`, which proposes and explains renames. The list's real targets, an old command name after `clm` in a skill, an old flag or key, a coined term of several words, never occur in ordinary English and have never misfired.

## Approaches

- **Remove the test and its requirements.** Ends the misfires, but `plain-terms`' promise that a renamed term doesn't come back would be enforced by nothing (an edit to `ARCHITECTURE.md`, moving it to `#### Untested`), and a stale command name in a skill would show only when someone's session failed on it. Rejected.
- **Keep it, listing only forms ordinary English never takes, and skip `changes/` (chosen).** Keeps every check that has caught or would catch a real slip, and removes every one that misfired. A live change's deltas still reach `specs/` when applied, which the test reads.
- **Keep the bare words with an allowlist of plain-English sentences.** A list of exceptions to a list; every new sentence would need one. Rejected.

## Outcome

- `clm/retired-terms`: the list holds only an old command after `clm `, an outcome line, a flag, a key, a `kind:` in backticks, a coined term of several words or a coined non-word (the old name of the status `unbuilt`); a test checks the list itself. The scan no longer reads `changes/`, live or archived.
- `clm/node-names-retired`: the bare old word for ancestors leaves the list; the old name of placeholder component stays.
- With `clm/old-state-and-key-names-retired` (`20261104-found-by-search-and-agents`), no bare English word is left on the list.

## Constraints

- `plain-terms`: a renamed term still doesn't come back in what CLM ships or works from; its `#### Enforced by` is unchanged.
- History stays exempt by not being read.
