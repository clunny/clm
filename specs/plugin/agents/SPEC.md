---
title: Subagents
status: active
owners: ["@clunny"]
depends_on: []
paths: ["plugin/agents/**", "crates/clm/tests/plugin_agents_suite/**"]
successor: ""
serves: [architecture/method-in-the-plugin]
---

## Contract
The plugin's four subagents, `plugin/agents/<name>.md`, none of which has `Edit` or `Write` among its tools: `spec-auditor`, which audits a diff against a component's spec; `architecture-auditor`, which audits a spec branch's design against `ARCHITECTURE.md` and the decisions it touches; `spec-slimmer`, which reads a component against the rule for a requirement (`requirements-state-what-the-contract-shows`) and returns what it would reword, newly state, move and cut; and `issue-verifier`, which judges the host's issues and pull requests against the backlog, the specs, the code and `ARCHITECTURE.md`, and has only `Read`, `Glob` and `Grep`. Each file's `name` is the name it is launched by, which `orchestrator.run`'s prompt (as `clm:spec-auditor`), `orchestrator.triage`'s prompts (`architecture-auditor`) and `plugin`'s skills use; its `model`, where it sets one, is the model a launch that names none runs it on. Which skill or prompt launches each, and the model it names, is the launcher's (`model-per-launch`).

## Design
Each agent carries out one decision. What it reads, reports and never does is its file's text, which is source (`prompt-text-is-source`): no requirement states it, and this Design doesn't repeat it.
- `spec-auditor` (`advisory-code-review`) reviews a build's diff against the component's requirements; a build session launches it (`AGENTS.md`), as does `clm run`'s prompt (`orchestrator.run`).
- `architecture-auditor` (`spec-architecture-audit`) reviews a spec branch's design; the `propose` and `contribution` skills launch it, as do `clm draft`'s and the fold's prompts (`orchestrator.triage`).
- `spec-slimmer` (`requirements-state-what-the-contract-shows`) is launched by the `health` skill.
- `issue-verifier` (`untrusted-text-read-only-verifier`) is launched by the `triage` skill on the issues and pull requests it saved under `.clm/intake/`.

## Requirements

### Requirement: spec-auditor
The plugin SHALL ship `agents/spec-auditor.md` with a `name` of `spec-auditor` and a non-empty `description` in its frontmatter, and with `tools`, a comma-separated list, none of whose entries, with the spaces around it trimmed, is `Edit` or `Write`.

#### Scenario: auditor that can edit
- GIVEN the plugin's `agents/spec-auditor.md` as shipped, and the same file with `, Edit` appended to its `tools`, and with `, Write` appended
- WHEN their frontmatter is read
- THEN the shipped file's `name` is `spec-auditor`, its `description` is non-empty and no entry of its `tools` is `Edit` or `Write`
- AND each of the other two files has an entry of `tools` that is the tool appended, so it doesn't meet this requirement

#### Serves
- capability/prove-by-tests
- architecture/proven-by-tests

### Requirement: spec-slimmer
The plugin SHALL ship `agents/spec-slimmer.md` with a `name` of `spec-slimmer` and a non-empty `description` in its frontmatter, and with `tools`, a comma-separated list, none of whose entries, with the spaces around it trimmed, is `Edit` or `Write`.

#### Scenario: agent that could edit
- GIVEN the plugin's `agents/spec-slimmer.md` as shipped, and the same file with `, Edit` appended to its `tools`, and with `, Write` appended
- WHEN their frontmatter is read
- THEN the shipped file's `name` is `spec-slimmer`, its `description` is non-empty and no entry of its `tools` is `Edit` or `Write`
- AND each of the other two files has an entry of `tools` that is the tool appended, so it doesn't meet this requirement

#### Serves
- capability/know-health
- architecture/slim-components

### Requirement: architecture-auditor
The plugin SHALL ship `agents/architecture-auditor.md` with a `name` of `architecture-auditor` and a non-empty `description` in its frontmatter, and with `tools`, a comma-separated list, none of whose entries, with the spaces around it trimmed, is `Edit` or `Write`.

#### Scenario: auditor that can edit
- GIVEN the plugin's `agents/architecture-auditor.md` as shipped, and the same file with `, Write` appended to its `tools`, and with `, Edit` appended
- WHEN their frontmatter is read
- THEN the shipped file's `name` is `architecture-auditor`, its `description` is non-empty and no entry of its `tools` is `Edit` or `Write`
- AND each of the other two files has an entry of `tools` that is the tool appended, so it doesn't meet this requirement

#### Serves
- capability/propose-intent
- architecture/right-over-convenient

### Requirement: issue-verifier
The plugin SHALL ship `agents/issue-verifier.md` with a `name` of `issue-verifier` and a non-empty `description` in its frontmatter, and with `tools`, a comma-separated list each of whose entries, with the spaces around it trimmed, is `Read`, `Glob` or `Grep`, and which holds each of those three, so it runs no command, writes no file and can't reach the host (`untrusted-text-read-only-verifier`).

#### Scenario: verifier that can run commands
- GIVEN the plugin's `agents/issue-verifier.md` as shipped, and the same file with `, Bash` appended to its `tools`
- WHEN their frontmatter is read
- THEN the shipped file's `name` is `issue-verifier`, its `description` is non-empty and its `tools` entries are `Read`, `Glob` and `Grep`
- AND the other file has the entry `Bash`, which is none of the three, so it doesn't meet this requirement

#### Serves
- capability/contribute-from-outside
- architecture/right-over-convenient

### Requirement: launch-models
`agents/spec-auditor.md`, `agents/architecture-auditor.md` and `agents/spec-slimmer.md` SHALL set `model: sonnet` in their frontmatter.

#### Scenario: the agents
- GIVEN the plugin's agents
- WHEN their frontmatter is read
- THEN the three agents' frontmatter has `model: sonnet`

#### Serves
- capability/right-model-per-task
