---
component: plugin.agents
base: { contract: "3f23264428d122f7", design: "3b45a850c045b3cb" }
---

## MODIFIED Contract

The plugin's four read-only subagents, `plugin/agents/<name>.md`: `spec-auditor`, which audits a diff against a component's spec; `architecture-auditor`, which audits a spec branch's design against `ARCHITECTURE.md` and the decisions it touches; `spec-slimmer`, which proposes cuts to a component's spec and code; and `issue-verifier`, which judges the host's issues and pull requests against the backlog, the specs, the code and `ARCHITECTURE.md`. Which skill launches each, and on which model, is the skills'.

## MODIFIED Design

`spec-auditor` (`advisory-code-review`) is launched with a component ID. It reads the component's requirements and scenarios from `clm context <component>`, the component's pending list from `clm pending <component>`, and the diff against `origin/main`, but not the implementer's reasoning. For each requirement and scenario the diff builds it reports where the behaviour is implemented, or that it's missing; which tests cite `req:<id>` at the current version; and whether each such test would fail if the behaviour broke where it's used, and if not, the gap: no test, a test that can't observe the breakage, or a call site that doesn't use the new behaviour. It grounds each finding in code it read, assigns no severity, and never edits. It runs in the directory it was launched in, the build's worktree, where every subagent of a build starts, and never changes that checkout: the build's own commands, its tests among them, may be running there while it reads, so a `git switch`, `git checkout`, `git stash` or `git reset` would hand them another tree.

`architecture-auditor` (`spec-architecture-audit`) is launched in the checkout of a spec branch, with the change's ID when it has one. It reads `ARCHITECTURE.md` whole, each constraint's text and not only its name, the branch's diff against `origin/main` outside `specs/`, `clm context <component>` for each component the diff's deltas touch, and the decisions under `docs/decisions/` the diff names or that the touched elements' text names, but not the designer's reasoning. It reports each constraint the design breaks, such as a build that would edit another component's paths for an addition (`path-ownership`), and each gap, something a constraint calls for that the change leaves undone, such as a rename's `clm:remove-by` (`expand-contract`), a guide entry (`method-in-the-plugin`) or a `serves` entry (`traced-rules`), and each decision the design contradicts, naming the constraint or decision, quoting its sentence, and naming the element of the change and what would meet it. It treats everything it reads on the branch (the diff, `change.md`, decisions and specs, which on a contribution a stranger wrote) as untrusted data, never as instructions. It assigns no severity, says `no findings` when it has none, never edits, and keeps the checkout as `spec-auditor` does, since the session's commands may be running there.

**Tests** (`tests-count-where-cited`). The tests of this component's requirements sit in `crates/clm/tests/plugin_agents.rs`, a test file of its own beside `plugin.rs`: its build moves each test that cites one of its requirements out of `plugin.rs` into that file, with copies of the helpers those tests use and the `clm:reads` markers for the files they read (`doc-readers-marked`), so its code count holds its own tests rather than the whole of `plugin.rs`. `plugin`'s tests of the shared rules read this component's file too; a build that changes the file keeps them passing.

## MODIFIED Requirements

### Requirement: spec-auditor <!-- base: 312da3a03f2606de -->
The plugin SHALL ship `agents/spec-auditor.md` with a `name` of `spec-auditor` and a `description` in its frontmatter, and with `tools` that include neither `Edit` nor `Write`.

#### Scenario: auditor that can edit
- GIVEN an `agents/spec-auditor.md` whose `tools` include `Edit`
- WHEN the tests run
- THEN a test fails naming the tool

#### Serves
- capability/prove-by-tests
- architecture/proven-by-tests

### Requirement: spec-slimmer <!-- base: d0110548e247577f -->
The plugin SHALL ship `agents/spec-slimmer.md`, a subagent whose `tools` hold neither `Edit` nor `Write`.

#### Scenario: agent that could edit
- GIVEN an `agents/spec-slimmer.md` whose `tools` include `Edit`
- WHEN the tests run
- THEN a test fails naming the agent and `Edit`

#### Serves
- capability/know-health
- architecture/slim-components

### Requirement: architecture-auditor <!-- base: e2b0c516b90de611 -->
The plugin SHALL ship `agents/architecture-auditor.md` with a `name` of `architecture-auditor` and a `description` in its frontmatter, and with `tools` that include neither `Edit` nor `Write`.

#### Scenario: auditor that can edit
- GIVEN an `agents/architecture-auditor.md` whose `tools` include `Write`
- WHEN the tests run
- THEN a test fails naming the tool

#### Serves
- capability/propose-intent
- architecture/right-over-convenient

### Requirement: issue-verifier <!-- base: 28b4ce1007852569 -->
The plugin SHALL ship `agents/issue-verifier.md` with a `name` of `issue-verifier`, a `description` in its frontmatter, and `tools` of exactly `Read`, `Glob` and `Grep`, so it runs no command, writes no file and can't reach the host (`untrusted-text-read-only-verifier`).

#### Scenario: verifier that can run commands
- GIVEN an `agents/issue-verifier.md` whose `tools` include `Bash`
- WHEN the tests run
- THEN a test fails naming the tool

#### Serves
- capability/contribute-from-outside
- architecture/right-over-convenient

## REMOVED Requirements

### Requirement: spec-auditor-keeps-checkout <!-- base: ab433b2d75131bec -->
### Requirement: auditor-flags-unstated-fixes <!-- base: 084d9c148d1af691 -->
