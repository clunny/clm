---
title: "The web review shows no cut and asks no review when a side's specs don't load"
kind: question
components: [web.worker.review, gate]
choices:
  - "Show the unloaded line and ask review"
  - "Leave it: clm land runs the gate"
---

`clm gate` prints `specs: gate/cut: the specs at <revision> don't load, so what the branch cuts isn't listed` and asks review when either side's specs load with a violation (`gate/cuts`). It does that in `read_cuts`, not in `clm_gate::cuts`, so the Worker's review, which calls `cuts` and `widened_exemptions` over the two graphs as `web.worker.review/pr-review` states and ignores load violations, shows neither the line nor `review` true for such a branch. No requirement states what the review does then, so the build kept to the text. `clm land` still runs the gate, so nothing lands unreviewed; only the browser's chip can say no review where the gate will ask one. A spec branch that settles it edits `web.worker.review/pr-review` (and perhaps offers the gate's unloaded rule as a call from `clm-gate`); no requirement waits on it.
