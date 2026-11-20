---
status: applied
resolves: [machine-report-states-total-zero-readings-the-code-judges-otherwise]
---

## Intent

The plan `backlog/a-passing-check-proves-less-than-the-spec-claims` ends with phase 5, as the human settled on 2026-11-20 after wave 1's measurement: all seven waves are finished, so that every component's spec states what CLM builds, and phases 6 to 8 (evals, brakes that gate, a second project) become backlog items for after the plan. The rule for a requirement gains what wave 1 showed it lacked, and the one question that keeps wave 1's change from being archived is answered.

## Problem

- **Wave 1 made its nine components larger.** 139 requirements became 142, spec tokens rose 16% and code tokens 10%. Measured by section, in characters, from the commit before the wave's branch to `main` at `cf87a27f`: Designs 52,152 to 41,410, Contracts 27,945 to 36,856, requirement statements 107,309 to 125,276, and scenarios 128,463 to 166,096, which is two thirds of the growth (496 scenarios became 631, for about 60 newly stated clauses). So the waves serve a spec that is true and proven, and not "the requirement count is well below 932" under the plan's Done means.
- **The plan grew while it ran.** Phase 2 left 852 requirements and `main` holds 940; the freeze has about a dozen exceptions; the backlog holds 89 items, most filed by the plan's own builds. Phases 6 to 8 are new work, not rework.
- **The rule has no stopping point.** Everything code does at its boundary is observable, so a reader states it all, and wave 1 wrote more than two scenarios for each clause.
- **`machine/machine-report` is blocked by a question.** Wave 1 stated two clauses about a reading whose total is 0. The build's audit read the memory one as false, since `clm machine` prints `mem  -  unknown` for a total of 0 from the system, and found the disk one false for a total of 0 with space free.

## Approaches

- **Taken: finish phase 5 and end the plan there.** An unstated behaviour has no protection under CLM's own rules: `clm land` refuses the loss of a test only when it cites a requirement, and `no-dead-code` removes code that serves nothing written. So the waves state what the code does at its boundary, odd input included, and the spec ends larger than it is.
- **Not taken: stop phase 5 at wave 1.** The other 42 components would keep Contracts and requirements wrong in the ways wave 1's nine were.
- **Not taken: state a clause only when something named relies on it.** Proposed in this session and withdrawn: a case such as a clock stepped back looks like a detail and is most likely there because something broke once, and a build that isn't told of it can remove it.
- **Taken: economy of form, and two exclusions.** One scenario for a clause, its cases inside it; no clause for input that can't reach the code; and a reader asks whether the code or the requirement is right before writing down what the code does.
- **`machine-report`, memory: the requirement was right and stays; the Design says what the audit missed.** `judge` and `text` are calls the Contract offers, and for a memory reading whose total is 0 they do as the requirement says, which a test proves. `sysinfo` reports a total of 0 when it has no reading, and `read` takes that as no reading, so the command prints `mem  -  unknown` with `null` numbers: how `read` maps it is the Design's, and "`null` when the system gave no memory reading" already states what is seen.
- **`machine-report`, disk: the requirement keeps its meaning and the build mends the code.** A disk whose total is 0 is `critical`; the code judges it so only with nothing free, and `ok` with space free, a reading `statvfs` can't give and a caller of `judge` can. A scenario states both zero-total cases, as wave 1 settled for `rename/replay`.
- **Not taken: a sentence that a total of 0 from the system is no reading, and dropping what `judge` gives for one.** Drafted on this branch and withdrawn on the architecture audit: the sentence's subject is what a dependency hands the component and no test can prove it, and dropping the `judge` clause leaves a guard and a test that no requirement states.
- **Not taken: printing `0B available of 0B` from the command.** It would hand the panel and the build loop a number for a reading that wasn't taken.

## Outcome

- The plan item: phase 5's paragraph records what was settled on 2026-11-20; phases 6 to 8 are replaced by a line naming their backlog items; Done means loses the requirement count and the budget clause, and gains the waves; the open questions that belong to evals or the brakes move with them.
- Three `idea` items hold what the plan had settled or noted for phases 6 to 8: `no-eval-proves-behaviour-only-a-model-produces` (which now blocks `capability/measure-by-evals`), `the-brakes-on-the-spec-s-growth-only-advise` and `the-method-has-only-been-tried-on-clm-s-own-repository`.
- `docs/decisions/requirements-state-what-the-contract-shows.md` and the guide's spec-format page gain the two exclusions and the scenario form.
- `machine/machine-report` (modified): one scenario, *a total of 0*, for the two clauses wave 1 stated; its statement is unchanged. `machine`'s build makes a disk whose total is 0 `critical` whatever is free, and proves it. `machine`'s Design says `read` takes a memory total of 0 from the system as no reading.
- `backlog/the-spec-slimmer-s-prompt-predates-what-wave-1-settled` (`deferred`, `plugin.agents`): the prompt is source, so a code branch brings it up to the rule before wave 2 launches it.

## Constraints

- Nothing `clm machine` prints changes: `statvfs` gives a total of 0 only with nothing free. What `judge` returns changes for a disk reading of total 0 with space free, from `ok` or `warn` to `critical`.
- This change names the plan item in neither `from` nor `resolves`, since `clm apply` would delete it.

## Open questions

- Whether a wave's build run designs fixes for what its builds find, or only builds and files them, is not answered; the session that opens wave 2 asks.
- Three questions wave 1's builds filed stay for the human: `bounded-runs-asks-for-a-wait-no-code-can-know-to-make`, `no-rule-states-that-a-refused-optional-root-leaves-its-tree-required` and `no-test-runs-clm-fingerprint-the-command`. The plan item holds this session's recommendation for each.
- Done means still asks for an `ARCHITECTURE.md` under a third of its size on 2026-11-13; it is at 66%, and the plan's Open questions still holds the choice.
