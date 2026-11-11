---
component: plugin.init
---

## ADDED Requirements
### Requirement: init-skill-marks-lockfiles
`skills/init/SKILL.md` SHALL, after the onboarding conversation and before it runs `clm init`, tell Claude to tell the human that a lockfile counts in full toward its component's code size until `.gitattributes` marks it `linguist-generated` or `linguist-vendored` (`check/node-size`), so an unmarked one can make a component read as oversized when it isn't; and to propose, for the human to confirm, one line `<name> linguist-generated` in the repository's root `.gitattributes` for each lockfile name that needs one, the name written bare so it marks a file of that name at any depth: for a repository with code, the name of each tracked lockfile that `git check-attr linguist-generated linguist-vendored` reports neither `set` nor `true`; for a new project, the lockfiles the stack's package managers will write, such as `Cargo.lock`, `package-lock.json` or `uv.lock`. On a yes it SHALL tell Claude to write the lines before running `clm init`, so `.gitattributes` is among the files the bootstrap commit holds and `clm init`'s root component covers it (`init/init-root-ownership`); on a no, to write nothing.

#### Scenario: the shipped init skill
- GIVEN the plugin's `skills/init/SKILL.md`
- WHEN a test reads it
- THEN it contains `.gitattributes`, `linguist-generated`, `git check-attr`, `uv.lock` and `check/node-size`, and the first line naming `.gitattributes` comes before the first line naming `--no-detectors`, the step that runs `clm init`

#### Serves
- capability/adopt-clm
- architecture/slim-components
