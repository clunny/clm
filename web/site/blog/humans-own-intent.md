---
title: "Stop reviewing AI's code. Review its intent."
series_part: 1
summary: "AI can write the code, but it can't decide what the system is for: CLM gives humans the intent, gives AI the spec and all of the code, and lets deterministic code (not reviewers) keep the two in sync."
tags: [clm, claude-code, spec-driven-development, ai-coding-agents, code-review, software-architecture]
date: 2026-11-06
draft: false
---

**AI didn't just make writing code faster. It made the way we build software obsolete. CLM redefines how to build software with AI.**

There is a moment most people using a coding agent will recognise. A pull request arrives, a few hundred lines long, and it looks fine. The tests pass. You scroll, you recognise the shapes, you approve. Somewhere in the back of your mind you know you didn't really read it, and that the approval means less than it used to.

It isn't laziness. The agent writes code faster than anyone can read it, and it will keep writing while you are still on the first file. Line-by-line review was designed for code that people wrote at the speed people write. Point it at a machine and it quietly turns into a ritual: the button still gets pressed, but nobody is checking anything.

And the risk was never really a bad line of code. Tests catch most of those, and the agent fixes them faster than you could. The risk is a decision about what the system is *for*, made by the agent on the way to a working diff and buried among the renamed variables, which ends up in the product because it passed review.

## Review the decision, not the diff

CLM is an open-source Claude Code plugin and Rust CLI built on one idea: the decisions are the part that is yours, so write them down and review those instead. Its README puts it in four sentences:

> Humans own the intent. AI writes the spec and all the code. Deterministic checks keep the two in sync, and git is the only channel between people and sessions.

You say what the system must do. Claude builds it. Programs check that the code proves what you said, and nothing reaches `main` until they agree. Your review moves from the diff to the decision.

## Intent you can read in five minutes

What you write is short on purpose. A CLM project keeps two files at its root for people rather than builds. `CAPABILITIES.md` says what a person can do with the product and why. `ARCHITECTURE.md` says what must always hold. One is the product manager's, the other the architect's, and on a small project they are the same person.

Here is one entry from CLM's own capabilities:

```markdown
### Capability: approve-intent
A human can read a change that needs review and approve it through a prompt only a human can confirm.

#### Why
Changes to what the product is for, or what must always hold, are the human's to decide; an AI must never be able to approve them for itself.
```

That is the level you work at. Below it, the AI writes and maintains everything else: the components, their designs, and requirements with GIVEN/WHEN/THEN scenarios. A change to the product arrives as a question you can actually answer, "is this what we want?", rather than "is this code right?"

## A yes or no, and only when it's yours

Moving review to the spec only helps if the spec doesn't become the new flood. So CLM doesn't ask you about everything. A program reads each change and decides whether it needs a human. It does when it changes what the product is for or what must always hold, changes who may approve, or spends something that is yours to spend: your money, your time, or a step that can't be undone. Everything else lands as soon as its checks pass.

What CLM won't do is land a change now and review it later. It tried that early on: of 32 changes that landed on their checks and were queued for a look afterwards, one was ever reviewed. "Later" meant never. So review in CLM is a yes or a no, before the change lands.

And the yes is genuinely yours. When you agree to a change, Claude Code asks *you* to confirm the approval, in a prompt the agent can't answer for you. The approval is pinned to that exact diff, so new spec text pushed afterwards drops it, and an agent can't approve its own change.

## Proof instead of reading

Not reading the code only works if something else holds it to account, and in CLM that is always a program, never a model's opinion. Every rule that must hold is checked by the CLI, a hook, or the checks that run on the merged tree, on your machine, before anything reaches `main`.

The most important of those checks connects the spec to the code. **Every requirement has a test that names it**, at the exact version of its wording:

```rust
// req:gate/needs-review@3276bfca
```

Change what a requirement says and that marker goes stale, so the code is known to be behind without anyone having to notice. "Is this built?" stops being a judgement call and becomes something a program answers. How that works will be a future blog post.

Code is still reviewed, just not by you. Each build runs an AI code review and a spec audit over its own diff, fixes what it agrees with, and writes the findings into the pull request. That review advises; the checks decide. Your confidence no longer comes from "someone looked at this". It comes from "this was proved against something I wrote".

## The whole loop is three commands

All of this machinery stays behind the scenes. Day to day, working with CLM comes down to three commands in Claude Code:

- **`/clm:propose`** is where new ideas go in. You describe what you want, Claude talks it through with you and writes it up as a change to the spec, and you say yes or no.
- **`/clm:build`** runs in the background. It is a reconciliation loop: it compares the spec with what the code proves, builds whatever has fallen behind, component by component and in parallel, and keeps going until nothing is left.
- **`/clm:triage`** is where you catch up. It gathers what is waiting on you, the questions a build couldn't settle alone and the changes that need your yes, and turns the rest of the backlog (bugs, ideas, deferred work) into changes ready to build.

Propose, build, triage, and round again. You stop being the bottleneck every diff waits on and become the person who decides what gets built and answers the questions only you can answer. Builds carry on while you are away. When one hits something it can't settle, it files a question, and only the requirements waiting on the answer stop; the rest carry on. CLM is built with CLM, this way, every day.

The split is the foundation, and most of what makes CLM different follows from taking it seriously. The rest of this series covers each in turn:

- **The build loop is a reconciler**, like a Kubernetes controller: it compares the spec with what the tests prove and builds whatever is behind.
- **Git is the whole coordination layer.** Claims, approvals and lands are branches and signed records. There is no server and no database.
- **Context is scoped, not maximal.** A build sees its component, its ancestors and the contracts of what it depends on, and nothing else.
- **One door to `main`**, and it runs on your machine.
- **The spec describes only the present.** Removed behaviour is deleted, code and tests with it.

## What it costs

CLM is opinionated. You don't edit code by hand; if you want to tweak a function yourself, it is the wrong tool. Someone has to own the intent and keep it current, or there is nothing for the AI to build against. Every behaviour needs a requirement and every requirement a test, which is real overhead on a throwaway script. And it is built around git and Claude Code specifically.

The trade is this: you give up the comfort of reading every line, which was mostly gone already. In return, every line is held to something you wrote and can read in a few minutes.

## Standing on shoulders

CLM's spec graph comes from the *Spec Growth Engine* paper ([arXiv:2606.27045](https://arxiv.org/html/2606.27045v2)). Anthropic's [AI-native SDLC playbook](https://claude.com/blog/the-ai-native-sdlc-playbook) shapes its workflow, writing intent down first and spending human attention at the gates; CLM goes one step further and takes the human off the diff entirely. Jesse Vincent's [superpowers](https://github.com/obra/superpowers) gives each build session its craft, and [OpenSpec](https://github.com/Fission-AI/OpenSpec) the shape of a requirement and the changes made to it.

## Try it

The site is [clm-app.dev](https://clm-app.dev), and the [quickstart](https://clm-app.dev/guide/quickstart) walks through your first change. In Claude Code:

```
/plugin marketplace add clunny/clm
/plugin install clm@clm
```

Then run `/clm:init` in a repository, new or existing.

Next in the series: **"Every requirement has a test that names it, at its version"**, on the `req:` markers that turn "is it built?" into a question a program can answer.
