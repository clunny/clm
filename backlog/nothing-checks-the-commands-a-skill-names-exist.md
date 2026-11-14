---
title: "Nothing checks that a clm command a skill names exists"
kind: idea
components: [plugin, clm]
---

A skill, agent or command file that names a `clm` subcommand the binary doesn't have breaks at the moment a session runs it. `plugin/renamed-commands` guarded against three old names by scanning `plugin/` for them, and is deleted in `20261113-phase-2-delete` as a scan of prompt text against a list in the test.

A program can check the general case: take the subcommands from `clm --help`, as `clm/glossary-covers-terms` does, and fail naming each `clm <word>` under `plugin/` that isn't one, as `plugin/method-pointers` does for guide pages.
