---
component: plugin
base: { frontmatter: "19eb2d1aaa937065", contract: "4598c35da9a61ddf" }
---

## MODIFIED Frontmatter

```yaml
paths: ["plugin/bin/**", "plugin/hooks/**", "plugin/commands/**", "plugin/.claude-plugin/**", ".claude-plugin/**", "plugin/skills/help/**", "plugin/skills/health/**", "plugin/skills/configure/**", "plugin/skills/contribution/**", "plugin/skills/verify/**"]
```

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
- skill `verify` (verify `origin/main` as it is now, triggers aside, with `clm verify --commit origin/main`; on a red result show the failing step and its last lines, re-run the failing tests when it can, and offer to take the failure in at `P1`);
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
- `renamed-commands`, `says-architectural-constraint`, `skills-say-new-words` and `skills-say-component`: every file under `plugin/` uses the current command names, terms and words.

This component's build never edits a child's file (`path-ownership`). A change to a shared rule adds a requirement to each child whose file must change, and the shared rule's `#### Needs` names them, so it waits until each child has built its part (`dependency-work-is-a-requirement`, `requirement-needs`).

The CLM panel, a second plugin `clm-panel` in this marketplace, is `plugin.panel`.

The plugin's version is the `clm` version it runs. Users update with `claude plugin update clm`, and the new version's shim downloads its binary on first use.

The skills carry CLM's method to every adopter (`method-in-the-plugin`): what they don't say themselves, they point at as a page of the guide that `clm guide <page>` prints, never as a path in this repository.

## ADDED Requirements
### Requirement: verify-skill
The plugin SHALL ship `skills/verify/SKILL.md`, with a `description` in its frontmatter saying it verifies `origin/main` as it is now, whatever `[verify]`'s triggers say, and that it applies whenever the person wants to know whether `main` is green or to run the slow tests now. It SHALL tell Claude to:
- run `clm verify --commit origin/main` and show the human its `verify` line, running it in the background or with no command time limit, since it may run the project's whole slow suite, and saying that the clone's lands wait while it runs, since it holds the clone's check lock;
- on `verify off`, say that `origin/main`'s `clm.toml` sets no `[verify]`, so the project's slow tests run in its land check, and point at `/clm:configure`; on `verify pending`, say the host's pipeline has no result for the head yet and to run the skill again later; on `verify passed`, say `main` is green at that commit;
- on `verify failed`, show the culprit, the components, the failing step and the last lines of output `clm verify` printed, and the log's path; then, only when those lines or the log show which tests failed and the project offers a way to run one test alone, re-run each such test at the failing commit, in a detached worktree of its own from that commit under `.claude/worktrees/`, removed afterwards, and say for each whether it failed again or passed; otherwise say why it re-ran nothing and guess nothing. A re-run SHALL change no record and decide nothing;
- then offer to take the failure in at `P1`, saying that taking it in is the human's decision that it is real: as a `kind: bug` item whose `verify_failures` holds the commit's full ID (`git rev-parse <sha>`), whose `components` are the `verify failed` line's, and whose body names the culprit land, its change, the failing step, the lines that show the failure, the log's path and what the re-run showed, saying that once the item is on `main` `clm land` holds code lands on those components until a fix whose commit carries `Resolves: backlog/<slug>` lands (`verify-after-land`); or, when every test it re-ran passed, as a `kind: bug` item for the flaky tests with no `verify_failures`, whose `components` hold those tests, which holds no land, saying that triage lists the red result until a later run on `main` passes;
- write an item the human takes in on a branch `verify/<YYYYMMDD>-<short sha>` cut from `origin/main` in a worktree of its own under `.claude/worktrees/`, or, in a session whose checkout already lies under `.claude/worktrees/`, with `git switch -c` in that checkout (`session-worktrees`), commit it, and land it by the first line of `clm gate --base origin/main` as `propose` lands a branch, with `clm branch push`, `clm pr open` and `clm land`; once it has landed, delete the pushed branch with `clm branch delete`, remove the worktree with `git worktree remove` from the directory it started in, or switch off the branch with `git switch --detach origin/main` in an isolated session, then run `clm wrap-up` and pass each `left <path>: ...` line it prints to the human as it stands.

It SHALL tell Claude never to push `main` by any means but `clm land`. A test in `crates/clm/tests/plugin.rs`, a function over the skill's text, SHALL read the skill and fail naming what it lacks.

#### Scenario: the skill
- GIVEN the plugin's `skills/verify/SKILL.md`
- WHEN a test reads it
- THEN it names `clm verify --commit origin/main`, `verify off`, `verify pending`, `verify failed`, `/clm:configure`, `verify_failures`, `P1`, `Resolves: backlog/<slug>`, `clm gate`, `clm land`, `clm branch delete` and `clm wrap-up`

#### Scenario: a skill that doesn't verify main
- GIVEN a `verify` `SKILL.md` that doesn't mention `clm verify --commit origin/main`
- WHEN the check runs on it
- THEN it reports `clm verify --commit origin/main`

#### Needs
- verify/verify-run
- verify/verify-commit

#### Serves
- capability/slow-tests-after-land
