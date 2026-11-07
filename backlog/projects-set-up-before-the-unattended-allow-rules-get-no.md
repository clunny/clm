---
title: "Projects set up before the unattended allow rules get no migration offering them"
kind: bug
components: [upgrade, init]
---

**Step:** upgrading a project set up by an earlier release to one whose `clm init` seeds the unattended allow rules in `.claude/settings.json` (`init-seeds-permissions`, change `20261105-unattended-steps-denied-in`).

**What happened:** the change shipped no migration. `clm upgrade` lists nothing for the rules and the session-start reminder never names them. An existing project learns of them only when `/clm:build` or `/clm:triage` runs `clm init --check-permissions` before an unattended launch, or from an `init` rerun's next steps. In a downstream project the builds ran in auto mode without the rules, so `backlog/check-subagent-honours-allow-rule` still can't be confirmed.

**Expected:** `upgrades-carry-migrations` says a change that alters a file projects set up by a published release already hold SHALL ship the migration that brings them up to date. An advised note migration fits here: its `done` is `clm init --check-permissions` passing, its note lists the rules to add, and the human can decline it, so it doesn't bring back a rule a project removed on purpose (the reason `unattended-steps-run-clm` rejected rewriting the settings on every `init` run). The decision never weighed a migration, since it landed the same day as `clm-upgrade` itself.

**Also:** the corpus test that enforces `upgrades-carry-migrations` didn't flag the change, so the next change to a file `clm init` writes can slip through the same way. Why it missed this one is not yet known.

**Rules involved:** `upgrades-carry-migrations`, `upgrade-migrations`, `init-seeds-permissions`, `init-check-permissions`, `unattended-steps-run-clm`.
