---
status: applied
resolves: [parallel-builds-test-runs-collide-on-fixed-host-resources]
---

## Intent

Make every CLM-built project's tests able to run concurrently on one host, as a rule CLM carries to every project, and give each build and its checks a number, `CLM_SLOT`, to keep the ports and paths its tests must know in advance apart from other builds' (GitHub issue #266).

## The human's answers

On backlog/parallel-builds-test-runs-collide-on-fixed-host-resources (issue 266), the human said:

> It is reasonable that a project, during development session, might run tests... but it is probably that project's responsibility to enable test sessions to run in parallel like that. Maybe we need that to be an opinionated requirement on CLM-built projects - what do you think?

and then, of the item as now written: "take it in as written".

## Problem

With several builds on one clone, a project whose check starts a preview server on a fixed port failed with "port already used", although clm 0.20.0 serialises every `[land] check` (`land/check-lock`). Builds run the project's tests outside that lock all the time: after each edit, in mutation spot-checks, in the review's re-runs. Serialising those would undo parallel builds. Nothing tells a CLM-built project that its tests must run concurrently on one host, and nothing gives a build a value to keep its ports and paths apart.

## Approaches

- **The project's tests must overlap, and CLM gives each build a slot (chosen).** A carried constraint, `tests-run-concurrently`, states the rule for every project; each build place holds a small number, `CLM_SLOT`, that `clm land` and `clm land --check-only` set for the check and the build's prompt has it set on hand-run tests. The human's answer puts the burden on the project; the slot is what makes it possible for a check that starts a server tests then reach.
- **Widen the check lock to every test run.** Rejected: serialises the builds.
- **A slot inside `[land] check` only.** Rejected: most test runs of a build are by hand.
- **State the rule and give no value.** Rejected: a browser suite pointed at a server its check starts must know the port in advance and has nothing to key it on.
- **Key on the place's name or the worktree's path.** Rejected: a port needs a number, and hashing a name into a port range can collide; a slot chosen under the places lock can't.

## Outcome

- `ARCHITECTURE.md` gains `tests-run-concurrently`, carried as `replaceable`, so it reaches every project through `clm guide constraints` and every build's context bundle, and a project that keeps its runs apart some other way (a container per run) can replace it while keeping the guarantee.
- `orchestrator/place-slots` (added): each place holds a slot, a `slot <n>` line in its file, the smallest free from 1 up, kept on reuse; `place_slot(root, branch)` reads it. `take_place`'s result type stays as it is, so `clm-run` and `clm-triage`, which match on it, build unchanged; `clm run` reads its slot back with `place_slot`. The Contract names the slot and the call.
- `land/check-slot` (added): every `[land] check` runs with `CLM_SLOT` set to the slot of the place naming the branch checked, or 0. `land` now depends on `orchestrator` for `place_slot`; the check lock is unchanged.
- `orchestrator.run/prompt-slot` (added): the prompt names the build's slot, has the session set `CLM_SLOT=<slot>` on every test command it runs by hand, spot-checks included, write tests with no fixed ports or paths, and report a project's colliding test as a bug rather than retry it.
- `plugin/init-skill-concurrent-tests` (added): `/clm:init` tells the human the rule when it writes the check script and, for existing code, files the fixed ports, paths and shared servers it finds as bugs.
- The guide: the glossary defines *slot* and stops calling build places slots; `ci-and-deployment` says that a `[land] check` deploying to a shared test environment breaks `tests-run-concurrently` unless the project keeps that environment out of its builds' tests or replaces the constraint.
- Decisions: `build-slots` (new) records the slot; `check-runs-one-at-a-time` is amended: the lock stays for every `[land] check`, and the per-run slot it rejected becomes the rule.

## Constraints

- The check lock is unchanged; the slot adds to it and doesn't replace it, so a project that hasn't yet made its tests overlap still has its land checks serialised.
- `CLM_SLOT` unset means 0, so a project's tests run as before outside CLM.
- `place_slot` doesn't fetch, lock or judge whether the place counts: it reads files, as `clm land --check-only` must stay quick.

## Edge cases and workarounds

- Runs outside a place share slot 0: the human's own checkout, a draft's code-fix branch (`impl/<component>` claimed by `clm claim --fix`, which the draft's place doesn't name), and `clm verify`'s local-mode check, which sets no `CLM_SLOT`. Accepted: they are few, and the check lock still serialises `[land] check` and `[verify] check` runs among them; the constraint's Guarantee covers runs from different build places only, and its `#### Untested` says so.
- A place file written by an older `clm` holds no slot until it is next taken; until then `place_slot` gives 0 for it. An older `clm` reading a newer place file can't parse the `slot` line and counts the place, which is the safe side.
- `place_slot` doesn't judge whether a place counts, so a landed or lapsed place not yet swept still gives its slot to a run on its branch; nothing builds on a landed branch, and a lapsed claim's resumed build retakes its place.
- Drafts' prompts (`orchestrator.triage`) don't name a slot: a draft runs the project's tests by hand only on its code-fix path. Left for a later change if it shows.

## Open questions

- The starter `scripts/clm-check.sh` that `clm init` writes says nothing of the rule. A comment there was drafted (`init/check-script-says-slot`) and dropped: the script is part of `upgrade`'s corpus, so it would need a new migration for every project set up by a release (`upgrades-carry-migrations`). A project set up before this change learns the rule from `clm guide constraints` and its builds' context bundles only; `/clm:upgrade` offering it is left out (`yagni`) until a project trips on it.
- No `CAPABILITIES.md` scenario covers builds' tests running at once; `work-concurrently`'s scenario is about claims. The added rules serve that capability as written; whether it wants a scenario for overlapping test runs is the human's call.
- Drafts' prompts name no slot (see edge cases).
