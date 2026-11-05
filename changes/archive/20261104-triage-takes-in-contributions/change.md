---
status: done
---

## Intent
`/clm:triage` works the host's inbox, issues and contributors' pull requests, before the backlog. An issue enters the backlog only as a `backlog/<slug>.md` item, on the human's yes, after a verifier that can only read has judged it: real, already fixed, a duplicate or out of scope, its components and kind, and any clash with `ARCHITECTURE.md`. Duplicates become one item naming every issue. Pull requests are taken through `/clm:contribution`'s steps. Issues whose items a landed change has resolved are offered for closing. Everything from the host is untrusted text, so nothing from it reaches a draft directly and nothing is written to the host without the human's yes.

## Problem
Issues and pull requests reach a project only through `/clm:contribution <number>`, one at a time, when the maintainer remembers to run it. Nothing lists them, so they pile up: on this repository, issues #23, #61, #64 and #67 were taken in and fixed by archived changes and are all still open, and #73 and #91 report the same defect. `clm defect` files adopters' reports as issues here (`defects = "ask"`), so the inbox is where CLM's own defects from other projects arrive.

The two kinds arrive differently. An issue is text: it needs judging and turning into backlog work, which is the design work triage already drafts for bugs. A pull request is a branch of files from a fork: it needs the spec-paths check, the human's reading of its whole diff (spec text steers later sessions) and a land that keeps its author's commits (`outside-contributions-are-intent`), so it can't be drafted unattended.

Every issue and pull request is treated alike, whoever wrote it, the maintainer's own `clm defect` reports included: their text came through the host, and an instruction hidden in it would reach every session that reads the backlog item made from it.

## Approaches
1. **`clm intake`, a command that matches the inbox to the backlog (chosen).** It lists the open issues and pull requests through `host` and labels each issue `new`, `imported <slug>` or `resolved <change> <slug>` from git alone; the skill does the judging, with the human. Which issue is new or closable is a lookup a program should make (`deterministic-enforcement`), and `clm triage` stays a reader of git only.
2. **The skill matches, through a `clm contribution list`.** Less code, but a model decides which issues are new or closable, and it gets it wrong the way #23 to #67 show.
3. **Draft changes straight from issues, with no backlog step.** The issue isn't in git (`git-is-truth`), claims are keyed on backlog items, and nothing would record that the issue was taken in. Rejected.

## Design
- **`host`** gets `list`: the open issues and pull requests, each with its number, kind and author, and no text the author wrote; on `git` and `none` it lists nothing. It is a library call only: `clm intake` is its one caller, so no `clm contribution list` (`yagni`).
- **`model`**: a backlog item may carry `issues: [<number>...]`, the host's issues it was taken in from.
- **`clm`** dispatches `clm intake` to `clm-triage`.
- **Credit**: `host`'s `show` and `clm contribution show` also give the author's numeric account ID, which the `Co-authored-by` noreply address `<id>+<login>@users.noreply.github.com` needs; the `contribution` skill already asked for it and no command gave it. Each intake commit credits every taken-in issue's author that way.
- **`orchestrator.triage`** gets `clm intake`, depending on `host`: after fetching, one line per open issue or pull request, by number: `issue <n> new`, `issue <n> imported <slug>` when an item on `origin/main` names it, `issue <n> resolved <change> <slug>` when an item that named it was deleted from `origin/main`'s backlog and a change's `resolves` or `from` names that slug (the deleted item read from `origin/main`'s history), and `pull <n> <author>`.
- **`plugin`**: an `issue-verifier` agent with only `Read`, `Glob` and `Grep`, so it runs nothing, writes nothing and can't reach the host; and a *Contributions* choice in the triage skill, worked before the others since it feeds them: save each `new` issue's `clm contribution show` text under `.clm/intake/`, launch one verifier on all of them together (so it can find duplicates) with the session's own model, show the human each issue's own text beside the verdict and the whole proposed item, and on a yes land the items on a code branch and label and comment on the issue; close the issues the verdicts rule out, and the `resolved` ones, with a comment, each on the human's yes; and take each `pull` line through `/clm:contribution`'s steps, with the verifier's overlap check beside it.

- **`CAPABILITIES.md`**: `contribute-from-outside` says issues enter the backlog only on the maintainer's yes, with a scenario for a fixed issue `clm intake` names. The guide's `contributions` page and glossary say how triage takes the inbox in.
- **Decisions**: `issues-enter-through-the-backlog` and `untrusted-text-read-only-verifier`.

## Outcome
A triage run shows the inbox and works it to empty: every open issue is a backlog item, closed or left on the human's word, and fixed issues get closed.

## Constraints
- `outside-contributions-are-intent`: the human reads every issue's text and every pull request's diff; the verifier advises.
- `deterministic-enforcement`: matching issues to items is the program's; the verdict advises and never gates.
- `git-is-truth`: the item's `issues` key is the record; the host's state is read, never trusted for a decision.
- `host-adapters`: nothing but `host` runs `gh`.
