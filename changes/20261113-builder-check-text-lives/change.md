---
status: applied
resolves: [builder-check-text-lives-in-the-propose-skill-file, reconsider-plugin-builder-check, workaround-plugin-builder-check-1]
---

## Intent

Move the builder-check rule to the component that owns the file it constrains. `plugin/builder-check` says what `plugin/skills/propose/SKILL.md` tells Claude, but that file is `plugin.propose`'s (`paths: plugin/skills/propose/**`), and `plugin`'s build may not edit it (`path-ownership`). So the requirement's latest wording, `coupled-changes-sequence` in place of the retired `forced-fixes-cover-additions`, can never be built. The requirement, its scenarios and its tests move to `plugin.propose` as `plugin.propose/builder-check`, beside `builder-check-asks-migrations` and `builder-check-capability-tests`. `plugin`'s Contract keeps one pointer and now states the rule the question waited on: a rule that asks something of one child's file alone is that child's requirement.

This answers both items: the question of which component states the paragraph, and the reconsider item for `plugin/builder-check` (score 6: one workaround, four rewrites, ten scenarios). The workaround flag `backlog/workaround-plugin-builder-check-1` goes with the requirement it flagged; this branch deletes it. The ten scenarios become five on the new requirement, each grouping the names one test checks, so the requirement starts under the scenario setting of 6.

## Problem

A requirement of `plugin` constrains text in a file only `plugin.propose` may edit. `plugin`'s build left it `behind` and flagged a workaround; no `plugin.propose` requirement states the wording, so no build can bring the skill up to it. The skill still holds the `forced-fixes-cover-additions` paragraph the spec retired.

## Approaches

- **Keep `plugin/builder-check` as a shared rule and add a `plugin.propose` requirement for the paragraph, named in its `#### Needs`.** That is how `plugin`'s Contract says a shared rule reaches a child, but the rule isn't shared: it asks something of one skill only. Two requirements would state one paragraph. Rejected.
- **Widen `plugin`'s `paths` to the skill file.** Breaks the nested split (`nested-splits`): two components would own one file. Rejected.
- **Chosen (the human's answer): move the requirement to `plugin.propose`**, the file's owner, as `plugin.propose/builder-check`. Its build edits the skill and moves the tests.

## Outcome

- `plugin.propose/builder-check`: the text of `plugin/builder-check` as it stands, without its last sentence about the build loop's queue (that is `orchestrator.loop/humans-queue`, which `plugin`'s Contract still points to). Five scenarios: the check's own step before `clm pr open` with `## Open questions`; `blocks` and `Needs`; the four sequencing names (`contract-removal-needs`, `rename-adds-then-removes`, `coupled-changes-sequence`, `contract-step-deferred`); `builds nothing`; `builder-check-asks-order` and `its own component`. Its build replaces the skill's `forced-fixes-cover-additions` paragraph with the `path-ownership` and `coupled-changes-sequence` one, and moves `skills_run_a_builder_check` and `builder_check_reapplies_through_rebase` from `crates/clm/tests/plugin.rs` to `crates/clm/tests/plugin_propose.rs`, citing the new requirement (the Design's `Tests` paragraph).
- `plugin/builder-check` is removed. `plugin`'s Contract bullet becomes `architecture-audit`, pointing at `plugin.propose/builder-check`, and its paragraph on children's files says a rule that asks something of one child's file alone is that child's requirement.
- `plugin.propose`'s Contract says the rules that ask something of its file alone are its own, the builder check's among them.
- The live change `20261112-components-own-their-tests` modified `plugin/builder-check`; the removal and the addition in this change are a move (`progress-follows-moves`), so that change counts `plugin.propose/builder-check` in its place.

## Edge cases and workarounds

- Between the two builds the old tests' markers in `plugin.rs` are `obsolete` and the new requirement is `unbuilt`; whichever builds first, the other's tests keep passing, since both read the same skill.
- The question item's `blocks: [plugin/builder-check]` is dropped on this branch: `clm check` models the resulting graph without `clm apply`'s deletion of answered questions, so a `blocks` naming the removed requirement fails it.

## The human's answers

> builder-check-text-lives-in-the-propose-skill-file: Move to plugin.propose (Recommended) — move the requirement, its scenarios and its tests to plugin.propose as plugin.propose/builder-check, beside builder-check-asks-migrations and builder-check-capability-tests; plugin's Contract keeps one pointer line. The file's owner then states its wording, which clears the workaround flag.

> reconsider-plugin-builder-check: Move to plugin.propose (Recommended) — same answer as builder-check-text-lives-in-the-propose-skill-file: the requirement moves to plugin.propose, its owner.

## Open questions

None.
