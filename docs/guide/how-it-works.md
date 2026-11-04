# How it works

What the workflows do underneath: the files a project holds, how a change travels from intent to `main`, how builds are proven, and the commands Claude runs on your behalf.

## The files

`/clm:init` runs `clm init`, which writes CLM's tooling. It never changes a file that already exists, apart from adding lines to `.gitignore`, so running it again is safe:

- `clm.toml`: settings: where the remote lives (`host`), whose approval counts (`approval`), what `clm land` runs (`[land] check`), and the dead-code detectors for your languages.
- `scripts/clm-check.sh`: builds and tests your project. It's yours to edit.
- `ARCHITECTURE.md`, `CAPABILITIES.md`, `specs/SPEC.md` and `CLAUDE.md`, when you don't have them yet.
- `.gitignore`: `clm.local.toml`, `.clm/` and `.claude/worktrees/`.

It writes nothing under `.github/`. On an existing project, `/clm:init` moves the build and test steps of your CI workflows into `scripts/clm-check.sh` and removes those workflows, because `clm land` lands on CLM's own checks. It tells you before it does this. Deploy, release and scheduled workflows are left alone.

## The human layers

Two short files at the repository's root are written for people rather than for builds:

- **`CAPABILITIES.md`** holds the capabilities: what a person can do with the product, each with a `#### Why` and a scenario. The product manager owns it.
- **`ARCHITECTURE.md`** holds the architectural constraints: what must always hold, each with SHALL or MUST and a scenario. The architect owns it, and every build reads it.

Each entry has an ID, `capability/<slug>` or `architecture/<slug>`, and a version, as a requirement does: `clm hashes capability` and `clm hashes architecture` print them. A capability is proven by a black-box test at the product's public boundary that cites it, `req:capability/<slug>@<version>`, unless an `#### Untested` block says why no program can check it. An architectural constraint is never cited by a test: it names under `#### Enforced by` the component rules that enforce it, and is current when every rule it names is current; it says what no program can check under `#### Untested`. `clm check` fails a constraint with neither block. A component says which entries its rules serve with `serves` in its frontmatter, and a requirement can override that with a `#### Serves` block.

Any edit to either file needs your review before it lands. Everything below them is the AI's to change.

## The spec

`specs/` is a tree of components, one `SPEC.md` each, under the root component `specs/SPEC.md`. A component has:

- a **Contract**: what other components may rely on;
- a **Design**: how it works inside, for whoever builds it;
- **Requirements**, each with an ID such as `billing/refund-window` and at least one scenario (GIVEN, WHEN, THEN);
- frontmatter: the `paths` of code it owns, and what it `depends_on`.

Significant choices are recorded one per file in `docs/decisions/`. The format is in [spec format](../spec-format.md) (`clm guide spec-format`), and how to size and split components in [Shaping the graph](shaping-the-graph.md).

When you adopt one part of an existing codebase first, the rest is listed under `outside` in the root's frontmatter. That code has no requirements, so no test proves it to CLM; it still lands only through `clm land`, whose check script runs its tests. Each later change adds a component for the next part and takes its globs out of `outside`. `clm check` fails on code that is in no component and not declared outside, so the list only shrinks.

## A change, from intent to `main`

`/clm:propose` turns your intent into one **spec branch**, `spec/<change-id>`:

1. `changes/<id>/change.md` holds the intent, the approaches considered, and `human_burdens`: what the change spends of your money or time, or can't undo.
2. `changes/<id>/deltas/<component>.md` holds one delta per component it touches: requirements, Contract or Design added, modified or removed.
3. `clm apply <id>` applies the deltas to `specs/`, so the branch shows the spec as it will be.
4. `clm gate --base origin/main` says whether the branch needs your review (`spec review`) or lands on checks (`spec`).
5. `clm pr open` opens a pull request, for reading the diff. Nothing waits on it.
6. `clm land spec/<id>` lands it.

Every piece of one design goes on one branch, so a change that needs your review asks for it once.

## Review

A spec branch needs your review when:

- it edits `ARCHITECTURE.md` or `CAPABILITIES.md`, or `docs/allowed_signers`, the keys a signed repository trusts;
- it changes the `approval` setting in `clm.toml`;
- a change it adds or edits declares `human_burdens`.

Everything else lands on checks, and nothing is reviewed after it lands. When you say yes, Claude runs `clm approve <branch>` and Claude Code asks you to confirm that command, so the approval is yours. It's a record in your ledger, pinned to the files the branch changes: a rebase or merge of `main` that touched only other files keeps it, and any change to the branch's own files drops it, so you review again.

## Landing

`clm land` is the only way anything reaches `main`. It gates the branch, merges it with `main`, runs `scripts/clm-check.sh` and `clm check` on the result on the machine that runs it, and pushes `main`. No hosted CI and no labels are involved. The bootstrap commit of `/clm:init` is the one push to `main` made any other way.

## Building

Work is per component, not per change. `clm next` lists the components ready to build: those with a requirement the code hasn't caught up with, leaving out requirements that wait on a backlog item or on another component's requirement they name under `#### Needs`. `/clm:build` runs `clm run <component>` for each, which claims the component by pushing `impl/<component>` and prints the prompt for an unattended build in its own worktree. The build works through the component's pending requirements, highest priority first, test-first, and lands its branch through `clm land`. A claim with no push for four hours lapses, so someone else can take it.

## Proof

Each test cites the requirement it proves at the version it was built against, `req:<component>/<slug>@<version>`, the version being the first 8 hex digits of the requirement's hash (`clm hashes <component>`). `clm pending` lists each requirement that isn't current:

- `unbuilt`: no test cites it;
- `behind`: a test cites an older version, because the requirement's text changed;
- `obsolete`: a test cites it, but a change removed it.

A change is complete once every requirement it added or changed is cited at its current version and no test cites one it removed; the next build's `clm rebase` then archives it. On an existing project, the first builds link your existing tests to the requirements they prove, write tests for the rest, and remove dead code.

A small code change that needs no spec change, such as a refactor or a clearer message, goes on a branch of its own named outside `impl/`, `spec/`, `propose/` and `autosync/`, and lands through `clm land`. A fix for a `bug` item takes `clm claim <component> --fix backlog/<slug>` instead, and behaviour no requirement states starts with a spec change.

## The backlog

`backlog/` holds one file per item, of one kind:

- `idea`: future work, brainstormed in `/clm:triage` and taken up by `/clm:propose`.
- `deferred`: future work waiting on a condition its body names, which you judge.
- `bug`: something broken, including problems with CLM itself, which Claude files with `clm defect`.
- `question`: a fact only you can supply. The requirements it `blocks` wait for it.
- `human-task`: a step only you can do. The requirements it `blocks` wait for it too.
- `workaround`: a requirement built with a workaround (`clm workaround`), kept until its component is redesigned or the workaround is accepted.

`clm triage` groups the items for `/clm:triage`, and `clm draft` writes the prompt for the background session that designs each group.

## Health

`clm health` prints one advisory report, and never fails anything: check violations, what is pending, oversized components, lapsed claims, questions left open, decisions nothing names, capabilities and constraints no rule serves, spec text that narrates history, Designs that dwarf their requirements, components far off the median in size, code no test runs (from an lcov file named under `[health] lcov` in `clm.toml`), and requirements that fight their implementation.

That last signal comes from builds. When a build needs a workaround to satisfy a scenario, fakes something the requirement doesn't mention, changes another component's Design, or stops to raise a spec change, it flags the requirement with `clm workaround`. Flags, rewrites and oversized scenario counts add up to a score, and when it reaches the `reconsider` threshold the next build files `backlog/reconsider-<component>-<slug>.md`, a question for you: drop the requirement, change it, or redesign around it. Thresholds live under `[health]` in `clm.toml`.

## Cost

Every land records the work its sessions spent, read from this machine's Claude Code transcripts, as `effort` records in the ledger. `clm capabilities` lists each capability with an estimate of what it cost, in dollars and tokens, the commits and reviews it took, and its unsettled workarounds; `clm capabilities <slug>` prints one capability's page. A component's cost is split over the capabilities its rules serve, so the numbers are estimates. `clm architecture` does the same for the architectural constraints. Your decision on one is recorded with `clm capabilities decide` or `clm architecture decide`, which Claude Code asks you to confirm.

A build that flags a requirement whose capability already has 3 unsettled workarounds (`workaround_flag` under `[effort]` in `clm.toml`) stops working on it and asks you whether to redesign the capability instead.

## Commands Claude runs for you

| Command | Does |
|---|---|
| `clm status` | the repository's state, shown at the start of every session |
| `clm backlog` | the backlog's items |
| `clm next` | the components ready to build |
| `clm pending [<component>]` | the requirements whose code hasn't caught up with the spec |
| `clm check` | every deterministic check |
| `clm gate --base origin/main` | whether a branch needs review, and whether it may land |
| `clm rebase` | rebase onto `main`, regenerate `specs/` and run the checks |
| `clm land <branch>` | land a branch on `main` |
| `clm wrap-up` | clean up landed work, and say whether the session can end |

The full list is in the [Reference](reference.md).

## When something goes wrong

- A branch conflicts with `main`, or the gate says it's out of date: Claude runs `clm rebase`, pushes and lands again.
- A check fails when landing: Claude reads the failure, fixes it and lands again.
- CLM itself got in the way: Claude files it with `clm defect`.
- Before a session ends, `clm wrap-up` reports anything not yet safe on `origin`.
