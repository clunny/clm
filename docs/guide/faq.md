# FAQ

Short answers to the questions people ask first about CLM: what it is, what it needs, what it costs, and how it compares with other ways of building software with AI.

## What is CLM?

CLM is a Claude Code plugin and a `clm` command-line tool for spec-driven development with AI. You declare what the system should do as a spec, a graph of components with requirements and scenarios; Claude writes all the code, test-first; and programs check that every requirement is proven by a passing test that cites it. When the spec changes, the code that has fallen behind it is built again until they match. CLM is free and open source under the MIT licence.

## Who is it for?

Anyone building software with AI who wants it to stay maintainable: a solo developer, or a team of product owners, architects and developers who would rather decide what a system must do than read every diff. It suits work that will keep changing. It is a poor fit if you want to write code by hand, or for a quick script that will never change.

## How do I install it?

In Claude Code, run `/plugin marketplace add clunny/clm` and then `/plugin install clm@clm`. Then open your repository in Claude Code and run `/clm:init`. The [Quickstart](quickstart.md) walks through the rest.

## What do I need?

Claude Code, git and `curl`, on Linux or macOS. The plugin downloads the `clm` binary from the project's public GitHub releases on first use; no GitHub account is needed. A git host such as GitHub is optional: it holds the remote and shows a pull request per branch, but runs no checks.

## Do I need superpowers installed?

No. CLM uses skills from [superpowers](https://github.com/obra/superpowers), but it fetches its own copy at a pinned version the first time a skill is needed and reads the skills from there. Keep the superpowers plugin turned off in projects that use CLM: its hooks tell every session to brainstorm before any task, while CLM brainstorms only when you propose a change, and builds would follow the plugin's version instead of the pinned one.

## What does it cost?

CLM itself is free. You pay for your own Claude usage, as you would for any Claude Code session. CLM runs no server and needs no paid service: it works on git alone.

## Do I still review code?

No. You review intent: each spec change is a yes or no on what the system should do, and only the changes that touch the architecture, the capabilities or something that costs you money or time wait for you. Code is written by Claude and checked by programs: every requirement needs a passing test citing it at its current version, dead code is flagged, and nothing lands on `main` except through `clm land`, which runs the checks.

## How does CLM stop the code drifting from the spec?

Every requirement has a version, a hash of its text. A test cites the requirement at that version (`req:<component>/<slug>@<version>`). When the requirement's text changes, its old citations no longer count: the requirement shows as behind in `clm pending`, and the queue builds it again until a passing test cites the new version. A marker naming a requirement that no longer exists fails `clm check`, and so does code that belongs to no component, so neither the spec nor the code can quietly leave the other behind.

## Can I use it on an existing codebase?

Yes. `/clm:init` maps the code with `clm survey`, proposes a set of components that covers it, and writes specs that describe what the code does today. You can adopt the whole codebase at once or one part first, and widen it change by change.

## Does it work for a team?

Yes. Git is the only channel: specs, changes, claims on work, approvals and lands are files and branches in the repository, so several people and several Claude sessions can work at once without a shared server. See [Teams](teams.md).

## Which languages does it support?

Any language whose tests can carry a requirement's marker in their source. `clm init` sets up the dead-code checks and the build-and-test script for Rust, TypeScript and Python, and you can add detectors for others.

## Does it work with agents other than Claude Code?

Not today. CLM's method is carried by Claude Code's plugins, hooks and skills. Supporting another agent would be a change to CLM itself.

## How is CLM different from GitHub Spec Kit or Kiro?

Spec Kit and Kiro help an agent turn a feature request into a spec, a plan and tasks, and then build it. CLM keeps one living spec of the whole system, split into components, and holds the code to it for as long as the project lives: each requirement is proven by a test that cites its current version, a change to the spec makes the requirements it touches pending until their code is rebuilt, and the checks on every land fail a broken rule. CLM also decides who must say yes to a change, and records approvals, claims and lands in git, so a team and many parallel sessions can share one repository.

## How is CLM different from writing a good CLAUDE.md?

A `CLAUDE.md` tells an agent how to work; nothing checks that it was followed. CLM's rules are enforced by programs: hooks, `clm check`, `clm gate` and `clm land`. A requirement without a citing test isn't done, and a branch that breaks a rule doesn't land, whatever the agent believed. CLM writes a `CLAUDE.md` for your project too, but the checks don't depend on it.

## Where does CLM come from?

It draws on the Spec Growth Engine paper, the controller loop of Kubernetes, Anthropic's AI-native SDLC playbook, superpowers and OpenSpec. [Why CLM](why-clm.md) says what it took from each. CLM is built with CLM: its own specs, changes and backlog are in [its repository](https://github.com/clunny/clm).
