---
status: applied
resolves: [gate-cuts-leaves-its-matching-rules-unwritten, obsolete-tests-in-another-components-file-are-listed-to-change, the-review-shows-no-cut-when-the-specs-dont-load, a-resent-pull-request-body-gains-a-second-cuts-section]
---

## Intent

The builds of `20261118-the-pass-s-tooling` made choices its requirements left open, and found three places where the gate, the web review, the build prompt and `clm pr` disagree or repeat themselves. This change writes each choice into the requirement that owns it, so a later rewrite can't drop it unseen, and closes the three gaps. It is part of step 2 of phase 5 of `backlog/a-passing-check-proves-less-than-the-spec-claims`; the human answered the three questions on 2026-11-19.

## Problem

- `gate/cuts` doesn't say how a sentence is read (the case of `SHALL`, whether a placement phrase is a whole word, what ends a sentence), what "the specs don't load" covers, or that the graphs hold the two human layers, without which every rule's `serves` fails to resolve and every spec branch would read as unloadable (`backlog/workaround-gate-cuts-1`).
- The gate's rule for specs that don't load sits outside `cuts`, so the web review, which calls `cuts`, shows no line and asks no review for such a branch. A `deleted-tests.txt` whose text GitHub withholds, and a truncated listing, are unstated too.
- The build prompt tells a build to delete or re-cite an obsolete requirement's test in a file another component owns, which `clm land` refuses (`land/build-holds-own-files`). A `behind` marker in such a file is already listed apart.
- A body that already starts with `## Cuts`, such as the copy `keep-unopened-body` keeps, gets a second section when sent again; and that the kept copy holds the section is said only in `host`'s Design.

## Approaches

- **Taken: the build's choices as they are** (the human's answer). `SHALL` in capitals is the spec format's keyword; a phrase as a whole word keeps "is left intact" a stating sentence; any load violation fails closed.
- **Taken: the gate offers the unloaded rule as a call, `cuts_or_unloaded`,** which `clm gate` and the Worker both read, so the browser's chip can't say no review where the gate asks one (the human's answer).
- **Taken: an obsolete marker in another component's file is listed apart, as a `behind` one is** (the human's answer).
- **Taken: a leading `## Cuts` section of a body is replaced, never kept.** The section is the gate's to write.
- **Not taken: matching `shall` in any case, or the phrases as substrings.** Offered by the question and declined.

## Outcome

- `gate/cuts` states the reading of a sentence, what loading means and which files the graphs hold, and offers `cuts_or_unloaded`.
- The web review shows the unloaded line and `review` true for a branch whose specs don't load at a side, asks review when a list's text is withheld, and lists no cut from a truncated listing.
- The build prompt lists apart an obsolete requirement's markers and tests in a file another component owns.
- `clm pr open` and `clm pr body` replace a leading `## Cuts` section; `keep-unopened-body` says the kept copy holds the section and that a gate that can't be evaluated keeps none.

## Constraints

- No behaviour of `clm gate` changes: each clause added to `gate/cuts` is what the build does, as the builder check confirmed against `crates/clm-gate/src/cuts.rs` and `lib.rs`: the reading of a sentence, the files loaded, the unloaded line followed by the lists' tests, the merge base named when both sides fail, and the graphs read only for a diff that touches `specs/` or a list.

## What the requirements no longer state

Nothing. No requirement, scenario or sentence is removed.

## What the builds do

`gate` first (a re-cite, the new call and the scenarios that have no test yet), then `web.worker.review`, `orchestrator.run` and `host`.

- `gate`'s build writes the tests no scenario had: a lowercase `shall`, a phrase inside a word, a sentence ending at a line break, a rule that serves a capability, both sides failing to load, and `cuts_or_unloaded` itself.
- `web.worker.review`'s build loads a second pair of graphs for the cut call, from only the files the gate loads, empties `cuts` for a truncated listing, and names the cache key anew, `compare6`.
- `host`'s build gives the two tests that prove the kept body the marker of `keep-unopened-body`.

## Open questions

- **An obsolete marker in another component's file has no owner step.** Listed apart, it stays, so its requirement stays `obsolete` on its own component, which the queue keeps offering, and the question a build files can block nothing. It can arise only in a file an exemption from `check/tests-cite-their-owner` covers. `backlog/an-obsolete-marker-in-another-component-s-file-is-nobody-s-to-remove` asks the human whether the marker should count as pending on the file's owner.
- **`host/keep-unopened-body` says `clm pr open` exits 1** where a failed `gh pr create` exits 2 today, which a test asserts. Older than this change; `backlog/a-failed-pull-request-open-exits-2-where-its-requirement-says-1` is a `bug` on `host`.
