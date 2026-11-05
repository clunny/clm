# Why CLM

AI can now write most of a system's code. What it can't do is decide what the system is for. CLM splits the work along that line: **humans own intent, AI owns code**, and the spec between them is the contract both sides keep.

## The ideas

**Humans review intent, not code.** You describe what the system should do as a graph of specs: components with contracts, designs and requirements with scenarios. You don't read diffs of code. A spec change either needs your approval before it lands or lands on checks, and `clm gate` decides which from what it touches: an edit to the architectural constraints (`ARCHITECTURE.md`), the capabilities (`CAPABILITIES.md`) or the keys a signed repository trusts (`docs/allowed_signers`), a change to who may approve, or a change that declares a burden, something that spends your money or time or can't be undone, waits for you. The rest lands on checks, and nothing is reviewed after it lands.

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

Not every opinion is equally fixed. `clm guide constraints` lists the architectural constraints CLM carries into your project, each with its tier: a `method` constraint is what CLM's machinery relies on, and can't be switched off; a `replaceable` one names the guarantee CLM needs, so you may swap its technique for your own (feature flags rather than expand-then-contract, say) as long as the guarantee holds; a `default` one, such as building nothing speculative, is an opinion you may drop. Replacing or dropping one is an edit to your `ARCHITECTURE.md` that a human reviews, and `/clm:propose` explains what you give up first.

CLM is a poor fit when you want to write code by hand, when the work is a quick script that will never change, or when no one on the team wants to own a spec.

## Where it comes from

- The **Spec Growth Engine** ([arXiv:2606.27045](https://arxiv.org/html/2606.27045v2)) gives CLM its spec graph: per-component contracts, ancestor-scoped context and blocking drift checks.
- **Kubernetes's [controllers](https://kubernetes.io/docs/concepts/architecture/controller/)** give CLM its loop. A controller compares the state you declared with the state that is, and acts until they match. CLM does the same with code: the spec is the declared state, the tests that cite each requirement at its current version show the actual state, and every requirement whose code is behind is built again until they agree. CLM isn't a Kubernetes tool; it borrows that one idea.
- Anthropic's **[AI-native SDLC playbook](https://claude.com/blog/the-ai-native-sdlc-playbook)** shapes the team workflow. From it CLM takes intent written down and committed before any spec or code (a change's `## Intent`, like the playbook's `intent.md`); human attention spent at the gates, on intent and risk, not on reading code line by line; an agent that can act up to a gate but can't pass it (only a human's `clm approve` lands a change that needs review); rules held by hooks and version-controlled settings rather than by meetings; a failing test first for every bug fix; and work as a loop rather than a chain of handoffs. Where the playbook still has a code owner approve each diff, CLM goes further: no human reviews code, and programs check it against the spec.
- **[superpowers](https://github.com/obra/superpowers)**, by Jesse Vincent (MIT licence), supplies the craft. CLM uses its brainstorming skill when you propose a change, to work out what you actually want before anything is written, and its execution skills (test-driven development, systematic debugging, verification before completion) in every session that builds. CLM pins the superpowers version it loads and fetches that copy itself, so every session follows the same method and you install nothing.
- **[OpenSpec](https://github.com/Fission-AI/OpenSpec)** gives CLM the shape of a requirement and its scenarios, and of the deltas a change makes to them ([Spec format](../spec-format.md)).

CLM itself is built with CLM: its own `specs/`, `changes/` and `backlog/` are in its repository.
