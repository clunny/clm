# CLM Architecture

The architectural constraints: what must always hold of CLM, whatever is built. The architect owns this file.
An edit to it needs a human's review before it lands (`humans-review-intent`).
It is prepended to every context bundle, so each entry is one or two sentences; its mechanics are in the decision of its name under `docs/decisions/`, or the one it names.
Each entry says which kind of rule it is (`deterministic-enforcement`): **checked**, which a program decides, the rules that do so named under `#### Enforced by`, which no other kind of entry holds; **measured**, by an eval; or **principle**, design judgement, with `#### Untested` saying why no program decides it. A checked entry is current when every rule it names is current (`human-layer-proof`). The format is in `clm guide spec-format`.
An entry holding a `#### Carries` block reaches every project built with CLM, at its tier, from the `clm` that project runs (`carried-constraints`); the others are CLM's own.

## Constraints

### Constraint: ai-writes-code
**Principle.** AI SHALL write and maintain all code and tests. No human is asked to write or review them, and code reaches `main` only from an AI session's branch.

#### Scenario: code reaches main
- GIVEN a change to a component's code
- WHEN it lands on `main`
- THEN an AI session wrote it, and no human was asked to review the diff

#### Carries
- method

#### Untested
Who wrote a diff isn't visible to a program.

### Constraint: humans-review-intent
**Checked.** Humans SHALL review intent, not code: a branch that changes what the product is for, what must always hold, who may approve or what the work costs the human waits for a human's approval, and every other branch lands on checks, with nothing reviewed after it lands. The exact triggers are in the guide (`clm guide how-it-works`).

#### Scenario: capability edit
- GIVEN a spec branch that edits only `CAPABILITIES.md`
- WHEN `clm gate` judges it
- THEN its first line is `spec review`, and `clm land` refuses it until a human has approved it

#### Scenario: requirement edit
- GIVEN a spec branch that only rewords a component's requirement and declares no `human_burdens`
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

### Constraint: removal-needs-a-yes
**Principle.** Removing behaviour SHALL always need the human's yes: a scenario, a requirement, a capability or an architectural constraint that states behaviour is never dropped without it. The AI proposes cuts, and never drops or carries a burden in silence.

#### Scenario: a change that drops a scenario
- GIVEN a change that would remove a scenario stating behaviour
- WHEN a session proposes it
- THEN it lists what would be lost and asks the human, and goes on only on their yes

#### Carries
- method

#### Untested
No program yet tells a removal of behaviour from a rewording, and such a change may still land on checks, so this yes is the human's answer in the session, not a `clm approve`: the session lists what a change removes and asks before going on.

### Constraint: deterministic-enforcement
**Principle.** Every rule SHALL be one of three kinds and say which: *checked*, decided by a program (the CLI, a hook or the land checks), preferred wherever a program can decide, model behaviour included; *measured*, by an eval against a threshold the human agreed, for behaviour that comes from a model or is statistical by nature; or *principle*, design judgement, which may be audited for advice. Only a checked rule is enforced, and only a program holds a land: an eval, an AI review or an audit never does (`three-kinds-of-rule`).

#### Scenario: advisory review
- GIVEN a branch whose checks pass and whose AI review found something
- WHEN `clm land` runs
- THEN the review's findings do not decide whether it lands

#### Carries
- method

#### Untested
A principle for how every rule is built; each entry's label and its `#### Enforced by` block are its record.

### Constraint: proven-by-tests
**Checked.** A requirement SHALL count as built only when a passing test cites its ID at its current version, or, for a requirement the human agreed to measure, an eval does at its threshold.

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

#### Untested
Today the check reads a marker wherever it sits, whether or not its test ran, and CLM knows no eval proof (`backlog/a-passing-check-proves-less-than-the-spec-claims`).

### Constraint: no-silent-drift
**Checked.** A divergence between spec and code that CLM can detect SHALL fail the check or the land. A requirement not yet built is pending, not drift.

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
**Checked.** Git SHALL be the source of truth and the only channel: all shared state (specs, changes, claims, approvals, checks and lands) is committed files, branches or ledger records (`per-writer-ledgers`), and no check, gate or approval reads anything else. A service may keep only a cache derived from git, and from a host's own data such as a pull request's URL, that it can rebuild with nothing lost.

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
- ledger/append
- ledger/writer-identity
- web.worker.review/approve
- clm/judges-read-only-git

#### Untested
That no server or database owns shared state is a judgement on the design.

### Constraint: hash-based-staleness
**Checked.** Anything derived from the spec SHALL record the content hashes of what it was based on. On a mismatch it is stale, and it is rebased, rebuilt or regenerated from its sources, never merged by hand.

#### Scenario: reworded requirement
- GIVEN a test citing a requirement at its version, and the requirement is reworded
- WHEN `clm pending` runs
- THEN it lists the requirement as `behind`

#### Scenario: conflicting derived file
- GIVEN two branches that each change a derived file
- WHEN the second is brought up to `main`
- THEN the file is regenerated from the merged sources

#### Carries
- method

#### Enforced by
- model/content-hash
- apply/stale-base
- check/stale-deltas
- check.coverage/pending-report
- context/bundle-hash
- rename/marker-rewrite
- sync/reapply
- sync/regenerate-renames
- land/derived-specs
- gate/up-to-date

### Constraint: ancestor-scoped-context
**Checked.** An agent building a component SHALL receive `ARCHITECTURE.md`, the carried constraints in force, the component's ancestors, the one-hop contracts of what it needs and the capabilities its rules serve, and nothing else.

#### Scenario: unrelated component
- GIVEN components `a` and `b`, neither needing the other
- WHEN `clm context a` runs
- THEN its bundle holds nothing of `b`'s spec

#### Carries
- method

#### Enforced by
- context/bundle-contents
- context/bundle-hash

### Constraint: traced-rules
**Checked.** Every requirement SHALL name the capability or architectural constraint it serves, and one that serves nothing fails the check.

#### Scenario: untraced rule
- GIVEN a requirement whose effective serves is empty
- WHEN `clm check` runs
- THEN it fails, naming the requirement

#### Carries
- method

#### Enforced by
- check/untraced
- model/serves

### Constraint: spec-is-current
**Principle.** The specs and both human layers SHALL describe only the system as it is now: what is removed is deleted, history lives in git and the change archive, and future intent in the backlog.

#### Scenario: removed requirement
- GIVEN a change that removes a requirement
- WHEN it is applied
- THEN the requirement's text is gone from `specs/`, and only the archived change records it

#### Carries
- method

#### Untested
A writing discipline; `clm apply` deletes what a change removes.

### Constraint: component-reconciliation
**Checked.** The unit of building SHALL be the component: a build brings its code up to its whole current spec, highest priority first, and the queue is derived from committed state, never stored. A component is ready when it has an unblocked pending requirement, whatever the state of what it depends on: only a requirement's `Needs` and the backlog items that block it order work, and a capability's test waits for the rules that serve it (`needs-alone-order`, `serving-rules-order-capabilities`).

#### Scenario: two changes on one component
- GIVEN two applied changes that both edit component `a`
- WHEN `a` is built
- THEN one build brings `a` up to both

#### Scenario: dependency pending
- GIVEN component `b` depending on `a`, both with pending requirements and no `Needs` between them
- WHEN `clm next` runs
- THEN both are ready

#### Enforced by
- orchestrator/queue-from-refs
- orchestrator/next
- check.coverage/pending-report
- orchestrator/priority-orders-components
- model/requirement-needs

#### Untested
That a build works through its component highest priority first is a session's conduct.

### Constraint: path-ownership
**Principle.** Every code file SHALL have one owning component, and a build SHALL change only its own component's files; a change that another component must follow is built in steps, each by its owner's build (`expand-contract`). The exceptions, for shared wiring and for a capability test that follows its rule, are in the guide (`clm guide how-it-works`).

#### Scenario: removal another component still uses
- GIVEN a build of `a` whose requirement removes a Contract function that component `b`'s code still calls, and no requirement of `b` that moves it off
- WHEN it builds
- THEN it leaves the requirement unbuilt, keeps `b` building, and adds a `question` item that blocks the requirement

#### Carries
- method

#### Untested
Which files a component owns is checked (`check/paths-disjoint`, `check/tests-cite-their-owner`). That a build stays in them is not yet: no gate or land check refuses a file outside them, so the build's instructions keep it there and its review reads the diff.

### Constraint: incremental-adoption
**Checked.** Once the root spec sets `outside`, every tracked code file SHALL be owned by a component or declared `outside`, so an existing codebase is adopted a part at a time and nothing is unowned by accident.

#### Scenario: new unowned file
- GIVEN a root spec that sets `outside`, and a new code file no component's `paths` cover and `outside` doesn't list
- WHEN `clm check` runs
- THEN it fails, naming the file

#### Carries
- method

#### Enforced by
- check/unowned-files
- model/outside-globs

### Constraint: exemptions-end
**Checked.** A code rule of `clm check` SHALL hold for every file a component owns except those an exemption names. Each exemption names its rule, its files and the backlog item whose work ends it, and only a change a human reviews adds one (`temporary-exemptions`).

#### Scenario: exemption added
- GIVEN a spec branch that only adds an exemption to a component's frontmatter, with the backlog item that ends it
- WHEN `clm gate` judges it
- THEN its first line is `spec review`

#### Carries
- method

#### Enforced by
- model/exemptions
- check.affected/affected-files
- gate/needs-review
- gate/exemption-names-its-item
- check.health/exemptions-signal

### Constraint: approval-policy
**Checked.** Only a human SHALL approve a change that needs review, through the command the agent harness's guard makes them confirm, or from their own terminal; the conversation alone never counts. An approval counts only for the diff it was given for (`approve-pins-the-review`), under the `approval` setting committed on `main` or set in the clone's own `clm.local.toml`, never the branch's (`land-policy-from-base`).

#### Scenario: session tries to approve
- GIVEN a session that runs `clm approve` on a branch
- WHEN the guard sees it
- THEN it asks the human to confirm, and nothing is approved without that

#### Scenario: branch moved after review
- GIVEN a reviewer viewing a branch that gains a commit changing its spec
- WHEN they approve the page they read
- THEN the approval is refused and nothing is recorded

#### Scenario: branch relaxes approval
- GIVEN a branch that changes `approval` to let its author approve
- WHEN `clm land` judges it
- THEN it applies the setting on `main`, not the branch's

#### Carries
- method

#### Enforced by
- land.approval/approval-policy
- land.approval/approval-from-base
- land.approval/approve
- land.approval/approve-refuses-unconfirming-agent-harness
- config/approval-setting
- gate/approval-setting
- gate.fingerprint/fingerprint
- gate.fingerprint/fingerprint-stable
- plugin.hooks.guard/approve-guard
- land/land
- web.worker.review/approve
- web/approve-by-branch

#### Untested
The guard prevents accidents and is not a hard boundary: it rests on each harness's confirmation prompt, an agent CLM doesn't recognise looks like the human's terminal, and a key on disk is as reachable to a session as to the human. The gap is accepted, and `clm ledger verify` audits after the fact (`approval-gap-accepted`).

### Constraint: signed-ledger-records
**Checked.** In a repository that lists its signers in `docs/allowed_signers`, an approval or land SHALL count only with a valid SSH signature by that person's listed key. The web app, which holds no one's key, refuses to approve there and points to `clm approve`.

#### Scenario: unsigned approval
- GIVEN a signed repository and an approval record without a valid signature
- WHEN `clm land` counts approvals
- THEN it doesn't count that record

#### Scenario: approve in browser
- GIVEN a signed repository
- WHEN a reviewer approves from the web app
- THEN it refuses, names `clm approve`, and writes no record

#### Enforced by
- land.approval/signed-land
- land.approval/signed-approve
- ledger/signature-check
- ledger/record-signing
- ledger/verify-signed-lands
- gate/needs-review
- web.worker.review/signed-repository
- web.views/signed-approvals

### Constraint: land-locally
**Checked.** A branch SHALL land only through `clm land`, on the trunk, which is always named `main`: it runs the checks on the merged tree on the machine that runs it and pushes `main`, and refuses when the remote can't be reached. Lands waiting on one clone may share one run of the checks, on the tree that holds them all, so every commit a push leaves `main` at is a checked tree, though not every commit behind it (`batched-lands`). Nothing else pushes `main` or decides a land, but the release workflow's merge of its version bump; each agent harness's guard blocks a session's push to `main` (`guard-blocks-push-to-main`).

#### Scenario: push to main
- GIVEN a session on `main` in a repository under CLM
- WHEN it runs `git push`
- THEN the guard blocks it

#### Scenario: remote unreachable
- GIVEN a repository with a remote that can't be reached
- WHEN `clm land` runs
- THEN it exits non-zero and `main` is unchanged

#### Scenario: a repository on master
- GIVEN a repository whose only branch is `master`
- WHEN `clm init` runs
- THEN its first next step names renaming `master` to `main`

#### Carries
- method

#### Enforced by
- land/land
- land/land-push-retry
- land/offline-land
- init/init-names-missing-main
- plugin.hooks.guard/push-guard
- plugin.hooks.guard/push-guard-scoped-to-clm
- plugin.hooks.guard/host-routes-guard
- plugin.hooks.guard/guard-per-agent-harness
- plugin.hooks.guard.shell/guard-heredocs
- plugin.hooks.guard.shell/guard-quotes
- plugin.hooks.guard.shell/guard-command-position
- plugin.hooks.guard.shell/guard-shell-scripts
- plugin.hooks.guard.shell/guard-quoted-substitutions
- plugin.hooks.guard.shell/guard-here-strings
- plugin.hooks.guard.shell/guard-checkout
- plugin/hooks-wired
- plugin/codex-hooks-wired
- orchestrator.runner/runner-needs-guard
- host/branch-push
- host/branch-delete
- ledger/verify
- ci/no-workflow-pushes-main

#### Untested
An agent with no adapter runs no guard: nothing stops its push to `main` but the host's branch protection and its instructions, and `clm ledger verify` finds afterwards a `main` commit no land recorded.

### Constraint: expand-contract
**Principle.** A contract change that another component's code or tests use SHALL land in steps that each leave `main` green: the provider offers the new beside the old, marked `clm:remove-by <change-id>`, each user moves in a requirement of its own, and the provider then removes the old (`coupled-changes-sequence`).

#### Scenario: renamed command
- GIVEN a change renaming a command other components call
- WHEN it lands
- THEN the old name still works, marked for removal by a named change

#### Carries
- replaceable

#### Guarantee
Every land leaves `main` green while the components that use a contract haven't moved off it yet: CLM builds one component at a time and never stacks branches, so no change lands everywhere at once.

#### Untested
Whether a contract change breaks a user, and so needs steps, is a judgement; only the markers it leaves are checked (`check/remove-by-markers`).

### Constraint: no-dead-code
**Checked.** Code SHALL exist only to serve a current requirement: a change that removes behaviour removes its code, tests and dependencies, at once or in a later change a `clm:remove-by <change-id>` marker names, and the detectors the project names in `clm.toml` find what is left.

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
**Principle.** A component SHALL stay small enough for one build session to hold its spec and its code, measured against its size budget (`node-size-budgets`). Splitting is the AI's business and needs no human, along architectural seams and never to satisfy a number.

#### Scenario: component over budget
- GIVEN a component over its size budget
- WHEN `clm status` runs
- THEN it shows the component as oversized, and its builds still run

#### Carries
- replaceable

#### Guarantee
One build session can hold a component's spec and its code: a build sees only its component's bundle and the code it owns.

#### Untested
Each component's size is measured against its budget and one over it is shown as oversized (`check/node-size`), but nothing refuses it; where its seams lie is a judgement.

### Constraint: tests-run-concurrently
**Principle.** A project's tests, and the check command that runs them, SHALL pass while other runs of them overlap on the same host: none binds a fixed port, writes a fixed path outside its checkout and the system's temporary directory, or relies on a server or database another run shares. What a run must know in advance is keyed on `CLM_SLOT`, the number CLM gives each build (`build-slots`).

#### Scenario: a preview server
- GIVEN a project whose check starts a preview server for its browser tests, and two builds of it on one clone
- WHEN both run their tests at once
- THEN each server listens on a port derived from its own `CLM_SLOT`, or one the system chose, and both runs pass

#### Carries
- replaceable

#### Guarantee
Two runs of the project's tests on one host, from different build places of a clone, never fail each other through a resource the host holds, so CLM can build in parallel without serialising the tests its builds run.

#### Untested
CLM gives each build its slot (`orchestrator/place-slots`, `land/check-slot`), but can't see whether a project's tests share a host resource: a collision shows as a test that fails beside another build and passes alone.

### Constraint: tests-run-in-their-own-tree
**Principle.** A build's tests SHALL run against the code and the dependencies of the tree it builds in, never another checkout's. The project declares in `[build] setup` the command that makes a fresh worktree ready, and every session runs it with `clm setup` before its first test there (`build-setup-step`).

#### Scenario: a package lookup that walks up
- GIVEN a project whose test runner looks for packages in parent directories, and a build in a fresh worktree under the main checkout
- WHEN the build runs its first test
- THEN `clm setup` has run the project's `[build] setup` there, and the test runs against the worktree's own packages and code

#### Carries
- replaceable

#### Guarantee
A build's red and green test runs exercise the branch's own code with the dependencies its own tree declares, so a test-first build's result means what it says, and what a build saw pass is what its land checks.

#### Untested
`clm setup` runs the project's command (`orchestrator.run/setup-command`), but no program sees a session's test runs, so that it runs `clm setup` first rests on its instructions; whether the project's command installs everything its tests need is the project's to get right.

### Constraint: land-checks-stay-fast
**Principle.** A project's `[land] check` SHALL run the tests `CLM_SELECTION` names, and those tests SHALL be fast, deterministic and able to overlap; every file a test reads SHALL be one the spec graph connects it to, or one a `clm:reads` marker in it names (`graph-test-selection`). A test that is slow, flaky or exclusive belongs in `[verify]`, after the land (`verify-after-land`).

#### Scenario: an unrelated move of main
- GIVEN a build that checked its branch whole, and a `main` that moved since by a land on a component the branch's components neither own nor depend on
- WHEN the branch lands
- THEN its land runs no test

#### Carries
- replaceable

#### Guarantee
A test that the spec graph doesn't connect to a change gives the same result with and without it, so CLM can land a branch on a `main` that moved without re-running the tests the two can't both affect, and slow or exclusive tests never hold a land.

#### Untested
CLM can't see before the fact whether a test is deterministic or reads only what the graph connects it to; a breach shows after the land, where `[verify]` finds it and names the land (`verify/culprit-recheck`).

### Constraint: upgrades-carry-migrations
**Checked.** A change that alters a file, key or format that projects set up by a published release already hold SHALL ship the migration that brings those projects up to date, and a migration a release has shipped SHALL never change.

#### Scenario: template change without a migration
- GIVEN a branch that changes the `AGENTS.md` `clm init` writes and adds no migration
- WHEN it is landed
- THEN the corpus test fails, since a project from an earlier release no longer upgrades to what `clm init` writes today

#### Enforced by
- clm/upgrade-corpus
- upgrade/shipped-migrations-frozen

#### Untested
A change in what an existing file means, with no change in what `clm init` writes or `clm check` accepts, shows in no corpus project; the builder check asks about it.

### Constraint: checks-relax-before-use
**Principle.** The running `clm` judges every branch by its own rules, so a change that relaxes a check or adds a settings key SHALL land, and be installed, before anything relies on it (`settings-before-use`).

#### Scenario: relaxed rule
- GIVEN a branch that both relaxes a check and relies on the relaxation
- WHEN it is landed
- THEN the running `clm` fails it, and the relaxation must land first

#### Untested
An order in which changes land, which no program sees; a key the running `clm` doesn't know is refused (`config/invalid-settings`).

### Constraint: pin-working-method
**Checked.** CLM SHALL give every build the same pinned version of the AI's working method, superpowers, whatever the session has installed.

#### Scenario: pinned method
- GIVEN the pinned superpowers version
- WHEN any build starts
- THEN it reads that version's skills, whatever the session has installed

#### Enforced by
- orchestrator.run/pin-file
- orchestrator.run/superpowers-path

#### Untested
That a build follows the text it reads is a model's behaviour, which no program sees.

### Constraint: redesign-before-workaround
**Checked.** When something gets in the way, the first question SHALL be whether the design is wrong: redesign it, accept it in writing, or make a small workaround, flagged against the requirement it is for (`workaround-flags`). When the unsettled flags on one capability reach the project's limit, three by default, the session MUST stop work on that requirement and ask.

#### Scenario: third workaround
- GIVEN a capability with two unsettled workaround flags
- WHEN a build flags a third on a requirement that serves it
- THEN `clm workaround` exits 3, and the build stops work on that requirement and asks the architect

#### Carries
- method

#### Enforced by
- check.health/workaround-command

#### Untested
Whether the design is wrong is a session's judgement, and that it stops when the command says so rests on its instructions.

### Constraint: fold-small-defer-large
**Principle.** Work found during a change SHALL be folded into it when it needs no spec change and stays within the change's components, and recorded as a backlog item otherwise.

#### Scenario: bug in another component
- GIVEN a build of component `a` that finds a bug in component `b`'s code
- WHEN it finishes
- THEN the bug is a backlog item on `b`, not a fix in `a`'s branch

#### Carries
- replaceable

#### Guarantee
Work found during a change is either done in it or recorded as a `backlog/` item, never lost, and a change never grows beyond its components unseen.

#### Untested
Small and significant are judgements made during a change.

### Constraint: reuse-first
**Principle.** The project SHALL reuse existing tools, libraries and conventions before writing its own. Writing its own needs a reason; adding a library or a tool is the preferred way to get something done.

#### Scenario: a standard tool exists
- GIVEN a proposal that needs something a widely used tool already does
- WHEN it is designed
- THEN it uses the tool, or its design says why not

#### Carries
- default

#### Untested
A judgement made in design and review.

### Constraint: yagni
**Principle.** The project SHALL build what a capability needs now, not what it might need later: no option, adapter, setting or extension point without a capability asking for it. Adding complexity needs a reason.

#### Scenario: speculative setting
- GIVEN a proposal adding a setting no capability uses
- WHEN it is drafted
- THEN the setting is left out

#### Carries
- default

#### Untested
A judgement made in design and review.

### Constraint: ask-the-architect
**Principle.** When there is no clear standard answer, or reuse and minimalism pull different ways, the AI SHALL ask the architect instead of guessing. An unattended session files the question as a backlog item that blocks only the requirements waiting on it (`blocked-by-backlog`).

#### Scenario: unattended question
- GIVEN an unattended build that needs a fact only the human has
- WHEN it reaches the requirement
- THEN it files a `question` item blocking it, and builds the others

#### Carries
- default

#### Untested
Whether a standard answer is clear is a judgement.

### Constraint: right-over-convenient
**Principle.** A session that recommends or chooses a design SHALL first read the code and specs involved, and recommend the design that is correct and maintainable over the long term, weighing the obvious fix as one option and saying why its choice beats the convenient one. A cheaper option is recommended only with the reason it is also the right one, which being minimal or standard is not alone.

#### Scenario: item proposes its own fix
- GIVEN a backlog item whose text proposes a fix
- WHEN a session recommends an answer for it
- THEN it has read the code and specs the item names, and says why its design beats the item's fix, or why that fix is also the right one

#### Carries
- default

#### Untested
Which design is correct and maintainable is a judgement made in design and review.

### Constraint: effort-accounted
**Checked.** CLM SHALL record in git the model work each land spent, so that what each capability costs can be shown. Keeping a capability is a decision made against that cost.

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

### Constraint: carried-constraints
**Checked.** CLM's constraints that govern how any project is built with it SHALL reach every project from the `clm` it runs, never as a copy that can fall behind. Each holds a `#### Carries` block naming its tier: `method`, which a project can't replace; `replaceable`, which it may replace with a constraint of its own that keeps the entry's `#### Guarantee`; or `default`, which it may drop, in each case by a `#### Replaces` block a human reviews.

#### Scenario: replacing a method constraint
- GIVEN a project whose `ARCHITECTURE.md` holds an entry with `#### Replaces` naming `proven-by-tests`
- WHEN `clm check` runs
- THEN it fails, naming the entry and `proven-by-tests`

#### Enforced by
- model/carried-constraints
- context/bundle-carries-constraints
- init/guide-constraints-page

#### Untested
Whether a replacement keeps the guarantee it replaces is a judgement, which the auditor advises on and the human's approval decides.

### Constraint: method-in-the-plugin
**Principle.** Whatever a person or session must know to run a project with CLM SHALL be said by the plugin's skills or by the guide the `clm` binary carries and prints with `clm guide`, never only in this repository's decisions and specs (`guide-in-the-binary`).

#### Scenario: offline adopter
- GIVEN an adopter with the plugin installed and no network
- WHEN they run `clm guide spec-format`
- THEN the page prints, matching the installed version

#### Untested
That no working rule is written only in this repository is a judgement about prose; that the guide ships in the binary is checked (`init/guide-command`).

### Constraint: plain-terms
**Checked.** CLM's names SHALL read right to a seasoned software engineer and to a newcomer alike: a term keeps its usual meaning, a term CLM coins says plainly what it names, every term is defined in the guide's glossary, and a term CLM has renamed doesn't come back (`plain-terms-renames`).

#### Scenario: a new command
- GIVEN a change that adds a `clm` command
- WHEN its build lands
- THEN the glossary defines the command, or the land's checks fail

#### Enforced by
- clm/glossary-covers-terms
- clm/retired-terms

#### Untested
Whether a name reads right is a judgement, made when the name is proposed and reviewed.

### Constraint: platform
**Principle.** CLM SHALL be built on git and the supported agent harnesses, which only a reviewed change adds or drops: Claude Code and Codex through an adapter each, and any other agent that reads `AGENTS.md` and runs shell commands with none. A git host is optional: it holds the remote and a pull request per branch, and may run a project's tests after a land (`verify-after-land`), but never decides one.

#### Scenario: two agent harnesses, one change
- GIVEN a spec branch proposed from a Claude Code session
- WHEN a Codex session builds it
- THEN the build reads the same specs, claims and rules, and lands through the same `clm land`

#### Untested
A choice of foundation, not a behaviour; changing it is an edit to this file, which needs review.

### Constraint: agent-harness-adapters
**Checked.** Every rule SHALL live in `clm` and git, never in an agent harness: an adapter only recognises its harness, translates its hook events and declares what it can do. Where a harness lacks an ability, CLM degrades to a path that keeps every rule, never to one that trusts the agent.

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
That an adapter holds no rule of its own is a judgement, and a harness's declared abilities rest on its documentation. An agent with no adapter runs no guard, a gap `approval-policy` and `land-locally` accept.

### Constraint: host-adapters
**Checked.** Everything host-specific SHALL go through the `host` adapter, and a repository SHALL work with no host at all.

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

### Constraint: web-optional
**Checked.** CLM SHALL work without the web app: everything a person or session must do can be done with git, the `clm` CLI, the plugin and the git host, and no required component depends on an optional one.

#### Scenario: no web app
- GIVEN a repository with no web app deployed
- WHEN a person proposes, approves, builds and lands a change
- THEN every step succeeds with the CLI and the plugin alone

#### Enforced by
- model/optional-nodes
- model/graph-check
- check/undeclared-imports

#### Untested
No program can list everything a person must be able to do.

### Constraint: invite-only-web
**Checked.** Only the logins listed in `web`'s Contract SHALL sign in to the web app and reach its API, and nothing it shows of a repository is public. The public site is open to everyone and shows no repository's specs, backlog or changes.

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

### Constraint: ui-quality
**Checked.** Every view of the web app SHALL show that it is loading while it waits, keep what it already shows while it refreshes, and use the space it is given, on a desktop and on a phone. A further detail a person finds worth keeping becomes a rule of its own.

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
Whether a view feels good to use is a judgement; what is checked is each detail someone chose to keep.
