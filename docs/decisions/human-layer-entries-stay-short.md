---
title: "Each architectural constraint is one or two sentences with its label; mechanics live in decisions and the guide"
date: 2026-11-13
status: accepted
---

`ARCHITECTURE.md` is prepended to every context bundle and every edit to it needs a human's review, yet it had grown to 8,636 words in 56 entries, some naming files and exceptions (`path-ownership` ran to 828 words). On 2026-11-13 it was rewritten by hand, with the human, as phase 1 of `backlog/a-passing-check-proves-less-than-the-spec-claims`.

**The form.** Each entry is one or two sentences, starting with its label (`three-kinds-of-rule`), with at least one scenario and its proof blocks. Its mechanics are in the decision of its name, or the one it names. Mechanics an adopter's project is held to are also in the guide, since `clm guide constraints` prints only an entry's statement and an adopter can't read this repository's decisions (`method-in-the-plugin`).

**What moved.** 56 entries became 43, and no enforcing requirement was deleted: each serves the surviving entry or a capability.

- Merged: `regenerate-dont-merge` into `hash-based-staleness`; `trunk-named-main`, `no-local-land-with-remote` and `guard-blocks-push-to-main` into `land-locally`; `land-policy-from-base` and `approve-pins-the-review` into `approval-policy`; `web-approve-off-when-signed` into `signed-ledger-records`; `needs-alone-order` into `component-reconciliation`; `settings-before-use` into `checks-relax-before-use`.
- Moved down to the decision of the same name: `per-writer-ledgers` (its rules serve `git-is-truth`), `lease-claims` and `autosync-policy` (the capability `work-concurrently`), `health-advises` (`know-health`), `say-guard-is-off` (`guard-sessions`) and `views-name-their-commit` (`see-project-state`).
- Added: `pin-working-method`, from `CAPABILITIES.md`, and `removal-needs-a-yes`.

`plain-terms` governs CLM's own vocabulary, not an adopter's. CLM may use a supported agent harness's own features directly where its adapter declares them (`platform`). For CLM itself, `reuse-first` means git, the agent harness's own primitives and superpowers, with CLM's code kept to glue and checks.

**Corrected.** `redesign-before-workaround` counts unsettled workaround flags per capability, as `clm workaround` and the guide always did, where the entry said "the same thing". `platform` lets a host run a project's slow tests after a land (`[verify] mode = "pipeline"`) and still decide no land. `land-locally` names the one push to `main` that isn't a land, the release workflow's merge of its version bump. `ai-writes-code` says code reaches `main` from an AI session's branch, since a small fix takes no build.

**Capabilities.** `unattended-runner` folded into `build-from-spec`, `measure-by-evals` was added, blocked by the plan until the eval method is built, and scenarios the product's boundary can't observe were reworded: `adopt-clm`, `propose-intent`, `right-model-per-task`, `guard-sessions`, `report-defects` and `work-from-any-agent-harness`. `catch-drift` and `no-dead-weight` restate the constraints `no-silent-drift` and `no-dead-code`; they stay, as proposed cuts for the human to decide later.

**Size.** The file is 5,700 words, about two thirds of what it was. The statements are about 1,800 words; the scenarios and proof blocks the format requires of every entry are the rest. Reaching the plan's third needs fewer entries or a bundle that carries only the statements, which is left to the plan.
