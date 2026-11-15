---
component: orchestrator.run
base: { contract: "3104a7150a2f4013" }
---

## MODIFIED Contract

`clm run <component>` claims a component through `orchestrator`'s `claim_local`, which records the claim in the local branch `impl/<component>` before pushing it, or picks up its unfinished build, so a launch interrupted at any point is resumed or claimed afresh by the next `clm run <component>`, never held until its claim lapses, and writes the prompt for a session that builds its pending requirements and lands them with `clm land` (`land-locally`) to `.clm/prompts/<component>.md` in the checkout it runs in. On stdout it prints a short launcher naming that file by its absolute path, which the build loop gives to a background subagent of the human's session through the `build` skill in its own Claude Code worktree (`builds-in-subagents`), and which the subagent follows by reading the file (`build-prompt-file`). It fetches `origin` first and takes `--expiry <hours>` (default 4) and `--review-model <model>`, the alias of the model the build's final review runs on, which the build loop sets to the session's model (`model-per-launch`). Each build holds one of the clone's `max_builds` places (`orchestrator`'s `build-places`) from when `clm run` starts it until it lands, is finished or its claim lapses. When it refuses a component, it says why. It exits 0 on success, 1 when the work isn't possible (with the reason), 2 when git fails, and 3 when the clone has no free place. `clm run --finish <component>`, which the build loop runs whenever a build's subagent returns, frees the component's place and tidies the worktree of a landed build, printing `landed impl/<component>` when the branch has landed (`unclosed-build-reads-finish`); a build that hasn't landed keeps its claim and its branch, pushed or not, so the next `clm run <component>` resumes it (`resume-what-push-accepts`). A build compiles into its own worktree's `target/` (`build-places`). `clm superpowers <skill>` prints the path of `skills/<skill>/SKILL.md` in the pinned superpowers copy, fetching it first if needed, so builds and interactive skills follow one version (`design-brainstorming`, `unattended-superpowers`). The pin is moved by hand, on an `impl/orchestrator.run` branch (`cut-evals`). `clm setup` runs the project's `[build] setup` (`config`) in the CLM root of the checkout it runs in, which a build runs before its first test run so that its tests run against its own worktree's code and dependencies (`tests-run-in-their-own-tree`, `build-setup-step`); it exits 0 when the command passes or none is set, and 1 otherwise.

A component whose pending requirements name a file Claude Code protects, such as `.claude/settings.json` or an `.npmrc`, is built like any other: Claude Code asks the human to confirm each such write (`builds-in-subagents`).

`clm-run` offers each of `clm run`, `clm setup` and `clm superpowers` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## ADDED Requirements

### Requirement: commands-in-crate
`clm-run` SHALL offer each of `clm run`, `clm setup` and `clm superpowers` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. What `crates/clm` passes it today, of the settings and of other components' results, it takes as further arguments, since a crate loads only the settings of a component it depends on. Every test of `orchestrator.run`'s requirements SHALL call `clm-run` itself, this call where the requirement states what a command prints or how it exits, and SHALL sit in `orchestrator.run`'s own files: none runs the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `orchestrator.run`'s own files
- WHEN it calls `clm-run`'s call with the arguments of `clm run` and two buffers
- THEN the buffers hold what `clm run` prints to standard output and to standard error there, and the call returns the status `clm run` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm run` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on
