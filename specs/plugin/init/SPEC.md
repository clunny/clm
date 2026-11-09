---
title: Init skill
status: unbuilt
owners: ["@clunny"]
depends_on: []
paths: ["plugin/skills/init/**"]
successor: ""
serves: [architecture/method-in-the-plugin]
---

## Contract
The skill `init`, `plugin/skills/init/SKILL.md`, which brings a repository under CLM (`init-cli-and-skill`): a short onboarding conversation, `clm init`, an explanation of graph shape before the first component map (`first-graph-teaches-shape`), the human's bootstrap push, then the first spec graph as a spec branch that needs review, drafted from the conversation for a new project or recovered from the code with `clm survey` for an existing one (`brownfield-survey`), carried on through `propose`'s landing steps. The rules it shares with other skills are `plugin`'s; `plugin`'s Contract says what each asks of this file.

## Design
`init` gives the first explanation of graph shape before it drafts the component map, so the human learns why a component is sized to one build session, and drafts the map by those rules (`method-in-the-plugin`, `first-graph-teaches-shape`). Nothing starts it in an isolated worktree, since it begins in a repository with no specs, so it has no isolated-session case (`session-worktrees`).

**Tests** (`tests-count-where-cited`). The tests of this component's requirements sit in `crates/clm/tests/plugin_init.rs`, a test file of its own beside `plugin.rs`: its build moves each test that cites one of its requirements out of `plugin.rs` into that file, with copies of the helpers those tests use and the `clm:reads` markers for the files they read (`doc-readers-marked`), so its code count holds its own tests rather than the whole of `plugin.rs`. `plugin`'s tests of the shared rules read this component's file too; a build that changes the file keeps them passing.

## Requirements

### Requirement: init-skill
The plugin SHALL ship `skills/init/SKILL.md` with a `description` in its frontmatter. It SHALL tell Claude to:
- when `specs/SPEC.md` already exists, say the repository is already under CLM and suggest `propose`, without running `clm init`;
- run a short onboarding conversation, following the skill `clm superpowers brainstorming` prints, asking what the project is about, what a person can do with it (its first capabilities), what must always hold (its architectural constraints), and the language and stack;
- run `clm init`, with `--no-detectors` when the repository already has code, naming what it writes (`clm.toml`, `scripts/clm-check.sh` and, where missing, `ARCHITECTURE.md`, `CAPABILITIES.md`, `specs/SPEC.md` and `CLAUDE.md`), and stop for the human to push the bootstrap commit to `main` themselves, showing them `clm init`'s `Next steps (yours):` lines; once they say it's done, and when the repository has an `origin`, `git fetch origin` and check that `origin/main` has `clm.toml` or `scripts/clm-check.sh`; with no `origin` there is nothing to check;
- for a repository with code, before drafting the component map, ask the human whether to adopt all of the code now or one part first (`incremental-adoption`), saying the trade-off: one part makes a smaller first review and a change that finishes once that part's tests cite its requirements, while the code left `outside` every component has no requirements, so no test proves it to CLM and nothing reports its drift until a later change gives it a component; and that each later change widens the graph by adding a component and taking its globs out of the root's `outside`;
- for a repository with code, build the component map from `clm survey`, with an entry for the root component holding the `paths` `clm init` gave it, and writing the part left for later, when the human chose one, as the map's `outside` globs, revise it until `clm survey --components` reports nothing, show it to the human and revise it until they agree, and only then fill each component with its own subagent, writing requirements that describe what the code does today and recording bugs it finds as `bug` backlog items. The root component's delta sets `outside` to the map's `outside` globs, or to `[]` when all of the code is adopted, replacing the top-level entries `clm init` wrote. The components' builds prove the requirements with versioned markers and remove the dead code in their components, and the root component's build adds the detectors to `clm.toml`;
- for a repository with code, move the build and test steps of its existing CI workflows, GitHub's under `.github/workflows/` and Azure Pipelines' (`azure-pipelines.yml`, and pipeline YAML the repository holds elsewhere, such as under `.azure-pipelines/`), into `scripts/clm-check.sh`, which `clm land` runs before `clm check`, and remove those workflows, or the build-and-test stages of a pipeline that also deploys, in the bootstrap commit, before the spec branch is cut, never on the spec branch, whose code paths would make it `mixed`; warning the human first, naming each workflow and what moves, and listing what moved under `## Intent` in the init change; deploy, release and scheduled workflows and pipelines stay (`deploy-is-downstream`); tell the human that the host's required checks or build-validation branch policies stop being the gate once `clm land` lands, that the identity running `clm land` must be allowed to push a protected `main` (on Azure DevOps, `Bypass policies when pushing`), and, for each deploy workflow or pipeline triggered by a push to `main` with no path filter, that spec-only lands (`specs/`, `changes/`, `backlog/`) trigger it too, suggesting the filter; and never change a branch policy, a protection rule or a pipeline's settings on the host, which are the human's; and, before committing, take out of the root's `outside` in `specs/SPEC.md` any glob that no longer matches a file, such as `.github/**` once its workflows are gone, so the bootstrap commit passes `clm check`;
- in the first spec branch, add the project's architectural constraints to `ARCHITECTURE.md`'s `## Constraints`, each a `### Constraint: <slug>` block, and a few capabilities to `CAPABILITIES.md`'s `## Capabilities`, adding either section when a kept file lacks it, each capability with a `#### Why` and a scenario, drafted from the conversation or the code, pointing to `clm guide spec-format` for the format, and keep the component map in the root component's Design, not in `ARCHITECTURE.md`; give each architectural constraint an `#### Enforced by` block naming the rules of the drafted components that enforce it, or an `#### Untested` block saying why no program can, or both (`human-layer-proof`); draft the component `capability`, `specs/capability/SPEC.md`, whose Design names the project's public boundary and the harness its capability tests drive, and whose `paths` hold those tests; and say in `change.md` that the `capability` component's build proves the capabilities with tests citing `req:capability/<slug>@<version>`;
- write the first spec graph as a change on a `spec/` branch and carry it on with `propose`'s steps from `clm check` to `clm land`; the change edits `ARCHITECTURE.md` and `CAPABILITIES.md`, so `clm gate` prints `spec review` and the branch needs the human's approval: run `clm approve` only after the human says yes in the conversation, and only through the guard's confirmation.

It SHALL NOT contain the text `clm.yml`, `clm-build.sh`, `spec-approved`, `spec-review`, `auto-merge`, `status check`, `regenerat` or `creates the labels`, since `clm init` writes no workflow, regenerates no file and creates no labels.

#### Scenario: skill without the human layers
- GIVEN an `init` `SKILL.md` whose sentence drafting `CAPABILITIES.md`'s `## Capabilities` doesn't name `ARCHITECTURE.md`'s `## Constraints`, `#### Why`, `clm guide spec-format`, the root component's Design, `#### Enforced by`, `#### Untested`, the component `capability` or `req:capability/`
- WHEN the plugin's tests run
- THEN they fail naming the `init` skill and the missing text

#### Scenario: skill without the survey check
- GIVEN an `init` `SKILL.md` that doesn't mention `clm survey --components`
- WHEN the tests run
- THEN a test fails naming the skill and `clm survey --components`

#### Scenario: skill without the rule
- GIVEN an `init` `SKILL.md` that doesn't mention `clm approve`
- WHEN the tests run
- THEN a test fails naming the skill and `clm approve`

#### Scenario: skill naming the removed workflow
- GIVEN an `init` `SKILL.md` that says `clm init` writes `.github/workflows/clm.yml`, or that moves workflow steps into `.github/scripts/clm-build.sh`
- WHEN the tests run
- THEN a test fails naming the skill and `clm.yml` or `clm-build.sh`

#### Scenario: bootstrap check
- GIVEN an `init` `SKILL.md` whose bootstrap step (the one saying the human `pushes this one commit`) doesn't name `scripts/clm-check.sh` and `origin/main`, though another step names `scripts/clm-check.sh`
- WHEN the tests run
- THEN a test fails naming the skill and `scripts/clm-check.sh`

#### Scenario: skill without Azure Pipelines
- GIVEN an `init` `SKILL.md` that doesn't mention `azure-pipelines.yml`, `Bypass policies when pushing` or `path filter`
- WHEN the tests run
- THEN a test fails naming the skill and the missing text

#### Scenario: skill moving workflows on the spec branch
- GIVEN an `init` `SKILL.md` that doesn't mention `bootstrap commit`
- WHEN the tests run
- THEN a test fails naming the skill and `bootstrap commit`

#### Scenario: skill without the choice to adopt a part
- GIVEN an `init` `SKILL.md` that doesn't mention `outside`, or doesn't mention `incremental-adoption`
- WHEN the tests run
- THEN a test fails naming the skill and the missing text

#### Serves
- capability/adopt-clm
- architecture/method-in-the-plugin

#### Needs
- model/reserved-ids
- init/init-root-ownership
- init/survey-components
- model/outside-globs
- model/human-layers

### Requirement: init-skill-asks-verify
`skills/init/SKILL.md` SHALL, once `clm init` has written `clm.toml`, tell Claude to ask the human whether the project has tests too slow to run on every land, and on a yes, which command runs them, where they run (`local` on the developers' machines, `pipeline` in the host's pipeline, which then records the result itself and needs the right to push `clm/ledger/*`, or `import`, where the pipeline only reports and `clm verify` reads its runs), and which triggers to use (`change-done`, `queue-drained`, `work-done`, `every`), recommending `change-done`; then to replace the commented `[verify]` section with the settings, keep those tests out of `[land] check`, and, for `pipeline` and `import`, say what the host's pipeline must run, naming `clm guide ci-and-deployment`. On a no it SHALL leave the section commented. It SHALL name `verify-after-land`.

#### Scenario: the shipped init skill
- GIVEN the plugin's `skills/init/SKILL.md`
- WHEN a test reads it
- THEN it contains `[verify]`, `local`, `pipeline`, `import`, `change-done`, `work-done`, `ci-and-deployment` and `verify-after-land`

#### Needs
- init/init-verify-comment
- verify/verify-due
- clm/verify-command

#### Serves
- capability/slow-tests-after-land

### Requirement: init-skill-concurrent-tests
`skills/init/SKILL.md` SHALL, once `clm init` has written `scripts/clm-check.sh`, tell Claude to tell the human that CLM builds several components at once on one machine, so the project's tests, and its check script with any server the script starts, must pass while other runs of them overlap: no fixed port, no fixed path outside the checkout and the system's temporary directory, and no server or database two runs share, with a port or path that must be known in advance keyed on `CLM_SLOT`, the number CLM gives each build and its checks (`tests-run-concurrently`). For a repository with code, it SHALL tell Claude to look through the check script and the tests for fixed ports, fixed paths and shared servers, and, once the component map exists, record each it finds on the first spec branch as a `bug` backlog item whose body names `tests-run-concurrently` and whose `components` name the component whose `paths` hold the fix, as it records other bugs it finds.

#### Scenario: the shipped init skill
- GIVEN the plugin's `skills/init/SKILL.md`
- WHEN a test reads it
- THEN it contains `CLM_SLOT`, `tests-run-concurrently`, `fixed port` and `shared`

#### Serves
- architecture/tests-run-concurrently
- capability/adopt-clm
