---
title: "A listed test of a requirement that stays reaches no build"
kind: bug
components: [orchestrator.run]
origin: 20261119-wave-1-by-the-rule
blocks: [vcs/one-git-layer, vcs/remove-worktree, model/says-component, model/optional-nodes, model/component-keys, model/human-layers, config/retired-terms-detector, apply/says-component, apply/hash-command, rename/marker-rewrite, gate.fingerprint/fingerprint]
---

A change lists in `deleted-tests.txt` each test its builds are to delete (`cuts-are-what-a-program-lists`): a duplicate, or the test of a sentence a requirement lost. `clm land` lets a build lose a listed test. But the build prompt names a listed test only under an `obsolete` requirement; for one whose requirement is `behind` it lists the test's marker to re-cite, so the build keeps the test and the cut the human agreed never happens. Found by the builder check of `20261119-wave-1-by-the-rule`, 17 of whose 19 listed tests cite a requirement that stays.

`orchestrator.run/pending-prompt` states the fix since that change: for a `behind` requirement the prompt lists, as tests to delete, each citing test an approved list names. This item blocks the requirements of that change whose listed tests wait on it, so the queue builds `orchestrator.run` first; the build that makes `pending-prompt` current deletes it.
