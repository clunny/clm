---
title: "prove-by-tests has three unsettled workarounds: tests only a clm process can show"
kind: question
components: [land, config, check.unreferenced, clm]
blocks: [land/commands-in-crate]
choices:
  - "Settled: the scenarios become the root's"
  - "Redesign prove-by-tests first"
---

`clm workaround land/commands-in-crate` exited 3 (`redesign-before-workaround`): `capability/prove-by-tests` now holds three unsettled workaround flags, `config`'s, `check.unreferenced`'s and `land`'s `commands-in-crate`, each for tests that observe only a `clm` process and stay in `crates/clm/tests/` (`backlog/two-config-tests-run-other-components-commands.md`, `backlog/two-unreferenced-tests-observe-only-the-clm-process.md`, `backlog/five-land-tests-observe-only-the-clm-process.md`).

`land`'s build had already done the rest of the requirement when the flag was refused: `clm-land` offers `clm land` and `clm reconnect` as calls, `crates/clm` routes to them, and 10 of the 15 listed tests call the crate. It stopped there.

The human already answered the first two items (`docs/decisions/commands-live-in-their-crates.md`, 2026-11-15): a scenario only a process can show becomes the root's, in one spec change once the build run drains. What is needed: whether that answer settles all three flags, so the capability needs no redesign, and the spec change that moves the scenarios deletes the flags with the items.
