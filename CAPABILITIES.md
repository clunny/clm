# CLM Capabilities

The capabilities: what a person can do with CLM, and why. The product manager owns this file.
An edit to it needs a human's review before it lands (`humans-review-intent`).
Tests cite an entry as `req:capability/<slug>@<version>` (`clm hashes capability`); the format is in `clm guide spec-format`.
What each capability costs is shown by `clm` from the ledger, not stored here.

## Capabilities

### Capability: learn-clm
A newcomer can learn in a browser what CLM is, why it matters for building software with AI solo or as a team, and how to adopt it: a homepage that makes the value clear within a minute, and the user guide.

#### Why
People adopt what they understand, and until now CLM explained itself only once installed.

#### Scenario: from homepage to quickstart
- GIVEN the site, opened by anyone, with no session
- WHEN they read the homepage's first screen and follow Get started
- THEN they have seen that CLM reconciles the code to the intent they declare, and they are on the guide's quickstart, with the commands that install CLM

### Capability: adopt-clm
A person can bring a new or existing repository under CLM: set up its tooling and get a first spec, drafted from a conversation or recovered from the code.

#### Why
Nothing else works until a repository has specs, and writing them by hand is the step most people would skip.

#### Scenario: existing codebase
- GIVEN a repository with code and no `specs/`
- WHEN the person runs `/clm:init` and agrees the component map it shows them
- THEN a spec branch holds a component per part of the code, describing what it does today, and `clm check` passes on it

### Capability: propose-intent
A person can turn an idea into a spec change that a fresh reader has checked can be built.

#### Why
Intent written down first is what the AI builds from and what the human reviews; an idea that never becomes a checked spec change is lost or built wrong.

#### Scenario: an idea becomes a spec branch
- GIVEN a person who describes a behaviour they want
- WHEN they run `/clm:propose`
- THEN a spec branch holds the change, `clm check` passes on it, and it lands on checks or waits for their approval

### Capability: approve-intent
A human can read a change that needs review and approve it through a prompt only a human can confirm.

#### Why
Changes to what the product is for, or what must always hold, are the human's to decide; an AI must never be able to approve them for itself.

#### Scenario: approve a change
- GIVEN a spec branch that edits `CAPABILITIES.md`
- WHEN the human runs `clm approve` and confirms the prompt
- THEN `clm land` lands it, and the ledger records whose approval it was

#### Scenario: a session cannot approve alone
- GIVEN the same branch
- WHEN a session runs `clm approve` and the human declines the prompt
- THEN nothing is approved and `clm land` refuses the branch

### Capability: keep-a-backlog
A person or session can record ideas, deferred work, questions and bugs, each linked to the work it blocks.

#### Why
Work found along the way must not be lost or silently folded into an unrelated change, and a question only a human can answer must hold back exactly the work that needs it.

#### Scenario: a question blocks work
- GIVEN a backlog item of kind `question` that blocks one requirement
- WHEN `clm pending` runs
- THEN that requirement shows as blocked by the item, and the component's other requirements don't

### Capability: triage-unattended
A person can have the backlog grouped into pieces of design work and drafted in the background, then approve the drafts that need review at once.

#### Why
A backlog grows faster than a person drafts changes for it, and reading drafts is cheaper than writing them.

#### Scenario: triage a backlog
- GIVEN a backlog with several items
- WHEN the person runs `/clm:triage`
- THEN the drafts that land on checks land, and the ones that need review are gathered into one branch for one approval

### Capability: build-from-spec
A person can have every pending requirement built automatically: claimed, built test-first in parallel isolated sessions, and landed.

#### Why
AI writes all the code, so building from the spec has to run without a human driving each step.

#### Scenario: drain the queue
- GIVEN two components with pending requirements and no `Needs` between them
- WHEN the person runs `/clm:build`
- THEN both are built in parallel and landed, and `clm pending` is empty

### Capability: right-model-per-task
A person can rely on every agent CLM launches running on a model suited to its work, named explicitly.

#### Why
An agent launched without a model inherits the session's, so cheap mechanical work runs on the most expensive model and the cost grows unseen.

#### Scenario: a build names its model
- GIVEN a component ready to build
- WHEN CLM launches its build
- THEN the build's agent names its model, a cheaper one than the main session's, and its final review runs on the main session's model

### Capability: unattended-runner
A person can run builds unattended on a machine they own, draining the queue with no session open.

#### Why
Building shouldn't wait for someone to keep a chat window open.

#### Scenario: runner drains the queue
- GIVEN a runner started on the person's machine and a component with pending requirements
- WHEN the runner runs
- THEN it builds and lands the component without anyone in a session

### Capability: prove-by-tests
A person can see, for every requirement, whether a passing test proves it at its current version, and what is behind or pending.

#### Why
The spec is only worth what proves it; a requirement no test cites is a promise, not a fact.

#### Scenario: reworded requirement
- GIVEN a requirement proven by a test, which is then reworded
- WHEN the person runs `clm coverage`
- THEN it shows the requirement as `behind`, naming the test

### Capability: catch-drift
A person can rely on deterministic checks that the spec, the code and everything derived from them agree.

#### Why
Drift between spec and code is silent until it breaks something; a program that refuses it keeps the spec true.

#### Scenario: test cites a removed requirement
- GIVEN a test citing a requirement that no spec holds
- WHEN the person runs `clm check`
- THEN it fails, naming the test's marker

### Capability: land-safely
A person can land any branch on `main` through one command that merges it, checks the result and records the land.

#### Why
One door, run on the lander's own machine, means `main` is always checked and nothing lands by a side route.

#### Scenario: a branch that breaks main
- GIVEN a branch whose merge with `main` fails a test
- WHEN the person runs `clm land`
- THEN `main` is unchanged and the failure is shown

### Capability: work-concurrently
Several people and sessions can work on the same repository at once without overwriting each other's work.

#### Why
Parallel builds are how CLM is fast, and parallel work that collides costs more than it saves.

#### Scenario: two sessions, one node
- GIVEN two sessions that try to claim the same component at once
- WHEN both claim it
- THEN exactly one gets it, and the other is refused

### Capability: no-dead-weight
A person can rely on code nothing uses, and dependencies nothing needs, being found and removed.

#### Why
Code without a purpose costs every later build that reads it, and nobody removes what nothing points at.

#### Scenario: unused dependency
- GIVEN a dependency nothing uses
- WHEN the person runs `clm check`
- THEN it fails, naming the dependency

### Capability: wrap-up-session
A person can end a session knowing what's left unsafe, with merged work cleaned up and finished changes archived.

#### Why
A session that ends with work unpushed or half-landed leaves the next one to find it out the hard way.

#### Scenario: end of session
- GIVEN a session with one branch landed and one not pushed
- WHEN the person runs `clm wrap-up`
- THEN it cleans up the landed one and names the unpushed one as unsafe to leave

### Capability: see-project-state
A person can see the project's state: its components, the work in flight and what is holding it up.

#### Why
People decide what to do next from what they can see, and the state lives scattered across branches and files.

#### Scenario: status
- GIVEN a repository with pending, claimed and blocked work
- WHEN the person runs `clm status`
- THEN it shows each, and names the commit it describes

### Capability: watch-beside-chat
A person can watch the project's state update in a pane beside their Claude Code chat.

#### Why
Switching to a terminal to ask for the state breaks the conversation it informs.

#### Scenario: pane updates
- GIVEN the CLM pane open in Claude Code
- WHEN a build lands
- THEN the pane shows the new state without the person asking

### Capability: follow-from-browser
A person can follow a change's progress and approve it from a browser or a phone.

#### Why
Approvals shouldn't wait for someone to reach a terminal.

#### Scenario: approve on a phone
- GIVEN a change that needs review, in an unsigned repository
- WHEN an invited reviewer opens its page on their phone and approves it
- THEN the approval is recorded and `clm land` counts it

### Capability: know-health
A person can read one advisory report of the project's health: what's oversized, what was hard to build, and what to reconsider.

#### Why
Problems that block nothing accumulate unseen; one report makes them visible without stopping work.

#### Scenario: health report
- GIVEN a repository with an oversized component and a flagged requirement
- WHEN the person runs `clm health`
- THEN it reports both and exits 0

### Capability: review-capability-cost
A product manager or architect can see what each capability and each architectural constraint has cost, and whether each architectural constraint is actually held, and decide whether it stays.

#### Why
Nothing showed what a feature cost, so nothing was ever cut; keeping a capability should be a decision made against its cost. An architectural constraint is a rule every change pays for, so keeping one is the same kind of decision, made knowing whether its enforcement is built.

#### Scenario: review page
- GIVEN capabilities whose rules have been built over several lands
- WHEN the person opens the review page
- THEN it shows each capability's cost, and records their decision to keep or cut it

#### Scenario: architecture review page
- GIVEN architectural constraints, one whose enforcing rules are all built and one whose enforcing rule is still pending
- WHEN the person opens the architecture review page
- THEN it shows each one's cost and that the second isn't held yet, and records their decision to keep or cut it

### Capability: guard-sessions
A person can rely on AI sessions being stopped from unsafe actions, such as approving their own change or pushing to `main`.

#### Why
A session that can approve or push for itself makes every other rule optional.

#### Scenario: push to main
- GIVEN a session on `main`
- WHEN it runs `git push`
- THEN the guard blocks it

### Capability: contribute-from-outside
A person without write access can suggest what a project should do, from a one-line issue to a spec change in a pull request from a fork, and its maintainer can land what they accept, with the person credited as its author. Code from outside can't land by that route, and landing a suggestion runs none of the person's code.

#### Why
Open-source projects grow through people outside them, but under CLM nobody reviews code and a land runs the branch's checks on the lander's machine, so what outsiders can safely offer is intent.

#### Scenario: a suggestion lands
- GIVEN a branch from a fork, by another author, that only adds a backlog item, pushed as `spec/x`, and a `main` whose code has already been checked
- WHEN the maintainer runs `clm land spec/x`
- THEN it lands, the contributor's commit is on `main` under their name, and the project's check command doesn't run

#### Scenario: code from outside
- GIVEN a branch from a fork that changes a code file, pushed as `spec/x`
- WHEN the maintainer runs `clm land spec/x`
- THEN it refuses, naming the file, the project's check command doesn't run and `main` is unchanged

### Capability: report-defects
A person or session can report a defect in CLM itself from inside a session, where it happened.

#### Why
CLM gets in the way most visibly while it is being used, and a defect not filed then is forgotten.

#### Scenario: file a defect
- GIVEN a session where a check misfired
- WHEN it runs `clm defect`
- THEN a backlog item of kind `bug` records it, on the component the fix lies in

### Capability: git-host
A person can use CLM with GitHub, a bare git server, or no host at all.

#### Why
Git alone is the platform; a host adds pull requests and issues but must not be required.

#### Scenario: bare server
- GIVEN a repository whose remote is a bare git server
- WHEN a person proposes, approves and lands a change
- THEN every step works, with no pull requests

### Capability: configure-clm
A person can set CLM's settings for the repository, for one clone and for themselves, and see where each came from.

#### Why
Teams and machines differ, and a setting whose source is hidden can't be trusted.

#### Scenario: where a setting comes from
- GIVEN a setting in `clm.toml` and an override in `clm.local.toml`
- WHEN the person runs `clm config get` for it
- THEN it prints the override's value and the file it came from

### Capability: install-and-release
A person can install CLM as a Claude Code plugin with its `clm` binary, and its maintainers can release new versions.

#### Why
CLM reaches people only through what they install, and a release that doesn't match its binary breaks every session.

#### Scenario: install
- GIVEN a machine with Claude Code and no Rust toolchain
- WHEN the person installs the plugin
- THEN its first session runs the release's `clm` binary of the same version

### Capability: pin-working-method
A person can rely on every build following the same fixed version of the AI's working method.

#### Why
A working method that changes under a build makes its results impossible to compare or trust.

#### Scenario: pinned method
- GIVEN the pinned superpowers version
- WHEN any build starts
- THEN it runs that version, whatever the session has installed
