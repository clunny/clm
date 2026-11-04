---
component: orchestrator.run
base: { contract: "f34811b59879b624" }
---

## MODIFIED Contract

`clm run <component>` claims a component through `orchestrator`'s `claim`, or picks up its unfinished build, and writes the prompt for a session that builds its pending requirements and lands them with `clm land` (`land-locally`) to `.clm/prompts/<component>.md` in the checkout it runs in. On stdout it prints a short launcher naming that file by its absolute path, which `build` gives to a background subagent of the human's session in its own Claude Code worktree (`builds-in-subagents`), and which the subagent follows by reading the file (`build-prompt-file`). It fetches `origin` first and takes `--expiry <hours>` (default 4) and `--review-model <model>`, the alias of the model the build's final review runs on, which `build` sets to its own session's model (`model-per-launch`). Each build holds one of the clone's `max_builds` places (`orchestrator`'s `build-places`) from when `clm run` starts it until it lands, is finished or its claim lapses. When it refuses a component, it says why. It exits 0 on success, 1 when the work isn't possible (with the reason), 2 when git fails, and 3 when the clone has no free place. `clm run --finish <component>`, which `build` runs whenever a build's subagent returns, frees the component's place and tidies the worktree of a landed build, printing `landed impl/<component>` when the branch has landed (`unclosed-build-reads-finish`); a build that hasn't landed keeps its claim and pushed work, so the next `clm run <component>` resumes. A build compiles into its own worktree's `target/` (`build-places`). `clm superpowers <skill>` prints the path of `skills/<skill>/SKILL.md` in the pinned superpowers copy, fetching it first if needed, so builds and interactive skills follow one version (`design-brainstorming`, `unattended-superpowers`). The pin is moved by hand, on an `impl/orchestrator.run` branch (`cut-evals`).

A component whose pending requirements name a file Claude Code protects, such as `.claude/settings.json` or an `.npmrc`, is built like any other: Claude Code asks the human to confirm each such write (`builds-in-subagents`).

## MODIFIED Requirements

### Requirement: run-finish <!-- base: df34cd4ab02473a8 -->
`clm run --finish <component>` SHALL fetch `origin`, free the place `<component>`, and, when `impl/<component>` has landed (at least one of `origin/impl/<component>` and the local `impl/<component>` exists, and the tip of each that exists is an ancestor of `origin/main`), print `landed impl/<component>` after the line for the freed place and before any line about worktrees, also when no worktree has the branch checked out, then remove each worktree under the main worktree's `.claude/worktrees/` that has `impl/<component>` checked out and no uncommitted changes or untracked files that aren't ignored, unlocking it first, printing `removed <path>`, and print `kept <path>: uncommitted changes` for one that has them. When the branch hasn't landed it SHALL print no `landed` line, remove no worktree and push or delete no branch. The `landed impl/<component>` line is how `build` learns that a build whose subagent ended without a closing line landed (`unclosed-build-reads-finish`). It SHALL never remove a worktree outside `.claude/worktrees/`, and SHALL exit 0, also when there was no place, and 2 when git fails.

#### Scenario: landed build tidied
- GIVEN a place `b`, and a clean, locked worktree at `.claude/worktrees/agent-1` with local `impl/b` checked out, whose tip is the second parent of a merge on `origin/main`, and no `origin/impl/b`
- WHEN `clm run --finish b` runs
- THEN it exits 0, the clone has no place `b`, the worktree is gone, and it printed `landed impl/b` and, after it, `removed` with its path

#### Scenario: stopped build kept
- GIVEN a place `b`, `origin/impl/b` with a commit `origin/main` lacks, and a clean worktree at `.claude/worktrees/agent-1` with `impl/b` checked out at that commit
- WHEN `clm run --finish b` runs
- THEN it exits 0, printed no line starting `landed`, the clone has no place `b`, the worktree and `origin/impl/b` are unchanged; and a following `clm run b` resumes the branch without claiming

#### Scenario: dirty worktree kept
- GIVEN the landed build of "landed build tidied" with an untracked file `notes.txt` in its worktree
- WHEN `clm run --finish b` runs
- THEN it exits 0, the worktree and `notes.txt` are still there, and it printed `landed impl/b` and `kept` with its path

#### Scenario: checkout outside the worktrees directory
- GIVEN local `impl/b`, landed, checked out in a clean linked worktree at `../other` outside `.claude/worktrees/`
- WHEN `clm run --finish b` runs
- THEN that worktree is still there

#### Scenario: nothing to finish
- GIVEN no place `b` and no branch `impl/b`
- WHEN `clm run --finish b` runs
- THEN it exits 0 and prints no line starting `landed`
