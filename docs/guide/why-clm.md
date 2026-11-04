# Why CLM

AI can now write most of a system's code. What it can't do is decide what the system is for. CLM splits the work along that line: **humans own intent, AI owns code**, and the spec between them is the contract both sides keep.

## The ideas

**Humans review intent, not code.** You describe what the system should do as a graph of specs: components with contracts, designs and requirements with scenarios. You don't read diffs of code. A spec change either needs your approval before it lands or lands on checks, and `clm gate` decides which from what it touches: an edit to the architectural constraints (`ARCHITECTURE.md`) or the capabilities (`CAPABILITIES.md`), a change to who may approve, or a change that declares a burden, something that spends your money or time or can't be undone, waits for you. The rest lands on checks, and nothing is reviewed after it lands.

**AI writes all the code.** Once a change's intent is approved, Claude builds it per component, test-first, in parallel sessions, each with only the context its component needs: the component, its ancestors, and the contracts of what it depends on. Each component's build lands as its own branch.

**Programs enforce every rule.** Anything that must hold is checked by `clm`, in a hook or when `clm land` lands a branch, never left to prose or to a model's judgment. A requirement counts as implemented only when a passing test cites it at its current version (`req:<component>/<slug>@<version>`); `clm pending` lists the ones whose code is behind. A spec that drifts from the code fails the checks. A spec change that needs your approval can't land without it.

**Git is the only channel.** Specs, changes and claims on work are files and branches in the repository, and approvals, checks and lands are records in each writer's own ledger branch. There's no server or database to keep in step. Two sessions, or two people, coordinate by pushing branches, and CLM resolves the rest by content hashes: anything built on a part of the spec that has since changed is stale, and is built again rather than landed.

## CLM is opinionated

CLM makes a lot of choices for you, and holds you to them. If they don't suit your team, CLM will get in your way. Here's what you sign up for:

- **Every behaviour has a requirement, and every requirement has a test.** Code without a requirement is dead code, and CLM's detectors flag it. A requirement without a citing test isn't done.
- **Humans don't write or review code.** Code changes only through the build flow. If you want to hand-edit code, CLM is the wrong tool.
- **Spec changes are reviewed as intent, yes or no.** A change that needs review waits for a human's `clm approve`; nothing is reviewed after it lands. An approval is pinned to the branch's diff, so pushing new spec text drops it.
- **One branch per piece of work, always based on `main`.** No stacked branches, no long-lived branches. Every branch reaches `main` through `clm land`, which runs the checks on the merged tree on the machine that runs it and lands what passes; nothing else pushes `main`.
- **The spec describes only the present.** Removed requirements are deleted, not marked deprecated. History lives in git and `changes/archive/`, and future intent lives in `backlog/`.
- **Small discoveries are folded in; large ones are deferred.** A session fixes a small bug in the code it's changing, and writes a backlog item for anything bigger.
- **Git and Claude Code.** CLM runs on git and Claude Code (plugins, hooks, skills). A git host (GitHub, a bare server, or none) only holds the remote and a pull request per branch for reading its diff; it runs no checks and decides nothing. Moving to another agent is a change to CLM itself.

CLM is a poor fit when you want to write code by hand, when the work is a quick script that will never change, or when no one on the team wants to own a spec.

## Where it comes from

- The **Spec Growth Engine** ([arXiv:2606.27045](https://arxiv.org/html/2606.27045v2)) gives CLM its spec graph: per-component contracts, ancestor-scoped context and blocking drift checks.
- Anthropic's **[AI-native SDLC playbook](https://claude.com/blog/the-ai-native-sdlc-playbook)** shapes the team workflow: intent written down first, humans reviewing intent, AI implementing.
- **[superpowers](https://github.com/obra/superpowers)** supplies the craft. CLM uses its brainstorming skill when you propose a change, to work out what you actually want before anything is written, and its execution skills (test-driven development, systematic debugging, verification before completion) in every session that builds. CLM pins the superpowers version it loads, so every session follows the same method.

CLM itself is built with CLM: its own `specs/`, `changes/` and `backlog/` are in its repository.
