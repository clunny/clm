---
status: applied
resolves: [a-requirement-merged-into-another-lists-none-of-the-sentences-it]
---

## Intent

The gate lists the `SHALL` sentences a requirement loses when it is merged into another: a requirement whose every scenario moves is counted with the requirements that took its scenarios, and a merge that states fewer sentences in all is the cut `<ids> merged into <ids> state <n> sentences where they stated <m>`, which makes the branch need review (`backlog/a-requirement-merged-into-another-lists-none-of-the-sentences-it`). Recorded as `a-merge-is-counted-whole`.

## Problem

- `gate/cuts` lists no `requirement <id> is gone` line for a requirement whose every scenario moves under its name to another requirement, and counts sentences only for a requirement both graphs hold. So a merged requirement loses every sentence it stated with no line.
- In `20261122-wave-4-orchestrator-by-the-rule`, `orchestrator.run/says-component` and `orchestrator.triage/says-component` lost their promise that the prompts and messages use no retired word, and the gate listed neither; the change named them by hand under "Cuts the gate doesn't count", as wave 3 did for `sync/agents-md-spec-path`. An unattended draft that doesn't notice lands such a branch on checks.

## Approaches

- **Taken: count a merge whole.** A *merge* is a merged requirement, each requirement that took one of its scenarios, and any other merged requirement those took from. Its stating sentences are summed on each side (a requirement a side doesn't hold counts none), and fewer at the head is a *shortened merge* cut. It extends the shortened-requirement rule to the one case that rule can't see, and a merge that carries its sentences over still lands on checks.
- **Rejected: a line `requirement <id> is merged into <id>` for every merge.** Simpler, but every slimming merge that drops nothing would need review, while a reword of a requirement both sides hold that keeps its count lands on checks.
- **Rejected: each merged requirement against its takers alone.** Two requirements merged into a third would each look kept against the third's count.

## Outcome

- `gate/cuts` (modified): defines which requirement *took* a moved scenario (the scenarios of a name taking the gone ones in the same order), a *merged requirement* and a *merge*; adds a fifth kind of cut, the *shortened merge*, listed after the shortened requirements by the ID of its first merged requirement, as `specs/<path>/SPEC.md: gate/cut: <ids> merged into <ids> state <n> sentences where they stated <m>`, the path the first merged requirement's component's. A requirement both sides hold that is in a merge is still compared on its own, so its *shortened requirement* line and the review versions 1 to 7 gave it stand (architecture audit, `audit-by-the-rules-in-force`). Five scenarios added (*a merge that loses a sentence*, *a merge that keeps its sentences*, *two merged into a new requirement*, *a merge into another component*, *a requirement that took scenarios and lost a sentence*). The `GIVEN` of *moved to another component* now carries `a/x`'s statement into `b/y`'s, and that of *two requirements merged* says the three state one sentence, so both outcomes stand under the new count.
- `gate/rules-version` (modified): `RULES` is 8; under versions 6 and 7 a shortened merge makes no branch need review, and versions 1 to 5 build on 6 as before; scenario *a merge that loses a sentence under each version* added.
- `gate`'s Design names the merge in **Cuts** and version 8 in **Rules versions**.
- `docs/decisions/a-merge-is-counted-whole.md` (new); `cuts-are-what-a-program-lists`, `yes-no-review`, `docs/guide/glossary.md` (**Cut**) and `docs/guide/how-it-works.md` name the new cut.

## Constraints

- No cut the gate lists today stops being listed, and the order of the existing kinds is kept, so every earlier rules version's answer stands for every diff.
- `cuts`, `cuts_or_unloaded` and `evaluate_by` keep their signatures; `Cut` gains a variant, which no crate outside `clm-gate` matches on (`web.worker.review`, `host`, `check.health`, `land` and `orchestrator.run` print a cut through its `Display`).
- `cuts` still runs no process and builds for `wasm32-unknown-unknown`.

## Edge cases and workarounds

- A gone requirement some of whose scenarios move and some don't is still a *gone requirement*, as now, and is in no merge.
- A requirement both sides hold that gives a scenario to another, and stays, is in no merge; its own shortened count stands as now.
- A merge that keeps its count while stating something else stays a reader's, as a reworded requirement does.
- Three fixtures of `crates/clm-gate/tests/` move a scenario into a requirement that keeps only its own sentence, and now give a merge cut: `cuts.rs::moved_to_another_component`, `cuts_gate.rs::scenario_moved_to_another_component` and `cuts.rs::a_moved_scenario_is_not_compared`; the build gives the requirement that takes the scenario the merged one's sentence too, as *moved to another component*'s `GIVEN` now says. No capability test merges requirements (builder check).
- `crates/clm-gate/tests/rules_version.rs` asserts `RULES` is 7, which `gate`'s build updates; `clm-ledger/tests/ledger.rs` writes `Gate-Rules: 7` for a light land, which stays true, since versions 7 and 8 treat a light branch alike.
- `plugin/skills/propose/SKILL.md`'s list of what makes a branch need review (line 7) doesn't name a merge, as it doesn't name a changed outcome; `backlog/the-propose-skill-lists-no-changed-outcome-among-the-cuts` already carries that list, and its fix names what the gate lists then.
- `ARCHITECTURE.md`'s `removal-needs-a-yes` says the gate lists "a requirement that states fewer sentences", which stays true of a merge read as a whole, so it is left as it is.

## What the requirements no longer state

Nothing: no `SHALL` sentence is removed and no scenario's outcome changes.

## Open questions
