---
title: "Nothing checks that a clm command or guide page a skill names exists"
kind: idea
components: [plugin, clm]
---

A skill, agent or command file that names a `clm` subcommand the binary doesn't have breaks at the moment a session runs it. So does a `clm guide <page>` naming a page the binary doesn't print: `plugin/method-pointers` checked those by scanning the skill and agent files against `docs/guide/`, and is cut in `20261123-wave-5-plugin-by-the-rule` as a scan of prompt text. `plugin/renamed-commands` guarded against three old names by scanning `plugin/` for them, and is deleted in `20261113-phase-2-delete` as a scan of prompt text against a list in the test.

A program can check both: take the subcommands from `clm --help`, as `clm/glossary-covers-terms` does, and the pages from `clm guide`'s index (`init/guide-command`), and fail naming each `clm <word>` under `plugin/` that isn't a subcommand and each `clm guide <page>` a skill, agent or command file names that `clm guide` doesn't print.
