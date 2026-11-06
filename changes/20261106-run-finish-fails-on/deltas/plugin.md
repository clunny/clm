---
component: plugin
---

## MODIFIED Requirements

### Requirement: build-finishes-builds <!-- base: 0c3d6e0a5ffda0a6 -->
`skills/build/SKILL.md` SHALL tell Claude to launch builds only for the components the latest `clm next` printed, which are at most the clone's free places, and to run `clm next` again after each build returns; to treat `clm run` exiting 3 as no free place, launching nothing for that component; to run `clm run --finish <component>` whenever a build's subagent returns, whether it landed or stopped, before retrying or launching anything else for that component, but not for a build a usage or rate limit cut off, which `build-resumes-limited-builds` resumes instead; to pass each `left <path>: ...` line that `clm run --finish` prints to the human as it stands, a worktree it couldn't remove, and go on, leaving the directory to them; and, when no place is free and none of its own builds is running, to tell the human the cap is reached, naming `max_builds` and `clm.local.toml`. It SHALL NOT tell Claude to run `git worktree remove` or `git worktree prune` on a build's worktree, or name `CARGO_TARGET_DIR`.

#### Scenario: build finishes builds
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `clm run --finish <component>`, `exits 3`, `max_builds`, `clm.local.toml` and `left <path>`, and doesn't contain `CARGO_TARGET_DIR` or `git worktree prune`

#### Scenario: build without finishing
- GIVEN an `build` `SKILL.md` that doesn't mention `clm run --finish`
- WHEN the test's check runs on it
- THEN it reports `clm run --finish`

#### Needs
- orchestrator.run/run-finish

#### Serves
- capability/build-from-spec
- capability/work-concurrently
