---
component: plugin
---

## MODIFIED Requirements

### Requirement: init-skill <!-- base: 8f00075e4e141b16 -->
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
