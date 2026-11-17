---
title: "A bug is fixed on a code branch only when a requirement as written states the behaviour the fix restores"
date: 2026-10-30
status: accepted
---

`code-fix-resolves-bug` let a `kind: bug` item whose fix changes no spec path be fixed straight on `impl/<node>`. On 2026-10-30 the `clm apply` defect was fixed that way and both reviews found its tests citing requirements whose text didn't state the fixed behaviour; the fix needed `apply/loaded-graph` first. A fix that no requirement states is code that serves nothing written: when the rule its test cites is reworded, the test is rebuilt to the new text, and a slimming or `no-dead-code` pass can remove the code without anyone seeing a requirement go.

So the code-branch route is for a bug that breaks a requirement or scenario as written, and the fix's test cites it. Anything else starts with a spec branch that adds the requirement or scenario, usually small and landing on checks, and the code fix follows citing it. `propose` and `clm draft`'s prompt say so; the `spec-auditor` flags a test whose cited rule doesn't state what it checks. Judging that is reading, not a program's, so it advises (`deterministic-enforcement`).

**An `ARCHITECTURE.md` entry is not a requirement as written.** An entry, carried or the project's own, doesn't count for this rule, since a test never cites an entry (`human-layer-proof`). A bug whose behaviour only an entry states is a bug no requirement states, and its fix starts on a spec branch that adds the requirement, unless its work is test plumbing, below.

**Outside the rule: the capability harness and test plumbing.** Two kinds of bug restore no behaviour a person or test observes, so there is nothing for a test to cite and nothing for a requirement to add:
- A bug in the capability suite's harness alone. It restores only the Design of the component that builds the capability tests, which holds no requirement of its own.
- Test plumbing in a component's own test suite: a fix that changes only how a component's tests run (its runner settings, the ports and servers its tests start, the paths they write) and nothing any of its tests asserts. `web/playwright.config.ts` reusing a preview server another run started was one, which only `tests-run-concurrently` states; a `web` requirement about its own test runner's ports would have stated test plumbing as a rule of the product.

Each is fixed on a small-change branch with no claim, whose commit deletes the item with `Resolves: backlog/<slug>`, never with `clm claim --fix` (`human-layer-proof`, `small-change-no-claim`). A test such a fix adds, which `small-change-no-claim` allows, cites no requirement, since none states plumbing, and is kept by the entry it checks, as the harness's tests are.

**Outside the rule: the fixed text of a prompt or skill** (2026-11-16). `prompt-text-is-source` makes the skills, agent definitions, command files, the fixed text of the prompts `clm run`, `clm draft` and the build loop write, and the instruction files source their component owns, and no requirement may state what they tell a session. A bug in that wording, such as a step that sends a session the wrong way, can neither cite a requirement nor add one, so this rule can't apply to it. It is fixed by component as a code-only bug is, with `clm claim <component> --fix backlog/<slug>` on a component the item names whose `paths` hold the text, and a commit whose trailer block holds `Resolves: backlog/<slug>` (`code-fix-resolves-bug`); work of the same kind filed as `kind: deferred` closes the same way (`deferred-work-resolves-in-code`). It adds no test of the wording, since a scan of prompt text is what `prompt-text-is-source` removed; a value a program computes into the prompt is still a requirement, and a fix to it follows the rule above. `AGENTS.md` and `CLAUDE.md` are spec paths here, so a fix to their wording goes on a branch with no change, as `small-change-no-claim` says of an edit to `docs/` or `CLAUDE.md`. Found when `20261116-a-build-whose-only` met a prompt step its draft had to work out the route for.

The same day showed the other half: `web/loading-indicator` and `web/graph-fills-height` had requirements, but nothing in the human layers asked for them, so the re-founding cut them as tracing to nothing. `ui-quality` in `ARCHITECTURE.md` names them under `#### Enforced by`, which puts their removal under review.

Rejected: a gate rule refusing a `Resolves:` code fix whose test cites no requirement added on the branch (most real bugs do break a rule as written, and a program can't tell which).
