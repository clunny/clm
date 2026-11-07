---
component: orchestrator.run
---

## ADDED Requirements

### Requirement: prompt-slot
The `clm run` prompt SHALL name the build's slot, what `place_slot` returns for `impl/<component>` once `clm run` has taken its place (`orchestrator/place-slots`), and SHALL tell the session that the project's tests run beside other builds' tests on this host (`tests-run-concurrently`), so:
- every test command it runs by hand, a spot-check's runs of a mutant included, runs with `CLM_SLOT=<slot>` set before the command, while `clm land --check-only` sets `CLM_SLOT` itself and is still run as `prompt-check-command` says, with no variable;
- a test it writes binds no fixed port, writes no fixed path outside its checkout and the system's temporary directory, and starts no server another run could share, keying a port or path it must know in advance on `CLM_SLOT`;
- a test of the project that fails because another run holds a port or a path is a defect of the project's tests, which it reports as a `bug` backlog item whose body names `tests-run-concurrently` when the fix lies outside its component, rather than retrying it.

This text SHALL be a paragraph of its own: the paragraphs `prompt-check-command` governs SHALL hold no `CLM_SLOT=`.

The text SHALL be in every prompt, fresh or resuming, and keep to the wording the component's other requirements allow (it names no `cd`, exit code or `clm watch`).

#### Scenario: prompt
- GIVEN a ready component `b` and a place `a` with slot 1 that counts, once fresh and once resuming a branch with unlanded commits while the place `b` the first run took still counts
- WHEN `clm run b` runs
- THEN each prompt it writes contains `CLM_SLOT=2`, `tests-run-concurrently` and `fixed port`, and no paragraph that tells the session to run `clm land --check-only` as one plain command holds `CLM_SLOT=`

#### Needs
- orchestrator/place-slots

#### Serves
- architecture/tests-run-concurrently
- capability/work-concurrently
