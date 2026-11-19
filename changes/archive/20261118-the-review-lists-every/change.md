---
status: done
resolves: [the-review-lists-every-line-of-a-test-list-whose-base-text]
---

## Intent

The web review lists no cut read from a `deleted-tests.txt` whose text GitHub withholds at the merge base, and asks review for the branch, as it already does for a list withheld at the head; for a list withheld at either side it shows one line naming the list instead, so a yes given on the web never covers tests the page hides without saying so. Today it reads such a list as new and lists every line of the head's list as a cut, lines `clm gate` doesn't print (`backlog/the-review-lists-every-line-of-a-test-list-whose-base-text`).

## Problem

- GitHub gives a blob's text as null when it isn't UTF-8 or is binary. `files::spec_files` then leaves the file out, so `review::test_lists` passes the list's base as `None`, and `clm_gate::cuts` counts every line at the head as added.
- `clm gate` reads the base text lossily and lists only the lines the branch adds, so the page shows cut lines the gate doesn't print. `review` is true either way, so no approval is lost.
- `web.worker.review/pr-review` states the rule for a list withheld at the head (`review` true; the build lists none of its lines) and none for the merge base, so the code's reading isn't wrong by any written rule, and the fix needs one (`code-fix-needs-a-written-rule`).

## Approaches

- **Taken: a list withheld at either side gives one line naming it, in place of its lines, and `review` true.** The Worker can't read the withheld text, so it can't tell which lines the branch adds; listing none of them and asking review fails closed, as the head's rule and a truncated listing already do, and the line keeps the cut from being silent (`cuts-are-what-a-program-lists`, `removal-needs-a-yes`), worded like the gate's line for specs that don't load. The two sides then follow one rule.
- **Rejected: no line for a withheld list** (the first draft, and the head's behaviour today). The architecture audit found it lets a web approval cover a list's deletions the page never shows, against `cuts-are-what-a-program-lists`.
- **Rejected: list every head line, as today.** It shows lines the gate doesn't print and that the branch may not add.
- **Rejected: fetch the raw blob and decode it lossily, as the gate does.** One more subrequest per such list for a case that arises only from a binary or non-UTF-8 list, against `web-on-free-plan`'s budget and `yagni`.

## Outcome

- `pr-review` says a list GitHub withholds at the merge base, its own text or, for an archived list the base doesn't hold, the live list's (as `gate/cuts` reads it), makes `review` true, and that `cuts` hold none of a withheld list's lines but one line naming it, at either side; the head's scenario asserts that line and a new one proves the base side.
- The build leaves such a list out of `test_lists`, extends `withheld_list` to the base side, appends the naming line to `cuts`, and names the cache key anew, `compare7`, as `pr-review` requires of any change to it; the Design names the new key.

## Constraints

- `clm gate`'s behaviour doesn't change: it still reads both sides lossily and lists what the branch adds. On a list withheld at a side, the page shows one line naming the list where the gate prints the lines it adds, and asks review; the gate reads the text lossily, which the Worker, given null, can't.
- The naming line is the Worker's own: `clm gate` never prints it, since it always reads the text.
- Only a list the head holds counts, as `cuts` read only those: a branch that deletes a list withheld at the base asks nothing for it.
- One edge case, accepted as failing closed: a clean archive move of a change whose live list GitHub withholds at the base asks review and shows the naming line, though the gate lists nothing for it.

## What the requirements no longer state

Nothing. No requirement, scenario or sentence is removed.
