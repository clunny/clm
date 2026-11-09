---
title: Propose skill
status: unbuilt
owners: ["@clunny"]
depends_on: []
paths: ["plugin/skills/propose/**"]
successor: ""
serves: [architecture/method-in-the-plugin]
---

## Contract
The skill `propose`, `plugin/skills/propose/SKILL.md`: an intent to one spec branch with its pull request, landed with `clm land` after the human's approval in the session, or, for a bug whose fix changes no spec, one code branch that deletes the bug's item; it ends with `clm wrap-up` and a summary that says whether the session can be ended. The rules it shares with other skills are `plugin`'s; `plugin`'s Contract says what each asks of this file.

## Design
**Claims before design** (`propose-claims`). `propose` cuts `spec/<id>` and pushes `change.md` right after claiming, so the spec work holds the items before the claim's expiry runs out, with or without autosync. It never renames the change ID after that; if it must, it deletes the old `spec/<id>` and `autosync/spec/<id>` on `origin`, which would otherwise hold the items. A claimed item that design finds already fixed, by an earlier change or by code that has landed, stays in `resolves`, so the change retires the item without claiming a fix it doesn't make. `clm proposals` reads only `from` and `resolves`, so an item the branch deleted without naming it would show as `backlog/<slug> claimed`, as if another session held it, while the pull request is open; named in `resolves`, it is the session's own work and left out. A change declares no order against other changes: the queue builds component by component, and the only ordering a change carries is its `priority` (`node-reconciliation`, `priority-orders-nodes`).

`propose` starts by brainstorming (`design-brainstorming`). It runs `clm superpowers brainstorming` and follows the skill at the printed path. The skill's own first step decides whether the intent needs the conversation: a request that already fixes what to build goes straight on. Its outputs map onto CLM's:
- the design doc becomes the `change.md` body;
- significant choices become `docs/decisions/` files;
- the skill's approval of the doc becomes the human's agreement in the conversation, before any delta is written;
- the skill's builder check is `propose`'s builder check (below), run once, before the pull request is opened.

It never hands over to `writing-plans`. Only the skill's text is read; the superpowers plugin stays off, so its hooks never run. When `clm superpowers` fails, for example with a `clm` released before the command existed, `propose` says so and carries on without brainstorming.

`propose` then follows the repo's change flow: write `changes/<id>/` with deltas based on `clm hashes`, record decisions and backlog edits, `clm check`, `clm apply`, commit, run `clm rebase` until its last line is `rebase ok`, reading its outcome from that line and never from its exit code (`rebase incomplete` leaves something to resolve, `rebase failed` means git itself failed), then run `clm gate --base origin/main`, whose first line says whether the branch needs review (`spec review`) or lands on checks (`spec`); a burden is never removed to make it land on checks. A change whose deltas add, modify and remove no requirement has nothing to build (`propose-archives-design-only`, `archive-in-spec-pr`): after the builder check, which may add a requirement, and the last `clm rebase` and `clm gate`, `propose` archives it as the last commit before opening the pull request, so it reaches `main` already archived instead of waiting as `applied` for an archive branch. The gate reads its `human_burdens`, since a new archived change is the change the branch adds, and `clm rebase` counts it as that change too (`sync/reapply`). Before opening the pull request it runs a **builder check** (`blocked-by-backlog`): a fresh subagent reads the change as the person who will build it, without the conversation, and reports the questions it would need answered. The check also looks for work a requirement relies on from another component that no requirement of that component states: a dependency's Contract or Design edit builds nothing (`node-reconciliation`), so its build leaves the work undone and settles, and the dependant's build, which can't edit the dependency's paths, dead-ends (`dependency-work-is-a-requirement`). For each, `propose` adds or modifies a requirement of the providing component in the change, which makes that component pending, and gives the relying requirement a `#### Needs` line naming it (`requirement-needs`). `propose` settles what the repo or the conversation already answers, writes those answers into the change, and lists the rest under `## Open questions` in the pull request body. Facts only a human knows, and human steps such as creating accounts, DNS records or secrets, go into the change as `question` or `human-task` backlog items, with `blocks` naming the requirements that can't be built before they are settled (`requirement-blocks`). Each `change.md` it writes starts its body with `## Intent`, a few sentences on what the change does and why, and lists under `human_burdens` in its frontmatter what the change spends of the human's money or time, or can't undo, one short line each; a `human-task` item it adds is one, since a spec branch adding a `human-task` item with no burden is blocked (`gate/undeclared-burden`). It pushes the branch and opens its pull request with `clm pr open spec/<id> --title <title> --body-file <file>`, whose body has `## Intent`, `## Decisions` (each new or changed decision, by name, with its title), `## Open questions` and `## CLM defects`, and says whether the branch needs the reviewer's `clm approve` or lands on checks. The builder check advises and never gates. Beside the builder check, in the same message, `propose` launches the `architecture-auditor` subagent on the session's own model (`architecture-audit`, `spec-architecture-audit`): the builder check asks whether the change can be built, the audit whether its design keeps the constraints, which nothing else reads a spec change against. `propose` fixes in the change each finding it agrees with and writes each one it leaves under `## Open questions` in `change.md`, with why, which the pull request body and the approval ask already carry to the human; the audit, too, advises and never gates (`deterministic-enforcement`).

**Code fixes** (`propose-code-fixes`, `code-fix-resolves-bug`). A bug's fix changes no spec when nothing under `specs/`, `changes/`, `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md` or `CLAUDE.md` changes, such as a wrong test or code that falls short of requirements it already has. `propose` offers the human a code branch instead of a spec branch, and on a no carries on with the spec branch. On a yes, it deletes the `spec/<id>` branch it pushed, locally and on `origin`, with `autosync/spec/<id>`, so no change names the item, and claims the component whose code the fix changes (`orchestrator/claim-fix`). A test file counts toward the components its `req:` markers cite (`fixes-count-where-cited`), so a wrong test of `plugin` in the root's `crates/clm/tests/plugin.rs` is a fix of `plugin`; when the files belong to several of the bug's components it claims any one of them, since one claim carries the fix. The trailer block is the message's last paragraph, beside `Co-Authored-By`; it lets the gate pass the item's deletion in a code land (`gate/backlog-additions`). The branch runs the repository's checks and reviews as `CLAUDE.md` asks, with `clm check` passing, and lands with no review to ask for. Its summary says the fix landed and nothing is left to build. A fix whose changed code files don't all belong to one component the bug names, by `paths` or by markers, or that needs a spec change (`code-fix-needs-a-written-rule`), stays a spec branch; so does one whose component's claim is refused.

**Landing** (`land-locally`). `propose` lands by what `clm gate` printed. `spec`: run `clm land spec/<id>` at once; on `gate/up-to-date` or a conflict, run `clm rebase`, resolve, push with `--force-with-lease` and land again; on a check failure, fix, push and land again. `spec review`: show the human the pull request's link and the change's `## Intent`, `human_burdens`, `## Decisions` and `## Open questions` in the conversation, and ask whether they approve its intent (`approve-in-session`). On a yes run `clm approve spec/<id>`, which the guard has them confirm, then `clm land spec/<id>` as above; on a no discuss what to change, edit the change, re-apply it with `clm rebase`, push, and ask again. When the branch has landed it runs `clm wrap-up` (`checkpoint-before-done`) and ends with a summary: what landed, what is open and who it waits on, backlog items added, and, once the checkpoint exits 0, a line that everything is saved, the session can be ended or cleared, and `/clm:build` in a new session builds the change. When the checkpoint can't pass for a reason the session can't fix, it says what and why instead of that line.

**In an isolated session** (`session-worktrees`). `propose` there cuts `spec/<id>` in the checkout it has, as `clm draft`'s prompt has a draft do (`orchestrator.triage`), and removes no worktree, since the one it is in belongs to whoever made it; once the branch has landed, `clm wrap-up` runs there and deletes it. A resumed `spec/<id>` is switched to there with `git switch spec/<id>` when no other worktree holds the branch; when one does, the session says so and stops, since it can neither enter that worktree nor check the branch out twice.

**Splits** (`method-in-the-plugin`). `propose` checks a split's cut before writing a delta (`propose-nests-splits`): `depends_on` is a DAG and `check/undeclared-imports` judges every edge, so a cut must leave imports running one way (`splits-import-one-way`), and `clm survey --components <file>` reports a `survey/undeclared-edge` or `survey/cycle` for one that doesn't.

**Tests** (`tests-count-where-cited`). The tests of this component's requirements sit in `crates/clm/tests/plugin_propose.rs`, a test file of its own beside `plugin.rs`: its build moves each test that cites one of its requirements out of `plugin.rs` into that file, with copies of the helpers those tests use and the `clm:reads` markers for the files they read (`doc-readers-marked`), so its code count holds its own tests rather than the whole of `plugin.rs`. `plugin`'s tests of the shared rules read this component's file too; a build that changes the file keeps them passing.

## Requirements

### Requirement: propose-brainstorms
`skills/propose/SKILL.md` SHALL tell Claude to follow the skill at `clm superpowers brainstorming` before writing the change, to put its approaches under `## Approaches` in `change.md`, and never to write under `docs/superpowers/`.

#### Scenario: skill without brainstorming
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `clm superpowers brainstorming`
- WHEN the tests run
- THEN a test fails naming the skill and `clm superpowers brainstorming`

#### Serves
- capability/propose-intent
- architecture/reuse-first

### Requirement: propose-claims
`skills/propose/SKILL.md` SHALL tell Claude to run `clm proposals` and leave out the items it names before suggesting or picking backlog items; to claim each backlog item the intent comes from with `clm claim backlog/<slug>` before brainstorming, and to stop and tell the human when a claim is refused; to cut the spec branch, write `change.md` with `from` or `resolves`, commit and push it right after claiming; to keep a claimed item that design finds already fixed in `resolves`, deleting its `backlog/<slug>.md` on the spec branch and naming in `## Intent` what fixed it; and, once it knows the change's components and again before opening the PR, to run `clm proposals --component` for them and ask the human what to do when it prints a `proposed` or `claimed` line. It SHALL tell Claude that a `main` line, a change already applied on `main`, needs no question. It SHALL write that command as `clm proposals --component <component> [--component <component>]...`, with `--component` once per component, as the CLI takes it, and never as `--component <component>...`, which reads as one flag followed by several components.

#### Scenario: skill without claims
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `clm claim backlog/`
- WHEN the tests run
- THEN a test fails naming the skill and `clm claim backlog/`

#### Scenario: skill without the overlap check
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `clm proposals --component`
- WHEN the tests run
- THEN a test fails naming the skill and `clm proposals --component`

#### Scenario: one flag for several nodes
- GIVEN a `skills/propose/SKILL.md` that writes `clm proposals --component <component>...`
- WHEN the tests run
- THEN a test fails naming the skill and the form `clm proposals --component <component> [--component <component>]...`

#### Scenario: skill without already-fixed items
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `already fixed`
- WHEN the tests run
- THEN a test fails naming the skill and `already fixed`

#### Scenario: asks about applied changes
- GIVEN a `skills/propose/SKILL.md` that doesn't mention a `main` line
- WHEN the tests run
- THEN a test fails naming the skill and `main` line

#### Serves
- capability/work-concurrently
- capability/keep-a-backlog
- architecture/lease-claims

### Requirement: propose-priority
`skills/propose/SKILL.md` SHALL tell Claude, when the human names no intent, to suggest only the backlog's `idea` items, highest priority first, as `clm backlog` lists them (`priority-caps`), and to point the human at `/clm:triage` for the `bug`, `question`, `human-task`, `deferred` and `workaround` items, saying so when no `idea` item is left (`triage-owns-backlog-work`); an item of any kind the human names is still taken as the intent. It SHALL also tell Claude to give a change from backlog items the highest `priority` among the items in its `from` and `resolves`, and to leave `priority` out when all of them are `P5`. When `clm check` reports `check/priority-caps`, Claude asks the human which holder to demote, never choosing one itself, since priority is the human's call.

#### Scenario: skill without priority
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `priority`
- WHEN the tests run
- THEN a test fails naming the skill and `priority`

#### Scenario: skill without the cap rule
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `check/priority-caps`
- WHEN the tests run
- THEN a test fails naming the skill and `check/priority-caps`

#### Scenario: propose suggests only ideas
- GIVEN the plugin's `skills/propose/SKILL.md`
- WHEN a test reads it
- THEN it contains `idea` and `/clm:triage`

#### Scenario: propose without the triage pointer
- GIVEN a `propose` `SKILL.md` that doesn't mention `/clm:triage`
- WHEN the test's check runs on it
- THEN it reports `/clm:triage`

#### Serves
- capability/keep-a-backlog
- capability/propose-intent

#### Needs
- model/parse-backlog

### Requirement: propose-nests-splits
`skills/propose/SKILL.md` SHALL tell Claude, when a change splits a component or carves a part out of one, to make the carved-out part a child of the component it came from (`specs/<component>/<part>/SPEC.md`, ID `<component>.<part>`), to keep the product's ID on the half that stays, to move requirements by removing them from the parent and adding them unchanged to the child, and to split only along architectural seams, never to satisfy a size budget (`slim-components`, `nested-splits`); to cut so that imports run one way, the child below the parent or what both halves share in a layer of its own below both (`splits-import-one-way`), which inside one crate or package means that a carved-out part using the crate's other modules goes into a crate or package of its own; to check the cut before writing a delta by running `clm survey --components <file>` on a component map of the component's halves, listing every file each will own, with `outside: ["**"]` for the rest of the code, and to change the cut until it exits 0, except that a `survey/undeclared-edge` line may stand when all three hold: the importing file moves into a crate or package of the child's own, so no file the child's `paths` match exists until the child is built; a requirement the change adds to the child, whose build moves the importing file, states the edge's removal, naming the importing file, the file it imports and what replaces the import (such as a value the parent passes in), and what the parent must do for it (such as passing that value) is a requirement of the parent's whose `#### Needs` names the child's; and `change.md` lists each such edge with the child's requirement ID under `## Edges the change removes`, as the record of what the child's build must remove. A `survey/cycle` line never stands. And it SHALL tell Claude to point at `clm guide shaping-the-graph`.

#### Needs
- init/guide-command
- init/survey-components

#### Scenario: skill text
- GIVEN `plugin/skills/propose/SKILL.md`
- WHEN a test reads it
- THEN it says a carved-out part becomes a child of its component, names `clm guide shaping-the-graph`, `clm survey --components`, `splits-import-one-way` and `## Edges the change removes`, and says never to split for a budget alone

#### Serves
- capability/propose-intent
- architecture/slim-components

### Requirement: propose-code-fixes
`skills/propose/SKILL.md` SHALL tell Claude that when the intent is a `kind: bug` backlog item whose fix changes no spec path, and whose code lies in one of the item's `components`, it offers the human a code branch instead of a spec branch. It SHALL say that a test file lies in the components its `req:` markers cite as well as in the component whose `paths` hold it (`fixes-count-where-cited`), so a fix to a component's test kept under another component's `paths` claims the component the test cites. On a yes, it deletes the `spec/<id>` branch it pushed, claims the component with `clm claim <component> --fix backlog/<slug>`, fixes the bug test first on that branch, deletes the item in a commit whose trailer block, beside `Co-Authored-By`, holds `Resolves: backlog/<slug>` (`code-fix-resolves-bug`), checks that `clm gate --base origin/main` prints `code`, pushes, opens the pull request with `clm pr open` and lands with `clm land`.

#### Scenario: skill without the code-fix path
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `Resolves: backlog/`
- WHEN the tests run
- THEN a test fails naming the skill and `Resolves: backlog/`

#### Scenario: skill without the fix claim
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `clm claim <component> --fix`
- WHEN the tests run
- THEN a test fails naming the skill and `clm claim <component> --fix`

#### Scenario: skill without the cited test's node
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `fixes-count-where-cited`
- WHEN the tests run
- THEN a test fails naming the skill and `fixes-count-where-cited`

#### Needs
- clm/renamed-commands

#### Serves
- capability/propose-intent
- capability/keep-a-backlog
- architecture/fold-small-defer-large

### Requirement: propose-archives-design-only
`skills/propose/SKILL.md` SHALL tell Claude that when the change's deltas add, modify and remove no requirement, so it has nothing to build (`archive-in-spec-pr`), it archives it as the last commit before opening the pull request, after the builder check: it sets the change's `status: done`, runs `clm archive <id>`, deletes each `backlog/<slug>.md` the change's `resolves` names, and runs `clm check` again, so the change lands on `main` already archived. It SHALL NOT tell Claude to resolve a conflict under `specs/` by hand on such a branch, since `clm rebase` regenerates `specs/` for an archived change as for an applied one (`sync/reapply`).

#### Scenario: skill without archiving
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `clm archive`
- WHEN the tests run
- THEN a test fails naming the skill and `clm archive`

#### Scenario: skill with archiving
- GIVEN the plugin's `skills/propose/SKILL.md`
- WHEN a test reads it
- THEN it names `clm archive`, `status: done` and `archive-in-spec-pr`, and no sentence of the paragraph that names `archive-in-spec-pr` both names `specs/` and says `by hand` (step 8's general rule for conflicted files elsewhere stays)

#### Serves
- capability/wrap-up-session
- capability/propose-intent

#### Needs
- clm/renamed-commands
- sync/reapply

### Requirement: code-fix-needs-a-written-rule
`skills/propose/SKILL.md` SHALL tell Claude that a bug takes the code-branch route (`propose-code-fixes`) only when a requirement or scenario as written already states the behaviour the fix restores, and the fix's test cites that requirement; when none does, the fix starts with a spec branch that adds the requirement or scenario, and the code fix follows on `impl/<component>` citing it, since a fix that no requirement states is code that serves nothing written, which the next rewrite, slimming or `no-dead-code` pass can remove unseen (`code-fix-needs-a-written-rule`). It SHALL say, in the same place, that an `ARCHITECTURE.md` entry is not such a requirement, since a test never cites an entry (`human-layer-proof`), so a bug whose behaviour only an architecture entry states is a bug no requirement states; and that a bug whose work is test plumbing only is the exception, never getting a requirement: a bug in the capability suite's harness alone is harness-only work (`propose-harness-only-work`), and a bug in the plumbing of a component's own test suite (its runner settings, the ports and servers its tests start, the paths they write), changing nothing any of its tests asserts, takes the same route (`small-change-no-claim`), any test it adds citing no requirement, and, when a component's Design states that plumbing and must change, the Design-change half of that route: a spec branch edits the Design with no requirement and adds a `kind: deferred` item for the code.

#### Scenario: skill without the rule
- GIVEN a `skills/propose/SKILL.md` whose code-fix section doesn't contain `code-fix-needs-a-written-rule`, or doesn't contain `as written`
- WHEN the tests run
- THEN a test fails naming the skill and the missing text

#### Scenario: skill without the harness exception
- GIVEN a `skills/propose/SKILL.md` whose code-fix section doesn't contain `harness-only`
- WHEN the tests run
- THEN a test fails naming the skill and `harness-only`

#### Scenario: skill without the test-plumbing exception
- GIVEN a `skills/propose/SKILL.md` whose code-fix section doesn't contain `own test suite`, or doesn't contain `only an architecture entry states`
- WHEN the tests run
- THEN a test fails naming the skill and the missing text

#### Serves
- capability/propose-intent
- architecture/fold-small-defer-large

### Requirement: propose-elaborates-ideas
`skills/propose/SKILL.md` SHALL tell Claude, in a section of its own outside the numbered steps, that an idea may be elaborated over several sessions in its own backlog item (`elaborate-ideas-on-main`):
- **Start.** When the human wants to elaborate an `idea` item, Claude first looks for an unlanded branch of it on `origin` with `git ls-remote origin 'refs/heads/spec/*-idea-<slug>'` and resumes that one when there is one, in a worktree of its own; otherwise it claims the item with `clm claim backlog/<slug>` and stops when the claim is refused, as for a change, and cuts `spec/<YYYYMMDD>-idea-<slug>` from `origin/main` in a worktree of its own. A new idea is cut the same way without a claim, its item written with `title`, `kind: idea` and `components` in its frontmatter.
- **Elaborate.** The branch holds no `change.md` and no delta. Claude keeps in the item's body the sections `## Intent`, `## Settled` (each choice made, with its reason), `## Approaches` (the options still open) and `## Open questions` (what the next session takes up), rewriting them as the brainstorm settles things, starting from the item's `## Open questions` when it has them, and commits and pushes as it goes, never landing at every pause.
- **Land once.** At the end of the session, or when the human asks for a checkpoint, Claude runs `clm rebase` until its last line is `rebase ok`, pushes, and lands the branch with `clm land`, landing by the first line of `clm gate --base origin/main` (`spec`, or `code` for a branch that only adds a new item, both landing on checks). On `landed <sha>` it releases the claim it took and the branch's autosync snapshot, deleting `propose/<slug>` and `autosync/spec/<YYYYMMDD>-idea-<slug>` on `origin`, each with `clm branch delete <ref>`, since nothing else deletes them while the item stays in the backlog, and then runs `clm wrap-up`. Work after a land in the same session goes on a branch cut afresh from `origin/main`. A session that runs past the claim's expiry claims the item again with `clm claim backlog/<slug>` once the claim has lapsed.
- **Propose.** Once the human agrees to the whole design, the change is proposed as any other, with `from: backlog/<slug>`, its `change.md` seeded from the item's sections and the item deleted on the spec branch.

#### Scenario: skill elaborates ideas
- GIVEN the plugin's `skills/propose/SKILL.md`
- WHEN a test reads it
- THEN it contains `## Settled`, `## Open questions`, `-idea-` and `clm wrap-up`

#### Scenario: skill without elaboration
- GIVEN a `propose` `SKILL.md` that doesn't mention `## Settled`
- WHEN the test's check runs on it
- THEN it reports `## Settled`

#### Needs
- host/branch-delete
- clm/renamed-commands

#### Serves
- capability/propose-intent
- capability/keep-a-backlog

### Requirement: propose-clears-workarounds
`skills/propose/SKILL.md` SHALL tell Claude, when writing a delta that modifies or removes a requirement, to delete each `workaround` backlog item whose `requirement` names it, or, when the workaround still applies to the new text, to re-record the item's `requirement` at the version `clm hashes` prints after `clm apply`, since the model rejects a flag at an old version; and to delete the `reconsider-<component>-<slug>` question the change answers.

#### Scenario: skill without the rule
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `workaround`
- WHEN the tests run
- THEN a test fails naming the skill and `workaround`

#### Serves
- capability/know-health
- capability/propose-intent

#### Needs
- model/parse-backlog

### Requirement: propose-in-a-fork
`skills/propose/SKILL.md` SHALL hold a section `## Contributing back from a fork`, telling Claude that when `clm contribution parent` names a parent, it asks the human once, before cutting the branch, whether this work goes back to the parent or the fork is their own project, which carries on as any other. For work going back, the section SHALL tell Claude to bring the fork's `main` up to the parent's first with `clm contribution sync`, to run neither `clm land` nor `clm approve`, to push the branch and open its pull request on the parent with `clm contribution open`, to leave "Allow edits by maintainers" on, and to end by saying the parent's maintainer reviews it and nothing more is needed unless they ask. It SHALL point at `clm guide contributions`.

#### Scenario: the skill says so
- GIVEN the plugin's `skills/propose/SKILL.md`
- WHEN a test reads it
- THEN the section `## Contributing back from a fork` holds `clm contribution parent`, `clm contribution sync`, `clm contribution open`, `Allow edits by maintainers` and `clm guide contributions`, and says not to run `clm land`

#### Scenario: a skill without it
- GIVEN a `propose` `SKILL.md` that doesn't mention `clm contribution parent`
- WHEN the check runs on it
- THEN it reports `clm contribution parent`

#### Needs
- clm/contribution-command

#### Serves
- capability/contribute-from-outside

### Requirement: propose-adds-glossary-terms
`skills/propose/SKILL.md` SHALL tell the session that when a requirement its change adds or modifies introduces a term `clm/glossary-covers-terms` covers (a top-level `clm` command, a requirement state, a component status, a change status, a backlog kind or a key of `change.md`'s frontmatter), the same spec branch adds the term's entry to the guide's glossary, `docs/guide/glossary.md`, with a definition in plain words: the glossary is a spec path, so the code branch that builds the term can't carry it, and without it that branch's land fails `clm/glossary-covers-terms`. It says so in one paragraph of the skill's design steps, without moving an existing step. A test in `crates/clm/tests/plugin.rs`, a function over the skill's text, SHALL read the skill and fail naming the missing text.

#### Scenario: the skill says so
- GIVEN the plugin's `skills/propose/SKILL.md`
- WHEN a test reads it
- THEN it holds `docs/guide/glossary.md` and `clm/glossary-covers-terms` in one paragraph, which also holds the words `command`, `status`, `kind` and `frontmatter`

#### Scenario: a skill without it
- GIVEN a `propose` `SKILL.md` that doesn't mention `docs/guide/glossary.md`
- WHEN the check runs on it
- THEN it reports `docs/guide/glossary.md`

#### Serves
- architecture/plain-terms
- capability/propose-intent

### Requirement: propose-replacing-constraints
`skills/propose/SKILL.md` SHALL tell Claude, when an intent replaces or drops one of CLM's carried constraints (`carried-constraints`), to read it with `clm guide constraints` and tell the human, before writing the entry, its tier, what CLM relies on, what a replacement must guarantee and what the project gives up; to refuse to replace a `method` constraint, saying why; and otherwise to write the project's own entry in `ARCHITECTURE.md`'s `## Constraints` with a `#### Replaces` block naming it, stating for a `replaceable` constraint how the project keeps its guarantee, with `#### Enforced by` or `#### Untested` like any entry. It SHALL say that the `architecture-auditor` judges whether the replacement keeps the guarantee, that its findings go under `## Open questions` and into the approval ask, and that the human's approval, not the audit, decides (`deterministic-enforcement`).

#### Scenario: propose without replacements
- GIVEN a `propose` `SKILL.md` that doesn't mention `#### Replaces`
- WHEN the tests run
- THEN a test fails naming the skill and `#### Replaces`

#### Needs
- init/guide-constraints-page
- model/carried-constraints

#### Serves
- architecture/carried-constraints

### Requirement: builder-check-asks-migrations
`skills/propose/SKILL.md` SHALL tell Claude, in the step that runs the builder check, to ask the subagent whether the change alters a file `clm init` writes, a settings key, or a format `clm` reads that projects set up by a published release already hold, and, when it does, to add to the change a requirement of `upgrade` for the migration: its level, its `done` check, and its `apply` or `note` (`upgrades-carry-migrations`). A `required` migration is always an `apply`.

#### Scenario: the question
- GIVEN the plugin's `skills/propose/SKILL.md`
- WHEN its builder-check step is read
- THEN it names `clm init`, the migration and `upgrade`

#### Serves
- architecture/upgrades-carry-migrations

### Requirement: builder-check-capability-tests
`skills/propose/SKILL.md` SHALL tell Claude, among the builder check's points, to ask the builder-check subagent which capability tests each requirement the change adds or modifies changes what they observe at the product's boundary, and that such work is neither a requirement of `capability`, which holds none of its own, nor a `#### Needs`, nor a backlog item: the requirement's own build updates those tests (`capability-tests-follow-rules`), so Claude names them in that requirement's text; and that a requirement under which a `CAPABILITIES.md` entry as written no longer holds edits that entry in the change instead.

#### Scenario: skill without the capability tests
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `capability-tests-follow-rules`
- WHEN the tests run
- THEN a test fails naming the skill and `capability-tests-follow-rules`

#### Serves
- capability/propose-intent
- architecture/path-ownership

### Requirement: propose-adds-settings-sections
`skills/propose/SKILL.md` SHALL tell the session that when a requirement its change adds or modifies adds a settings key, or a field or value of one, the same spec branch adds or edits the key's section in the guide's settings page, `docs/guide/settings.md`, saying what it does, what each choice trades off and where it may be set: the page is a spec path, so the code branch that builds the key can't carry it, and without the section that branch's land fails `config/settings-guide-covers-keys`. It says so in one paragraph of the skill's design steps, beside the glossary's, without moving an existing step. A test in `crates/clm/tests/plugin.rs`, a function over the skill's text, SHALL read the skill and fail naming the missing text.

#### Scenario: the skill says so
- GIVEN the plugin's `skills/propose/SKILL.md`
- WHEN a test reads it
- THEN it holds `docs/guide/settings.md` and `config/settings-guide-covers-keys` in one paragraph

#### Scenario: a skill without it
- GIVEN a `propose` `SKILL.md` that doesn't mention `docs/guide/settings.md`
- WHEN the check runs on it
- THEN it reports `docs/guide/settings.md`

#### Serves
- capability/configure-clm
- capability/propose-intent

### Requirement: propose-harness-only-work
`skills/propose/SKILL.md` SHALL tell Claude that harness-only work, which changes only the plumbing of the project's capability suite (its runner settings, the script that starts the system under test, ports, worker counts, values the tests repeat) and so changes no `CAPABILITIES.md` entry and nothing a capability test observes, is never a requirement of any component: the harness is the Design of the component that builds the capability tests, `capability` or, until a project has it, the root (`human-layer-proof`). It SHALL say that when that Design stays true as written, the work takes the small-change route (`small-change-no-claim`) with no claim, even when it touches another component's files, and that when a `kind: bug` or `kind: deferred` item names the work, the commit that deletes the item holds the trailer `Resolves: backlog/<slug>` (`code-fix-resolves-bug`), never a `clm claim --fix`, since no requirement states the harness for the fix's test to cite. It SHALL say that when the Design must change, a spec branch edits the Design with no requirement, is archived in that branch (`archive-in-spec-pr`), and adds a `kind: deferred` item whose `components` name the Design's component and whose body names the code the edit owes, since a Design edit alone builds nothing (`node-reconciliation`); and that once that branch has landed, the same session does the code on a small-change branch from `origin/main` that resolves the item with that trailer (`deferred-work-resolves-in-code`).

#### Scenario: skill without the harness route
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `harness-only`
- WHEN the tests run
- THEN a test fails naming the skill and `harness-only`

#### Serves
- capability/propose-intent
- architecture/path-ownership
