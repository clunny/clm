---
status: applied
resolves: [propose-skill-teaches-the-retired-forced-fix-rule-for-widened-types]
---

# Widened-type requirements stop citing the retired forced-fix rule

## Intent

Take the retired forced-fix rule (`forced-fixes-cover-additions`, superseded by `coupled-changes-sequence`) out of the three requirements that still cite it: `config/host-setting`, `host/azure-pull-requests` and `host/azure-selected`. Each is reworded to say what holds now, that the `azure` value selects `clm-host`'s `Azure` adapter through `adapter`'s arm for it, and no longer how a past build split the work. No behaviour changes.

The skill half of the item is already fixed in the spec by `20261112-components-own-their-tests`: `plugin/builder-check` now states the sequencing rule (`path-ownership`, `coupled-changes-sequence`, the default arm marked `clm:remove-by`) and its scenario `skill without the widened type` names `coupled-changes-sequence`. `plugin/builder-check` is `behind`, so `plugin/skills/propose/SKILL.md` and `crates/clm/tests/plugin.rs` follow when it builds; that build waits on `backlog/builder-check-text-lives-in-the-propose-skill-file`, the open question of which component's requirement states the skill's paragraph, since the skill file is `plugin.propose`'s. This change adds nothing for that half.

## Problem

`coupled-changes-sequence` withdrew the forced fixes: no build edits another component's code to follow a change, and a widened type is followed by the user component's default arm marked `clm:remove-by`. Three requirements still cite `forced-fixes-cover-additions` as the reason for their shape, describing how `20261106-azure-devops-host` was built rather than what the code does, which specs no longer do since `20261107-specs-describe-the-present`. A session reading them beside the draft prompt or the new `plugin/builder-check` meets two contradicting rules for widened types.

## Approaches

- **Reword the three requirements to the present (chosen).** The arm already exists in `clm-host`'s `adapter`, so the history adds nothing a build needs; dropping it leaves each requirement stating only what holds. Each becomes `behind`, and its build only re-cites the tests at the new versions.
- **Leave them, since the code is built.** Rejected: the text keeps teaching a retired rule to every session that reads `clm context config` or `clm context host`.
- **Also edit `plugin/skills/propose/SKILL.md` here.** Not possible on a spec branch, and not needed: the requirement already says the new text, and the open question decides who builds it.

## Outcome

- `config/host-setting`: its last sentence says the `azure` value selects `clm-host`'s `Azure` adapter (`host/azure-pull-requests`, `host/azure-selected`).
- `host/azure-pull-requests`: the adapter is public and callable directly, so its tests call it directly; no forced-fix rationale.
- `host/azure-selected`: `adapter`'s arm for the `azure` value gives the adapter, and its tests prove it through the commands; no forced-fix rationale.
- Nothing in code changes; the builds of `config` and `host` move their tests' markers to the new versions.

## Constraints

- Scenarios and `#### Serves` are unchanged. `config/host-setting` drops its `#### Needs` on `host/azure-pull-requests` and `host/azure-other-operations`: they ordered the forced-fix arm after the adapter, and the arm is built, so `config` re-cites without waiting on `host`. `host/azure-selected` keeps its need on `config/host-setting`.

## Edge cases and workarounds

None.

## Open questions

None.
