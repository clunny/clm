---
component: plugin.init
---

## ADDED Requirements
### Requirement: init-skill-marks-doc-readers
`skills/init/SKILL.md` SHALL, once `clm init` has written `scripts/clm-check.sh` and before the bootstrap commit, tell Claude to tell the human that a land whose code was already checked runs only `clm check` and the tests whose `clm:reads <glob>` markers match a file the branch changes, so every test that reads a file of the repository's own tree (not a fixture's) at `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `README.md` or a file under `specs/`, `changes/`, `backlog/` or `docs/`, or that reads the repository's own `clm.toml`, holds such a marker, usually a comment, whose glob matches each such path it reads; an unmarked one lets a spec branch that breaks it land and leaves `main` failing every later code land's check (`land-checks-stay-fast`). For a repository with code, it SHALL tell Claude to look through the existing tests for ones that read such paths and add each one's markers in the bootstrap commit, as it moves the CI steps there, never on the spec branch, whose code paths would make it `mixed`.

#### Scenario: the shipped init skill
- GIVEN the plugin's `skills/init/SKILL.md`
- WHEN a test reads it
- THEN it contains `clm:reads`, `land-checks-stay-fast`, `CLAUDE.md`, `ARCHITECTURE.md`, `specs/` and `clm.toml`, and a line naming `clm:reads` is an earlier line than the first line naming `Make the bootstrap commit`

#### Serves
- capability/adopt-clm
- architecture/land-checks-stay-fast
