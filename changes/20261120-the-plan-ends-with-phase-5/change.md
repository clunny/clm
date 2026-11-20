---
status: applied
resolves: [machine-report-states-total-zero-readings-the-code-judges-otherwise]
---

## Intent

The plan `backlog/a-passing-check-proves-less-than-the-spec-claims` ends with phase 5, as the human settled on 2026-11-20 after wave 1's measurement: all seven waves are finished, so that every component's spec states what CLM builds, and phases 6 to 8 (evals, brakes that gate, a second project) become backlog items for after the plan. The rule for a requirement gains what wave 1 showed it lacked, and the one question that keeps wave 1's change from being archived is answered.

## Problem

- **Wave 1 made its nine components larger.** 139 requirements became 142, spec tokens rose 16% and code tokens 10%. Measured by section, in characters, from the commit before the wave's branch to `main` at `cf87a27f`: Designs 52,152 to 41,410, Contracts 27,945 to 36,856, requirement statements 107,309 to 125,276, and scenarios 128,463 to 166,096, which is two thirds of the growth (496 scenarios became 631, for about 60 newly stated clauses). So the waves serve a spec that is true and proven, and not "the requirement count is well below 932" under the plan's Done means.
- **The plan grew while it ran.** Phase 2 left 852 requirements and `main` holds 940; the freeze has about a dozen exceptions; the backlog holds 89 items, most filed by the plan's own builds. Phases 6 to 8 are new work, not rework.
- **The rule has no stopping point.** Everything code does at its boundary is observable, so a reader states it all. `machine/machine-report` gained two clauses about a reading whose total is 0, written from the code and a test, one of them for a reading the system never gives the judge; the build's audit found both false as worded, and the question it filed blocks the requirement.

## Approaches

- **Taken: finish phase 5 and end the plan there.** An unstated behaviour has no protection under CLM's own rules: `clm land` refuses the loss of a test only when it cites a requirement, and `no-dead-code` removes code that serves nothing written. So the waves state what the code does at its boundary, odd input included, and the spec ends larger than it is.
- **Not taken: stop phase 5 at wave 1.** The other 42 components would keep Contracts and requirements wrong in the ways wave 1's nine were.
- **Not taken: state a clause only when something named relies on it.** Proposed in this session and withdrawn: a case such as a clock stepped back looks like a detail and is most likely there because something broke once, and a build that isn't told of it can remove it.
- **Taken: economy of form, and two exclusions.** One scenario for a clause, its cases inside it; no clause for input that can't reach the code; and a reader asks whether the code or the requirement is right before writing down what the code does.
- **`machine-report`, memory: the text follows the code.** `sysinfo` reports a total of 0 when it has no reading, and the command prints `mem  -  unknown` with `null` numbers, which is what "the system gave no memory reading" means. Printing `0B available of 0B` would hand the panel and the build loop a number for a reading that wasn't taken.
- **`machine-report`, disk: the text says what the code does for the one such reading there is.** `statvfs` gives a total of 0 only with 0 free, which the code judges `critical`; a total of 0 with space free can't occur and is left unstated.

## Outcome

- The plan item: phase 5's paragraph records what was settled on 2026-11-20; phases 6 to 8 are replaced by a line naming their backlog items; Done means loses the requirement count and the budget clause, and gains the waves; the open questions that belong to evals or the brakes move with them.
- Three `idea` items hold what the plan had settled or noted for phases 6 to 8: `no-eval-proves-behaviour-only-a-model-produces` (which now blocks `capability/measure-by-evals`), `the-brakes-on-the-spec-s-growth-only-advise` and `the-method-has-only-been-tried-on-clm-s-own-repository`.
- `docs/decisions/requirements-state-what-the-contract-shows.md` and the guide's spec-format page gain the two exclusions and the scenario form.
- `machine/machine-report` (modified): a memory total of 0 from the system is no memory reading; a disk whose total and free space are both 0 is `critical`.
- `backlog/the-spec-slimmer-s-prompt-predates-what-wave-1-settled` (`deferred`, `plugin.agents`): the prompt is source, so a code branch brings it up to the rule before wave 2 launches it.

## What the requirements no longer state

- `machine/machine-report`: that memory is "`unknown` for a reading whose total is 0, whose numbers are printed all the same". No reading the system gives reaches the judge with a total of 0. The code keeps its guard against dividing by zero and its test, which no requirement states.
- `machine/machine-report`: that "a disk whose total is 0" is `critical` whatever is free; it now says so of a disk whose total and free space are both 0.

Neither is a sentence holding `SHALL`, and both were written on 2026-11-19 by wave 1.

## Constraints

- No behaviour of `clm machine` changes: `machine`'s build re-cites its tests and changes no code.
- This change names the plan item in neither `from` nor `resolves`, since `clm apply` would delete it.

## Open questions

- No test can show that a total of 0 from the system is no reading without replacing `sysinfo`; the clause states what `read` does at the one line that maps it.
- Whether a wave's build run designs fixes for what its builds find, or only builds and files them, is not answered; the session that opens wave 2 asks.
- Three questions wave 1's builds filed stay for the human: `bounded-runs-asks-for-a-wait-no-code-can-know-to-make`, `no-rule-states-that-a-refused-optional-root-leaves-its-tree-required` and `no-test-runs-clm-fingerprint-the-command`. The plan item holds this session's recommendation for each.
- Done means still asks for an `ARCHITECTURE.md` under a third of its size on 2026-11-13; it is at 66%, and the plan's Open questions still holds the choice.
