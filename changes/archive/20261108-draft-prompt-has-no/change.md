---
status: done
resolves: [draft-prompt-has-no-route-for-a-bug-only-an-architecture-entry]
---

## Intent

State the route for a bug whose behaviour only an `ARCHITECTURE.md` entry states. An entry, carried or the project's own, is not a requirement as written, since a test never cites one (`human-layer-proof`), so such a bug is a bug no requirement states and its fix starts on a spec branch that adds one, unless its work is test plumbing. Test plumbing widens the harness-only exception from the capability suite to a component's own test suite: a fix that changes only how a component's tests run (runner settings, the ports and servers its tests start, the paths they write) and nothing any of its tests asserts is never a requirement, and takes the small-change route with no claim, any test it adds citing no requirement, its `Resolves: backlog/<slug>` trailer deleting the item. `clm draft`'s prompt (`orchestrator.triage/draft-code-fix-written-rule`) and the propose skill (`plugin/code-fix-needs-a-written-rule`) say so; `code-fix-needs-a-written-rule` is amended. Resolves `backlog/draft-prompt-has-no-route-for-a-bug-only-an-architecture-entry`.

## Problem

The draft prompt calls a bug code-only only when a requirement or scenario as written states the behaviour its fix restores and the fix's test cites it. Drafting `20261108-web-suites-take-a` met a bug in `web/playwright.config.ts`, which reused a preview server another run started: only `tests-run-concurrently` states that behaviour, and no `web` requirement covers `web`'s own test-suite config. Read literally, the prompt asked for a `web` requirement about how its own test runner treats ports, test plumbing stated as a rule of the product. The prompt's harness-only route covers only the capability suite. The draft fixed it with `clm claim web --fix` and an uncited test (`a7b3c04`), a route no rule states, and which `20261107-harness-only-capability-work` rejected for the harness, since `--fix` is for a bug a requirement states and a test cites.

## Approaches

1. **Widen the harness-only route to test plumbing (chosen).** A component's own test-suite plumbing, like the capability harness, restores nothing a person or test observes, so no requirement states it; the small-change route (`small-change-no-claim`) and the `Resolves:` trailer (`code-fix-resolves-bug`) already close an item without a claim. Product behaviour that only an entry states keeps the existing rule: it gets a requirement. Reuses the route that exists (`reuse-first`).
2. **`clm claim <component> --fix` with an uncited test.** What the draft did. Rejected: it makes `--fix` mean two things, and `20261107-harness-only-capability-work` already rejected it for the harness for that reason.
3. **A requirement of the component for its test plumbing.** Rejected: it states how a component's tests run as a rule of the product, which no person relies on, and grows the spec with rules that serve only the suite (`slim-components`).
4. **Let a test cite an architecture entry.** Rejected: `human-layer-proof` proves entries through the requirements under `#### Enforced by`, and a cited entry would bypass them.

## Outcome

- `orchestrator.triage/draft-code-fix-written-rule` (modified): the prompt says an architecture entry is not a requirement as written, and routes test plumbing, of the capability suite or a component's own tests, as harness-only work.
- `plugin/code-fix-needs-a-written-rule` (modified): the propose skill's code-fix section says the same.
- Hand edits: `docs/decisions/code-fix-needs-a-written-rule.md` and `docs/decisions/small-change-no-claim.md` amended; `docs/guide/glossary.md` gains **Test plumbing** (`plain-terms`); `docs/guide/how-it-works.md` and `docs/spec-format.md`'s "Proving a capability" name the route (`method-in-the-plugin`).

## Constraints

- `code-fix-needs-a-written-rule` still holds for every bug whose behaviour a person or a test observes.
- `human-layer-proof`: no test cites an entry; entries stay proven by the requirements that enforce them.
- `small-change-no-claim`: test plumbing adds nothing a requirement would have to state, so the route fits as written.

## Edge cases and workarounds

- A component whose Design states its test plumbing and must change: the change edits the Design and adds a `kind: deferred` item for the code, as for the harness.
- A fix that is both product behaviour and plumbing: the product part needs a requirement; only the plumbing part takes the small-change route.

## Open questions

- Architecture audit: `CLAUDE.md`'s line "A `kind: bug` or `kind: deferred` item takes `clm claim <component> --fix` instead" doesn't name the test-plumbing exception (nor, before this change, the harness one). Left unedited by this unattended draft: a session doesn't edit `CLAUDE.md` on an agent's say-so; the decisions and guide state the route. The human may add the exception there.
- Architecture audit (`no-dead-code`): a test a test-plumbing fix adds cites no requirement. Kept, as `small-change-no-claim` allows a test: it checks the plumbing an entry such as `tests-run-concurrently` asks for, and is removed with that plumbing.
- Builder check: what counts as plumbing (a shared fixture, a test helper, a mock) is judged by reading: anything that sets up what a test asserts is not plumbing. Nothing enforces it (`deterministic-enforcement`).
