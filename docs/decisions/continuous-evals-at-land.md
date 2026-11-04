---
title: "A land that changes the superpowers eval's inputs runs arm D, and fails when it fails"
date: 2026-10-27
status: superseded
superseded_by: cut-evals
---

Generalises `superpowers-upgrade-check`, which runs the `superpowers-unattended` cases only when a new superpowers release appears. Between releases, an edit to the cases or a hand-moved pin landed without them, unless someone remembered to run `run.sh`.

**When.** The check script `clm land` runs (`ci`'s `eval-at-land`) runs the eval when the land's selection names one of its inputs: a file under `plugin/evals/superpowers-unattended/`, or `crates/clm-run/superpowers.pin`. Those are the only inputs the cases have. They load superpowers alone, never the `clm` plugin, so the plugin's skills and hooks don't reach them, and their prompt is `prompts/skills.txt`, not `clm run`'s prompt; the prompt's skill list can't change without `skills.txt` changing (`orchestrator.run/eval-names-launch-skills`), so a change to it selects the eval through that file. Triggering on `plugin/skills/`, `plugin/hooks/` or all of `crates/clm-run/` as the item suggested would pay for runs that can't observe the change.

**Where.** On the machine that lands, as `superpowers-upgrade-check` decided for the upgrade: `land-locally` took checks off hosted CI, so no workflow, repository secret or API key is involved; the `claude -p` sessions use the human's own login. Without `claude` on `PATH` the step is skipped with a message, as `panel-validate-skips-without-claude` does for the panel, so a land by someone without Claude Code isn't blocked. A run with no selection (a session's own check, the release's `prepare` job) never runs it.

**What decides.** Arm D alone, judged by `upgrade-judge`, a program (`deterministic-enforcement`): the other arms are controls whose expected result is that they *don't* plan or test first, so they can't regress in a way that matters to builds. `run.sh` gains `EVAL_ARMS` so the land pays for one arm, and `clm superpowers eval <out>` runs and judges it with the tree's own pin and judge, writing no verdict and pushing nothing.

**Budget.** About $0.40 and a few minutes per land that touches those inputs, bounded by `run.sh`'s `--max-turns 60`; they change a few times a month. A pin move by `clm superpowers upgrade` pays it a second time, which also checks the merged tree. No budget setting: the trigger is the budget.

**Chance failures.** One model run can fail by chance. The check names the failed criteria and keeps the logs (`superpowers eval failed: <out>`), and landing again runs it again. Recorded runs of arm D have passed every time; a flaky rate would show up as repeated failures and be filed as a defect.

Rejected:
- **`claude plugin eval`** (Claude Code's plugin eval runner, with deterministic graders, repeated runs and a cost ceiling): the right home for cases that test the `clm` plugin itself, but the current cases test superpowers under `clm run`'s prompt, which the runner's "plugin under test" model doesn't fit, and moving them is a rewrite. Cases for the plugin's own skills and hooks, and where they run, are `backlog/plugin-skill-evals`.
- **A scheduled run with unchanged inputs**, to catch a model update that changes behaviour: nothing in the tree changes, so nothing would land or fail; it belongs with the scheduled upgrade job, and is in `backlog/plugin-skill-evals` too.
- **Every arm at land:** $1 a run for controls that decide nothing.
