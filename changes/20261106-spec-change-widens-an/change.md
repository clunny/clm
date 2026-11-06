---
status: applied
resolves: [spec-change-widens-an-exhaustively-matched-enum-with-no, azure-host-variant-breaks-clm-host]
---

# A widened enum's arm lands with the variant, once its adapter is built

## Intent

Let `20261106-azure-devops-host` build, and let the next change that widens an exhaustively matched type build too. `host` builds its `Azure` adapter first, tested by calling it directly, with no `#### Needs` on `config/host-setting`; then `config/host-setting` adds `Host::Azure` and, as a forced fix, the arm `Host::Azure => Box::new(Azure)` in `clm-host`'s `adapter`; then `host`'s requirements that reach the adapter through the setting (the new `azure-selected`, `host-for-remote`, `host-setup-steps`) and `clm/init-host-values` build on it. `forced-fixes-cover-additions` is refined: an arm for an added variant is a forced fix when the other component's requirement names what the variant selects and that code is built. The `clm run` prompt names the case, and the draft prompt and the `propose` skill tell the builder check to look for exhaustive uses of a widened type and order the builds this way.

## The human's answers

## azure-host-variant-breaks-clm-host

Adapter first, then wire:

1. host: azure-pull-requests, azure-other-operations
   build `Azure` adapter, tested directly
   (Needs config/host-setting dropped)
2. config: host-setting adds Host::Azure
   + forced arm  Host::Azure => Box::new(Azure)
   (allowed: selects built code its spec names)
   + `clm init --host azure` string arm
3. host: host-for-remote, host-setup-steps
   (still Need config/host-setting)

Decision forced-fixes-cover-additions refined;
resolves azure-host-variant-breaks-clm-host.

## Problem

`config/host-setting` adds the value `azure`, so `clm_config::Host` gains `Azure`, and `clm-host`'s `adapter` matches `Host` exhaustively. `forced-fixes-cover-additions` ruled an arm for an added variant out of the forced fixes, since it decides what the other component does; and every `host` requirement for Azure needed `config/host-setting`. Neither side could build first (`backlog/azure-host-variant-breaks-clm-host`). The draft that wrote `20261106-azure-devops-host` didn't catch it: its builder check looked for exhaustive uses only of what a change removes or renames, never of what it widens (`backlog/spec-change-widens-an-exhaustively-matched-enum-with-no`).

## Approaches

- **Adapter first, then wire (chosen, the human's answer).** The adapter's requirements test `Azure` directly and need nothing; the widening needs them and adds an arm that only selects built code a requirement names, so the arm decides nothing; the requirements that use the setting need the widening. The `#### Needs` blocks carry the order, one change, no placeholder.
- **A placeholder arm (`Host::Azure => Box::new(NoHost)`).** Ships `azure` doing nothing, which no requirement states. Rejected.
- **Two changes, the variant hidden until the adapter exists.** Two reviews and a deferred step for an order `#### Needs` already expresses. Rejected.

## Outcome

- `docs/decisions/forced-fixes-cover-additions.md` refined: the arm that selects built code its spec names is a forced fix, and the three-step order for a widened type.
- `ARCHITECTURE.md`'s `path-ownership` names the case among its forced fixes, with a scenario *added variant selects built code* (the architecture audit's finding: without it the constraint forbade what the decision allows).
- `host/azure-pull-requests` and `host/azure-other-operations` state the `Azure` adapter's operations, tested by calling the adapter, and drop `#### Needs: config/host-setting`.
- `host/azure-selected` (added): with `host = "azure"`, `adapter` gives `Azure`, proved through `clm pr open` and `clm issue`; needs `config/host-setting`. `capability/git-host`'s Azure DevOps scenario follows from it.
- `config/host-setting` needs the two adapter requirements and says its build adds the arm as a forced fix.
- `orchestrator.run/removal-breaks-dependant`: the `clm run` prompt says such an arm is a forced fix, with a scenario.
- `orchestrator.triage/draft-contract-step-deferred` and `plugin/builder-check`: the draft prompt and the `propose` skill tell the builder check to search for exhaustive uses of a widened type and order the builds as the decision says.

The human's step 2 lists the `clm init --host azure` string arm beside the forced arm. That match in `crates/clm/src/main.rs` doesn't stop compiling when `Host` widens, since clap allows only the values it lists and the last arm is `unreachable!`, so it isn't a forced fix (`path-ownership`): `clm/init-host-values` adds the value and its arm, and it already needs `config/host-setting`, so it builds straight after, in the same wiring step. `clm-init`'s host detection compares with `if`, not an exhaustive match, and moves to `host_for_remote` under `init/init-files`, which already needs `host/host-for-remote`.

## Constraints

- `path-ownership`, `forced-fixes-cover-additions`: config's build edits `clm-host` only for the arm, named under `## Fixes in other components`.
- `requirement-needs`, `needs-alone-order`: the order is the `#### Needs` blocks; `config/host-setting` needing a `host` requirement is a need against `depends_on`'s direction, which `needs-alone-order` allows.
- `yagni`: no new mechanism, only the order and one exception.

## Edge cases and workarounds

None known.

## Builder check, settled

- Order: `host/azure-pull-requests` and `host/azure-other-operations`; then `config/host-setting`; then `host/azure-selected`, `host/host-for-remote`, `host/host-setup-steps` and `clm/init-host-values`; then `init/init-files` and `init/init-next-steps`. No cycle. The prompt-text requirements rely on none.
- The adapter is exported as `clm_host::Azure`, so its tests call it directly with a fake `az` on `PATH` (now stated in `azure-pull-requests`).
- The requirement that names what the variant selects may itself wait on the widening (`host/azure-selected` does); only the selected code must be built. Stated in the decision, `path-ownership` and the run prompt's requirement.
- `config/host-setting`'s build also corrects `crates/clm-config/tests/config.rs`, which lists `azure` as a bad value; `crates/clm/tests/init.rs`'s refusal of `--host azure` belongs to `clm/init-host-values`.
- Capability tests: only `host/azure-selected` changes what `capability/git-host`'s test sees (its Azure DevOps scenario); the rest change nothing at the boundary.

## Open questions

- Until `host/azure-selected` is built, no test catches a wrong arm (say `Host::Azure => Box::new(NoHost)`): `config/host-setting`'s mutation spot-checks can't cover its forced fix. Accepted, since the next build in the order proves it.
- Editing `path-ownership` in `ARCHITECTURE.md` (the architecture audit's finding, fixed) widens a `method` constraint's exception; the human's answer asked for the refinement, and the review shows the wording.
