---
title: "Stop reviewing AI's code. Review its intent."
series_part: 1
summary: "AI can write the code, but it can't decide what the system is for: CLM gives humans the spec, gives AI all of the code, and lets programs, not reviewers, keep the two in sync."
tags: [clm, claude-code, spec-driven-development, ai-coding-agents, code-review, software-architecture]
date: 2026-11-06
draft: true
---

Your coding agent can open a pull request faster than you can read it. So you skim the diffs, approve the ones that look plausible, and hope the tests mean what you think they mean. Line-by-line review was built for code that people wrote at the speed people write. It doesn't scale to code a machine writes in minutes, and it was never the best use of your attention anyway.

What an AI can't do is decide what the system is *for*: which behaviour matters, which trade-offs are acceptable, what must never happen. That part is still yours. CLM is built on the idea that it is the *only* part that should be yours.

## The split

CLM is an open-source Claude Code plugin and Rust CLI for spec-driven development. Its README puts the founding idea in four sentences:

> Humans own the spec and design. AI writes all the code. Deterministic checks keep the two in sync, and git is the only channel between people and sessions.

Each of those is an architectural constraint in CLM's own `ARCHITECTURE.md`, not a slogan. Three of them carry the weight:

- **`ai-writes-code`**: "AI SHALL write all code. Humans do not author or review code, and code changes only through the build flow."
- **`humans-review-intent`**: humans review intent, not code, and a spec branch needs a human's approval exactly when it touches what only a human can judge.
- **`deterministic-enforcement`**: "Every rule that must hold SHALL be checked by a program (the CLI, a hook or the land checks), never by prose or model judgment alone. Model judgment may advise but never gates."

Put together: you write down what the system must do, as a spec. Claude builds it. Programs check that the code proves the spec, and nothing reaches `main` that fails those checks. Your review moves from the diff to the decision.

## What "intent" looks like on disk

A CLM project has two short files at its root written for people rather than builds. `CAPABILITIES.md` says what a person can do with the product, and why; the product manager owns it. `ARCHITECTURE.md` says what must always hold; the architect owns it. These are roles, not headcount: one person can hold both.

Here is one entry from CLM's own `CAPABILITIES.md`:

```markdown
### Capability: approve-intent
A human can read a change that needs review and approve it through a prompt only a human can confirm.

#### Why
Changes to what the product is for, or what must always hold, are the human's to decide; an AI must never be able to approve them for itself.

#### Scenario: a session cannot approve alone
- GIVEN the same branch
- WHEN a session runs `clm approve` and the human declines the prompt
- THEN nothing is approved and `clm land` refuses the branch
```

That is the level you work at. Below it sits the implementation spec, a tree of components under `specs/`, each with a contract, a design and requirements with GIVEN/WHEN/THEN scenarios. The AI writes and changes that layer too, and it lands on checks alone.

## Who decides what needs you

If every spec edit went to a human, you would just have swapped one review queue for another. So a program decides. `clm gate` reads a branch's diff and prints its kind on the first line: `code`, `spec`, or `spec review`. Only `spec review` waits for a person, and the rule for it is written down and tested. A spec branch needs review when it edits `ARCHITECTURE.md` or `CAPABILITIES.md`, changes who may approve, changes the keys a signed repository trusts, or carries a change that declares `human_burdens`: something that spends your money or your time, or can't be undone. Everything else lands on its checks, and nothing is reviewed after the fact (`yes-no-review`).

That rule is itself a requirement, `gate/needs-review`, and the tests that prove it name it:

```rust
#[test]
fn architecture_needs_review() {
    // req:gate/needs-review@3276bfca
    let dir = tier_repo(&[("ARCHITECTURE.md", "invariants\n")]);
    let out = tier_after(&dir, &[("ARCHITECTURE.md", Some("changed\n"))], true);
    assert_eq!(verdict(&out), (Kind::Spec, true, true), "{:?}", out.violations);
}
```

`@3276bfca` is the first eight hex digits of the requirement's content hash. Change the requirement's wording and that marker goes stale, so the code counts as behind until a build catches it up. That mechanism gets a post of its own next.

When a branch does need you, you say yes in the conversation, Claude runs `clm approve`, and Claude Code asks *you* to confirm the command. The approval is a record pinned to the branch's diff, so new spec text pushed afterwards drops it. A later post covers how CLM stops an AI approving its own change.

And code? It is still reviewed, just not by you. Each build session runs Claude Code's `/code-review` and a `spec-auditor` subagent over its own diff before opening a pull request, fixes what it agrees with, and lists the findings in the PR body. That review advises. It never decides whether a branch lands (`advisory-code-review`).

## What else is unusual

The split is the foundation. Most of what makes CLM different follows from taking it seriously, and each of these gets its own post in the series:

- **Tests cite requirements at a version**, so "is this built?" has a mechanical answer.
- **Context is scoped**, not maximal: a build session sees its component, its ancestors and the contracts of what it depends on.
- **The build loop is a reconciler**, like a Kubernetes controller: it compares the declared spec with what the tests prove and builds whatever is behind.
- **Git is the whole coordination layer.** Claims, approvals and lands are branches and ledger records. There is no server or database.
- **One door to `main`.** `clm land` runs the checks on the merged tree, on your machine. No stacked branches.
- **Builds can run unattended**, and a question for you blocks only the requirements waiting on its answer.
- **The spec describes only the present.** Removed behaviour is deleted, code and tests with it.

CLM is built with CLM. Its own `clm status` reads, at the time of writing:

```
CLM status
  components 45: 45 active, 0 unbuilt; 707/719 requirements current, architecture 37/39 enforced, 23 untested, 18 oversized
  changes 1 live: 1 applied
  pending 4 components: 2 claimed, 2 blocked
  waiting on you 2: backlog/off-site-presence, backlog/search-console-and-bing
```

Note the "waiting on you" line: two items in CLM's own backlog that only its human can act on. That is the human's whole job description, made visible.

## Where it comes from

CLM didn't invent these ideas, and it says so. Its spec graph comes from the *Spec Growth Engine* paper ([arXiv:2606.27045](https://arxiv.org/html/2606.27045v2)): components with contracts, ancestor-scoped context, and drift between spec and code as a blocking error. Anthropic's [AI-native SDLC playbook](https://claude.com/blog/the-ai-native-sdlc-playbook) shapes the team workflow: intent written down first, human attention spent at the gates. CLM goes one step further than the playbook, which still has a code owner approve each diff. Jesse Vincent's [superpowers](https://github.com/obra/superpowers) supplies the craft every build session follows: brainstorming before design, test-driven development, systematic debugging, verification before claiming done. And [OpenSpec](https://github.com/Fission-AI/OpenSpec) gives CLM the shape of a requirement, its scenarios and the deltas a change makes to them.

## What it costs you

CLM is opinionated, and the docs are blunt about it: if its opinions don't suit your team, it will get in your way.

- **You don't edit code by hand.** Code changes only through the build flow. If you want to tweak a function yourself, CLM is the wrong tool.
- **Someone has to own intent.** If nobody on the team wants to write and keep the capabilities and constraints, there is nothing for the AI to build against.
- **Every behaviour needs a requirement, and every requirement a test.** That is overhead on a throwaway script, and CLM says plainly it's a poor fit there.
- **Git and Claude Code, specifically.** Your git host only holds the remote and a pull request for reading the diff. It runs no checks and decides nothing. Moving to another agent would be a change to CLM itself.
- **It is young.** The README warns to expect rough edges and changes to commands and formats. CLM also learned some of its lessons the hard way. Its first versions let the spec grow until its human was proofreading machinery rather than steering a product. That story, with real numbers, comes later in the series.

The deal is this: you give up the comfort of reading every line, and in return the lines are held to something you did write and can read in a few minutes.

## Try it

The site is [clm-app.dev](https://clm-app.dev). In Claude Code:

```
/plugin marketplace add clunny/clm
/plugin install clm@clm
```

Then run `/clm:init` in a repository, new or existing, and follow the quickstart.

Next in the series: **"Every requirement has a test that names it, at its version"**, on the `req:` markers that turn "is it built?" into a question a program can answer.

<!-- social: Your AI agent opens PRs faster than you can read them. CLM moves your review from the diff to the decision: humans own the spec, AI writes all the code, and programs keep them in sync, with git as the only channel. Open source: clm-app.dev -->
