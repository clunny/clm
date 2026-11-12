---
component: plugin.build
---

## MODIFIED Requirements

### Requirement: build-finishes-builds <!-- base: 6b0cdcd03b3f0ac5 -->
`skills/build/SKILL.md` SHALL tell Claude to launch builds only for the components the latest `clm next` printed, which are at most the clone's free places, and to run `clm next` again after each build returns; to treat `clm run` exiting 3 as no free place, launching nothing for that component; to treat a `clm next` that prints nothing and writes a `no free build place` line to stderr (`orchestrator/next`) as no free place too, never as an empty queue, so that it neither goes on to step 5 as if the queue were drained nor tells the human nothing is left to build; to run `clm run --finish <component>` whenever a build's subagent returns, whether it landed or stopped, before retrying or launching anything else for that component, but not for a build it resumes instead (`build-resumes-sound-builds`), such as one a usage or rate limit cut off; to pass each `left <path>: ...` line that `clm run --finish` prints to the human as it stands, a worktree it couldn't remove, and go on, leaving the directory to them; and, when no place is free, whichever of the two said so, and none of its own builds is running, to tell the human the cap is reached and by how many places, naming `max_builds` and `clm.local.toml`, and that other sessions' builds or drafts on the clone may hold the places. It SHALL NOT tell Claude to run `git worktree remove` or `git worktree prune` on a build's worktree, or name `CARGO_TARGET_DIR`.

#### Scenario: build finishes builds
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `clm run --finish <component>`, `exits 3`, `no free build place`, `max_builds`, `clm.local.toml` and `left <path>`, and doesn't contain `CARGO_TARGET_DIR` or `git worktree prune`

#### Scenario: build without finishing
- GIVEN an `build` `SKILL.md` that doesn't mention `clm run --finish`
- WHEN the test's check runs on it
- THEN it reports `clm run --finish`

#### Scenario: full cap read as an empty queue
- GIVEN an `build` `SKILL.md` that doesn't mention `no free build place`
- WHEN the test's check runs on it
- THEN it reports `no free build place`

#### Needs
- orchestrator.run/run-finish
- orchestrator/next
