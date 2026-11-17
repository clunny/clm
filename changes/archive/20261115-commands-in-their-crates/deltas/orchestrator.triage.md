---
component: orchestrator.triage
base: { contract: "195b4c75ba832cb2" }
---

## MODIFIED Contract

`clm triage` groups the backlog's untaken items into units of design work and prints them with the proposed spec work already on `origin` and the items waiting on the human, each line with its priority and the number of requirements it would release, sorted most urgent first. `clm draft <slug>...` claims those items through `orchestrator`'s backlog claim and writes the prompt for a session that designs one change from them unattended to a file, printing a short launcher that names it (`build-prompt-file`); the session lands the change with `clm land` when it needs no review, and leaves one that needs review on its branch with no pull request. `clm draft --resume <change-id>` writes, the same way, the prompt for a session that continues a draft whose session ended before it returned, on the spec branch it pushed (`draft-resume`), for the build loop (`orchestrator.loop`). `clm draft --fold <id>...` writes, the same way, the prompt for one session that folds the changes needing review that a triage run's drafts left into one spec branch and opens one pull request for it, so the human approves them once (`backlog-triage`, `yes-no-review`). The `triage` skill gives each launcher to a background subagent of the human's session in its own worktree, as `build` gives `clm run`'s. Each draft holds one of the clone's `max_builds` places (`orchestrator`'s `build-places`), named by the change ID `clm draft` chooses for it, from when `clm draft` starts it until it lands, is finished or its claims lapse; `clm draft --finish <change>`, which the `triage` skill runs whenever a draft's subagent returns, frees it, and the skill launches drafts only while a place is free. A fold takes no place. `clm triage` prints every group whatever the places. `clm intake` matches the host's inbox to the backlog: for each open issue it says whether a backlog item takes it in, whether a landed change or fix has resolved the item that did, or whether it is new, it lists the open pull requests from forks, and it matches the host's open dependency alerts to the backlog the same way, by an item's `alerts`, so the `triage` skill can take the inbox in before it drafts (`issues-enter-through-the-backlog`). After the host's lines it lists the red `verified` results on `main` that no green one has followed, each with its culprit land and components, matched to the backlog by an item's `verify_failures`, and, after each such result no item has taken in, the path of the log its run kept when this clone holds one (`verify`'s `kept_log`), so a slow test that failed after the land reaches triage as a bug on the components it broke, with the failure's own output for the human to judge it by (`verify-after-land`). All of them fetch `origin` first and read git and the clone's places only, `clm intake` also reading the clone's kept verify logs and `host`'s list of open issues and pull requests and its open dependency alerts, a failed read of the alerts printing `alerts unavailable` rather than failing the command, and exit 0 on success, 1 when the work isn't possible (with the reason), and 2 when git fails; `clm draft` also exits 3 when the clone has no free place.

`clm-triage` offers each of `clm triage`, `clm draft` and `clm intake` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the directory it runs in, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## ADDED Requirements

### Requirement: commands-in-crate
`clm-triage` SHALL offer each of `clm triage`, `clm draft` and `clm intake` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, which the command's `--root` overrides, the directory it runs in, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. What `crates/clm` passes it today, of the settings and of other components' results, it takes as further arguments, since a crate loads only the settings of a component it depends on. Every test of `orchestrator.triage`'s requirements that sat in the top-level test files of `crates/clm/tests/` SHALL sit in `orchestrator.triage`'s own files and call `clm-triage` itself, this call where the requirement states what a command prints or how it exits: none runs the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `orchestrator.triage`'s own files
- WHEN it calls `clm-triage`'s call with the arguments of `clm triage` and two buffers
- THEN the buffers hold what `clm triage` prints to standard output and to standard error there, and the call returns the status `clm triage` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm triage` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Serves
- architecture/path-ownership
- capability/prove-by-tests
