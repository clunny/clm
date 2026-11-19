---
status: applied
resolves: [the-review-lists-every-line-of-a-test-list-whose-base-text]
---

## Intent

The web review lists no cut from a `deleted-tests.txt` whose text GitHub withholds at the merge base, and asks review for the branch, as it already does for a list withheld at the head. Today it reads such a list as new and lists every line of the head's list as a cut, lines `clm gate` doesn't print (`backlog/the-review-lists-every-line-of-a-test-list-whose-base-text`).

## Problem

- GitHub gives a blob's text as null when it isn't UTF-8 or is binary. `files::spec_files` then leaves the file out, so `review::test_lists` passes the list's base as `None`, and `clm_gate::cuts` counts every line at the head as added.
- `clm gate` reads the base text lossily and lists only the lines the branch adds, so the page shows cut lines the gate doesn't print. `review` is true either way, so no approval is lost.
- `web.worker.review/pr-review` states the rule for a list withheld at the head (`review` true; the build lists none of its lines) and none for the merge base, so the code's reading isn't wrong by any written rule, and the fix needs one (`code-fix-needs-a-written-rule`).

## Approaches

- **Taken: a list withheld at either side gives no cut line and `review` true.** The Worker can't read the base text, so it can't tell which lines the branch adds; listing none and asking review fails closed, as the head's rule and a truncated listing already do. The two sides then follow one rule.
- **Rejected: list every head line, as today.** It shows lines the gate doesn't print and that the branch may not add.
- **Rejected: fetch the raw blob and decode it lossily, as the gate does.** One more subrequest per such list for a case that arises only from a binary or non-UTF-8 list, against `web-on-free-plan`'s budget and `yagni`.

## Outcome

- `pr-review` says a list GitHub withholds at the merge base, its own text or, for an archived list the base doesn't hold, the live list's (as `gate/cuts` reads it), makes `review` true, and that `cuts` hold no line for a list withheld at either side; a scenario proves the base side.
- The build leaves such a list out of `test_lists`, extends `withheld_list` to the base side, and names the cache key anew, `compare7`, as `pr-review` requires of any change to it; the Design names the new key.

## Constraints

- `clm gate`'s behaviour doesn't change: it still reads both sides lossily and lists what the branch adds. On a list withheld at a side, the page shows fewer cut lines than the gate prints, and asks review; that difference already holds for the head and is accepted for both.
- No edge cases or workarounds are known.

## What the requirements no longer state

Nothing. No requirement, scenario or sentence is removed.
