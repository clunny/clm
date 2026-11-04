# The CLM project

*An article about CLM as a whole: what it is for, what it believes, where it went wrong and where it is going. It is not part of the spec and no check reads it. It describes the direction agreed on 2026-10-29. Much of it is not built yet, and where it differs from `ARCHITECTURE.md` or `specs/`, those describe CLM as it is today.*

## What CLM is

CLM is a way of building software in which AI writes and maintains all of the code, and people decide what the software is for and what it must never do. It ships as a `clm` command-line tool and a Claude Code plugin, and it runs on git alone.

The idea underneath it is simple. AI can now write most of a system's code. What it can't do is decide what the system is for, which trade-offs are acceptable, or whether a feature is worth what it costs. CLM draws the line there: humans own intent, AI owns everything below it, and a written spec between them is the contract both sides keep, enforced by programs rather than good intentions.

## Where it comes from

- The **Spec Growth Engine** paper ([arXiv:2606.27045](https://arxiv.org/html/2606.27045v2)) gives CLM its spec graph. The system is a tree of components, each with a contract that other components can rely on. An AI working on a component is given only the component's ancestors: the path from the root down to it, plus the contracts of what it depends on. Drift between spec and code is a blocking error.
- Anthropic's **[AI-native SDLC playbook](https://claude.com/blog/the-ai-native-sdlc-playbook)** shapes the workflow: write intent down first, have humans review intent, and let AI implement.
- **[superpowers](https://github.com/obra/superpowers)** supplies the craft: brainstorming before designing, test-driven development, systematic debugging, and verification before claiming anything is done.

CLM is built with CLM. Its own specs, changes and backlog live in its repository, and every rough edge in CLM shows up first in the work of building CLM.

## What went wrong

In its first eight days CLM grew to 3,414 commits, about 104,000 lines of Rust in 26 crates, 30 spec components, 454 requirements and 211 recorded decisions, at roughly 237 million tokens of model work. At the end of that time, 70 items were waiting on its one human, 21 of them reviews of changes that had already landed. Eighteen of the 30 components were over their size budget, and 18 backlog items existed only to split components further.

Some of that is velocity and is fine. Most of it came from three structural mistakes.

**The system fed itself.** CLM builds CLM, and its rules required every defect to be reported. Each report became a backlog item, each item became a spec change, and each change added requirements. More requirements pushed components over budget, which created split items, which created more structure and more rough edges. Nothing in the loop asked whether a fix should be a redesign, or whether the rough edge should simply be accepted.

**The spec was organised the way the code is organised.** Components are units of code ownership and build order: `orchestrator.run`, `check.unreferenced`. No level of the spec matched how a person thinks about the product. You couldn't point at "skill evals" and see what it was for, what it cost, and whether you could live without it. Its requirements were spread across three components and five slugs, with four workarounds holding it together.

**Humans reviewed the machine, not the intent.** "Humans review intent" was the founding principle. But everything under `specs/` went through human review, and `specs/` was written at the AI's level of detail. A rule like "the judge counts a test run only when its command has no `|`" was a requirement, the same as "a reviewer can approve a change from a phone". By the usual meaning of the word, most of the 454 were acceptance criteria or test cases, not requirements. The person meant to steer the product ended up proofreading its internals.

The engine itself worked. It parsed the graph, tracked staleness by content hash, built components in parallel, proved them with tests and landed them, and it built itself. What was wrong was the information model: what humans own, and at what level they see it. That is why the plan is to re-found CLM, not to start over.

## The model: two human roles, three layers

Two human roles remain in software development. Every other traditional role, including developer, tester, reviewer, release engineer and technical writer, is handled by AI.

- The **product manager** decides what the product lets people do and why. They own the **capabilities**.
- The **architect** decides what must always be true of the product and how it is shaped at the largest scale. They own the **architecture requirements**.

These are two kinds of content, not two kinds of person. Anyone trusted on the project may approve either, and one person may hold both roles, as CLM's own author does.

| Layer | Owner | What it holds | Rough size | Review |
|---|---|---|---|---|
| **Capabilities** | Product manager | What the product lets you do, why, and how you would know it works | 15–30 | A trusted human |
| **Architecture requirements** | Architect | Constraints, invariants, quality bars: "git is the only channel", "no API key is stored", "every land runs on the lander's machine" | 30–60 | A trusted human |
| **Implementation spec** | AI | Components, contracts, designs, detailed rules, edge cases, tests | Whatever it takes | Deterministic checks only |

The implementation spec is fluid. AI changes it freely, and it lands on checks alone. It is held to the human layers in three ways:

- **Traceability.** Every implementation rule names the capability or architecture requirement it serves, and a check fails any rule that names none. A rule nobody can trace to a purpose is dead weight, and the check finds it.
- **Proof at every level.** Capabilities and architecture requirements have their own tests, usually end to end, so the human layers can't turn into prose nobody checks. Implementation rules keep their own tests, as today.
- **Cost rolls up.** The effort spent on every rule is added to the capability it serves (see below), so a capability's cost is visible to the person who decides whether it is worth having.

### What the words mean

CLM uses its terms close to their everyday meaning in software teams. Earlier versions did not.

- A **capability** is something the product lets someone do: "approve a change from a browser", "see what's holding the work up". It is the unit of value, and the unit a product manager keeps or cuts. (Earlier CLM had a `capabilities` field on each component that nothing read. It goes away.)
- A **feature** is the everyday name for a capability or part of one. CLM needs no separate concept for it.
- An **architecture requirement** is a requirement in the traditional sense: something the system must satisfy, written by the architect. Today's Layer 1 invariants are the start of this layer.
- An **implementation rule** is what CLM used to call a requirement: one SHALL statement with scenarios, an ID of the form `<component>/<slug>`, and tests that cite it at its current version.
- A **component** is a unit of code ownership: a module or component. Components belong to the AI layer. Humans rarely need to look at them.
- A **change** is a proposal: an RFC, with its intent, the deltas it makes, and its decisions. It is an event, not a lasting thing. What lasts is the capability or requirement it creates or changes.
- **The spec** is all three layers together, and it always describes the system as it is now.

## Principles

These are CLM's invariants, grouped by what they protect. The first two groups were there from the start. The last group is new, learned the hard way.

### Who owns what

- **AI writes all the code.** Humans don't author or review code. Code changes only through the build flow.
- **Humans own intent.** Humans own capabilities and architecture requirements and approve changes to them. They don't approve implementation detail.
- **Review is yes or no.** A change either needs human review or it doesn't. A change to a capability or an architecture requirement, or one that adds a burden a human must agree to (see *How the AI layer reaches you*), waits for a trusted person's approval. Everything else lands on checks alone, with no review after the fact. Earlier CLM had three tiers, `HARD`, `SOFT` and `AUTO`, with an after-the-fact review queue for `SOFT`. The queue grew faster than anyone could read it, and the word "hard" made review sound difficult rather than necessary. The tiers are gone. A change *needs review*, or it doesn't.

### Proof and enforcement

- **Programs enforce every rule.** Anything that must hold is checked by a program (the CLI, a hook, the land script), never by prose or by a model's judgment alone. Judgment can advise, but it never stops a land.
- **Tests prove the spec.** A rule or requirement counts as built only when a passing test cites it at its current version (`req:<id>@<version>`). A citation of an older version means the code is behind.
- **No silent drift.** A divergence between spec and code is a blocking error on `main`.
- **The spec describes only the present.** Removed requirements are deleted, not marked deprecated. History lives in git and the change archive, and future intent lives in the backlog.

### Git and teams

- **Git is the only channel.** Specs, changes, claims, approvals and lands are files, branches and ledger records in the repository. No server or database owns any state. Two people, or two AI sessions, coordinate by pushing branches.
- **Optimistic concurrency by content hash.** Everything derived from the spec records the hashes of what it was based on. When those change, it is stale and gets rebuilt rather than merged.
- **Derived files are regenerated, never merged.**
- **Every branch lands on `main` through one door,** `clm land`, which runs the checks on the merged tree on the lander's own machine. No stacked branches, no long-lived branches.
- **Small human layers merge easily.** Because capabilities and architecture requirements are few and change slowly, a team can edit them concurrently with few conflicts. The fluid AI layer is reconciled by machines.

### Building

- **Build by reconciliation.** Work is per component, not per change. A component's code is brought up to its whole current spec at once. Rules that haven't caught up are pending, and the queue builds the most important pending work first, in parallel. Ordering comes only from the explicit needs between rules (`Needs`), never from coarse node-level dependency lists. A change that touches only one component builds straight away, whatever else is pending.
- **Minimal context.** An AI building a component sees the component's ancestors and the contracts of what it needs, and nothing else.

### Restraint (new)

- **You ain't gonna need it.** Build what a capability needs now, not what it might need later. No speculative options, adapters, settings or extension points without a capability asking for them. Keeping things minimal is the default, and adding complexity needs a reason.
- **Reuse first.** A standard, widely used solution or industry practice is the default. Writing our own needs a reason. Adding a library or a tool is the preferred way to get something done, not a cost to be avoided. CLM's own code should be glue and checks.
- **Redesign before you work around.** When something gets in the way (a defect, an edge case, a tool's limit), the first question is "is the design wrong?", not "what can we patch?". There are three acceptable answers:
  - **Redesign** it.
  - **Accept it** as the way things are: write it down and do nothing. That is a complete answer, not a deferral.
  - **Make a small workaround** for a genuine edge case.

  Workarounds are counted against the capability they serve. Several on one capability, or one with many edge cases, is a red flag: stop and decide whether to redesign, drop or keep it. Where possible this should be foreseen. A proposal lists the edge cases and workarounds it already knows it will need, and a long list at that stage raises the flag before anything is built. An AI session that hits its third workaround on the same thing stops and asks instead of pushing on.
- **The right model for the task.** Each kind of work runs on a model suited to it, and every agent CLM launches names its model explicitly, because an agent launched without one silently inherits the session's model. CLM reuses superpowers' model policy (the Model Selection rules of `subagent-driven-development`) rather than inventing its own.
  - **"The most capable model" means the human's main session model**, whatever that is (Opus, say), never a fixed top-tier name. It is for work where judgment is the product: brainstorming intent, design, hard debugging, and the final review of a build.
  - **A faster, cheaper model** (Sonnet, or Haiku for mechanical steps) is for work that is well specified and checked by programs anyway: building a pending rule test-first, running checks, rebasing, writing up a pull request.

  Builds keep superpowers' **inline ("Native") execution**: one session builds every step of its plan, then one fresh reviewer on the main session's model reviews the whole branch. They don't use `subagent-driven-development` itself. A fresh implementer and reviewer per task, with up to five fix rounds, multiplies the agents on builds that are deliberately small, and the skill's worktree setup and finishing steps would have to be blocked, which is a skill used with workarounds. So the policy applies where CLM does the launching: the build subagents, the drafts, the spec auditor and the headless runner. A build that keeps failing on the cheaper model steps up rather than grinding on. Model choice is part of effort accounting, so the policy is judged by its results, which is also what replaces the eval that used to gate changes to a build's skill list.
- **Ask the architect rather than guess.** When there is no clear standard answer, or reuse and minimalism pull in different directions, the AI asks the architect instead of guessing.
- **No dead code.** Code exists only to serve a current rule. Removing behaviour removes its code, tests and dependencies. A breaking contract change goes expand, then contract.
- **Small nodes.** A component stays small enough for one build session to hold its spec and its code. Splitting components is the AI's business and needs no human.

### Effort is accounted (new)

- **Effort is part of the spec.** Every capability, and every rule under it, carries a running record of what it has cost. Keeping a capability is a decision made against that cost, and it is reviewed periodically. This is an invariant, not a report: CLM records the effort whatever else happens, and the review page is how the product manager and the architect decide what stays.

## How a change flows

1. **Propose.** A person, or an AI session working from the backlog, opens a spec branch with a change: its intent, the deltas it makes, and the edge cases and workarounds it already expects. Proposing starts with a conversation (superpowers' brainstorming) to find out what the person actually wants.
2. **Review.** If the change touches a capability or an architecture requirement, or adds a burden that needs agreement, it waits for a trusted person's approval through a guarded prompt. Otherwise it lands on checks.
3. **Apply.** The change's deltas are folded into the spec, and new rules start out pending.
4. **Build.** The queue brings each pending component up to its spec, test-first, in parallel isolated worktrees, highest priority first. A rule waits only for the specific rules it needs.
5. **Prove.** Every test cites the rule it proves at its current version. Each build also checks that its tests would fail if the key rules were broken, and gets a code review and a spec audit before it lands.
6. **Land.** `clm land` runs the checks on the merged tree and pushes `main`.
7. **Archive.** A change whose rules are all current moves to the archive.

### How the AI layer reaches you

Implementation work is free, but some of it creates work or cost for people. There are exactly three ways it reaches a human, and no others:

- **It waits** when a change spends your money or your time, or can't be undone. Examples: recurring model runs or paid services, a step only you can do (installing something, creating an account, a DNS record, a secret), or anything outward-facing (publishing, a release, sending data to an outside service). These need a yes before they land, like a capability change. A step only you can do should be asked about at design time, not discovered during the build.
- **It flags** a burden that can be undone: workarounds piling up, new edge cases, slower or flakier lands. The change lands, and the flag appears on the capability's review page, next to the change that caused it, and as a line in `clm status`. It stays there until someone decides: keep, cut, redesign, or accept as is.
- **It asks** when an architectural choice isn't clear. A question is filed as a backlog item that blocks only the rules waiting on the answer, and the rest gets built.

Programs can detect some burdens: a new human-only backlog item, a new budget or cost setting, a release step. Others, like "this now calls a paid model on every land", can't be detected reliably. So the AI declares the burdens a change adds, and the checks verify what they can. Effort accounting is the safety net for what neither catches: a cost nobody declared still shows up in the numbers.

## Effort and value

CLM records what every capability costs, so that keeping it is a decision rather than an accident.

**What is measured**, per implementation rule and rolled up to the capability it serves:

- **Model work:** tokens spent designing, building and fixing, recorded per model and converted to a cost, because a token on Opus and a token on Haiku are not the same spend.
- **Time:** wall-clock time of the sessions that worked on it.
- **Churn:** commits, and the number of changes that touched it. A rule that keeps changing has a design that never settled.
- **Human attention:** approvals, answered questions and resolved flags. This is the scarcest resource, and it counts the most.
- **Workarounds and edge cases:** each one with what it is, why it exists, when it was added, and whether it is resolved.
- **Money:** spend that the product incurs as it runs, such as model runs at land.
- **Blocking:** work the rule has held up.
- **Size:** rules, and lines of code.

Some of this can be counted exactly: commits, changes, reviews, workarounds, blocking items. Model work and time come from session transcripts, which record the branch each session worked on. That attributes cleanly to a component (`impl/<component>`) or a change (`spec/<id>`), and through them to capabilities, but only by estimate to a single rule. CLM's own history can be filled in after the fact this way. About a quarter of it happened on `main` and can't be attributed, and sessions will record what they work on so that gap closes.

**Value can't be computed. It is asked for.** The cost view is sorted against a value rating that the product manager gives each capability during review. Cost alone would mislead: the core build loop is expensive and obviously worth it. What review looks for is capabilities that are expensive, churn a lot, and are rated low.

**The review page** answers three questions for each capability:

1. **Why do we need it?** Its purpose, written once, in the product manager's words.
2. **What is it costing?** The measures above, with each workaround and flag spelled out in full, not as a bare slug.
3. **Can we live without it?** What would be lost, and what else already covers part of it.

There are two levels:

- **The list** shows every capability ranked by cost, with red flags highlighted.
- **A capability's page** shows the three answers, with drill-down into its rules, because sometimes one rule is the problem rather than the whole capability. Each capability ends with four decisions:
  - **Keep** it.
  - **Cut** it: a removal change, after which `no-dead-code` deletes the code.
  - **Redesign** it.
  - **Accept it as is**, which settles its open flags.

The review is a `clm` command whose text output works in a Claude Code session. The web app and the panel show the same thing, and the web view works on a phone, where the list becomes a stack of cards and each workaround opens to its details.

## What you sign up for

CLM is opinionated. If its opinions don't suit a team, it will get in the team's way.

- Humans don't write or review code. If you want to hand-edit code, CLM is the wrong tool.
- Someone must own the product's capabilities and someone its architecture, even if that is the same person.
- Every capability earns its place against its cost, and gets reviewed periodically.
- Standard solutions over home-made ones. Minimal over clever. Redesign over patching.
- One branch per piece of work, based on `main`, landed through `clm land`.
- Git and Claude Code. A git host (GitHub, Azure DevOps, a bare server, or none) only holds the remote and a pull request per branch. It runs no checks and decides nothing.

CLM is a poor fit when you want to write code by hand, when the work is a script that will never change, or when no one wants to own the product's intent.

## Strategy: re-found, don't restart

Starting over would bring the same habits and the same rules, and would throw away the part that works: an engine that parses the graph, tracks staleness, builds in parallel, proves by tests and lands safely, and has been proven by building itself. What needs to change is the information model above it. So CLM is re-founded in place.

1. **Stop the growth.** For now, defects are answered with redesign or acceptance, not new features. No new components. Only fixes and removals.
2. **Quick removals that need no new model:**
   - **Drop the node-level build wait.** A component currently can't build until every component it depends on has nothing pending. The `clm` root lists 22 dependencies, so a one-line change to the root's build settings waited on unrelated work across the tree. Explicit `Needs` between rules already state the real dependencies precisely.
   - **Drop the evals at land.** The superpowers eval fails its own judge whenever a build pipes `cargo test`, which blocks upgrades and some lands. The plugin's skill evals would spend money on most lands that touch the plugin, can fail by chance, and need tools installed by hand. Keep only `claude plugin validate`, which is cheap and gives the same answer every time.
3. **Write the two human layers.** Draft the capabilities from the current specs (a first draft is below) for the product manager to cut down. Lift the architecture requirements out of today's Layer 1 invariants and the decisions that carry real constraints.
4. **Map the 454 rules under them.** Each rule names the capability or architecture requirement it serves. Rules that trace to nothing, and the code behind them, are removed.
5. **Collapse review to yes or no.** Remove `SOFT`, the after-the-fact review queue and most of the tier computation. A change needs review exactly when it touches a human layer or adds a burden that waits.
6. **Build effort accounting,** with the history filled in from git and the session transcripts, and the review page in the CLI, the panel and the web app.
7. **Review and prune.** The first review, run against real numbers, decides what else goes.

The re-founding probably has to be done by hand on a branch, as the earlier redesign of the build queue was: CLM can't easily carry a change to its own information model through itself.

### Risks

- **The free AI layer bloats again.** The 454 rules were written by AI, and this plan gives AI more freedom. What holds it in check is the combination of traceability, cost rolled up to capabilities that people review, the workaround red flag and the minimalism invariant. If those don't hold, it will be visible in the numbers, which is the point of having them.
- **"Not clear" is a judgment.** Asking the architect when a choice isn't clear can produce a stream of questions, or none. The cost view keeps it honest: a capability whose questions keep coming, or whose guesses keep being overturned, shows up there.
- **Burdens that can't be detected.** Declaration plus accounting catches most of them late rather than never. A burden that is both undeclared and invisible in the numbers is a hole.

## Appendix: a first draft of CLM's capabilities

Drafted from the current specs and accepted as is on 2026-10-29, with its three candidate cuts agreed. Each line names what it lets you do, and the components that hold most of it today. The parts marked **cut** go in the re-founding.

**Defining the product**

1. **Adopt CLM in a repository:** set up the tooling and draft a first spec, from a conversation for a new project or from the code for an existing one (`init`, `clm survey`, `clm guide`).
2. **Propose a change of intent:** turn an idea into a reviewed spec change, with a conversation to find out what's wanted and a fresh reader checking it can be built (`plugin` propose skill, `apply`, `model`).
3. **Approve intent:** read a change's pull request and approve it through a guarded prompt, only as a human (`land` approve, `plugin.hooks` guard, `ledger`).
4. **Keep a backlog:** record ideas, deferred work, questions and bugs, each linked to what it blocks (`orchestrator` backlog, `clm backlog`).
5. **Triage the backlog unattended:** group the backlog into pieces of design work and draft them in the background (`orchestrator.triage`).

**Building it**

6. **Build from the spec automatically:** work out what's pending, claim it, and build it test-first in parallel isolated sessions (`orchestrator`, `orchestrator.run`, `context`).
7. **Use the right model for each task:** superpowers' model policy, applied wherever CLM launches an agent. Each build, draft, audit and runner session names its model: a cheaper one for well-specified building, and the main session's own model for judgment and final review, stepping up when a cheaper model keeps failing (new, not built: today everything inherits the session's model).
8. **Run builds unattended on your own machine:** a headless runner that drains the queue (`orchestrator.runner`).
9. **Prove the code meets the spec:** versioned test citations, and lists of what's current, behind or pending (`check.coverage`).
10. **Catch drift and staleness:** deterministic checks that the spec, the code and everything derived from them agree (`check`, `model`).
11. **Land safely on `main`:** one door that merges, runs only the tests a change can affect, checks the result and records it (`land`, `ci` check script, `check.affected`, `ledger`).
12. **Work concurrently without collisions:** claims, rebasing and syncing, snapshot pushes, and merge drivers for derived files (`orchestrator` claims, `sync`, `vcs`).
13. **Keep the code free of dead weight:** detect code nothing uses and dependencies nothing needs, and track planned removals (`check.unreferenced`, `clm check` detectors).
14. **Wrap up a session cleanly:** say what's left unsafe, remove merged work, archive finished changes (`orchestrator.checkpoint`, `sync`).

**Seeing what's happening**

15. **See the state of the project:** status, the architecture, the work in flight, and what's holding it up (`view`, `clm status`).
16. **Watch it beside the chat:** the CLM pane in Claude Code (`plugin.panel`).
17. **Follow and approve from a browser or phone:** the web app, with live updates and push notifications (`web`, `web.worker`, `web.live`).
18. **Know the project's health:** one advisory report, including size budgets and workaround flags (`check.health`).
19. **Review what each capability costs:** effort accounting and the review page (new, not built).

**Running CLM itself**

20. **Guard AI sessions:** hooks that stop unsafe actions, such as an AI approving its own change (`plugin.hooks`).
21. **Report defects in CLM:** file a defect from inside a session (`clm defect`, `plugin.hooks`).
22. **Work with your git host:** pull requests and issues on GitHub, Azure DevOps, a bare server or no host (`host`). **Cut**, in part: the Azure DevOps adapter has no user and is blocked on an account.
23. **Configure CLM:** settings for the repository, the clone and the user (`config`).
24. **Install and release CLM:** the release workflow, the plugin packaging, and the self-hosting install for CLM's own repository (`ci`, `plugin`, `clm` root).
25. **Pin the AI's working method:** a fixed superpowers version that every build follows, with an eval that gates upgrades (`orchestrator.run`, `clm superpowers`). **Cut**: the eval, not the pin.
26. **Check that the plugin's skills are followed:** model-run evals of each skill at land (`ci`, `plugin`). **Cut**: keep only `claude plugin validate`.

Some of what the specs hold today is not a capability. It belongs to the architecture layer or the AI's own machinery: content hashes, minimal context bundles, component size budgets and splitting, gate tiers and the significance rubric, merge drivers. Gate tiers and the significance rubric mostly go away under yes-or-no review. The rest moves under the capabilities it serves.

## Open questions

- **How often to review.** At CLM's pace, a weekly review of the capability list is probably right, but this hasn't been decided.
- **Workaround threshold.** Three workarounds on one capability is a placeholder for the red flag.
- **Attributing cost to single rules.** Branches attribute cleanly to components and changes. Splitting that cost across individual rules needs an estimate, for example from which commits and tests cite each rule. The method is still open.
- **Capability tests.** What an end-to-end test of a capability looks like in practice, and how many of them a capability needs.
