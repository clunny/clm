---
title: "No check runs a test the selection misses"
kind: idea
components: [verify]
---

## Intent

A land's check, and a check shared by lands taken together (`land/batched-lands`), runs only the tests the spec graph and the `clm:reads` markers connect to what changed (`graph-test-selection`). That is sound only while every test reads nothing else, which `land-checks-stay-fast` states as a principle: no program checks it. A test that reads a file the graph doesn't connect it to is selected by no check when that file changes, so a land that breaks it passes.

## What is known (2026-11-15)

- `clm verify` hands its check a selection too, from the last green commit (`verify`'s Design, **Since when**). A run on this repository that day ran 83 tests, not the suite. So a test the selection misses is missed after the land as well.
- `20261113-land-starvation` and `config`'s Design describe this repository's `[verify]` as "the whole suite after the land, with no selection". That didn't match the run above. Which is right, and whether `[affected] all` is what was meant, is to be checked first.
- The change names a build's own `clm land --check-only` as running every test on its branch merged with `main`. That was not confirmed in the code.
- Lands taken together lean on the selection harder: one check's selection is joined from several lands' parts.

## Approach to start from

On the `every` backstop, `clm verify` runs its check with no selection. A test that fails there and that no selection since the last green commit named is a selection miss; the bisect names the land (`verify-bisect`), and `verify/culprit-recheck` already says when a culprit's verdict was composed. One requirement of `verify`, at the cost of one whole suite per `every.lands` lands.

## Open questions

- Whether the unselected run belongs on `every` alone, or also on `clm verify --commit` before a release.
- What a selection miss files: a `bug` on the component that owns the test, naming the file it read undeclared, if the run can tell which.
- Whether a stronger check is worth its cost: running tests under a file-access trace and comparing what each read with what the graph declares.
- Whether `land/batched-lands` should have waited for this. The human chose on 2026-11-15 to build batching first and file this.
