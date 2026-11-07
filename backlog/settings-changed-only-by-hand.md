---
title: "CLM's settings can be changed only by editing clm.toml by hand, with no guide to the options and their trade-offs"
kind: idea
priority: P2
components: [plugin, config]
---

## Intent

Once a project is set up, changing how CLM behaves means editing `clm.toml` (or the layered settings) by hand: `host`, `defects`, `max_builds`, `approval`, `[land]`, and the `[verify]` section `20261107-slow-tests-after-land` adds. Nothing walks the human through the options, says what each trades off, or lands the edit the right way (a change to `approval` needs review; most don't).

## Settled

- A general `/clm:configure` skill covering every setting, not one per feature: its value is in knowing them all, and a skill made for one section would be redesigned as it grew. Raised by the human on 2026-11-07 while designing `20261107-slow-tests-after-land`, which keeps its own setup to `/clm:init` and `clm upgrade`.
- It should also work from plain chat: a session asked to change a setting reaches the same guidance.

## Open questions

- Does it read the current values through a `clm` command (so the skill holds no parser), and does `clm` validate a proposed edit before it lands?
- Which settings are per clone (layered, never committed) and which are the project's, and how does the skill say which it is changing?
