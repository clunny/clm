# Reference

The skills, the commands' settings and where to read more: a page to look things up in once you've used CLM.

- [Spec format](../spec-format.md) (`clm guide spec-format`): components, requirements and scenarios, deltas, building by component, decisions and the backlog.
- Why CLM works the way it does: CLM's own repository records each significant choice in CLM as a file in its `docs/decisions/`, with its reasons and what was rejected. Your project records its own choices the same way, in its `docs/decisions/`.
- CLM's own architectural constraints are in its repository's `ARCHITECTURE.md`, and its component map in the Design of its root component, `specs/SPEC.md`. The architectural constraints CLM carries into every project come from there: `clm guide constraints` prints them from the `clm` you run.

## Skills

| Skill | Use it to |
|---|---|
| `/clm:init` | bring a new or existing repository under CLM |
| `/clm:propose` | turn an intent into one spec branch, and see it landed |
| `/clm:build` | run the build loop: build everything ready and draft the backlog's bugs, in parallel and in priority order, until you ask it to stop |
| `/clm:triage` | work the backlog down from a menu: approve designed changes, answer questions, brainstorm ideas, draft bugs in the background, and approve the drafts that need review in one go |
| `/clm:contribution` | take in a contributor's pull request or issue: judge its files, show it to you, close it or land it on your yes |
| `/clm:help` | ask how CLM works or how to do something; it answers from the guide and changes nothing |
| `/clm:health` | read the health report and propose cuts, for you to take to `/clm:propose` |
| `/clm:configure` | review every setting, or change one (`/clm:configure set concurrent builds`): it explains the trade-offs, edits the right file and lands the edit |
| `/clm:verify` | verify `main` as it is now, whatever the triggers say: on a red result it shows what failed, re-runs the failing tests when it can, and offers to take the failure in as a `P1` bug |

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
| `clm contribution` | what `/clm:contribution` and `/clm:propose` use for a fork or outside contribution: `parent`, `sync`, `open`, `show`, `fetch`, `comment`, `label` |
| `clm approve <branch>` | record your approval of the diff of a branch that needs review; Claude Code asks you to confirm it |
| `clm land <branch>` | land a branch on `main`: gate it, merge it, run the checks on the result and push `main` |
| `clm wrap-up` | clean up landed work, and say whether the session can end |
| `clm survey` | map a codebase, for drafting its spec graph |
| `clm init` | set up CLM's tooling in a repository |
| `clm upgrade` | list the migrations a project set up by an older `clm` still needs; `--apply` takes the mechanical ones, `--note <id>` prints one to take by hand, `--decline <id>` declines an advised one; `/clm:upgrade` walks you through them |
| `clm defect` | report a problem with CLM itself |
| `clm health` | one advisory report of the project's health: checks, pending, sizes, claims, stale questions, spec bloat, unexercised code and the requirements to reconsider ([How it works](how-it-works.md#health)) |
| `clm capabilities [<slug>]` | what each capability costs (an estimate from the ledger's `effort` records and git), or one capability's page: its cost, workarounds, overlap, rules and decisions |
| `clm capabilities decide <slug>` | record your review decision on a capability (`--value`, `--decision`), settling its open workarounds; Claude Code asks you to confirm it |
| `clm architecture [<slug>]` | what each architectural constraint costs (counted apart from the capabilities') and whether its enforcing rules are built, or one constraint's page |
| `clm architecture decide <slug>` | record your review decision on an architectural constraint (`--value`, `--decision`), settling its open workarounds; Claude Code asks you to confirm it |
| `clm workaround <component>/<slug>` | flag, from a build, that a requirement was hard to build; exits 3 when the capability has too many unsettled workarounds |

## Settings

Every setting, what each choice trades off and where it may be set is in [Settings](settings.md). `clm config list` prints them all with where each came from, and `/clm:configure` changes them.

## How a change flows

[How it works](how-it-works.md) says each step in full, and [Using CLM](using-clm.md) the workflows that run them.

1. **Propose** (humans and AI): a spec branch adds `changes/<id>/` with deltas. When it needs review, a human approves it with `clm approve`; otherwise it lands on checks.
2. **Apply:** the deltas are folded into `specs/`. New requirements are `unbuilt`.
3. **Build** (AI only): each component with pending requirements is brought up to its spec, in parallel worktrees, test-first, highest priority first. Each test cites the version of the requirement it proves.
4. **Land:** `clm land` runs the checks on the merged tree and pushes it to `main`, recording the land in the ledger. Requirements cited at their current version by passing tests count as built; the rest stay pending for the next build.
5. **Archive:** a change whose requirements are all current moves to `changes/archive/`. Discoveries made along the way go to `backlog/`.

## Glossary

Every term CLM uses, in plain words, is in the [Glossary](glossary.md) (`clm guide glossary`).
