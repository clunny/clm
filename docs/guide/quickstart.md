# Quickstart

From nothing to your first change built, for a new project or an existing one.

## You need

- [Claude Code](https://claude.com/claude-code) or [Codex](https://developers.openai.com/codex), on Linux or macOS. Another coding agent works too, if it reads `AGENTS.md` and runs shell commands: see [Other agents](#other-agents).
- `git` and `curl`.
- A git repository for your project, where you can push to `main`. GitHub is optional.

## 1. Install the plugin

In Claude Code:

```
/plugin marketplace add clunny/clm
/plugin install clm@clm
```

In Codex, add the same marketplace, `clunny/clm`, with Codex's plugin commands and install `clm` from it. The skills are the same, and so is the binary.

The plugin downloads the `clm` binary the first time it needs it. No account or sign-in is needed.

## 2. Run `/clm:init`

Open Claude Code in your repository and run `/clm:init`. Claude asks you a few questions, one at a time:

- what the project is and who it is for;
- what a person can do with it;
- what must always hold, such as "all money is integer pence";
- the language and stack.

For an existing project, Claude reads the code first and asks only what it can't work out. It also asks whether to bring the whole codebase under CLM now or one part first.

## 3. Push the first commit

Claude commits CLM's setup files and then stops, because only you push to `main` directly, and only this once. Push the commit, and if your host protects `main`, let your own account push to it. Then tell Claude it's done.

## 4. Approve your first design

Claude drafts the first design of your project: what it does, what must always hold, and how it splits into components. For an existing project, it describes what the code does today and shows you the split before writing the rest.

Read it as intent: is this what the system is, or should be? Ask for changes in the conversation. When you're happy, say yes, and confirm the command Claude Code shows you. Codex can't ask you to confirm a command, so there the agent asks you to run `clm approve` in your own terminal instead. The design lands on `main`.

## 5. Run `/clm:build`

Claude builds every part of the design that is ready, in parallel, test first, and lands each part once its checks pass. You don't review the code: programs check that it matches the design.

## What next

Day to day you use four workflows, each described in [Using CLM](using-clm.md):

| Run | To |
|---|---|
| `/clm:propose` | change what the system does |
| `/clm:build` | build what has been approved |
| `/clm:triage` | work through the backlog: approvals, questions for you, bugs and ideas |
| `/clm:help` | ask how CLM works |
| `/clm:health` | see how healthy the project is |

To understand what happens underneath, read [How it works](how-it-works.md).

## Other agents

A project under CLM keeps its instructions in `AGENTS.md`, and `CLAUDE.md` imports it, so every agent reads the same rules, and people on different tools can work on one project. With an agent that has no CLM plugin, put the `clm` binary from the [release](https://github.com/clunny/clm/releases) on your `PATH`, ask the agent to follow the skills' steps from `AGENTS.md`, and always run `clm approve` yourself, in your own terminal. `clm harness` says which agent a command runs under and what CLM can use there.
