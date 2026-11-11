---
component: orchestrator.run
---

## ADDED Requirements
### Requirement: prompt-doc-readers
The `clm run` prompt SHALL tell the session, in the paragraph that tells it to build each pending requirement test-first, that a test it writes or changes that reads a file of the repository's own tree (not a fixture's) at `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `README.md` or a file under `specs/`, `changes/`, `backlog/` or `docs/`, whether it opens the file or runs a command that reads it, or that reads the repository's own `clm.toml`, holds a `clm:reads <glob>` marker, usually a comment, whose glob matches each such path it reads. It SHALL give the reason: a land whose code was already checked runs only `clm check` and the tests whose markers match a file the branch changes (`land-checks-stay-fast`), so an unmarked reader lets a spec branch that breaks it land and leaves `main` failing every later code land's check. The text SHALL be in every prompt, fresh or resuming. The code that writes it, and the tests that read it, SHALL build the marker's text so `clm:reads` and the space after it never appear whole in their files, or `clm affected` would take those files for readers (`check.affected`'s Design).

#### Scenario: prompt
- GIVEN a ready component `b`, and once resuming a branch with unlanded commits
- WHEN `clm run b` runs
- THEN in each prompt it writes, the paragraph that contains `test-first` also contains `clm:reads`, `CLAUDE.md`, `ARCHITECTURE.md`, `specs/`, `clm.toml` and `land-checks-stay-fast`

#### Serves
- capability/build-from-spec
- architecture/land-checks-stay-fast
