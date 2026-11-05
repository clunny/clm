---
status: proposed
---

## Intent

CLM's own architectural constraints that govern how any project is built with it (proof by tests, landing through `clm land`, expand-contract, no dead code, and the rest) reach a project built with CLM only as a frozen, hand-kept, incomplete list `clm init` writes once. This change flags each of CLM's constraints that carries to every project with one of three tiers, `method`, `replaceable` or `default`, and has the installed `clm` supply the flagged set: `clm context` adds it to every build agent's bundle and `clm guide constraints` prints it, so a project always holds the version its `clm` enforces. A project may replace a `replaceable` constraint, provided its own constraint keeps the guarantee CLM relies on, or drop a `default` one, by an entry of its own `ARCHITECTURE.md` with a `#### Replaces` block; a `method` constraint can't be replaced. It also adds `/clm:help`, a read-only skill that answers a new user's questions from the installed guide, the carried constraints and the project's state.

## Problem

- `clm init` writes an `## Enforced by clm` section listing 12 of CLM's constraints as prose. It is written once and never updated, so a project never sees a constraint CLM adds or rewords.
- The list is incomplete: `expand-contract`, `slim-components`, `land-locally`, `path-ownership` and `traced-rules` hold in every project and are checked there, yet aren't on it, while the plugin's skills tell Claude to "split it per `expand-contract`", a name that means nothing in a project built with CLM.
- Nothing in CLM's `ARCHITECTURE.md` says which constraints are the method every project follows and which are CLM's own (`web-optional`, `invite-only-web`, `ui-quality`), so the list is kept by hand.
- Some constraints are opinions CLM's machinery doesn't rely on (`yagni`, `reuse-first`), and some name a technique where CLM relies only on a guarantee (`expand-contract`: every land leaves `main` green). A project can neither drop the first nor substitute the second, and nothing tells it why it shouldn't.
- A new user has no skill to ask "how do I...?"; the guide is in the binary but nothing reads it for them.

## Approaches

- **A. Copy at init, generated from a flag.** The flag picks the constraints, and `clm init` writes them into the project. Simple, but still frozen once written.
- **B. The installed `clm` supplies them (chosen).** The model crate compiles in CLM's `ARCHITECTURE.md`; the flagged entries reach every bundle through `clm context` and print with `clm guide constraints`. The project's `ARCHITECTURE.md` holds only its own constraints and its replacements. The rules a project is told are always those its `clm` enforces, as `method-in-the-plugin` already does for the guide.
- **C. B plus a command that refreshes a copy in the file.** A second copy that can drift, against `spec-is-current` and `regenerate-dont-merge`; not worth it.

## Outcome

### Tiers

Each carried constraint in CLM's `ARCHITECTURE.md` holds a `#### Carries` block with one line, its tier:

- **`method`**: CLM's machinery relies on it; a project can't replace it. `ai-writes-code`, `humans-review-intent`, `deterministic-enforcement`, `proven-by-tests`, `no-silent-drift`, `git-is-truth`, `hash-based-staleness`, `regenerate-dont-merge`, `ancestor-scoped-context`, `spec-is-current`, `traced-rules`, `path-ownership`, `approval-policy`, `land-locally`, `incremental-adoption`.
- **`replaceable`**: CLM relies on a guarantee, which the entry states in a `#### Guarantee` block, and the entry's technique is one way to give it. A project may replace the entry with its own constraint that keeps the guarantee. `expand-contract` (every land leaves `main` green while the components that use a contract haven't moved yet), `no-dead-code` (all code serves a current requirement, and a program the project names finds what doesn't), `slim-components` (one build session can hold a component's spec and code), `fold-small-defer-large` (work found during a change is either in it or recorded in the backlog, never lost).
- **`default`**: an opinion CLM's machinery doesn't rely on; a project may drop it. `yagni`, `reuse-first`, `right-over-convenient`, `redesign-before-workaround`, `ask-the-architect`.

Every other constraint is about CLM's own code (its guard, ledgers, web app, claims) and isn't carried. Carried entries that say "CLM SHALL" of what any project does are reworded to say "the project", so they read right in either repository.

### Delivery

- The model compiles in CLM's `ARCHITECTURE.md` and exposes its carried entries. A repository whose own `ARCHITECTURE.md` holds a `#### Carries` block is the source, CLM's own, and takes the carried set from its own file; any other takes it from the compiled copy.
- In a project that isn't the source, an entry of its own `ARCHITECTURE.md` may hold a `#### Replaces` block of `- <slug>` lines naming carried entries. The model reports a `#### Replaces` that names no carried entry or names a `method` one, an own entry whose slug is a carried one, and a `#### Replaces` in the source. A replaced entry is out of force in that project. A replacing entry is an ordinary constraint: it needs `#### Enforced by` or `#### Untested` like any other.
- `clm context` adds, after `ARCHITECTURE.md`, a `# CLM's constraints` section holding each carried entry in force in a project that isn't the source: its heading, statement, tier and guarantee, in CLM's order. CLM's own repository gets nothing more, since its `ARCHITECTURE.md` already holds them.
- `clm guide constraints` prints the carried set of the `clm` you run, with each tier explained, as a page generated from the compiled copy rather than a file under `docs/guide/`.
- `clm init`'s `ARCHITECTURE.md` drops the `## Enforced by clm` list and points at `clm guide constraints`, saying how to replace or drop one. A project initialised earlier keeps its old list, which is prose the model doesn't read; it does no harm and the guide says it can be deleted.

### Replacing a constraint

`propose` explains, when an intent replaces or drops a carried constraint, its tier, what CLM relies on, what the replacement must guarantee and what the project loses, before writing the entry. The `architecture-auditor` reads the carried set beside `ARCHITECTURE.md` and judges whether each replacement keeps the guarantee; its finding goes under `## Open questions` and into the approval ask. The gate stays the human's `clm approve`, which every `ARCHITECTURE.md` edit already needs: a model's judgment advises and never gates (`deterministic-enforcement`).

### `/clm:help`

A read-only skill, `plugin/skills/help/SKILL.md`. It answers from what the installed `clm` carries: `clm guide` and the pages the question needs, `clm guide constraints`, and, in a repository under CLM, `clm status` and `clm backlog`. It explains why a rule exists and what replacing it costs, points to the skill that acts (`/clm:init`, `/clm:propose`, `/clm:build`, `/clm:triage`, `/clm:health`, `/clm:contribution`) and changes nothing. Outside a CLM repository it explains how to start, from the quickstart. The `CLAUDE.md` `clm init` writes names it.

## Constraints

- A new architectural constraint, `carried-constraints`, states the delivery; it is CLM's own, not carried. It is an `ARCHITECTURE.md` edit, so this branch needs review.
- `deterministic-enforcement`: the auditor advises; the model's `#### Replaces` checks and the human's approval gate.
- `method-in-the-plugin`: what a project must know about CLM's constraints comes from the binary, never only from this repository.
- `doc-readers-marked`: tests that read this repository's `ARCHITECTURE.md` through the compiled copy carry a `clm:reads` marker naming it.
