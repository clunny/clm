---
title: "Code tied together at compile time across components fits CLM's one-component-at-a-time builds only through case-by-case exceptions"
kind: idea
---

## Problem

CLM builds one component at a time. A build changes only its own `paths` (`path-ownership`), and builds are ordered only by `#### Needs`. But the code of different components is often tied together at compile time: a change to one crate's types can stop another crate compiling until both move together. When a change creates such a tie, it may have no build order the rules allow, and nothing finds that out until a build gets stuck after the change was approved and applied.

This has come up three times, and each time it was settled with one more exception in `path-ownership`:
- compile-only fixes forced by a removal from a Contract;
- the same fixes forced by an added field;
- an arm for a variant added to a type another component matches exhaustively (`20261106-spec-change-widens-an`, after `20261106-azure-devops-host` could not be built in any order).

Each exception covers only the case that came up, so the next kind of tie will stall a build again, and `path-ownership` grows longer with each one. Catching these ties while a change is designed is left to the builder check, which is a model's review rather than a mechanical check, so it can miss them.

## To explore

- What kinds of compile-time tie between components exist beyond the three seen so far (traits, generics, macros, other languages' equivalents)?
- Whether ties like these can be found while a change is designed, before it is approved, and how reliably.
- Whether `path-ownership` and per-component builds are the right shape for such changes, or whether the exceptions are the right answer and only need to be stated more generally.
- What each approach costs the human in review and the build in parallelism.
