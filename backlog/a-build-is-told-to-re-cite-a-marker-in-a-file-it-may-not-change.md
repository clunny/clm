---
title: "A build is told to re-cite a marker in a file it may not change"
kind: deferred
components: [orchestrator.run]
origin: 20261115-builds-change-only-their-own-files
blocks: [gate/own-files]
---

`orchestrator.run/pending-prompt` has the `clm run` prompt tell a build to re-cite each `behind` marker "wherever its file sits, another component's `paths` included", with the scenario "stale marker in another component's files"; the Design's Parallelism bullet and the prompt text in `crates/clm-run/src/lib.rs` say the same. Once `gate/own-files` is built without the clause "a file that cites the component", `clm land` refuses that edit, and nothing asks the file's owner to make it, so the requirement would stay `behind`.

In this repository no such marker is left by then: every test sits in the files of the component it cites. For any project, the instruction and the land must agree before the clause goes, so this item blocks `gate/own-files`.

Due when `clm pending` no longer lists `clm/routes-only`. It needs a spec change to `orchestrator.run/pending-prompt`, and a choice to weigh there: the prompt lists a `behind` marker in another component's file as that owner's to move or re-cite, as `obsolete` lines are already split by the owner of their file; or `clm check` refuses a marker that cites a component other than its file's owner in every file, not only under a `tests` glob.
