---
title: "No model evals: the superpowers eval, the upgrade it judged and the plugin's skill evals are cut; the pin is moved by hand"
date: 2026-10-29
status: accepted
---

Refines `unattended-superpowers`. Part of the re-founding (`docs/the-clm-project.md`), whose capability list the human accepted with this cut.

**What goes.** The `superpowers-unattended` cases under `plugin/evals/`, `clm superpowers eval`, `clm superpowers upgrade` with its judge and `orchestrator`'s `claim_upkeep`, the check script's eval step, and the `clm` plugin's skill eval cases with their wrapper and land-time step (never built: they waited on a cost ceiling and a sandbox install only the human could give).

**What stays.** The superpowers pin, `crates/clm-run/superpowers.pin`, which builds and `propose` follow through `clm superpowers <skill>`; and `claude plugin validate` for `plugin/` and `plugin-panel/` (`plugin-validate-at-land`, `panel-validate-skips-without-claude`), which loads the plugins as Claude Code does and runs no model.

**Why.** The evals cost more than they caught. The superpowers eval failed the pinned release on its own judge whenever a build piped `cargo test` (`backlog/superpowers-eval-fails-on-a-piped-cargo-test`), which made every land that selected it, `.gitignore` edits included, fail for a reason that said nothing about the change. The skill evals spent the human's money at each plugin land, needed a sandbox install and a budget only the human could give, and grew a design of their own around a runner CLM doesn't control. A model run judged by a program is a weak signal for a skill's text; the skills are judged where they are used, in sessions, and correctness rests on CLM's checks and gates, not on the skills (`deterministic-enforcement`, as `unattended-superpowers` already says).

**Moving the pin.** A human, or a session they ask, edits the one line of `superpowers.pin` on an `impl/orchestrator.run` branch after reading the release's notes, and lands it with `clm land` like any code; `$CLM_SUPERPOWERS_DIR` tries a release first. Nothing watches for releases: superpowers releases rarely enough that a scheduled job isn't worth its upkeep (YAGNI).

**Would change if** a skill regression ships that a session's use didn't show and a deterministic check couldn't catch; then a single, cheap case for that skill is the first step, not a suite.
