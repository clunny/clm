---
status: done
resolves:
  - init-keeps-its-own-copy-of-the-exemption-rules
---

## Intent

`clm survey --components` judges an `exemptions` entry by the model's own check, so the two can't disagree. The model offers that check in its Contract, loading runs the same function, and `init` drops the rules it wrote for itself.

## Problem

`init/survey-components` says a proposed component map is refused for an exemption "the model would refuse". `crates/clm-init/src/nodes.rs` decides that with rules of its own (`exemption_globs`), written while `model/exemptions` was unbuilt. The model's check is built now, but it is private to `clm-model` (`bad_exemptions`), so `init` can't call it.

The two copies already disagree: the model refuses `until: backlog/a/b` and `until: "backlog/a b"`, and `init` accepts both. So `clm survey --components` accepts a map that fails `clm check` once it is written as the spec graph, which is what the command exists to prevent. `init`'s test feeds a hand-written list of bad entries and asserts only that the message names `exemptions`, so it passes whatever the model does.

The design is incomplete, not wrong: the requirement already names the model as the judge, and the model had no way to be asked.

## Approaches

- **The model exposes its check of an `exemptions` value, and loading runs it** (chosen). `clm_model::check_exemptions` takes the value as parsed YAML and returns the entries or one message per problem. The model's frontmatter check judges `exemptions` by that function alone, and `init` calls it for each component of the map. One rule, one place, as `graph-check` and the frontmatter check already do for `apply`.
- **`init` builds a frontmatter text around each entry list and runs `check_node_frontmatter`.** Rejected: it needs no Contract change, but `init` would have to invent a title, a status and the other required keys, and take apart violations that name a `SPEC.md` that doesn't exist. That is a workaround for a missing entry point (`redesign-before-workaround`).
- **Keep both copies and add a test that feeds each the same entries.** Rejected: it proves agreement only for the entries the test lists, and leaves two rules to maintain (`reuse-first`).
- **Drop the check from `clm survey --components`.** Rejected: the map would then pass the survey and fail `clm check` when written, and recovery would learn it a step late.

## Outcome

- `model/exemptions`: the model exposes `clm_model::check_exemptions`, and loading and the frontmatter check judge a component's `exemptions` by it and by nothing else. A value that isn't a list is now reported by it, under `model/exemptions`, instead of by the typed parse under `model/valid-frontmatter`; the file still fails to load.
- `model`'s Contract names the exposed check.
- `init/survey-components`: the survey judges each component's `exemptions` by that check, holds no rule of its own, prints each message the check gives with the file and the component, and takes the files exempt from `undeclared-imports` from the entries the check returns. It needs `model/exemptions`.

No capability test sees anything different at the product's boundary: none passes a component map with a refused exemption to `clm survey --components`.

## Constraints

- `init` builds after `model` (`#### Needs` on `init/survey-components`), since the function doesn't exist until `model/exemptions` is built.
- The check's signature holds a `serde_yaml::Value`. Both crates already take `serde_yaml` from the workspace, and the model's public types already derive from it, so this adds no dependency.

## Edge cases

- A null `exemptions` value (`exemptions:` with nothing after it) is refused by both, as a value that isn't a list.
- A map with several components whose values are refused: the survey may stop at the first, printing every message for that component's value, as it stops at a map's first parse error. The requirement says so.
- A `why` that isn't a string is refused by the check with a message naming the entry, where the typed parse reported it under `model/valid-frontmatter` without the entry's position.

No workaround is known to be needed.

## Open questions

None. The architecture audit had no findings; its two observations (what the survey prints when several components are refused, and how the exposed check sits beside `model/optional-nodes`' rule against a frontmatter check with no component ID) are settled in the requirements' text.
