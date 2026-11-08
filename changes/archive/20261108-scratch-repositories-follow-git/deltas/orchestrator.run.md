---
component: orchestrator.run
---

## MODIFIED Requirements

### Requirement: superpowers-path <!-- base: 24230501bbca9b88 -->
`clm superpowers <skill>` SHALL print the absolute path of `skills/<skill>/SKILL.md` in the pinned superpowers directory builds follow: `$CLM_SUPERPOWERS_DIR` when it is set and not empty, otherwise the pinned commit's cache directory, fetched first when absent. It SHALL exit 1, naming the skill, when that file doesn't exist, and 2 with the reason when fetching the pin fails. No word SHALL be reserved: `upgrade` and `eval` are resolved as skills like any other, and `clm-run` SHALL NOT read, run or name anything under `plugin/evals/` (`cut-evals`). It SHALL fetch the pin into a repository of its own in a temporary directory, running every git command there (`init`, `fetch`, `checkout`) through `clm-git`'s `git_isolated`, so a `GIT_DIR` or `GIT_WORK_TREE` the environment sets, naming a clone, as in a git hook, never makes the clone's git directory reinitialized, fetched into or checked out in.

#### Scenario: pinned copy
- GIVEN `XDG_CACHE_HOME` holding `clm/superpowers/<pinned commit>/skills/brainstorming/SKILL.md`
- WHEN `clm superpowers brainstorming` runs
- THEN it prints that file's path and exits 0, without fetching

#### Scenario: override
- GIVEN `CLM_SUPERPOWERS_DIR` set to a directory holding `skills/brainstorming/SKILL.md`
- WHEN `clm superpowers brainstorming` runs
- THEN it prints the path under that directory

#### Scenario: unknown skill
- GIVEN the same override
- WHEN `clm superpowers no-such-skill` runs
- THEN it exits 1 naming `no-such-skill`, and prints no path

#### Scenario: no reserved words
- GIVEN the same override, holding no `skills/upgrade/` or `skills/eval/`
- WHEN `clm superpowers upgrade` and `clm superpowers eval out` run
- THEN the first exits 1 naming `upgrade` as it names any skill it can't find, and the second exits non-zero with the usage error any extra argument gives, and neither runs `claude` or `git ls-remote`

#### Scenario: no eval in the crate
- GIVEN the source of `crates/clm-run`
- WHEN the tests read it
- THEN no file under its `src/` names `plugin/evals`

#### Scenario: a clone the environment names
- GIVEN an empty `XDG_CACHE_HOME`, `GIT_ALLOW_PROTOCOL=file` so git refuses the pin's https fetch, and a work tree `w` with a commit whose git directory `g` is elsewhere, with `GIT_DIR=g` and `GIT_WORK_TREE=w` set
- WHEN `clm superpowers brainstorming` runs
- THEN it exits 2 as when fetching the pin fails, every file under `g` is byte for byte as it was, no file is added there, and `w` is unchanged

#### Serves
- capability/pin-working-method

#### Needs
- vcs/git-isolated
