---
component: orchestrator.triage
base: { contract: "c1d17eb282500eaf" }
---

## MODIFIED Contract
`clm triage` groups the backlog's untaken items into units of design work and prints them with the proposed spec work already on `origin` and the items waiting on the human, each line with its priority and the number of requirements it would release, sorted most urgent first. `clm draft <slug>...` claims those items through `orchestrator`'s backlog claim and writes the prompt for a session that designs one change from them unattended to a file, printing a short launcher that names it (`build-prompt-file`); the session lands the change with `clm land` when it needs no review, and leaves one that needs review on its branch with no pull request. `clm draft --resume <change-id>` writes, the same way, the prompt for a session that continues a draft whose session ended before it returned, on the spec branch it pushed (`draft-resume`), for the build loop (`orchestrator.loop`). `clm draft --fold <id>...` writes, the same way, the prompt for one session that folds the changes needing review that a triage run's drafts left into one spec branch and opens one pull request for it, so the human approves them once (`backlog-triage`, `yes-no-review`). The `triage` skill gives each launcher to a background subagent of the human's session in its own worktree, as `build` gives `clm run`'s. Each draft holds one of the clone's `max_builds` places (`orchestrator`'s `build-places`), named by the change ID `clm draft` chooses for it, from when `clm draft` starts it until it lands, is finished or its claims lapse; `clm draft --finish <change>`, which the `triage` skill runs whenever a draft's subagent returns, frees it, and the skill launches drafts only while a place is free. A fold takes no place. `clm triage` prints every group whatever the places. `clm intake` matches the host's inbox to the backlog: for each open issue it says whether a backlog item takes it in, whether a landed change or fix has resolved the item that did, or whether it is new, it lists the open pull requests from forks, and it matches the host's open dependency alerts to the backlog the same way, by an item's `alerts`, so the `triage` skill can take the inbox in before it drafts (`issues-enter-through-the-backlog`). After the host's lines it lists the red `verified` results on `main` that no green one has followed, each with its culprit land and components, matched to the backlog by an item's `verify_failures`, and, after each such result no item has taken in, the path of the log its run kept when this clone holds one (`verify`'s `kept_log`), so a slow test that failed after the land reaches triage as a bug on the components it broke, with the failure's own output for the human to judge it by (`verify-after-land`). All of them fetch `origin` first and read git and the clone's places only, `clm intake` also reading the clone's kept verify logs and `host`'s list of open issues and pull requests and its open dependency alerts, a failed read of the alerts printing `alerts unavailable` rather than failing the command, and exit 0 on success, 1 when the work isn't possible (with the reason), and 2 when git fails; `clm draft` also exits 3 when the clone has no free place.

## ADDED Requirements
### Requirement: draft-resume
`clm draft --resume <change-id>` SHALL fetch `origin`, take the place `<change-id>` (reusing it when it still counts, `orchestrator/places`), and, when `origin/spec/<change-id>` or `origin/autosync/spec/<change-id>` exists, write to `.clm/prompts/draft/<change-id>.md` the prompt for a session that switches its checkout to that branch (the newer of the two when both exist) as `spec/<change-id>`, reads its `changes/<change-id>/change.md` and the items it names, and carries the draft on from where its commits leave off under the same rules as a draft `clm draft` writes, then print the launcher as `clm draft` does and `draft <change-id>` as the last line on stderr, and exit 0. When neither branch exists it SHALL free the place it took, claim nothing, print that the draft left nothing to resume, and exit 1; the items the draft claimed stay held until their claims lapse. It SHALL exit 3, taking nothing, when no place is free.

#### Scenario: a draft that pushed
- GIVEN `origin/spec/20261112-x` holding `changes/20261112-x/change.md`, whose draft's session ended without returning
- WHEN `clm draft --resume 20261112-x` runs
- THEN it exits 0, `.clm/prompts/draft/20261112-x.md` names `spec/20261112-x`, the place `20261112-x` counts, and stderr ends `draft 20261112-x`

#### Scenario: nothing pushed
- GIVEN no `spec/20261112-y` or `autosync/spec/20261112-y` on `origin`
- WHEN `clm draft --resume 20261112-y` runs
- THEN it exits 1 saying the draft left nothing to resume, and no place `20261112-y` is left

#### Serves
- capability/build-from-spec
- capability/triage-unattended
