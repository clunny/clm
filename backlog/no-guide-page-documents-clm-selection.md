---
title: "No guide page says when CLM_SELECTION is set, what its file holds, or how a check script uses it"
kind: bug
components: [land, init]
issues: [375]
---

A project that wants `[land] check` to run only the tests a branch can break has to find the interface itself. `clm land` hands the check a selection through `CLM_SELECTION`, but the guide names the variable only in passing, in the section on doc-only lands. It never says:
- that every land running the check sets it;
- that the file is `clm affected`'s output as printed (its component, file and `all` lines), or the `--readers` output on an already-checked tree;
- that it is unset when `clm affected` fails and under `clm land --check-only`, where every test should run.

An adopter learned the format only by dumping the environment from a probe script, which goes against `method-in-the-plugin`. The `scripts/clm-check.sh` that `clm init` writes ignores the variable, so a new project runs its whole suite on every land.

Expected: `ci-and-deployment` (or `settings` under `land`) documents the variable: when it is set and when it isn't, the line format, and a short example script that maps the lines to test commands and falls back to everything when the variable is unset. `CLM_AFFECTED` is a command override, not part of what the check reads, and stays out of that text. Whether the starter script should read the selection, and what a project with an existing script is offered (`upgrades-carry-migrations`), is a separate design question, best weighed with `backlog/init-check-starter-marks-no-steps`.
