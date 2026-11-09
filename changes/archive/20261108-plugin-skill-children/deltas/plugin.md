---
component: plugin
base: { frontmatter: "ae83a700e9922f0c", contract: "dcda741d126a65f2", design: "60afb45dcd0120fb" }
---

## MODIFIED Frontmatter
```yaml
paths: ["plugin/bin/**", "plugin/hooks/**", "plugin/commands/**", "plugin/.claude-plugin/**", ".claude-plugin/**", "plugin/skills/help/**", "plugin/skills/health/**", "plugin/skills/configure/**", "plugin/skills/contribution/**"]
depends_on: [check, context, orchestrator, orchestrator.run, orchestrator.checkpoint, orchestrator.triage, sync, config, view, plugin.hooks, plugin.panel, land, host, plugin.propose, plugin.triage, plugin.build, plugin.init, plugin.upgrade, plugin.agents]
```

The skills and subagents that move to children leave `paths`; `depends_on` gains the children, as `plugin` already depends on `plugin.hooks` and `plugin.panel`.

## MODIFIED Contract
The Claude Code plugin `clm`, installed from this repo's marketplace. It provides:
- the `clm` command on the Bash tool's PATH;
- `clm defect`, which reports a defect in CLM itself as `config`'s `defects` setting says: as a backlog item in CLM's own repository, or as an issue filed through the host that the human approves (`defect-reports`, `host-adapters`); its rules are `plugin.hooks`';
- hooks that call `clm hook <name>`, whose rules are `plugin.hooks`': to show the repo's state at session start, with the rule for reporting CLM defects, to have the human confirm `clm approve`, to block cutting releases and pushing to `main`, to have the human approve each defect issue, and to autosync spec work at the end of each turn;
- the skills `propose` (`plugin.propose`), `build` (`plugin.build`), `triage` (`plugin.triage`), `init` (`plugin.init`) and `upgrade` (`plugin.upgrade`), each its child's;
- skill `contribution` (take in a contributor's pull request or issue on the maintainer's side: judge its files, show it to the human, close it or land it on their yes, and credit its author; `clm guide contributions` says why);
- skill `help` (answer a person's questions about CLM, read-only, from what the installed `clm` carries: the guide, `clm guide constraints` and, in a repository under CLM, its state; it points at the skill that acts and changes nothing);
- skill `health` (run `clm health`, summarise its sections for the human, launch `spec-slimmer` on each component its bloat signals mark, and list what to take to `propose`: the reconsider candidates and the cuts; it changes nothing itself);
- skill `configure` (review CLM's settings or change one);
- the subagents `spec-auditor`, `architecture-auditor`, `spec-slimmer` and `issue-verifier`, which are `plugin.agents`'.

The rules several skills share are this component's, though the files they constrain are its children's. What each asks of the skills:
- `skills`: `propose` and `build` never push `main` by any means but `clm land`, and run `clm approve` only after the human's yes, through the guard; no skill or agent names `clm hash`, only `clm hashes`;
- `builder-check` and `architecture-audit`: `propose` runs a builder check and, beside it, the `architecture-auditor` before its pull request; `build` lists the `question` and `human-task` items that block what it leaves; `contribution` launches the auditor too;
- `session-worktrees`: `propose`, `init` and `build` cut each branch in a worktree of its own under `.claude/worktrees/`, never switching the checkout the session started in, except in a session already isolated there;
- `defect-check-before-pr`: `propose` and `build` review the session for CLM defects before any pull request, and fix the reported ones once their branches have landed;
- `recommend-the-right-design`: `triage` and `propose` read the specs and code before recommending, and recommend the correct design over the convenient one;
- `skills-write-serves` and `skills-write-enforcement`: `propose` and `init` give components `serves`, and constraints `#### Enforced by` or `#### Untested`;
- `launch-models`: every subagent a skill launches has its model named;
- `skills-push-with-clm`: no skill says `git push`; pushes and remote deletes go through `clm branch push` and `clm branch delete`;
- `unattended-permissions-check` and `skills-run-verify`: `build` and `triage` check the allow rules before their first launch, and run `clm verify --due --event work-done` before `clm wrap-up`;
- `wrap-up-left-lines`: a skill that runs `clm wrap-up` passes each `left` line to the human and never prunes a worktree itself;
- `method-pointers`: no skill or agent names `docs/guide/` but `propose`'s two pages, each `clm guide <page>` named is a page, and `init` explains graph shape before the first component map;
- `renamed-commands`, `says-architectural-constraint`, `skills-say-new-words` and `skills-say-component`: every file under `plugin/` uses today's command names, terms and words.

This component's build never edits a child's file (`path-ownership`). A change to a shared rule adds a requirement to each child whose file must change, and the shared rule's `#### Needs` names them, so it waits until each child has built its part (`dependency-work-is-a-requirement`, `requirement-needs`).

The CLM panel, a second plugin `clm-panel` in this marketplace, is `plugin.panel`.

The plugin's version is the `clm` version it runs. Users update with `claude plugin update clm`, and the new version's shim downloads its binary on first use.

The skills carry CLM's method to every adopter (`method-in-the-plugin`): what they don't say themselves, they point at as a page of the guide that `clm guide <page>` prints, never as a path in this repository.

## MODIFIED Design
The plugin is `plugin/` in this repository: `.claude-plugin/plugin.json` (`clm`, versioned with the binary), `bin/clm` (the shim), `hooks/hooks.json`, the skills and agents the Contract lists, and `commands/view.md`. The skills `propose`, `build`, `triage`, `init` and `upgrade` and the four subagents are children's, one per file (`nested-splits`); this component keeps the files no child owns and the rules the skills share. The last makes `/clm:view` exist: with `clm-panel` installed, its hooks module answers the command itself (`plugin.panel`); without it, the file has the model run `clm status` or `clm view <name>` through the Bash tool. The `clm` CLI holds every rule; the plugin only runs it (`plugin-plus-rust-cli`, `hook-logic-in-clm`). The rules behind `clm hook` and `clm defect` are `plugin.hooks`'; the panel is `plugin.panel`.

**The shim** (`binary-from-releases`). `bin/clm` resolves the binary for the plugin's version: `${XDG_CACHE_HOME:-~/.cache}/clm/<version>/clm`, downloading `clm-<target>.tar.gz` from the matching GitHub release and verifying its `.sha256` when it is missing, then runs it with the arguments it was given. `$CLM_BIN` overrides it, for developing against a local build. It is a POSIX `sh` script and uses `curl`, `tar` and `sha256sum` or `shasum`: it downloads both files with `curl -fsSL` from `https://github.com/clunny/clm/releases/download/v<version>/`, the public repository's release, so no `gh`, sign-in or token is needed (`binary-from-releases`). A missing `curl` is a failed download. The guard fails open (`guard-fails-open`): when the binary `$CLM_BIN` names is missing or can't run, or `clm hook guard` crashes, the hook exits with neither 0 nor 2, which Claude Code doesn't treat as a block, and the shim adds no wrapper that exits 2, since a session whose every `Bash` call is blocked can't rebuild the binary that would unblock it.

**Hooks** (`hooks-wired`). What each hook does with Claude Code's event is `plugin.hooks`.

**Skills.** The skills follow the repo's change flow (`CLAUDE.md`). A spec branch either needs review or lands on checks, and nothing is reviewed after it lands (`yes-no-review`). There are no labels and no `gh` calls in the skills: pull requests are opened with `clm pr open` and bodies set with `clm pr body` (`host-adapters`).

**A worktree per session branch** (`session-worktrees`). A skill that cuts a branch in the human's session never switches the checkout the session started in, which other sessions on the same clone share: one session's `git switch` would move another's `HEAD` between its commands, so that session's next commits would land on the first session's branch, and autosync would push them to its `autosync/spec/<id>`, with nothing to warn either. The worktree is inside the session's project, so the session's `cd` into it persists between Bash calls, and the `Stop` hook's autosync, which finds the repository from the event's `cwd`, snapshots the worktree's branch. The checkout the session started in stays on its branch, usually `main`. When the branch has landed, the skill changes back to that checkout, removes the worktree, then runs `clm wrap-up` there, which deletes the landed branch. A worktree whose branch hasn't landed, such as a change waiting for approval, stays with its branch for the session that picks the work up: a session resuming `spec/<id>` changes into `.claude/worktrees/<id>` when it exists, and otherwise adds it for the existing branch with `git worktree add .claude/worktrees/<id> spec/<id>` (no `-b`), tracking `origin/spec/<id>` when only `origin` has it. The session writes its scratch files, such as the pull request body, under `.clm/`, which `clm run` and `clm draft` keep out of git, so a plain `git worktree remove` succeeds; when it refuses anyway, the skill reports the path and leaves the worktree, never forcing it. Wherever the session started, on `main` or another branch of the shared checkout, or in a worktree the human added, it adds the new worktree from `origin/main` and changes back to that same directory at the end.

A session whose checkout already lies under `.claude/worktrees/` (its `git rev-parse --show-toplevel` says so), in a directory the skills don't add, is the exception. Claude Code names a subagent's worktree `agent-<id>`, while the skills name theirs by change ID, `archive` or `defects`; a human session started in one of the skills' worktrees is no more isolated than the checkout that made it, so it adds a worktree as any other session does. Claude Code isolates a subagent's session in its worktree, refusing every write and every git operation that targets another worktree, so such a session can't use a worktree it adds, even when the `git worktree add` itself goes through, and needs none, since a worktree there is one session's own and no other session's `HEAD` moves when it switches. Builds and drafts need none of this: they already run in the worktree the Agent tool's `isolation: "worktree"` gives them (`builds-in-subagents`).

**Shared steps** (`defect-check-before-pr`). Both skills resolve what `clm rebase` leaves them the same way: a `spec changed` report by reading it and `clm pending <component>` and adapting the work, conflicted files by hand, stale deltas by rebasing them on `clm hashes`, check failures by fixing them. Both skills review the session for CLM defects before opening any pull request (`defect-check-before-pr`), then, where `clm defect` writes to the backlog, **fix reported defects** once their branches have landed, before `clm wrap-up` and the summary (`defect-reports`), in a worktree of the session's own: `propose` reuses the branch's, switched with `git switch --detach origin/main`; `build` adds one from `origin/main`. The fix branch's pull request lists the defect as fixed; the landed pull request that reported it isn't edited.

**The method** (`method-in-the-plugin`, `first-graph-teaches-shape`). An adopter's project has none of this repository's `docs/`, `ARCHITECTURE.md` rationale or specs, so the skills are where it learns the method. `clm guide <page>` prints a page from the binary the shim runs, offline and at the plugin's version (`guide-in-the-binary`), so a skill names it and not `docs/guide/...`, a path only this repository has, except the two pages a spec branch edits that `method-pointers` lets `propose` name. A test per skill keeps its pointers, and a test checks that each page a skill names is a file in `docs/guide/`.

Builds use superpowers as `unattended-superpowers` decides; their prompt is `orchestrator.run`'s `clm run`.

**No eval cases** (`cut-evals`). The plugin has no `evals/` directory. The skills' tests read their text, `claude plugin validate plugin` checks that Claude Code loads the plugin (`ci/plugin-validate`), and a skill's behaviour is judged where it is used, in sessions.

## REMOVED Requirements
### Requirement: propose-brainstorms <!-- base: 3e173409f2fb73e8 -->
### Requirement: propose-claims <!-- base: 63373bf050b66f3e -->
### Requirement: propose-priority <!-- base: b217daba1806b66e -->
### Requirement: propose-nests-splits <!-- base: 7b38199827fc6078 -->
### Requirement: propose-code-fixes <!-- base: 528d2c049361aa56 -->
### Requirement: propose-archives-design-only <!-- base: 1cb1a00ec3a3ba40 -->
### Requirement: code-fix-needs-a-written-rule <!-- base: 00b46babaf7618ed -->
### Requirement: propose-elaborates-ideas <!-- base: 839ccda30b759bab -->
### Requirement: propose-clears-workarounds <!-- base: 33b6c8ba595f2295 -->
### Requirement: propose-in-a-fork <!-- base: c67f83af9dd20ce3 -->
### Requirement: propose-adds-glossary-terms <!-- base: fd6a2597f159faa3 -->
### Requirement: propose-replacing-constraints <!-- base: 44f3128f1d17056e -->
### Requirement: builder-check-asks-migrations <!-- base: 16ee864dce343d32 -->
### Requirement: builder-check-capability-tests <!-- base: 5dd84ff9b4ef4603 -->
### Requirement: propose-adds-settings-sections <!-- base: ae00a3f6e3e7dc28 -->
### Requirement: propose-harness-only-work <!-- base: a8431dc3ccb029e1 -->
### Requirement: triage-skill <!-- base: d7e0d41fc0480a6a -->
### Requirement: triage-caps-drafts <!-- base: d3a894ba1708ed2f -->
### Requirement: triage-answers <!-- base: 4cbcff5fecfb8371 -->
### Requirement: triage-menu <!-- base: e683c40d9f30e346 -->
### Requirement: triage-checks-splits <!-- base: e29c400883834afb -->
### Requirement: triage-folds-renames-alone <!-- base: 9181638e9fcb1bbb -->
### Requirement: triage-enters-branch-worktree <!-- base: fcd0a66e6848d392 -->
### Requirement: triage-waiting-drafts <!-- base: 31624f18f331912b -->
### Requirement: triage-revises-declined-fold <!-- base: b4e8e3cf51f839c6 -->
### Requirement: triage-contributions <!-- base: 7a586b6e0bd24462 -->
### Requirement: skill-named-build <!-- base: 80390f85c145aaa9 -->
### Requirement: build-subagents <!-- base: 002f1bf49843cdf8 -->
### Requirement: build-archive-branch <!-- base: 1f2645726b4ec36d -->
### Requirement: build-reads-origin <!-- base: c1f35a762f67dd91 -->
### Requirement: build-finishes-builds <!-- base: 6b0cdcd03b3f0ac5 -->
### Requirement: build-watches <!-- base: 67db2340647c7be1 -->
### Requirement: build-reports-workarounds <!-- base: 75374dd503582551 -->
### Requirement: build-waiting-builds <!-- base: b49b8327c5a033b1 -->
### Requirement: build-stops-landed-builds <!-- base: ee9902e8128a88a7 -->
### Requirement: build-says-what-is-ready <!-- base: d0ad7ac6397b1f4f -->
### Requirement: build-unclosed-builds <!-- base: 7df50a2edc28bfd6 -->
### Requirement: build-resumes-sound-builds <!-- base: d890ad937386e51a -->
### Requirement: init-skill <!-- base: 27762fbb89ad3ff0 -->
### Requirement: init-skill-asks-verify <!-- base: e80fbfc7144383e8 -->
### Requirement: init-skill-concurrent-tests <!-- base: 3def0849a908a9da -->
### Requirement: upgrade-skill <!-- base: 75544804ba6b1c11 -->
### Requirement: upgrade-skill-offers-verify <!-- base: 4e293808cf63b730 -->
### Requirement: upgrade-skill-settings-by-hand <!-- base: 9902cf03ee526114 -->
### Requirement: upgrade-skill-notes-without-edit <!-- base: 7cb403fe6a63ba3c -->
### Requirement: spec-auditor <!-- base: 312da3a03f2606de -->
### Requirement: spec-auditor-keeps-checkout <!-- base: ab433b2d75131bec -->
### Requirement: auditor-flags-unstated-fixes <!-- base: 084d9c148d1af691 -->
### Requirement: spec-slimmer <!-- base: 8de89fe379a4facd -->
### Requirement: architecture-auditor <!-- base: e2b0c516b90de611 -->
### Requirement: issue-verifier <!-- base: 28b4ce1007852569 -->
