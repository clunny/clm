---
component: host
---

## MODIFIED Requirements

### Requirement: git-through-layer <!-- base: 6c8b70b2290bbfe6 -->
No crate in the workspace but `clm-git`, `clm-host` and `clm-guard` SHALL run the `gh` CLI, whether through `clm_git::gh` or through `clm_git::run` or `clm_git::run_within` with `"gh"` as its program; `clm-guard`'s only `gh` call SHALL be the release guard's run lookup (`plugin.hooks.guard/release-guard`, `release-guard-runs-gh`): one `run` or `run_within` call with `"gh"` as its program, and no `clm_git::gh`.

The scan reads each file with its whitespace removed, so a call rustfmt splits across lines still matches. A *`run` call with `"gh"` as its program* is the text `run(` or `run_within(`, with or without a path before it, whose second top-level argument (after the first comma outside brackets and quotes) is the literal `"gh"`; other `"gh"` literals in `clm-guard`, such as its tokenizer's `t == "gh"`, aren't calls. Outside the exempt crates any `"gh"` literal counts, whatever surrounds it.

No crate but `clm-git` and `clm-host` SHALL run the Azure CLI: outside those two, no `src/` file SHALL hold the string literal `"az"`, whatever surrounds it, `clm-guard` included (`azure-through-az-cli`).

`clm-web`'s `gh` module is an HTTP client for GitHub's API that runs no process, so it isn't a call to the `gh` CLI and the rule doesn't reach it; `web` reaches GitHub through its GitHub App (`host-adapters`, `web-optional`).

#### Scenario: no gh elsewhere
- GIVEN the workspace
- WHEN a test reads every `src/` file of every crate but `clm-git`, `clm-host` and `clm-guard`
- THEN none calls `clm_git::gh`, by its path or by importing it, and none holds the string literal `"gh"`

#### Scenario: no az elsewhere
- GIVEN the workspace
- WHEN a test reads every `src/` file of every crate but `clm-git` and `clm-host`
- THEN none holds the string literal `"az"`

#### Scenario: the release guard's lookup only
- GIVEN the workspace
- WHEN a test reads every `src/` file of `clm-guard`
- THEN none calls `clm_git::gh`, and exactly one `run` or `run_within` call, across all of them, has the literal `"gh"` as its program

#### Scenario: a run of gh is caught
- GIVEN the source texts `clm_git::run(&dir, "gh", &args, options)` and `clm_git::run_within(&dir, "gh", &args, options, limit)`, and each split across lines as rustfmt splits it
- WHEN the matcher for other crates and the `clm-guard` call counter each read them
- THEN both report a `gh` call in each, and the counter reports none in `t == "gh"` or in `prerun_within(&dir, "gh")`

#### Serves
- architecture/host-adapters
