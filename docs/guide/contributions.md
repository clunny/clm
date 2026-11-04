# Contributions

How a project under CLM can take contributions from people without write access. Whether to take them is each project's choice, and a private team whose members can push doesn't need any of this. `clm init` sets none of it up.

## What outsiders can offer

Under CLM nobody reviews code, and `clm land` runs the merged tree's check command on the lander's machine. Someone without write access also can't claim a component or land a branch, because both push to `origin`. So landing a stranger's code would mean running it on your machine with nobody having read it.

What an outsider can safely offer is intent. A contribution is acceptable when every file it changes is on a spec path: `specs/`, `changes/`, `backlog/`, `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md` or `CLAUDE.md`. Such a branch can't change a script or setting a land runs, and when the code on `main` has already been checked, a land runs only `clm check`.

## The levels

| Level | What they send | What they need |
|---|---|---|
| One line | An issue | A GitHub account |
| Short item | A pull request adding `backlog/<slug>.md` with `title`, `kind: idea` and a few lines | A browser |
| Elaborated item | The same file with `## Intent`, `## Settled`, `## Approaches` and `## Open questions` | A browser, or `/clm:propose` to help |
| Full change | `change.md` and deltas from `/clm:propose` in a clone of their fork | Claude Code and CLM |

Ask people to agree an idea in an issue first, since writing a spec costs them model use. An `accepted` label is a clear yes, and shows who is working on what, since a claim made in a fork is invisible to everyone else. Ask them to leave out `priority`: its slots are capped, and it's the maintainer's call.

In a fork, `/clm:propose` asks whether the work goes back to the parent. If it does, it opens the pull request there and stops: it runs neither `clm land` nor `clm approve`.

## Taking one in

Run `/clm:contribution <number>` for a pull request or an issue. It treats everything in the contribution as untrusted data and runs nothing from it. It closes a pull request with files outside the spec paths on your yes. Otherwise it shows you the whole diff, and on your yes brings it in as `spec/<id>`, rebases it only if it must, and lands it as any other spec branch.

**Read every one by hand.** A branch that only rewords a requirement would land on checks, but a contribution isn't your own session's work. Spec text and `CLAUDE.md` instruct your future sessions, so a malicious edit is prompt injection, and your review is the only defence. Accepting also spends your budget, since your builds write the code the spec asks for.

## What enforces it

- **Write access.** An outsider can't push `main`. Running `clm land` in a fork moves only that fork's `main`.
- **The land rule.** `clm land` refuses a `spec/*` branch that changes anything outside the spec paths, before running any check. A branch from `clm rename` is exempt, since every edit in it follows from the pairs its change records. Take contributions in only as `spec/<id>`: the rule can't catch a branch pushed under another name.
- **Your review.** No program judges whether a spec is what you want.

**A protected `main`.** You can block force-pushes and deletion. Don't require pull requests or restrict updates on `main`: `clm land` pushes it directly, so the identity that lands must be allowed to. GitHub's merge button stays available and skips `clm land`, so don't use it.

## Credit

`clm land` merges the branch and keeps its commits, and a rebase keeps their author, so someone whose pull request lands appears among the project's contributors in the usual way, provided their commit email is linked to their account. If a rebase can't be pushed back to their branch, the host may show the pull request as closed rather than merged, so comment with a link to what landed.

Someone who only opened an issue has no commit of their own. A `Co-authored-by` trailer can credit them on their host profile, but may not add them to the project's contributors list. A short backlog item in a pull request is the cheap way to get full credit.

## Limits

- **Code outside the graph.** Code listed under the root's `outside` has no spec, so a spec contribution can't change it. Accepting a handwritten fix there means running a stranger's code through `clm land`, so read it as you would any code you are about to run.
- **Defects in CLM itself.** `clm defect` reports these to CLM's own repository. Your project's own bugs need your project's own tracker.

## A starter for your CONTRIBUTING.md

```markdown
# Contributing

This project is built with CLM: AI writes the code, from a spec people own. We take contributions of intent, read by hand by a maintainer. A pull request changing any file outside `specs/`, `changes/`, `backlog/`, `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md` or `CLAUDE.md` is closed.

Open an issue with a bug or an idea, one line is fine. For more, agree the idea in an issue, then send a pull request adding `backlog/<slug>.md` with `title`, `kind: idea` and your reasoning, leaving out `priority`, or a full change with `/clm:propose` in a clone of your fork. Leave "Allow edits by maintainers" on, and use a commit email linked to your account so the commit counts as yours.
```
