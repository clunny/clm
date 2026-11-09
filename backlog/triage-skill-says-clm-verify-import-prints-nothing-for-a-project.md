---
title: "Triage skill says clm verify --import prints nothing for a project without [verify]"
kind: bug
components: [plugin.triage]
---

Step: /clm:triage step 1, which runs `clm verify --import` before `clm intake`.
What happened: a triage draft, reading the verify spec, found that the skill text says `clm verify --import` "prints nothing when the project sets no `[verify]`", while the command prints a `verify off` line in that case (and, after change 20261109-the-after-land-verify, a line naming `origin/main:clm.toml`).
Expected: the skill describes the line the command prints for a project with no `[verify]`, so a session doesn't read that line as an unexpected result.
Rule involved: method-in-the-plugin (the skill states the command's output).
