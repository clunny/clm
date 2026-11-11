---
status: applied
from: backlog/project-tests-that-read-claude-md-or-specs-aren-t-told-to-carry
resolves:
  - project-tests-that-read-claude-md-or-specs-aren-t-told-to-carry
---

## Intent

Tell a project's sessions, where they write tests, that a test reading the repository's own doc-only files (`ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `README.md`, `specs/`, `changes/`, `backlog/`, `docs/`) or its own `clm.toml` carries a `clm:reads` marker for them. The build prompt (`orchestrator.run/prompt-doc-readers`) says so beside its test-first instruction, and the init skill (`plugin.init/init-skill-marks-doc-readers`) tells the human the rule and, for a repository with code, adds the markers to the existing tests that need them in the bootstrap commit.

## Problem

A spec branch edited `CLAUDE.md`, and a project test that reads `CLAUDE.md` rejected the edit. The test had no `clm:reads CLAUDE.md` marker, so the spec land ran only `clm check` on code already checked, landed, and `main` then failed every code branch's land check until a build fixed the test outside its own component. Lands work as designed: `land-checks-stay-fast` makes a test's markers what connects it to the files it reads. But the rule reaches a project's sessions only through `docs/guide/ci-and-deployment.md` and the carried constraint's text; neither the build prompt, which is where a project's tests are written, nor the init skill, which recovers an existing project's tests, says it, so a project's sessions write unmarked readers.

## Approaches

1. **State the rule where tests are written: the build prompt and the init skill.** Chosen. The two are where a project's tests come into being under CLM: a build writes every new test, and init takes in the tests a project already has. Each gets one requirement, as `tests-run-concurrently` and `tests-run-in-their-own-tree` already have (`prompt-setup`, `init-skill-concurrent-tests`, `init-skill-build-setup`).
2. **Detect unmarked readers in `clm check`.** Rejected (`yagni`, `doc-readers-marked`'s rejected "finding the readers automatically"): which files a test opens isn't knowable from its text without the language and the runner, which `check` doesn't know.
3. **Treat a doc-only path a test names as a reader without a marker.** Rejected for the same reason: a test may read a path it builds at run time, or through a command.

## Outcome

- `orchestrator.run/prompt-doc-readers` (added): the build prompt's test-first paragraph names the doc-only paths and `clm.toml`, says a test reading them holds a `clm:reads <glob>` marker, and why. Code: a sentence in `crates/clm-run/src/lib.rs`'s prompt text, built so the marker literal and a space never appear whole; a test in the component's tests reading the fresh and resuming prompts.
- `plugin.init/init-skill-marks-doc-readers` (added): `plugin/skills/init/SKILL.md` step 3 or 4 says the rule to the human and, for a repository with code, has Claude mark the existing readers in the bootstrap commit. Code: the skill's text and a test in `crates/clm/tests/plugin_init.rs`.

## Constraints

- No edge cases or workarounds are known.
- `land-checks-stay-fast`'s text is unchanged; its `#### Untested` block still holds, since no program sees whether a session marked what its test reads.
- The test of each requirement reads only the prompt or the skill, never this repository's doc-only files, so it needs no `clm:reads` marker of its own.

## Open questions

- The draft prompt (`orchestrator.triage`) and `/clm:propose`'s code-fix path also write tests, for code-only bug fixes. Left unchanged (`yagni`): the item names the build prompt and the init skill, and those fixes are few and small. Say so if they should carry the sentence too.
