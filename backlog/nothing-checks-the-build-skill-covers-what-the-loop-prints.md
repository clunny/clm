---
title: "Nothing checks that the build skill covers every action and cause the loop prints"
kind: idea
components: [plugin, orchestrator.loop]
---

`clm build` prints actions (`launch`, `resume`, `stop`, `run`, `wait`, `say`, `end`) that the build skill carries out, and `clm build returned` takes a `--cause` word the skill chooses. The requirements that pinned the skill's wording for each (`plugin.build/build-runs-the-loop`, `plugin.build/build-reads-the-cause`) were scans of prompt text against lists written into the test, and are deleted in `20261113-phase-2-delete`.

A program can hold the part that matters: a test that takes the action words and the causes from `clm-loop` itself and fails naming each one the skill doesn't mention, as `clm/glossary-covers-terms` does for commands. Then an action added to the loop without a line in the skill fails the land's checks.
