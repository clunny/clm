# Reference

- [Spec format](../spec-format.md) (`clm guide spec-format`): components, requirements and scenarios, deltas, building by component, decisions and the backlog.
- Why CLM works the way it does: CLM's own repository records each significant choice in CLM as a file in its `docs/decisions/`, with its reasons and what was rejected. Your project records its own choices the same way, in its `docs/decisions/`.
- CLM's own architectural constraints are in its repository's `ARCHITECTURE.md`, and its component map in the Design of its root component, `specs/SPEC.md`. The architectural constraints that `clm init` puts in your project's `ARCHITECTURE.md` come from there.

## Skills

| Skill | Use it to |
|---|---|
| `/clm:init` | bring a new or existing repository under CLM |
| `/clm:propose` | turn an intent into one spec branch, and see it landed |
| `/clm:build` | build everything ready, in parallel, until the queue is empty |
| `/clm:triage` | draft the backlog into spec changes unattended, then ask you to approve the ones that need review |
| `/clm:contribution` | take in a contributor's pull request or issue: judge its files, show it to you, close it or land it on your yes |
| `/clm:health` | read the health report and propose cuts, for you to take to `/clm:propose` |

## Commands

`clm help` lists every command and `clm help <command>` explains one. The ones you'll use most:

| Command | Does |
|---|---|
| `clm status` | the repository's state: components (`oversized` where one is over its budget), changes, backlog, claims |
| `clm size [<component>]` | each component's spec and code size in tokens against its budgets, `oversized` where over ([Shaping the graph](shaping-the-graph.md)) |
| `clm backlog` | list backlog items |
| `clm context <component>` | the context a session building a component gets |
| `clm check` | every deterministic check; exits 1 on a violation |
| `clm coverage` | each requirement as `current`, `behind` or `unbuilt`, with the tests citing it, then the `obsolete` ones a test still cites |
| `clm pending [<component>]` | the requirements whose code hasn't caught up with the spec, and what blocks them |
| `clm gate --base origin/main` | whether a branch needs review (`spec review`) or lands on checks (`spec`), and whether it may land |
| `clm next` | the components ready to build |
| `clm rebase` | rebase onto `main`, regenerate `specs/`, run the checks |
| `clm pr open` | open a branch's pull request on the host, for reading its diff |
| `clm approve <branch>` | record your approval of the diff of a branch that needs review; Claude Code asks you to confirm it |
| `clm land <branch>` | land a branch on `main`: gate it, merge it, run the checks on the result and push `main` |
| `clm wrap-up` | clean up landed work, and say whether the session can end |
| `clm survey` | map a codebase, for drafting its spec graph |
| `clm init` | set up or upgrade CLM's tooling in a repository |
| `clm defect` | report a problem with CLM itself |
| `clm health` | one advisory report of the project's health: checks, pending, sizes, claims, stale questions, spec bloat, unexercised code and the requirements to reconsider ([Using CLM](using-clm.md)) |
| `clm capabilities [<slug>]` | what each capability costs (an estimate from the ledger's `effort` records and git), or one capability's page: its cost, workarounds, overlap, rules and decisions |
| `clm capabilities decide <slug>` | record your review decision on a capability (`--value`, `--decision`), settling its open workarounds; Claude Code asks you to confirm it |
| `clm architecture [<slug>]` | what each architectural constraint costs (counted apart from the capabilities') and whether its enforcing rules are built, or one constraint's page |
| `clm architecture decide <slug>` | record your review decision on an architectural constraint (`--value`, `--decision`), settling its open workarounds; Claude Code asks you to confirm it |
| `clm workaround <component>/<slug>` | flag, from a build, that a requirement was hard to build; exits 3 when the capability has too many unsettled workarounds |

## Settings

`clm.toml` at the repository's root holds the project's settings, and `~/.config/clm/config.toml` holds yours. `clm.local.toml` beside `clm.toml`, never committed, holds one clone's and may set any key. In the table, "the repository's" means `clm.toml` or `clm.local.toml`, never yours, and "any" means any of the three. `clm config get <key>` shows a setting and where it came from.

| Key | Where | Means |
|---|---|---|
| `detectors` | the repository's | dead-code and unused-dependency tools `clm check` runs |
| `host` | the repository's | where the remote lives: `github` (the default), `git` or `none`; pull requests, branch deletion and issues go through it |
| `approval` | the repository's | whose approval lets a branch that needs review land: `any` (the default), anyone's, the author's included; `other`, only someone who wrote none of the branch's commits |
| `land` | the repository's | `[land]` with `check`: the command `clm land` runs on the tree it lands, before `clm check` |
| `component_size` | the repository's | `[component_size]` with `spec_tokens` (default 12000) and `code_tokens` (default 40000): the size above which a component with no `size_budget` is oversized ([Shaping the graph](shaping-the-graph.md)) |
| `health` | the repository's | `[health]` with `reconsider` (default 3), `scenarios` (6), `exceptions` (4), `stale_days` (30) and `lcov` (a path): the thresholds behind `clm health` and the reconsider questions `clm rebase` files, and the coverage file it reads |
| `effort` | the repository's | `[effort]` with `workaround_flag` (default 3): the unsettled workarounds on one capability at which `clm workaround` stops a build to ask whether to redesign it |
| `writer` | yours, `clm.local.toml` or `CLM_WRITER`, never `clm.toml` | the name your approvals, checks and lands are recorded under; defaults to the repository's git `user.name`, as a slug |
| `autosync` | any | push spec work in progress to `autosync/<branch>` after each turn |
| `defects` | any | where `clm defect` sends reports: `ask`, `backlog` or `off` |
| `max_builds` | any | how many builds and drafts may run on this clone at once (default 3) |

## How a change flows

[Using CLM](using-clm.md) says each step in full.

1. **Propose** (humans and AI): a spec branch adds `changes/<id>/` with deltas. When it needs review, a human approves it with `clm approve`; otherwise it lands on checks.
2. **Apply:** the deltas are folded into `specs/`. New requirements are `unbuilt`.
3. **Build** (AI only): each component with pending requirements is brought up to its spec, in parallel worktrees, test-first, highest priority first. Each test cites the version of the requirement it proves.
4. **Land:** `clm land` runs the checks on the merged tree and pushes it to `main`, recording the land in the ledger. Requirements cited at their current version by passing tests count as built; the rest stay pending for the next build.
5. **Archive:** a change whose requirements are all current moves to `changes/archive/`. Discoveries made along the way go to `backlog/`.

## Glossary

Every term CLM uses, in plain words, is in the [Glossary](glossary.md) (`clm guide glossary`).
