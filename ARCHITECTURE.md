# CLM Architecture

The architectural constraints: what must always hold of CLM, whatever is built. The architect owns this file.
An edit to it needs a human's review before it lands (`humans-review-intent`).
It is prepended to every context bundle, so keep each entry short; the mechanism lives in the decision or node spec it names.
Each entry names the rules that enforce it under `#### Enforced by`, and says under `#### Untested` what no program can check and why; it is current when every rule it names is current (`human-layer-proof`). The format is in `clm guide spec-format`.
An entry holding a `#### Carries` block reaches every project built with CLM, at its tier, from the `clm` that project runs (`carried-constraints`); the others are CLM's own.

## Constraints

### Constraint: ai-writes-code
AI SHALL write all code. Humans do not author or review code, and code changes only through the build flow.

#### Scenario: code reaches main
- GIVEN a change to a component's code
- WHEN it lands on `main`
- THEN it was written by a build session, and no human was asked to review the diff

#### Carries
- method

#### Untested
Who wrote a diff isn't visible to a program; the build flow it relies on is enforced through `land-locally` and `humans-review-intent`.

### Constraint: humans-review-intent
Humans SHALL review intent, not code. A spec branch needs a human's approval, given with `clm approve` before it lands, exactly when it edits `ARCHITECTURE.md`, `CAPABILITIES.md` or the trusted keys in `docs/allowed_signers`, changes `clm.toml`'s `approval` setting, or adds or edits a change that declares `human_burdens` or whose `change.md` can't be read, which fails closed. A branch whose whole diff `clm rename` reproduces from the base and the renames its change records is a spec branch, its rewritten test markers included, and its replayed edits to `ARCHITECTURE.md` and `CAPABILITIES.md` don't need review, since renaming an ID changes no intent; one that swaps a term does, since only a human can judge that a swapped word keeps a rule's meaning (`rename-by-replay`). Every other change lands on checks, and nothing is reviewed after it lands (`yes-no-review`).

#### Scenario: capability edit
- GIVEN a spec branch that edits only `CAPABILITIES.md`
- WHEN `clm gate` judges it
- THEN its first line is `spec review`, and `clm land` refuses it until a human has approved it

#### Scenario: node rule edit
- GIVEN a spec branch that only rewords a component's requirement and declares no `human_burdens`
- WHEN `clm gate` judges it
- THEN its first line is `spec`, and it lands on checks

#### Scenario: rename of an architecture entry
- GIVEN a branch holding only what `clm rename` produced from a change renaming an `ARCHITECTURE.md` entry's slug
- WHEN `clm gate` judges it
- THEN its first line is `spec`, and it lands on checks

#### Carries
- method

#### Enforced by
- gate/classify
- gate/needs-review
- rename/replay
- gate/approval
- gate/approval-setting
- gate/approval-changed
- gate/undeclared-burden
- land/land
- land.approval/approve
- plugin.hooks.guard/approve-guard
- ledger/verify-recomputes-review

### Constraint: deterministic-enforcement
Every rule that must hold SHALL be checked by a program (the CLI, a hook or the land checks), never by prose or model judgment alone. Model judgment may advise but never gates.

#### Scenario: advisory review
- GIVEN a branch whose checks pass and whose AI review found something
- WHEN `clm land` runs
- THEN the review's findings do not decide whether it lands

#### Carries
- method

#### Untested
A principle for how every rule is built, applied in design and review; this file's `#### Enforced by` blocks are its record.

### Constraint: proven-by-tests
A requirement SHALL count as implemented only when a passing test cites its ID at its current version.

#### Scenario: no citing test
- GIVEN a requirement no test cites
- WHEN `clm pending` runs
- THEN it lists the requirement as pending

#### Carries
- method

#### Enforced by
- check.coverage/coverage-report
- check.coverage/pending-report
- check.coverage/markers-from-files
- check/skipped-tests
- land/land

### Constraint: no-silent-drift
A divergence between spec and code that CLM can detect SHALL block: `clm check` fails a marker that names no requirement and a delta whose base has moved, and `clm land` fails a branch whose applied `specs/` don't match their sources. A requirement not yet built is pending, not drift (`node-reconciliation`).

#### Scenario: cited ID gone
- GIVEN a test citing a requirement that no spec holds
- WHEN `clm check` runs
- THEN it fails, naming the marker

#### Carries
- method

#### Enforced by
- check/report-and-exit
- check/dangling-markers
- check/stale-deltas
- land/derived-specs
- sync/judged-by-check

### Constraint: git-is-truth
Git SHALL be the source of truth and the only channel. All shared state (specs, changes, claims, approvals, checks and lands) is committed files or branches, and no server or database owns any. A service may keep a cache derived only from git (and from a host's own data, such as a pull request's URL) that can be deleted and rebuilt with nothing lost; no check, gate or approval reads anything but git (`per-writer-ledgers`).

#### Scenario: cache emptied
- GIVEN the web app's cache is deleted
- WHEN a person opens a change's page
- THEN it is rebuilt from git and shows the same state

#### Carries
- method

#### Enforced by
- web.worker/cache-is-disposable
- sync/judged-by-check
- orchestrator/queue-from-refs
- check.coverage/markers-from-files
- ledger/fold
- clm/judges-read-only-git

#### Untested
That no server or database owns shared state is a judgement on the design; that the judges read only git is enforced.

### Constraint: hash-based-staleness
Anything derived from the spec SHALL record the content hashes of the elements it was based on, and on a mismatch it is stale: a delta must be rebased before use, and a requirement cited at an old version is behind and is built again. The one re-cite that isn't a build is a rename's, which the gate replays: an ID rename changes no text a test proves, and a term swap is reviewed (`rename-by-replay`).

#### Scenario: reworded requirement
- GIVEN a test citing a requirement at its version, and the requirement is reworded
- WHEN `clm pending` runs
- THEN it lists the requirement as `behind`

#### Carries
- method

#### Enforced by
- model/content-hash
- apply/stale-base
- check/stale-deltas
- check.coverage/pending-report
- context/bundle-hash
- rename/marker-rewrite

### Constraint: regenerate-dont-merge
Derived files SHALL be regenerated, never merged. Indexes, registries and the applied `specs/` are rebuilt from their sources (`derived-specs-fingerprint`).

#### Scenario: conflicting derived file
- GIVEN two branches that each change a derived file
- WHEN the second is brought up to `main`
- THEN the file is regenerated from the merged sources, not merged by hand

#### Carries
- method

#### Enforced by
- sync/reapply
- land/derived-specs
- gate/up-to-date
- sync/regenerate-renames

### Constraint: ancestor-scoped-context
An agent working on a component SHALL receive `ARCHITECTURE.md`, CLM's carried constraints in force in the project (`carried-constraints`), the ancestors (root to component), the one-hop contracts of what it needs and the capabilities its rules serve, and nothing else.

#### Scenario: unrelated node
- GIVEN components `a` and `b`, neither needing the other
- WHEN `clm context a` runs
- THEN its bundle holds nothing of `b`'s spec

#### Carries
- method

#### Enforced by
- context/bundle-contents
- context/bundle-hash

### Constraint: reuse-first
The project SHALL reuse existing tools (standard libraries, the platform's primitives and conventions; for CLM, git, Claude Code primitives and superpowers, its own code kept to glue and checks) over reinventing them. Writing our own needs a reason; adding a library or tool is the preferred way to get something done.

#### Scenario: a standard tool exists
- GIVEN a proposal that needs something a widely used tool already does
- WHEN it is designed
- THEN it uses the tool, or its design says why not

#### Carries
- default

#### Untested
A judgement made in design and review.

### Constraint: spec-is-current
`specs/`, `ARCHITECTURE.md` and `CAPABILITIES.md` SHALL describe only the system as it is now. Removed requirements are deleted, not marked deprecated. History lives in `changes/archive/` and git, and future intent in `backlog/`.

#### Scenario: removed requirement
- GIVEN a change that removes a requirement
- WHEN it is applied
- THEN the requirement's text is gone from `specs/`, and only the archived change records it

#### Carries
- method

#### Untested
A writing discipline: `clm apply` deletes removed requirements, and `clm health`'s `history-words` flags prose that narrates history, advising without gating (`health-advises`).

### Constraint: fold-small-defer-large
Small work discovered during a change SHALL be folded into it: anything that needs no spec change and stays within the change's components. Significant out-of-scope work SHALL be recorded as a `backlog/` item instead: anything that needs a new or modified requirement, touches other components, or would noticeably grow the change.

#### Scenario: bug in another node
- GIVEN a build of component `a` that finds a bug in component `b`'s code
- WHEN it finishes
- THEN the bug is a backlog item on `b`, not a fix in `a`'s branch

#### Carries
- replaceable

#### Guarantee
Work found during a change is either done in it or recorded as a `backlog/` item, never lost, and a change never grows beyond its components unseen.

#### Untested
Small and significant are judgements made during a change.

### Constraint: web-optional
CLM SHALL work without the web app: everything a person or session must do can be done with git, the `clm` CLI, the plugin and the git host. No required component depends on an optional one, which the model checks (`optional-nodes`).

#### Scenario: no web app
- GIVEN a repository with no web app deployed
- WHEN a person proposes, approves, builds and lands a change
- THEN every step succeeds with the CLI and the plugin alone

#### Enforced by
- model/optional-nodes
- model/graph-check
- check/undeclared-imports

#### Untested
That everything can be done without the web app holds as far as the CLI and plugin scenarios go; no program can list everything.

### Constraint: no-dead-code
Code SHALL exist only to serve a current requirement. A change that removes or replaces behaviour removes the code, tests and dependencies that served it, in the same change or a later one it names with a `clm:remove-by <change-id>` marker. Each project names its dead-code detectors in `clm.toml`, and `clm check` runs them (`dead-code-detectors`).

#### Scenario: unused dependency
- GIVEN a crate dependency nothing uses after a removal
- WHEN `clm check` runs
- THEN its detector reports the dependency and the check fails

#### Carries
- replaceable

#### Guarantee
All code serves a current requirement, and a program the project names in `clm.toml` finds code that doesn't: code no requirement asks for is never kept by accident.

#### Enforced by
- check/detectors
- check/remove-by-markers
- config/unreferenced-detector
- check.unreferenced/report
- ci/detector-tools

### Constraint: slim-components
A component SHALL stay small enough for one build session to hold its spec and its code, measured against its size budget (`node-size-budgets`). Components split along architectural seams, never to satisfy a number, and the part carved out becomes a child (`nested-splits`). Splitting is the AI's business and needs no human; an oversized component blocks nothing but stays visible (`oversized-is-a-state`).

#### Scenario: node over budget
- GIVEN a component over its size budget
- WHEN `clm status` runs
- THEN it shows the component as oversized, and its builds still run

#### Carries
- replaceable

#### Guarantee
One build session can hold a component's spec and its code: a build sees only its component's bundle and the code it owns.

#### Enforced by
- check/node-size
- model/size-budget
- view/status-report

#### Untested
Splitting along seams rather than to satisfy a number is a judgement.

### Constraint: method-in-the-plugin
Whatever a person or session must know to run a project with CLM SHALL be said by the plugin's skills or by a page of the guide, which the `clm` binary carries and prints with `clm guide <page>`: the pages of `docs/guide/`, and `constraints`, generated from the carried entries of this file (`carried-constraints`). This repository's decisions and specs may say why, but are never the only place a working rule is written (`guide-in-the-binary`).

#### Scenario: offline adopter
- GIVEN an adopter with the plugin installed and no network
- WHEN they run `clm guide spec-format`
- THEN the page prints, matching the installed version

#### Enforced by
- init/guide-command
- init/guide-table-from-files
- init/guide-links-inside
- init/claude-md-points-at-guide
- init/guide-constraints-page

#### Untested
That no working rule is written only in this repository is a judgement about prose.

### Constraint: carried-constraints
CLM's constraints that govern how any project is built with it SHALL reach every project from the `clm` it runs, never as a copy that can fall behind. Each such entry of CLM's `ARCHITECTURE.md` holds a `#### Carries` block naming its tier: `method`, which a project can't replace; `replaceable`, whose `#### Guarantee` block states what CLM relies on, which a project may replace with its own constraint keeping that guarantee; or `default`, an opinion a project may drop. A project replaces or drops one with a `#### Replaces` block in an entry of its own `ARCHITECTURE.md`, an edit a human reviews; the `architecture-auditor` advises whether a replacement keeps the guarantee (`deterministic-enforcement`).

#### Scenario: replacing a method constraint
- GIVEN a project whose `ARCHITECTURE.md` holds an entry with `#### Replaces` naming `proven-by-tests`
- WHEN `clm check` runs
- THEN it fails, naming the entry and `proven-by-tests`

#### Scenario: a constraint CLM adds
- GIVEN a project set up by an older `clm`, and a newer `clm` whose `ARCHITECTURE.md` carries a new `method` entry
- WHEN `clm context <component>` runs with the newer `clm`
- THEN the bundle holds the new entry, with no edit to the project's files

#### Enforced by
- model/carried-constraints
- context/bundle-carries-constraints
- init/guide-constraints-page
- init/architecture-template-proof

#### Untested
Whether a replacement keeps the guarantee it replaces is a judgement, which the auditor advises on and the human's approval decides.

### Constraint: tests-run-concurrently
A project's tests, and the check command that runs them, SHALL pass while other runs of them overlap on the same host, each in its own checkout: no test binds a fixed port, writes a fixed path outside its checkout and the system's temporary directory, or relies on a server or database another run shares. A port or path a run must know in advance, such as the port of a server the check starts, is keyed on `CLM_SLOT`, the number CLM gives each build and its checks, distinct among the builds of one clone, and 0 when unset (`build-slots`). CLM runs builds in parallel and runs their tests outside any lock, so making the tests able to overlap is the project's part of that.

#### Scenario: a preview server
- GIVEN a project whose check starts a preview server for its browser tests, and two builds of it on one clone
- WHEN both run their tests at once
- THEN each server listens on a port derived from its own `CLM_SLOT`, or one the system chose, and both runs pass

#### Carries
- replaceable

#### Guarantee
Two runs of the project's tests on one host, from different build places of a clone, never fail each other through a resource the host holds, so CLM can build in parallel without serialising the tests its builds run.

#### Enforced by
- orchestrator/place-slots
- land/check-slot
- orchestrator.run/prompt-slot
- plugin.init/init-skill-concurrent-tests

#### Untested
Whether a project's tests take a host resource another run shares is something CLM can't see; a collision shows as a test that fails beside another build and passes alone. Runs outside a build place (the human's own checkout, `clm verify`'s local run, a draft's code fix) all have slot 0, so two of them can still collide on a port keyed on it; the check lock keeps their `[land] check` and `[verify] check` runs apart, but not their tests run by hand.

### Constraint: tests-run-in-their-own-tree
A build's tests SHALL run against the code and the dependencies of the tree it builds in, never against another checkout's: a toolchain whose lookup reaches a parent directory, or an environment the clone's checkouts share, must not resolve the main checkout's packages or source in its place. A build's tree is a fresh worktree inside the main checkout with nothing installed, so the project declares in `[build] setup` the command that makes such a tree ready to run its tests, such as one installing its dependencies into it, and every session that tests code in such a tree (a `clm run` build, a draft's code fix, a skill's hand-run fix) runs it with `clm setup` before its first test run there; CLM names no toolchain, and the project's command holds what its toolchains need (`build-setup-step`).

#### Scenario: a package lookup that walks up
- GIVEN a project whose test runner looks for packages in the parent directories of the tree it runs in, a main checkout with its packages installed, and a build in a fresh worktree under it
- WHEN the build runs its first test
- THEN `clm setup` has run the project's `[build] setup` in the worktree, and the test runs against the worktree's own packages and code

#### Carries
- replaceable

#### Guarantee
A build's red and green test runs exercise the branch's own code with the dependencies its own tree declares, so a test-first build's result means what it says, and what a build saw pass is what its land checks.

#### Enforced by
- config/build-setup-setting
- orchestrator.run/setup-command
- orchestrator.run/prompt-setup
- orchestrator.triage/draft-prompt-setup
- init/init-build-setup
- plugin.init/init-skill-build-setup
- plugin.propose/propose-fix-runs-setup

#### Untested
That a session runs `clm setup` before its first test, and again when its dependencies change, rests on the prompts and skills that tell it to, as `path-ownership`'s rule rests on the build prompt: no program sees a session's test runs. Whether a project's toolchains reach outside the tree they run in, and whether its `[build] setup` installs everything its tests need, are things CLM can't see; a gap shows as a test that passes in a build and fails in a fresh tree, or the reverse. `[land] check` stays a command that needs no setup before it, since CI and `[verify]` run it in trees no build prepared.

### Constraint: land-checks-stay-fast
A project's `[land] check` SHALL run the tests `CLM_SELECTION` names, or every test when it is unset, empty or names `all`, and those tests SHALL be fast, deterministic and able to run beside other runs (`tests-run-concurrently`); every file a test reads SHALL be one the spec graph connects it to: a file of the component that owns it or of one it depends on (`depends_on`), or one a `req:` or `clm:reads` marker in it names. A test that is slow, fails or passes from run to run on the same tree, or needs a resource no other run may share belongs in `[verify]`, after the land (`verify-after-land`). CLM builds a land's verdict from checks of its parts: a land whose branch's head was checked whole runs only the tests the branch and what `main` gained since can both affect (`land/overlap-recheck`), so the tests' honesty about what they read is what that verdict rests on.

#### Scenario: an unrelated move of main
- GIVEN a project whose build checked its branch whole, and `main` moved since by a land on a component the branch's components neither own nor depend on
- WHEN the branch lands
- THEN its land runs no test, and a test that reads a file its markers don't name is the project's defect, which `[verify]` shows after the land, or, in a project without `[verify]`, the next build's full check

#### Carries
- replaceable

#### Guarantee
A test that the spec graph doesn't connect to a change gives the same result with and without it, so CLM can land a branch on a `main` that moved without re-running the tests the two can't both affect, and slow or exclusive tests never hold a land.

#### Enforced by
- land/overlap-recheck
- land/check-time-report
- verify/culprit-recheck
- orchestrator.run/prompt-doc-readers
- plugin.init/init-skill-marks-doc-readers

#### Untested
Whether a test is deterministic, and whether every file it reads is one the graph connects it to, are things CLM can't see before the fact: a gap shows as a failure after the land, which `[verify]` finds, bisects to a land and reports with how that land re-checked, or, in a project that sets no `[verify]`, as the next build's full `clm land --check-only` failing on a `main` it didn't change (`graph-test-selection`'s safeguard), without presuming whether the graph, the code or the test was wrong. A check that runs more than its selection is never wrong, only slower, and the check-time report says where its time went.

### Constraint: platform
CLM SHALL be built on git and the supported agent harnesses: Claude Code and Codex, and, for any other agent that reads `AGENTS.md` and runs shell commands, a generic tier that needs no adapter. A project under CLM can be worked from any of them, by contributors on different tools or one person switching tools, with the same rules and state (`agent-harness-adapters`). CLM may use a supported agent harness's own features directly where its adapter declares them. A git host (GitHub, Azure DevOps, a bare server, or none) is an optional adapter that holds the remote and, when it has one, a pull request per branch; it runs no checks and makes no decisions (`host-adapters`). Adding or dropping a supported agent harness is a change that needs review.

#### Scenario: no host
- GIVEN a repository whose remote is a bare git server
- WHEN a branch lands
- THEN `clm land` checks and pushes it, and nothing needed a host

#### Scenario: two agent harnesses, one change
- GIVEN a spec branch proposed from a Claude Code session
- WHEN a Codex session builds it
- THEN the build reads the same specs, claims and rules, and lands through the same `clm land`

#### Untested
A choice of foundation, not a behaviour; changing it is an edit to this file, which needs review.

### Constraint: agent-harness-adapters
Every rule SHALL live in `clm` and git, never in an agent harness. An agent harness adapter only recognises its agent harness from the environment of the command it runs, translates the agent harness's hook events into `clm`'s and `clm`'s decisions back into the agent harness's format, and declares what the agent harness can do: make the human confirm a command, run a hook before a shell command, run hooks at a session's start and end, launch subagents, give a subagent a worktree of its own, and choose a subagent's model. Where a recognised agent harness lacks an ability, CLM degrades to a path that keeps every rule (the human's own terminal for an approval, builds one at a time where there are no subagents, the session's own model where it can't choose one), never to one that trusts the agent. An agent of the generic tier has no adapter, so CLM can't tell it from the human's terminal: there no guard runs and `clm approve` can't refuse, and what holds is what `approval-policy` and `guard-blocks-push-to-main` say under `#### Untested`.

#### Scenario: an agent harness that can't confirm
- GIVEN a session under a recognised agent harness whose adapter declares no confirmation
- WHEN it runs `clm approve`
- THEN `clm approve` refuses and says to run it in the human's own terminal

#### Enforced by
- agent-harness/detect
- agent-harness/abilities
- agent-harness/hook-events
- plugin.hooks.guard/guard-per-agent-harness
- plugin.hooks/hooks-per-agent-harness
- land.approval/approve-refuses-unconfirming-agent-harness

#### Untested
That an adapter holds no rule of its own is a judgement in design and review; that an agent harness's declared abilities match what it really offers rests on its documentation, which CLM doesn't test. The generic tier is accepted with the two gaps above, since it needs no adapter at all; a project that needs every rule held by a program works from a recognised agent harness.

### Constraint: yagni
The project SHALL build what a capability needs now, not what it might need later: no speculative options, adapters, settings or extension points without a capability asking for them. Adding complexity needs a reason.

#### Scenario: speculative setting
- GIVEN a proposal adding a setting no capability uses
- WHEN it is drafted
- THEN the setting is left out

#### Carries
- default

#### Untested
A judgement made in design and review.

### Constraint: redesign-before-workaround
When something gets in the way, the first question SHALL be whether the design is wrong. The answers are to redesign it, to accept it in writing, or to make a small workaround for a genuine edge case, flagged against what it serves (`workaround-flags`). An AI session that hits its third workaround on the same thing MUST stop and ask instead of pushing on.

#### Scenario: third workaround
- GIVEN a build that has already made two workarounds on one requirement
- WHEN it needs a third
- THEN it stops and asks the architect

#### Carries
- method

#### Enforced by
- orchestrator.run/prompt-workaround
- check.health/workaround-command

#### Untested
Whether the design is wrong, and stopping to ask at the third workaround, are a session's judgement; the flags and the exit at the third are enforced.

### Constraint: ask-the-architect
When there is no clear standard answer, or reuse and minimalism pull different ways, the AI SHALL ask the architect instead of guessing. An unattended session files the question as a backlog item that blocks the requirements waiting on it, and builds the rest (`blocked-by-backlog`).

#### Scenario: unattended question
- GIVEN an unattended build that needs a fact only the human has
- WHEN it reaches the requirement
- THEN it files a `question` item blocking it, and builds the others

#### Carries
- default

#### Untested
Whether a standard answer is clear is a judgement; the blocking an unattended session relies on is `blocked-by-backlog`'s.

### Constraint: right-over-convenient
When an AI session, attended or not, recommends or chooses a design (an answer to a backlog item, a workaround fix, a proposal, a component split), it SHALL first read the code and specs involved, weigh the obvious fix, the item's own suggestion among them, as one option, and recommend the design that is correct and maintainable over the long term, saying why it beats the convenient one. A cheaper option is recommended only with the reason it is also the right one; being minimal or standard (`yagni`, `reuse-first`) is not that reason alone.

#### Scenario: item proposes its own fix
- GIVEN a backlog item whose text proposes a fix
- WHEN a session recommends an answer for it
- THEN it has read the code and specs the item names, and its recommendation says why the chosen design beats the item's fix, or why that fix is also the right one

#### Carries
- default

#### Untested
Which design is correct and maintainable is a judgement made in design and review. It reaches every session through the context bundle, and the skill steps that recommend a design (triage's answers and workaround fixes, propose's design step) apply it.

### Constraint: effort-accounted
CLM SHALL record the model work each land spent, in git, so that what each capability costs can be shown whatever else happens. Keeping a capability is a decision made against that cost.

#### Scenario: a land
- GIVEN a branch built by AI sessions
- WHEN it lands
- THEN the ledger records the work spent on it

#### Enforced by
- land/records-effort
- ledger/effort-records
- effort/record-command
- effort/attribution
- effort/capabilities-list
- effort/capability-page

#### Untested
Keeping a capability against its cost is a decision people make.

### Constraint: traced-rules
Every implementation rule SHALL name what it serves: a capability or an architectural constraint, through its component's `serves` or its own `#### Serves` block. A rule that traces to nothing is removed.

#### Scenario: untraced rule
- GIVEN a requirement whose effective serves is empty
- WHEN `clm check` runs
- THEN it fails, naming the requirement

#### Carries
- method

#### Enforced by
- check/untraced
- model/serves

### Constraint: path-ownership
A component's build SHALL change only its own component's files: those its `tests` match, wherever they sit, and those its `paths` match that no other component's `tests` match (`components-own-their-tests`). A change that another component's code or tests must follow is built in steps, each by the build of the component whose files it changes: the provider expands, each user moves in a requirement of its own that needs the expansion, and the provider contracts once every move is built (`coupled-changes-sequence`, `expand-contract`); a build whose requirement needs an edit outside its files leaves it unbuilt with a `question` item. One exception is permanent: a build whose requirement changes what a capability test observes at the product's boundary updates that test's expectations, and the steps that reach them, to what the requirement states, keeping its citation and every check its capability's scenarios, as `CAPABILITIES.md` now states them, state, and names each one in its pull request; a requirement under which the entry as written no longer holds is left unbuilt, with a `question` item, until a change edits the entry, since its own land runs that test and the `capability` component holds no requirement to carry the move (`capability-tests-follow-rules`): a capability test observes the whole product, so a behaviour and the test that observes it can't land apart. Two are transitional, each until a backlog item is done: a build whose requirement changes the text of a file `clm init` writes writes the `upgrade` migration that carries the change, as the `upgrade` requirement that needs it states it, brings `upgrade`'s tests that the new migration makes fail to what `upgrade`'s requirements as applied state, and names both in its pull request, since the upgrade corpus test fails a land of either half alone (`template-change-ships-its-migration`), until a template and its migration share a component (`backlog/a-template-and-its-migration-live-in-different-components`); and a build may re-cite, and bring up to the current text, a test in a file no component's `tests` match that cites one of its own requirements at another version, since that test is part of the cited component's build (`tests-count-where-cited`, `stale-markers-keep-behind`), until the project gives its tests to their components (`backlog/clm-s-cli-tests-belong-to-the-root`).

#### Scenario: test in its own directory of a shared suite
- GIVEN a suite directory `tests/` that a harness component's `paths` own, and component `a` with `tests: ["tests/a/**"]` and `test_depends_on` naming the component that holds the app's entry point
- WHEN `a`'s build adds a test
- THEN it writes it under `tests/a/`, importing the entry point, and changes no file of the harness

#### Scenario: removal another component still uses
- GIVEN a build of `a` whose requirement removes a Contract function that component `b`'s code still calls, and no requirement of `b` that moves it off
- WHEN it builds
- THEN it leaves the requirement unbuilt, keeps `b` building, and adds a `question` item whose `blocks` names the requirement

#### Scenario: split sequenced
- GIVEN a change that moved requirement `a/x` to its new child `a.b`, and gave `a` a requirement that removes its copy and reaches the code through `a.b`, needing `a.b/x`
- WHEN `a.b` builds, then `a`
- THEN `a.b`'s build writes the code and its tests in `a.b`'s files, and `a`'s build deletes its copy and rewires its imports, each in its own component's files

#### Scenario: rule changes what a capability test sees
- GIVEN a build of `web` whose requirement changes the opening screen that a capability test under `capability`'s `paths` checks
- WHEN it builds
- THEN it updates that test to expect the new screen, keeping the capability it cites, and its pull request names the fix

#### Scenario: template change carries its migration
- GIVEN a build of `init` whose requirement changes the starter check script, and an `upgrade` requirement stating the migration that carries it and needing that requirement
- WHEN it builds
- THEN it writes that migration as stated, and its pull request names the fix

#### Scenario: stale marker in another component's files
- GIVEN a build of `a` whose requirement `a/x` is cited at an older version by a test in `b`'s `paths`
- WHEN it builds
- THEN it re-cites that test at `a/x`'s current version, fixing the test first where it no longer proves the current text

#### Carries
- method

#### Enforced by
- orchestrator.run/stays-in-its-component
- orchestrator.run/pending-prompt
- orchestrator.run/capability-tests-follow-rule
- orchestrator.run/template-migration-fix
- check/paths-disjoint
- check/tests-cite-their-owner

#### Untested
Accepted: no gate or land check refuses a build's file outside its component's files. The build prompt keeps it there, and the build's review and spec audit read the diff (`path-ownership-by-prompt`). Which files a component owns is checked: `paths` and `tests` stay disjoint, and a test a component's `tests` own cites only that component.

### Constraint: approval-policy
Only a human SHALL approve a change that needs review, through the command the agent harness's guard asks them to confirm, or from the human's own terminal outside any agent; the conversation alone never counts. Under a recognised agent harness that can't make the human confirm a command, `clm approve` refuses (`agent-harness-adapters`). Who may approve is the project's `approval` setting.

#### Scenario: session tries to approve
- GIVEN a session that runs `clm approve` on a branch
- WHEN the guard sees it
- THEN it asks the human to confirm, and nothing is approved without that

#### Carries
- method

#### Enforced by
- land.approval/approval-policy
- land.approval/approval-from-base
- config/approval-setting
- gate/approval-setting
- plugin.hooks.guard/approve-guard
- land.approval/approve-refuses-unconfirming-agent-harness
- web.worker.review/approve

#### Untested
That the conversation alone never counts rests on each agent harness's confirmation prompt, which CLM doesn't test. An agent CLM doesn't recognise looks like the human's terminal to `clm approve`: there, what keeps an approval the human's is the instruction `AGENTS.md` carries, signed approvals where the repository signs (`signed-ledger-records`), and `clm ledger verify` after the fact.

### Constraint: trunk-named-main
A CLM repository's trunk SHALL be a branch named `main`: every command, skill and prompt names it so, and no setting names another branch. `clm init` SHALL say so up front in a repository without one, with the command that renames its branch, rather than leave the first `clm land` to find it (`trunk-is-main`).

#### Scenario: a repository on master
- GIVEN a repository whose only branch is `master`
- WHEN `clm init` runs
- THEN its first next step names renaming `master` to `main`

#### Carries
- method

#### Enforced by
- init/init-names-missing-main

### Constraint: land-locally
A branch SHALL land on `main` only through `clm land`, which runs the checks on the merged tree on the machine that runs it and pushes `main`. No hosted CI, host or script pushes `main` or decides a land.

#### Scenario: push to main
- GIVEN a session that tries to push a branch straight to `main`
- WHEN it runs
- THEN the push is blocked (`guard-blocks-push-to-main`)

#### Carries
- method

#### Enforced by
- land/land
- land/land-push-retry
- plugin.hooks.guard/push-guard
- plugin.hooks.guard/host-routes-guard
- plugin/hooks-wired
- ledger/verify
- ci/no-workflow-pushes-main

### Constraint: no-local-land-with-remote
In a repository with a remote, `main` SHALL have one copy, the remote's: `clm land` refuses rather than land locally when the remote can't be reached.

#### Scenario: remote unreachable
- GIVEN a repository with a remote that can't be reached
- WHEN `clm land` runs
- THEN it exits non-zero and `main` is unchanged

#### Enforced by
- land/offline-land

### Constraint: guard-blocks-push-to-main
The guard of each agent harness whose adapter declares one SHALL block every `git push` from a session whose destination is `main`, by refspec, `--all`, `--mirror` or implicitly from a checkout on `main`, unless the push runs from a git repository that isn't under CLM (one holding no `specs/SPEC.md` at its top or in any subdirectory), where no `clm land` owns `main`; a push whose repository the guard can't tell counts as under CLM. Every unattended build SHALL load the guard. A `clm` command that pushes or deletes a branch for a session, whose own `git push` the guard never sees, SHALL refuse `main` itself.

#### Scenario: implicit push
- GIVEN a session on `main` in a repository under CLM
- WHEN it runs `git push`
- THEN the guard blocks it

#### Scenario: a repository not under CLM
- GIVEN a session that runs `git push -u origin main` from a scratch repository with no `specs/SPEC.md`
- WHEN the guard judges it
- THEN the push goes through

#### Enforced by
- plugin.hooks.guard/push-guard
- plugin.hooks.guard/push-guard-scoped-to-clm
- plugin.hooks.guard/host-routes-guard
- plugin.hooks.guard.shell/guard-heredocs
- plugin.hooks.guard.shell/guard-quotes
- plugin.hooks.guard.shell/guard-command-position
- plugin.hooks.guard.shell/guard-shell-scripts
- plugin.hooks.guard.shell/guard-quoted-substitutions
- plugin.hooks.guard.shell/guard-here-strings
- plugin.hooks.guard.shell/guard-checkout
- plugin.hooks.guard/guard-per-agent-harness
- plugin/hooks-wired
- plugin/codex-hooks-wired
- orchestrator.runner/runner-needs-guard
- host/branch-push
- host/branch-delete

#### Untested
An agent of the generic tier runs no guard, so nothing stops its direct `git push` to `main` but the host's branch protection, where the host has it, and the instruction `AGENTS.md` carries; `clm ledger verify` finds a `main` commit no `landed` record names after the fact (`agent-harness-adapters`).

### Constraint: per-writer-ledgers
Approvals, checks and lands SHALL be records in per-writer ledger branches that only their writer appends to, so appends never race, rebase or force-push.

#### Scenario: two lands at once
- GIVEN two people landing at the same time
- WHEN each records its land
- THEN each appends to its own ledger branch and neither rewrites the other's

#### Enforced by
- ledger/append
- ledger/writer-identity
- ledger/fold
- web.worker.review/approve

### Constraint: signed-ledger-records
In a repository that lists its signers in `docs/allowed_signers`, an approval or land SHALL count only with a valid SSH signature by that person's listed key, and `clm ledger verify` audits every signed record.

#### Scenario: unsigned approval
- GIVEN a signed repository and an approval record without a valid signature
- WHEN `clm land` counts approvals
- THEN it doesn't count that record

#### Enforced by
- land.approval/signed-land
- land.approval/signed-approve
- ledger/signature-check
- ledger/record-signing
- ledger/verify-signed-lands
- gate/needs-review

### Constraint: land-policy-from-base
`clm land` SHALL take the `approval` setting from the `clm.toml` committed at the base it lands on, so a branch can't weaken the policy it is judged by.

#### Scenario: branch relaxes approval
- GIVEN a branch that changes `approval` to let its author approve
- WHEN `clm land` judges it
- THEN it applies the setting on `main`, not the branch's

#### Enforced by
- land.approval/approval-from-base

### Constraint: approve-pins-the-review
An approval SHALL name the branch fingerprint it was given for, and counts only while the branch still matches it; an approval of a branch that has moved on is refused and writes nothing.

#### Scenario: branch moved after review
- GIVEN a reviewer viewing a branch that gains a commit changing its spec
- WHEN they approve the page they read
- THEN the approval is refused and nothing is recorded

#### Enforced by
- gate.fingerprint/fingerprint
- gate.fingerprint/fingerprint-stable
- land.approval/approve
- land/land
- web.worker.review/approve
- web/approve-by-branch

### Constraint: web-approve-off-when-signed
In a signed repository the web app SHALL NOT approve: its approve route refuses and writes nothing, and points to `clm approve`, because the Worker holds no one's SSH key.

#### Scenario: approve in browser
- GIVEN a signed repository
- WHEN a reviewer approves from the web app
- THEN it refuses, names `clm approve`, and writes no record

#### Enforced by
- web.worker.review/signed-repository
- web.views/signed-approvals

### Constraint: component-reconciliation
The unit of building SHALL be the component: a build brings a component's code up to its whole current spec, highest priority first, and the queue is derived from committed state, never stored.

#### Scenario: two changes on one node
- GIVEN two applied changes that both edit component `a`
- WHEN `a` is built
- THEN one build brings `a` up to both

#### Enforced by
- orchestrator/queue-from-refs
- orchestrator/next
- check.coverage/pending-report
- orchestrator/priority-orders-components

### Constraint: needs-alone-order
A component SHALL be ready when it has an unblocked pending requirement, whatever the state of the components it depends on; only a requirement's `Needs`, and for a capability the rules serving it that are never built, or changed and themselves blocked (`serving-rules-order-capabilities`), order work between components.

#### Scenario: dependency pending
- GIVEN component `b` depending on `a`, both with pending requirements and no `Needs` between them
- WHEN `clm next` runs
- THEN both are ready

#### Enforced by
- orchestrator/next
- model/requirement-needs
- check.coverage/pending-report

### Constraint: lease-claims
Claims SHALL be compare-and-swap pushes with an expiry and takeover, so that of two sessions racing for the same component or backlog item exactly one wins.

#### Scenario: race
- GIVEN two sessions claiming component `a` at once
- WHEN both push their claims
- THEN exactly one claim holds

#### Enforced by
- orchestrator/claim
- orchestrator/backlog-claim
- orchestrator/claim-fix

### Constraint: expand-contract
A contract change that another component's code or tests use SHALL land in steps, so that every land leaves `main` green and each step is built in the files of the component it changes (`coupled-changes-sequence`): the provider expands, offering the new beside the old; each user moves in a requirement of its own that needs the expansion; and the provider contracts, removing the old once every move is built. The old path is marked `clm:remove-by <change-id>`. An addition a user must handle counts: a variant it matches exhaustively, which it first meets with a default arm, or a field it builds by struct literal, which it first builds through a constructor or defaults.

#### Scenario: renamed command
- GIVEN a change renaming a command other components call
- WHEN it lands
- THEN the old name still works, marked for removal by a named change

#### Scenario: added variant
- GIVEN a change adding a variant to an enum that component `b` matches exhaustively
- WHEN it is built
- THEN `b`'s build first adds a default arm, the provider's build then adds the variant, and `b`'s next build handles it, each land leaving `main` green

#### Carries
- replaceable

#### Guarantee
Every land leaves `main` green while the components that use a contract haven't moved off it yet: CLM builds one component at a time and never stacks branches, so no change lands everywhere at once.

#### Enforced by
- check/remove-by-markers

#### Untested
Whether a contract change breaks a caller, and so needs two steps, is a judgement; the markers it leaves are checked.

### Constraint: upgrades-carry-migrations
A change that alters a file, key or format that projects set up by a published release already hold SHALL ship the migration that brings those projects up to date, and a migration a release has shipped SHALL never change.

#### Scenario: template change without a migration
- GIVEN a branch that changes the `CLAUDE.md` `clm init` writes and adds no migration
- WHEN it is landed
- THEN the corpus test fails, since a project from an earlier release no longer upgrades to what `clm init` writes today

#### Enforced by
- clm/upgrade-corpus
- upgrade/shipped-migrations-frozen

#### Untested
A change in what an existing file means, with no change in what `clm init` writes or `clm check` accepts, shows in no corpus project; the builder check asks about it.

### Constraint: checks-relax-before-use
A change that relaxes a `clm check` rule SHALL land before anything relies on it, because the running `clm` judges every branch by its own rules.

#### Scenario: relaxed rule
- GIVEN a branch that both relaxes a check and relies on the relaxation
- WHEN it is landed
- THEN the running `clm` fails it, and the relaxation must land first

#### Untested
An order in which changes land; no program sees that a later change will rely on a relaxed rule.

### Constraint: settings-before-use
A new settings key or value SHALL be read by a landed requirement before this repository's `clm.toml` sets it, and only once the installed `clm` can read it.

#### Scenario: new key
- GIVEN a change adding a settings key that this repository's `clm.toml` will set
- WHEN it is built
- THEN the key's reader lands and is installed before `clm.toml` sets it, since the installed `clm` rejects a key it doesn't know

#### Enforced by
- config/invalid-settings
- sync/judged-by-check

#### Untested
The order in which a reader lands, is installed and its key is set is a sequence no program sees; a key the running `clm` doesn't know is refused.

### Constraint: host-adapters
Everything host-specific SHALL go through the `host` adapter, and a repository SHALL work with no host at all.

#### Scenario: no host configured
- GIVEN a repository with `host = "none"` and no remote
- WHEN a change is proposed, approved, built and landed
- THEN every step works, with no pull requests

#### Enforced by
- host/git-through-layer
- vcs/one-git-layer
- config/host-setting
- host/pr-open
- host/none-keeps-a-remote
- clm/none-remote-before-commands
- land/offline-land

### Constraint: incremental-adoption
Once the root spec sets `outside`, every tracked code file SHALL be owned by a component or declared `outside`, so nothing is unowned by accident; an existing codebase may be adopted a part at a time.

#### Scenario: new unowned file
- GIVEN a root spec that sets `outside`, and a new code file no component's `paths` cover and `outside` doesn't list
- WHEN `clm check` runs
- THEN it fails, naming the file

#### Carries
- method

#### Enforced by
- check/unowned-files
- model/outside-globs

### Constraint: health-advises
`clm health` SHALL be advisory: it exits 0 whatever it finds, and no check, gate or land reads it.

#### Scenario: unhealthy repository
- GIVEN a repository with oversized components and workaround flags
- WHEN `clm health` runs
- THEN it reports them and exits 0

#### Enforced by
- check.health/health-report
- clm/health-isolated

### Constraint: autosync-policy
Spec-work snapshots SHALL go only to `autosync/<branch>`, never to the spec branch itself, a pull request's head or `main`, so half-finished edits can't land.

#### Scenario: end of turn
- GIVEN a session editing specs on `spec/x`
- WHEN its turn ends
- THEN a snapshot is pushed to `autosync/spec/x`, and `spec/x` is unchanged

#### Enforced by
- sync/autosync
- plugin.hooks/autosync-hook

### Constraint: ui-quality
Every view of the web app SHALL show that it is loading while it waits, keep what it already shows while it refreshes, and lay itself out to use the space it is given, on a desktop and on a phone. The specifics are the rules below; a rule is added to them when a person finds another detail of the experience worth keeping.

#### Scenario: opening a view
- GIVEN a person on their phone opens the spec graph while the Worker is slow to answer
- WHEN the answer arrives
- THEN a loading bar showed until the graph was drawn, and the graph fills the screen down to the tab bar

#### Enforced by
- web/loading-indicator
- web/graph-fills-height
- web/live-refresh
- web/mobile-layout

#### Untested
Whether a view feels good to use is a judgement; what is enforced is each detail someone chose to keep.

### Constraint: invite-only-web
Only the logins listed in `web`'s Contract SHALL sign in to the web app under `/app/` and reach its API; nothing it shows of a repository is public. The public site (`web.site`) is open to everyone and shows none of the repository data the web app keeps behind sign-in: no repository's specs, backlog or changes. CLM's own documentation, which it builds from CLM's repository, is public by design, and it may name and link repositories, such as CLM's own in its install commands. Adding a login edits that list, and lands on checks like any change without a burden.

#### Scenario: stranger signs in
- GIVEN a GitHub login not on the list
- WHEN it signs in to the web app
- THEN it is refused and sees nothing

#### Scenario: stranger opens the site
- GIVEN someone with no session
- WHEN they open the homepage and the guide, and then ask the API for the repo list
- THEN they read the site, and the API answers 401

#### Enforced by
- web/invited-users
- web/site-served-directly
- web.worker/sign-in
- web.worker/repo-access
- web.worker/reserved-paths

### Constraint: say-guard-is-off
When the plugin can't get a `clm` binary, its hooks SHALL let calls through, but session start MUST tell the session that CLM's state and the guard are unavailable.

#### Scenario: binary missing
- GIVEN a plugin whose shim can't find or download `clm`
- WHEN a session starts
- THEN it is told the guard is off

#### Enforced by
- plugin/binary-shim

### Constraint: views-name-their-commit
Every view SHALL name the commit it describes and warn when that commit is behind `origin/main`.

#### Scenario: stale checkout
- GIVEN a checkout behind `origin/main`
- WHEN `clm status` runs
- THEN it names its commit and says it is behind

#### Enforced by
- view/status-report
- view/view-command

### Constraint: plain-terms
CLM's names SHALL read right to a seasoned software engineer and to a newcomer alike: a term with a usual meaning in software development keeps that meaning in CLM, and a term CLM coins says plainly what it names. Every term CLM's commands, guide and skills use is defined in the guide's glossary, and a term CLM has renamed doesn't come back. The agreed renames are in the decision `plain-terms-renames`. This entry governs CLM's own vocabulary, not an adopter's.

#### Scenario: a new command
- GIVEN a change that adds a `clm` command
- WHEN its build lands
- THEN the glossary defines the command, or the land's checks fail

#### Scenario: a renamed term slips back
- GIVEN a term a rename has retired
- WHEN a skill or guide page uses it again
- THEN CLM's tests fail, naming the file and line

#### Enforced by
- clm/glossary-covers-terms
- clm/retired-terms

#### Untested
Whether a name reads right is a judgement, made when the name is proposed and reviewed; what programs check is that every term is defined and that a retired one stays gone.
